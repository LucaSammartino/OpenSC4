// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub struct NetworkPath {
    pub key: u32,
    pub flags: u32,
    pub points: Vec<Point>,
    pub has_heights: bool,
}

pub struct RouteSet<'a> {
    pub routes: &'a [NetworkPath],
    pub chosen: Option<usize>,
    pub track_type: u32,
}

pub trait NetworkOccupant {
    fn paths(&self) -> &[NetworkPath];
    /// Foreign network-manager slot 41; this is not a view-owner algorithm.
    fn routes(&self, entry_edge: usize) -> RouteSet<'_>;
}

pub trait Network {
    fn visible_cell_rectangle(&self) -> CellRect;
    fn occupants(&self, cell: [i32; 2]) -> Vec<&dyn NetworkOccupant>;
}

pub struct SC4PathInfoView {
    pulse: u8,
    path_type: i32,
    ignore_depth: bool,
    state: Option<DrawState>,
}

impl SC4PathInfoView {
    pub fn new(path_type: i32, ignore_depth: bool) -> Self {
        Self {
            pulse: 0,
            path_type,
            ignore_depth,
            state: None,
        }
    }

    pub fn init(&mut self) {
        self.state = Some(DrawState {
            colour_write: None,
            depth_write: false,
            depth: if self.ignore_depth {
                DepthTest::Disabled
            } else {
                DepthTest::LessOrEqual
            },
            // Unpublished fields stay unknown, rather than gaining native defaults (NOV2).
            culling: None,
            blend: Blend::SourceAlpha,
            textures: 0,
            alpha_greater_than: Some(0.0),
        });
    }

    pub fn pulse(&self) -> u8 {
        self.pulse
    }
    pub fn set_path_type(&mut self, path_type: i32) {
        self.path_type = path_type;
    }
    pub fn path_type(&self) -> i32 {
        self.path_type
    }
    pub fn state(&self) -> Option<DrawState> {
        self.state
    }

    pub fn path_colour(&self, key: u32, flags: u32) -> Result<Colour, Error> {
        if flags & 2 != 0 {
            // Red-to-white waveform/component mapping is not given (NOV2).
            return Err(Error::Gap(Gap::PathInfoColourNov2));
        }
        let rgb = match (key >> 24) & 15 {
            2 => [0, 0, 255],
            4 => [255, 255, 0],
            6 | 7 => [255, 0, 255],
            // "Light green" / "light red" do not define byte values.
            _ => return Err(Error::Gap(Gap::PathInfoColourNov2)),
        };
        Ok(Colour([rgb[0], rgb[1], rgb[2], 200]))
    }

    pub fn draw_path(&self, path: &NetworkPath, terrain: &dyn CellTerrain) -> Result<Vec<Draw>, Error> {
        if path.points.is_empty() {
            return Ok(Vec::new());
        }
        if path.points.iter().any(|p| !p.finite()) {
            return Err(Error::InvalidInput);
        }
        let elevated = |p: Point| {
            Point::new(
                p.x,
                if path.has_heights {
                    p.y + 0.5
                } else {
                    terrain.ground_altitude(p.x, p.z) + 0.5
                },
                p.z,
            )
        };
        if path.key & 0xF0000000 != 0 {
            let p = elevated(path.points[0]);
            let colour = Colour([255, 0, 0, 200]);
            // Literal x/z cross, 4m on either side (NOV2 asks axes and altitude).
            let points = [
                Point::new(p.x - 4.0, p.y, p.z),
                Point::new(p.x + 4.0, p.y, p.z),
                Point::new(p.x, p.y, p.z - 4.0),
                Point::new(p.x, p.y, p.z + 4.0),
            ];
            return Ok(vec![line_draw(Primitive::Lines, &points, colour)]);
        }
        let colour = self.path_colour(path.key, path.flags)?;
        let points: Vec<_> = path.points.iter().take(25).copied().map(elevated).collect();
        let mut draws = vec![line_draw(Primitive::LineStrip, &points, colour)];
        let Some(&end) = points.last() else { return Ok(draws) };
        // Search backwards for the final non-tiny segment, keeping the actual path end.
        if let Some(pair) = points
            .windows(2)
            .rev()
            .find(|p| (p[1].x - p[0].x).hypot(p[1].z - p[0].z) > 0.001)
        {
            let d = unit_xz(pair[0], pair[1]).ok_or(Error::InvalidInput)?;
            let scale = 3.0 / 2.0_f32.sqrt();
            let left = Point::new(end.x + (-d[0] - d[1]) * scale, end.y, end.z + (-d[1] + d[0]) * scale);
            let right = Point::new(end.x + (-d[0] + d[1]) * scale, end.y, end.z + (-d[1] - d[0]) * scale);
            draws.push(line_draw(Primitive::Lines, &[left, end, end, right], colour));
        }
        if points.len() >= 2 {
            if (path.key >> 8) & 255 == 255 {
                if let Some(d) = unit_xz(points[0], points[1]) {
                    draws.push(bar(points[0], d, colour));
                }
            }
            if path.key & 255 == 255 {
                let n = points.len();
                if let Some(d) = unit_xz(points[n - 2], points[n - 1]) {
                    draws.push(bar(end, d, colour));
                }
            }
        }
        Ok(draws)
    }

    pub fn update(
        &mut self,
        context: &mut dyn DrawContext,
        network: &dyn Network,
        terrain: &dyn CellTerrain,
    ) -> Result<(), Error> {
        let state = self.state.ok_or(Error::NotInitialized)?;
        self.pulse = self.pulse.wrapping_add(5);
        let mut draws = Vec::new();
        for cell in network.visible_cell_rectangle().cells() {
            for occupant in network.occupants(cell) {
                for path in occupant.paths() {
                    if self.path_type == -1 || self.path_type == ((path.key >> 16) & 255) as i32 {
                        draws.extend(self.draw_path(path, terrain)?);
                    }
                }
            }
        }
        context.set_state(state);
        context.set_texture(None);
        for draw in &draws {
            context.draw(draw)?;
        }
        Ok(())
    }
}

fn line_draw(primitive: Primitive, points: &[Point], colour: Colour) -> Draw {
    // No guessed native format identifier (NOV2).
    Draw {
        format: None,
        primitive,
        vertices: points
            .iter()
            .map(|&position| Vertex {
                position,
                colour,
                uv: [0.0; 2],
            })
            .collect(),
        indices: Vec::new(),
    }
}

fn bar(p: Point, d: [f32; 2], colour: Colour) -> Draw {
    line_draw(
        Primitive::Lines,
        &[
            Point::new(p.x - d[1] * 3.0, p.y, p.z + d[0] * 3.0),
            Point::new(p.x + d[1] * 3.0, p.y, p.z - d[0] * 3.0),
        ],
        colour,
    )
}

pub trait TrainResourceFactory {
    fn picture(&self, group: u32, instance: u32) -> Option<Arc<dyn Texture>>;
    fn model(&self, resource_type: u32, group: u32, instance: u32) -> Option<Arc<dyn Model>>;
}

/// Resource references only. No geometry or matrices can be injected here.
pub struct TrainResources {
    pub pictures: [Arc<dyn Texture>; 6],
    pub models: [Arc<dyn Model>; 2],
}

pub struct SC4TrainJunctionView {
    directions: [[Option<[f32; 2]>; 4]; 4],
    resources: Option<TrainResources>,
}

impl Default for SC4TrainJunctionView {
    fn default() -> Self {
        Self::new()
    }
}

impl SC4TrainJunctionView {
    pub fn new() -> Self {
        Self {
            directions: [[None; 4]; 4],
            resources: None,
        }
    }

    /// Acquires exactly the documented resources. Failure drops only acquired references.
    /// This is a core helper, not a substitute for the incomplete published Init.
    pub fn load_resources(&mut self, factory: &dyn TrainResourceFactory) -> Result<(), Error> {
        let mut pictures = Vec::new();
        for instance in 0x6BE09921..=0x6BE09926 {
            pictures.push(
                factory
                    .picture(0x1ABE787D, instance)
                    .ok_or(Error::ResourceUnavailable)?,
            );
        }
        let a = factory
            .model(0x5AD0E817, 0xBADB57F1, 0x8BB70000)
            .ok_or(Error::ResourceUnavailable)?;
        let b = factory
            .model(0x5AD0E817, 0xBADB57F1, 0x8BB80000)
            .ok_or(Error::ResourceUnavailable)?;
        let pictures = pictures.try_into().map_err(|_| Error::InvalidInput)?;
        let midpoints = [
            Point::new(0.0, 0.0, 8.0),
            Point::new(8.0, 0.0, 0.0),
            Point::new(16.0, 0.0, 8.0),
            Point::new(8.0, 0.0, 16.0),
        ];
        for (i, &from) in midpoints.iter().enumerate() {
            for (j, &to) in midpoints.iter().enumerate() {
                self.directions[i][j] = unit_xz(from, to);
            }
        }
        self.resources = Some(TrainResources {
            pictures,
            models: [a, b],
        });
        Ok(())
    }

    pub fn resources(&self) -> Option<&TrainResources> {
        self.resources.as_ref()
    }
    pub fn directions(&self) -> &[[Option<[f32; 2]>; 4]; 4] {
        &self.directions
    }

    pub fn init(&mut self) -> Result<(), Error> {
        // §2 says "the draw state" without publishing it. Never return placeholder success.
        Err(Error::Gap(Gap::TrainGeometryNov3))
    }

    pub fn transparency(at_or_heading_for_junction: bool) -> f32 {
        if at_or_heading_for_junction {
            0.8
        } else {
            0.5
        }
    }

    pub fn get_path_bitmap(route: &[Point], entry_direction: [f32; 2], selected: bool) -> Result<u32, Error> {
        if route.len() < 2 || route.iter().any(|p| !p.finite()) || entry_direction.iter().any(|v| !v.is_finite()) {
            return Err(Error::InvalidInput);
        }
        let end = route[route.len() - 1];
        let start = route[route.len().saturating_sub(3)];
        // Literal "last three points" = chord from third-last to last (NOV3).
        let start = if (end.x - start.x).hypot(end.z - start.z) < 0.25 {
            route[0]
        } else {
            start
        };
        let d = unit_xz(start, end).ok_or(Error::InvalidInput)?;
        let c = d[0] * entry_direction[1] - d[1] * entry_direction[0];
        let plain = if c < -0.25 {
            0x6BE09923
        } else if c <= 0.25 {
            0x6BE09925
        } else {
            0x6BE09921
        };
        Ok(plain + u32::from(selected))
    }

    pub fn get_arrow_bitmap_quad(&self) -> Result<Draw, Error> {
        Err(Error::Gap(Gap::TrainGeometryNov3))
    }
    pub fn draw_train_junction(&self) -> Result<(), Error> {
        Err(Error::Gap(Gap::TrainGeometryNov3))
    }

    pub fn update(&self, network: &dyn Network) -> Result<(), Error> {
        // Traversal is ours, not a provider returning finished junction draw output.
        for cell in network.visible_cell_rectangle().cells() {
            for occupant in network.occupants(cell) {
                for edge in 0..4 {
                    let routes = occupant.routes(edge);
                    if !routes.routes.is_empty() {
                        self.draw_train_junction()?;
                    }
                }
            }
        }
        // Even an empty frame cannot turn an unimplemented Init/state into success.
        Err(Error::Gap(Gap::TrainGeometryNov3))
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::sync::Arc;

/// The owner cSC4PathView::PathInfo, represented outside a Rust struct namespace.
pub struct PathInfo {
    id: u32,
    priority: i32,
    flags: u32,
    points: Vec<Point>,
    texture: Option<Arc<dyn Texture>>,
    attributes: Option<(Colour, f32)>,
    sequence: u64,
}

impl PathInfo {
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn priority(&self) -> i32 {
        self.priority
    }
    pub fn flags(&self) -> u32 {
        self.flags
    }
    pub fn points(&self) -> &[Point] {
        &self.points
    }

    pub fn texture(&self) -> Option<&Arc<dyn Texture>> {
        self.texture.as_ref()
    }

    /// Explicit fixture/input data only; the native initializer and repeat-factor source
    /// are unknown (OVV1/OVV2). NewPath never fabricates these values.
    pub fn supply_attributes(&mut self, colour: Colour, repeat: f32) -> Result<(), Error> {
        if !repeat.is_finite() {
            return Err(Error::InvalidInput);
        }
        self.attributes = Some((colour, repeat));
        Ok(())
    }
}

#[allow(non_camel_case_types)]
pub struct cSC4PathView {
    paths: Vec<PathInfo>,
    thickness: f32,
    current_texture: Option<Arc<dyn Texture>>,
    next_sequence: u64,
    newest: Option<u64>,
    states: Option<[DrawState; 2]>,
}

impl cSC4PathView {
    pub fn new(thickness: f32) -> Result<Self, Error> {
        if !thickness.is_finite() || thickness <= 0.0 {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            paths: Vec::new(),
            thickness,
            current_texture: None,
            next_sequence: 0,
            newest: None,
            states: None,
        })
    }

    pub fn init(&mut self, prefer_opaque: bool) {
        let plain = DrawState {
            colour_write: Some(true),
            depth_write: false,
            depth: DepthTest::Disabled,
            culling: Some(true),
            blend: if prefer_opaque {
                Blend::Disabled
            } else {
                Blend::SourceAlpha
            },
            textures: 0,
            alpha_greater_than: None,
        };
        self.states = Some([plain, DrawState { textures: 1, ..plain }]);
    }

    pub fn states(&self) -> Option<[DrawState; 2]> {
        self.states
    }
    pub fn paths(&self) -> &[PathInfo] {
        &self.paths
    }
    pub fn thickness(&self) -> f32 {
        self.thickness
    }

    pub fn set_thickness(&mut self, thickness: f32) -> Result<(), Error> {
        if !thickness.is_finite() || thickness <= 0.0 {
            return Err(Error::InvalidInput);
        }
        self.thickness = thickness;
        Ok(())
    }

    pub fn set_path_texture(&mut self, key: Option<u32>, factory: &dyn TextureFactory) -> Result<(), Error> {
        let texture = match key {
            Some(key) => Some(factory.bind(key).ok_or(Error::ResourceUnavailable)?),
            None => None,
        };
        self.current_texture = texture;
        Ok(())
    }

    pub fn new_path(&mut self, id: u32, priority: i32, flags: u32) -> Result<&mut PathInfo, Error> {
        let sequence = self.next_sequence;
        self.next_sequence = sequence.checked_add(1).ok_or(Error::Overflow)?;
        let at = self
            .paths
            .iter()
            .position(|p| p.priority > priority)
            .unwrap_or(self.paths.len());
        self.paths.insert(
            at,
            PathInfo {
                id,
                priority,
                flags,
                points: Vec::new(),
                texture: self.current_texture.clone(),
                attributes: None,
                sequence,
            },
        );
        self.newest = Some(sequence);
        Ok(&mut self.paths[at])
    }

    pub fn add_path_vertex(&mut self, point: Point) -> Result<bool, Error> {
        if !point.finite() {
            return Err(Error::InvalidInput);
        }
        let sequence = self.newest.ok_or(Error::NoCurrentPath)?;
        let path = self
            .paths
            .iter_mut()
            .find(|p| p.sequence == sequence)
            .ok_or(Error::NoCurrentPath)?;
        if let Some(prev) = path.points.last() {
            // Literal "within" uses a strict comparison; OVV2 asks about the boundary.
            if (prev.x - point.x).powi(2) + (prev.z - point.z).powi(2) < 1.0e-5 {
                return Ok(false);
            }
        }
        path.points.push(point);
        Ok(true)
    }

    pub fn clear_paths(&mut self) {
        self.paths.clear();
        self.newest = None;
    }

    /// Unit endpoint normal in x-z (the y coordinate never affects ribbon width).
    pub fn endpoint_normal(a: Point, b: Point) -> Result<[f32; 2], Error> {
        let d = unit_xz(a, b).ok_or(Error::InvalidInput)?;
        Ok([-d[1], d[0]])
    }

    pub fn joint_normal(a: Point, b: Point, c: Point) -> Result<[f32; 2], Error> {
        let u = Self::endpoint_normal(a, b)?;
        let v = Self::endpoint_normal(b, c)?;
        let bisector = [u[0] + v[0], u[1] + v[1]];
        let length = bisector[0].hypot(bisector[1]);
        // Literal unit bisector of normals, not an invented miter correction (OVV3).
        Ok(if length < 0.0001 {
            v
        } else {
            [bisector[0] / length, bisector[1] / length]
        })
    }

    pub fn draw_arrowhead(&self) -> Result<Draw, Error> {
        // Tip/base widths, length factors, winding and UVs are absent from §1.1.
        Err(Error::Gap(Gap::ArrowheadGeometryOvv3))
    }

    pub fn draw_path(&self, path: &PathInfo) -> Result<Option<Draw>, Error> {
        let n = path.points.len();
        if n < 2 {
            return Ok(None);
        }
        let t = self.thickness;
        let start_len = path.points[0].distance(path.points[1]);
        let end_len = path.points[n - 2].distance(path.points[n - 1]);
        if (path.flags & 1 != 0 && start_len >= t) || (path.flags & 2 != 0 && end_len >= t) {
            return Err(Error::Gap(Gap::ArrowheadGeometryOvv3));
        }
        let (colour, repeat) = path.attributes.ok_or(Error::Gap(Gap::PathAttributesOvv2))?;
        let mut vertices = Vec::with_capacity(n * 2);
        let mut distance = 0.0;
        for (i, &p) in path.points.iter().enumerate() {
            if i > 0 {
                distance += path.points[i - 1].distance(p);
            }
            let normal = if i == 0 {
                Self::endpoint_normal(p, path.points[1])?
            } else if i + 1 == n {
                Self::endpoint_normal(path.points[i - 1], p)?
            } else {
                Self::joint_normal(path.points[i - 1], p, path.points[i + 1])?
            };
            for (sign, u) in [(1.0, 0.0), (-1.0, 1.0)] {
                vertices.push(Vertex {
                    position: Point::new(p.x + sign * t * normal[0], p.y, p.z + sign * t * normal[1]),
                    colour,
                    uv: [u, distance * repeat / (2.0 * t)],
                });
            }
        }
        Ok(Some(Draw {
            format: Some(10),
            primitive: Primitive::TriangleStrip,
            vertices,
            indices: Vec::new(),
        }))
    }

    pub fn update(&self, context: &mut dyn DrawContext) -> Result<(), Error> {
        if self.paths.is_empty() {
            return Ok(());
        }
        let states = self.states.ok_or(Error::NotInitialized)?;
        // Validate before touching the device; a missing body cannot look like a successful frame.
        let draws: Vec<_> = self.paths.iter().map(|p| self.draw_path(p)).collect::<Result<_, _>>()?;
        let old_offset = context.depth_offset();
        context.set_depth_offset(15.0);
        context.set_identity_model_transform();
        context.set_state(states[0]);
        let result = (|| {
            for (path, draw) in self.paths.iter().zip(&draws) {
                let Some(draw) = draw else { continue };
                if let Some(texture) = &path.texture {
                    context.set_state(states[1]);
                    context.set_texture(Some(texture));
                    context.set_repeat_wrapping();
                }
                let result = context.draw(draw);
                if path.texture.is_some() {
                    context.set_state(states[0]);
                }
                result?;
            }
            Ok(())
        })();
        context.set_depth_offset(old_offset);
        result
    }
}

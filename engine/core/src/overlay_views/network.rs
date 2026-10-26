// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Clone, Debug)]
pub struct NetworkPath {
    pub key: u32,
    pub flags: u32,
    pub has_heights: bool,
    pub points: Vec<Vec3>,
}
#[derive(Clone, Debug)]
pub struct RouteSet {
    pub keys: Vec<u32>,
    pub chosen: u32,
}
pub trait NetworkHost {
    fn manager_available(&mut self) -> bool;
    fn visible_cells(&mut self) -> (Cell, Cell);
    /// Preserve the manager's linked occupant order.
    fn occupants(&mut self, cell: Cell) -> Vec<Handle>;
    /// Preserve bucket order then chain order, without key sorting.
    fn paths(&mut self, occupant: Handle) -> Vec<NetworkPath>;
    fn altitude(&mut self, occupant: Handle, x: f32, z: f32) -> f32;
    fn routes(&mut self, occupant: Handle, entry: u8) -> RouteSet;
    fn path(&mut self, occupant: Handle, key: u32) -> Option<NetworkPath>;
    fn route_origin(&mut self, occupant: Handle, first_key: u32) -> Option<Vec3>;
    fn terrain_altitude(&mut self, x: f32, z: f32) -> f32;
}
/// Loader-zero process pulse; constructors, Init and Shutdown never reset it (NO 1.1).
#[derive(Default)]
pub struct Pulse {
    value: u8,
}
impl Pulse {
    pub fn with_observed_value(value: u8) -> Self {
        Self { value }
    }
    pub fn value(&self) -> u8 {
        self.value
    }
    pub fn advance(&mut self) -> u8 {
        let old = self.value;
        let next = if old == 255 { 0 } else { old.wrapping_add(5) };
        self.value = next;
        next
    }
}
pub struct PathInfoView {
    pub initialized: bool,
    pub ignore_depth: bool,
    pub filter: u32,
    pub state: Option<RenderState>,
}
impl Default for PathInfoView {
    fn default() -> Self {
        Self {
            initialized: false,
            ignore_depth: false,
            filter: u32::MAX,
            state: None,
        }
    }
}
impl PathInfoView {
    /// Folded byte store: an initialized view keeps its existing packed state.
    pub fn set_should_ignore_depth_buffer(&mut self, ignore: bool) {
        self.ignore_depth = ignore;
    }
    /// Models the caller's inline u32 store, not a recovered native setter entry.
    pub fn set_filter(&mut self, filter: u32) {
        self.filter = filter;
    }
    pub fn shutdown(&mut self) {
        self.initialized = false;
    }
    pub fn init(&mut self, resources: &mut impl Resources) -> bool {
        if !self.initialized {
            self.initialized = true;
            if resources.render_service_available() {
                let ignore = self.ignore_depth;
                self.state = Some(RenderState {
                    culling: false,
                    tuple: Some([
                        1,
                        0,
                        i32::from(!ignore),
                        0,
                        1,
                        0,
                        0,
                        if ignore { -1 } else { 3 },
                        4,
                        5,
                        4,
                        0,
                        1,
                        -1,
                    ]),
                    ..overlay_state(
                        if ignore { DepthTest::Off } else { DepthTest::LessEqual },
                        0,
                        Blend::SourceAlpha,
                    )
                });
            }
        }
        true
    }
    pub fn update(
        &self,
        pulse: &mut Pulse,
        network: &mut impl NetworkHost,
        renderer: &mut impl Renderer,
    ) -> Result<()> {
        let pulse = pulse.advance();
        if !network.manager_available() {
            return Ok(());
        }
        let (min, max) = network.visible_cells();
        renderer.emit(Command::Identity);
        if let Some(state) = self.state {
            renderer.emit(Command::State(state));
        }
        let mut alpha = [None; 25];
        for x in min.0..=max.0 {
            for z in min.1..=max.1 {
                for occupant in network.occupants(Cell(x, z)) {
                    for path in network.paths(occupant) {
                        if self.filter == u32::MAX || self.filter == (path.key >> 16) & 255 {
                            draw_network_path(&path, occupant, pulse, &mut alpha, network, renderer)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
impl ViewOwner for PathInfoView {
    fn deleting_cleanup(&mut self, _resources: &mut impl Resources) {}
}
pub fn path_info_colour(key: u32, flags: u32, pulse: u8) -> Colour {
    if flags & 2 != 0 {
        return Colour::rgba(255, pulse, pulse, 200);
    }
    match (key >> 24) & 15 {
        1 => Colour::rgba(128, 255, 128, 200),
        2 => Colour::rgba(0, 0, 255, 200),
        4 => Colour::rgba(255, 255, 0, 200),
        6 | 7 => Colour::rgba(255, 0, 255, 200),
        _ => Colour::rgba(255, 128, 128, 200),
    }
}
fn line(points: impl IntoIterator<Item = (Vec3, Colour)>, renderer: &mut impl Renderer) {
    renderer.emit(Command::Draw(Draw {
        primitive: 5,
        format: 1,
        vertices: points
            .into_iter()
            .map(|(position, colour)| Vertex {
                position,
                colour: Some(colour),
                uv: None,
            })
            .collect(),
        indices: Vec::new(),
    }));
}
pub fn draw_network_path(
    path: &NetworkPath,
    occupant: Handle,
    pulse: u8,
    alpha: &mut [Option<u8>; 25],
    network: &mut impl NetworkHost,
    renderer: &mut impl Renderer,
) -> Result<()> {
    let height = |p: Vec3, network: &mut dyn NetworkHost| {
        (if path.has_heights {
            p.y
        } else {
            network.altitude(occupant, p.x, p.z)
        }) + 0.5
    };
    if path.key & 0xF000_0000 != 0 {
        let p = *path.points.first().ok_or(Boundary::Undefined("empty cross path"))?;
        for [a, b] in [
            [Vec3::new(p.x - 2.0, p.y, p.z), Vec3::new(p.x + 2.0, p.y, p.z)],
            [Vec3::new(p.x, p.y, p.z - 2.0), Vec3::new(p.x, p.y, p.z + 2.0)],
        ] {
            let vertices = [a, b].map(|mut v| {
                v.y = height(v, network);
                (v, Colour::rgba(255, 0, 0, 255))
            });
            alpha[..2].fill(Some(255));
            line(vertices, renderer);
        }
        return Ok(());
    }
    if path.points.len() < 2 {
        return Err(Boundary::Undefined("regular network path needs endpoint loads"));
    }
    let colour = path_info_colour(path.key, path.flags, pulse);
    let mut strip = Vec::new();
    for (i, &point) in path.points.iter().take(25).enumerate() {
        let mut point = point;
        point.y = height(point, network);
        alpha[i] = colour.a;
        strip.push((point, colour));
    }
    line(strip, renderer);
    let last = path.points.len() - 1;
    let end = path.points[last];
    let mut neighbour = last - 1;
    loop {
        let delta = path.points[neighbour].minus(end);
        let length = delta.length();
        if length >= 0.001_f32 {
            let d = delta.times(1.0 / length);
            let angle = std::f32::consts::FRAC_PI_4;
            let (sin, cos) = angle.sin_cos();
            let wing_plus = Vec3::new(
                end.x + 3.0 * (d.x * cos - d.z * sin),
                end.y + 0.5,
                end.z + 3.0 * (d.x * sin + d.z * cos),
            );
            let wing_minus = Vec3::new(
                end.x + 3.0 * (d.x * cos + d.z * sin),
                end.y + 0.5,
                end.z + 3.0 * (-d.x * sin + d.z * cos),
            );
            let points = [wing_plus, Vec3::new(end.x, end.y + 0.5, end.z), wing_minus];
            line(
                points
                    .into_iter()
                    .enumerate()
                    .map(|(i, p)| (p, Colour { a: alpha[i], ..colour })),
                renderer,
            );
            break;
        }
        if neighbour == 0 {
            break;
        }
        neighbour -= 1;
    }
    let endpoint = if (path.key >> 8) as u8 == 255 {
        Some((path.points[0], path.points[1]))
    } else if path.key as u8 == 255 {
        Some((end, path.points[last - 1]))
    } else {
        None
    };
    if let Some((p, q)) = endpoint {
        let delta = q.minus(p);
        let offset = Vec3::new(delta.z, 0.0, -delta.x).times(3.0 / delta.length());
        line(
            [p.plus(offset), p.minus(offset)]
                .into_iter()
                .enumerate()
                .map(|(i, point)| {
                    (
                        point,
                        Colour {
                            r: 255,
                            g: 0,
                            b: 0,
                            a: alpha[i],
                        },
                    )
                }),
            renderer,
        );
    }
    Ok(())
}

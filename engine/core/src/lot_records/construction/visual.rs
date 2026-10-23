//! Construction-owned meshes and renderer protocol, lot-developer §6.7.15.
use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextureVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColourVertex {
    pub position: [f32; 3],
    pub bgra: [u8; 4],
}

#[derive(Clone, Debug, Default)]
pub struct ConstructionMeshes {
    pub sides: [Option<[TextureVertex; 4]>; 4],
    pub planks: Option<[TextureVertex; 16]>,
    pub girder: Option<[TextureVertex; 4]>,
    pub posts: Option<[ColourVertex; 80]>,
    pub crane_post: Option<[ColourVertex; 16]>,
}

pub const TEXTURE_BASES: [u32; 6] = [0x696F3000, 0x696F2000, 0x696F1000, 0x696F2000, 0x696F4000, 0x696F5000];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureBinding {
    pub class: u32,
    pub interface: u32,
    pub resource: [u32; 3],
}

pub fn texture_bindings() -> [[TextureBinding; 5]; 6] {
    TEXTURE_BASES.map(|base| {
        std::array::from_fn(|j| TextureBinding {
            class: 0x1AC0E11A,
            interface: 0xFAC0E219,
            resource: [0x7AB50E44, 0x1ABE787D, base + j as u32],
        })
    })
}

/// Init argument tuple in the draw-context §5.1 field order. -1 leaves a field alone.
pub const INITIAL_RENDER_STATE: [f32; 14] = [1., 1., 1., 0., 1., 1., 0., 3., 4., 5., -1., -1., 1., -1.];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RenderCommand {
    ViewModelTransform,
    PackedState([f32; 14]),
    ModelShade,
    Blend(bool),
    DepthOffset(i32),
    Texture { row: usize, column: usize },
    Wrap(u8, u8),
    TextureTransform(Option<[[f32; 4]; 4]>),
    Texturing(bool),
    ResetShade,
    DefaultState,
    CraneModel(bool),
}

/// Foreign draw context/lighting. Mesh topology, ordering and state changes stay owned here.
pub trait ConstructionRenderer {
    fn rotation(&self) -> usize;
    fn zoom_column(&self) -> usize;
    fn project_to_view(&mut self, world: [f32; 3]) -> [f32; 3];
    fn lighting_rgb(&mut self, normal: [f32; 3]) -> [u8; 3];
    fn command(&mut self, command: RenderCommand);
    fn textured(&mut self, vertices: &[TextureVertex], primitive: u8, format: u8);
    fn coloured(&mut self, vertices: &[ColourVertex], primitive: u8, format: u8);
}

fn vertical(b: Bounds) -> [[f32; 3]; 16] {
    let (lo, hi) = b;
    let corners = [[lo[0], lo[2]], [hi[0], lo[2]], [lo[0], hi[2]], [hi[0], hi[2]]];
    let indices = [[0, 0, 1, 1], [2, 2, 3, 3], [3, 3, 1, 1], [0, 0, 2, 2]];
    std::array::from_fn(|i| {
        let c = corners[indices[i / 4][i % 4]];
        [c[0], if i % 4 == 1 || i % 4 == 2 { hi[1] } else { lo[1] }, c[1]]
    })
}

fn horizontal(b: Bounds, scale: f32) -> [TextureVertex; 4] {
    let (lo, hi) = b;
    [
        [lo[0], hi[1], hi[2]],
        [lo[0], hi[1], lo[2]],
        [hi[0], hi[1], lo[2]],
        [hi[0], hi[1], hi[2]],
    ]
    .map(|position| TextureVertex {
        position,
        uv: [position[0] * scale, position[2] * scale],
    })
}

fn raised(vertices: &mut [ColourVertex], h: f32) {
    for (i, v) in vertices.iter_mut().enumerate() {
        if i % 20 >= 16 || i % 4 == 1 || i % 4 == 2 {
            v.position[1] = h;
        }
    }
}

impl ConstructionMeshes {
    pub fn rebuild(&mut self, geom: ConstructionGeometry, t: &ConstructionTuning, cached_plank: f32) {
        let (lo, hi) = geom.scaffold;
        let positions = vertical(geom.scaffold);
        self.sides = std::array::from_fn(|q| {
            Some(std::array::from_fn(|v| TextureVertex {
                position: positions[q * 4 + v],
                uv: [0.; 2],
            }))
        });
        let p = t.plank_width;
        let rectangles = [
            (lo, [hi[0], hi[1], lo[2] + p]),
            ([hi[0] - p, lo[1], lo[2] + p], [hi[0], hi[1], hi[2] - p]),
            ([lo[0], lo[1], hi[2] - p], hi),
            ([lo[0], lo[1], lo[2] + p], [lo[0] + p, hi[1], hi[2] - p]),
        ];
        // No code default is specified for absent texture-scale properties; NaN explicitly
        // preserves that unspecified input instead of inventing a rendering scale (LD39).
        let scale = t.plank_texture_scale.unwrap_or(f32::NAN);
        let quads = rectangles.map(|b| horizontal(b, scale));
        self.planks = Some(std::array::from_fn(|i| quads[i / 4][i % 4]));
        self.girder = Some(horizontal(
            (
                [lo[0] + cached_plank, lo[1], lo[2] + cached_plank],
                [hi[0] - cached_plank, hi[1], hi[2] - cached_plank],
            ),
            0.,
        ));
        let corners = [[lo[0], lo[2]], [hi[0], lo[2]], [lo[0], hi[2]], [hi[0], hi[2]]];
        let posts = corners.map(|[x, z]| {
            let half = f64::from(t.post_width) * 0.5;
            let bounds = (
                [(f64::from(x) - half) as f32, geom.ground, (f64::from(z) - half) as f32],
                [
                    (f64::from(x) + half) as f32,
                    (f64::from(geom.top_y) + f64::from(t.post_width)) as f32,
                    (f64::from(z) + half) as f32,
                ],
            );
            let sides = vertical(bounds);
            let top = horizontal(bounds, 0.);
            std::array::from_fn::<_, 20, _>(|i| ColourVertex {
                position: if i < 16 { sides[i] } else { top[i - 16].position },
                bgra: [0, 0, 0, 255],
            })
        });
        self.posts = Some(std::array::from_fn(|i| posts[i / 20][i % 20]));
    }

    pub fn crane_post(&mut self, x: f32, z: f32, ground: f32, half: f32) {
        let positions = vertical((
            [x - half, ground, z - half],
            [x + half, (f64::from(ground) + 0.1) as f32, z + half],
        ));
        self.crane_post = Some(positions.map(|position| ColourVertex {
            position,
            bgra: [128, 128, 128, 255],
        }));
    }

    pub fn set_height(&mut self, h: f32, post_width: f32, crane_height: Option<f32>) {
        for side in self.sides.iter_mut().flatten() {
            side[1].position[1] = h;
            side[2].position[1] = h;
        }
        for v in self.planks.iter_mut().flatten().chain(self.girder.iter_mut().flatten()) {
            v.position[1] = h;
        }
        if let Some(posts) = &mut self.posts {
            raised(posts, (f64::from(h) + f64::from(post_width)) as f32);
        }
        if let (Some(post), Some(y)) = (&mut self.crane_post, crane_height) {
            raised(post, y);
        }
    }
}

impl ConstructionOccupant {
    pub fn set_scaffold_height(&mut self, host: &mut dyn ConstructionHost, t: &ConstructionTuning, h: f32) {
        self.scaffold_height = h;
        let crane_y = self
            .rec
            .has_crane
            .then(|| (f64::from(h) + f64::from(t.crane_height_above_scaffold)) as f32);
        self.meshes.set_height(h, t.post_width, crane_y);
        if let Some(y) = crane_y {
            self.rec.crane_position[1] = y;
            host.translate_crane(self.rec.crane_position);
        }
        for prop in &self.rec.roof_props {
            let mut p = host.prop_position(prop);
            p[1] = h;
            host.move_roof_prop(prop, p, 0, 0x99EF1146, 0);
        }
        for crew in &self.rec.crew {
            if crew.second {
                let mut p = host.automaton_position(&crew.automaton);
                p[1] = h;
                host.move_automaton(&crew.automaton, p);
            }
        }
    }

    pub fn create_crane(
        &mut self,
        host: &mut dyn ConstructionHost,
        st: &mut ConstructionStatics,
        x: f32,
        z: f32,
        kind: u8,
    ) {
        let s = self.rec.scalars;
        let angle = (f64::from(s.swing_max) - f64::from(s.swing_min) * 0.5 + f64::from(s.swing_min)) as f32;
        self.rec.has_crane = host.create_crane(CraneRequest {
            resource: [0x5AD0E817, 0xBADB57F1, CRANE_MODELS[usize::from(kind)]],
            instance: 0,
            position: [x, 0., z],
            relative_y_rotation: angle,
        });
        if self.rec.has_crane {
            self.rec.crane_position = [x, 0., z];
        }
        let half = *st.crane_post_half.get_or_insert(st.tuning.crane_post_width * 0.5);
        self.meshes
            .crane_post(x, z, self.geometry.map_or(0., |g| g.ground), half);
    }

    pub fn do_rotation_change(&mut self, renderer: &mut dyn ConstructionRenderer) {
        self.model_dirty = false;
        if self.meshes.posts.is_none() && self.meshes.crane_post.is_none() {
            return;
        }
        let colours = [[0., 0., -1.], [0., 0., 1.], [1., 0., 0.], [-1., 0., 0.], [0., 1., 0.]].map(|n| {
            let [r, g, b] = renderer.lighting_rgb(n);
            [b >> 1, g >> 1, r >> 1]
        });
        if let Some(posts) = &mut self.meshes.posts {
            for (i, v) in posts.iter_mut().enumerate() {
                v.bgra[..3].copy_from_slice(&colours[(i % 20) / 4]);
            }
        }
        if let Some(post) = &mut self.meshes.crane_post {
            for (i, v) in post.iter_mut().enumerate() {
                v.bgra[..3].copy_from_slice(&colours[[0, 2, 1, 3][i / 4]]);
            }
        }
    }

    pub fn draw(&mut self, renderer: &mut dyn ConstructionRenderer, t: &ConstructionTuning, skip: bool) -> bool {
        if skip {
            return true;
        }
        if self.model_dirty {
            self.do_rotation_change(renderer);
        }
        renderer.command(RenderCommand::ViewModelTransform);
        renderer.command(RenderCommand::PackedState(INITIAL_RENDER_STATE));
        renderer.command(RenderCommand::ModelShade);
        let order = [[0, 3, 1, 2], [2, 0, 3, 1], [1, 2, 0, 3], [3, 1, 2, 0]][renderer.rotation()];
        let column = renderer.zoom_column();
        for (row, side) in order.into_iter().enumerate() {
            if let Some(v) = &self.meshes.sides[side] {
                texture_draw(
                    renderer,
                    v,
                    row,
                    column,
                    row == 2,
                    if row < 2 { 1 } else { -1 },
                    t.scaffold_texture_scale,
                );
            }
        }
        if let Some(v) = &self.meshes.girder {
            texture_draw(renderer, v, 5, column, true, -1, t.girder_texture_scale);
        }
        renderer.command(RenderCommand::TextureTransform(None));
        if let Some(v) = &self.meshes.planks {
            renderer.command(RenderCommand::Texture { row: 4, column });
            renderer.command(RenderCommand::Blend(false));
            renderer.command(RenderCommand::DepthOffset(-1));
            renderer.textured(v, 6, 2);
        }
        renderer.command(RenderCommand::Texturing(false));
        if let Some(v) = &self.meshes.posts {
            renderer.coloured(v, 6, 1);
        }
        let crane = (0..=2).contains(&self.rec.scalars.state);
        if crane {
            if let Some(v) = &self.meshes.crane_post {
                renderer.coloured(v, 6, 1);
            }
        }
        renderer.command(RenderCommand::Texturing(true));
        renderer.command(RenderCommand::ResetShade);
        renderer.command(RenderCommand::DefaultState);
        renderer.command(RenderCommand::DepthOffset(0));
        if crane && self.rec.has_crane {
            renderer.command(RenderCommand::CraneModel(false));
        }
        true
    }
}

fn texture_draw(
    renderer: &mut dyn ConstructionRenderer,
    v: &[TextureVertex],
    row: usize,
    column: usize,
    blend: bool,
    depth: i32,
    scale: Option<f32>,
) {
    renderer.command(RenderCommand::Texture { row, column });
    renderer.command(RenderCommand::Blend(blend));
    renderer.command(RenderCommand::DepthOffset(depth));
    renderer.command(RenderCommand::Wrap(3, 3));
    let s = f64::from(scale.unwrap_or(f32::NAN));
    let [x, y, z] = renderer.project_to_view(v[0].position).map(f64::from);
    renderer.command(RenderCommand::TextureTransform(Some([
        [s as f32, 0., 0., 0.],
        [0., -s as f32, 0., 0.],
        [0., 0., s as f32, 0.],
        [(-s * x) as f32, (s * y) as f32, (-s * z) as f32, 1.],
    ])));
    renderer.textured(v, 6, 2);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CraneRequest {
    pub resource: [u32; 3],
    pub instance: u32,
    pub position: [f32; 3],
    pub relative_y_rotation: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffectRequest {
    pub name: &'static str,
    pub position: [f32; 3],
    pub slot: u8,
    pub enabled: bool,
}

pub fn dust_name(bounds: Bounds) -> &'static str {
    let extent = (bounds.1[0] - bounds.0[0]).max(bounds.1[2] - bounds.0[2]);
    if extent < 10. {
        "small_dust"
    } else if extent < 20. {
        "medium_dust"
    } else {
        "large_dust"
    }
}

pub fn normalized_direction(x: f32, z: f32) -> [f32; 2] {
    let inv = (f64::from(x) * f64::from(x) + f64::from(z) * f64::from(z))
        .sqrt()
        .recip();
    [(f64::from(x) * inv) as f32, (f64::from(z) * inv) as f32]
}

/// The only intermediate stores are d and the ratios in states 1–3 (§6.7.15).
pub fn phase_value(s: ConstructionScalars, ground: f32, segment_height: f32) -> Option<(bool, f32)> {
    let d = f64::from((f64::from(s.ms_acc) * f64::from(f32::from_bits(0x3B03126F))) as f32);
    let ratio = |n: u32| {
        let q = (d / f64::from(n)) as f32;
        f64::from(if q > 1. { 1. } else { q })
    };
    let k = f64::from(s.counter.wrapping_sub(1));
    let ground = f64::from(ground);
    let start = f64::from(s.start_y);
    let h = f64::from(segment_height);
    let (completion, value) = match s.state {
        0 => (
            false,
            ground + (start - ground) * if d >= f64::from(s.a) { 1. } else { d / f64::from(s.a) },
        ),
        1 => (false, start + k * h + h * ratio(s.b)),
        2 => (
            true,
            k * (1. / f64::from(s.segments)) + ratio(s.e) * (1. / f64::from(s.segments)),
        ),
        3 => (
            false,
            ground + ((start - ground) + f64::from(s.span)) * (1. - ratio(s.c)),
        ),
        _ => return None,
    };
    Some((completion, value as f32))
}

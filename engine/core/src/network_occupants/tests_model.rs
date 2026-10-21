//! Conformance for §10 (test 24) and the model-instance contracts.
use super::model::*;
use super::record::{Triangle, Vertex};

struct Grid;

impl ModelProvider for Grid {
    fn flat_model(&self, key: u64) -> Option<Vec<Vertex>> {
        if key == 0 {
            return None;
        }
        // A flat 5x5 grid over one 16 m cell, y = 0.
        let mut out = Vec::new();
        for i in 0..5 {
            for j in 0..5 {
                out.push(Vertex {
                    pos: [i as f32 * 4.0, 0.0, j as f32 * 4.0],
                    color: 0xFF80_8080,
                    u: 0.0,
                    v: 0.0,
                });
            }
        }
        Some(out)
    }
}

#[derive(Default)]
struct Rend {
    calls: Vec<(&'static str, usize, bool)>,
}

impl ModelRenderer for Rend {
    fn flip_and_light(&mut self, verts: &mut [Vertex], flip: bool) {
        self.calls.push(("flip_and_light", verts.len(), flip));
    }
    fn clip(&mut self, verts: &mut [Vertex]) {
        self.calls.push(("clip", verts.len(), false));
    }
    fn prelight(&mut self, verts: &mut [Vertex]) {
        self.calls.push(("prelight", verts.len(), false));
    }
}

const C: CornerHeights = [10.0, 20.0, 40.0, 30.0];

#[test]
fn overrides_and_slot() {
    assert_eq!(GET_MODEL_INSTANCES_SLOT, 33);
    assert_eq!(ModelOverride::ALL.len(), 4);
    assert_eq!(ModelOverride::ALL[0], ModelOverride::Deform);
    assert_eq!(ModelOverride::ALL[3], ModelOverride::Prelight);
}

#[test]
fn corner_order_matches_geometry_8_1() {
    let f = DeformFrame::default();
    // (x, z), (x, z+1), (x+1, z+1), (x+1, z)
    assert_eq!(corner_height(&C, &f, 0.0, 0.0), 10.0);
    assert_eq!(corner_height(&C, &f, 0.0, 16.0), 20.0);
    assert_eq!(corner_height(&C, &f, 16.0, 16.0), 40.0);
    assert_eq!(corner_height(&C, &f, 16.0, 0.0), 30.0);
    assert_eq!(corner_height(&C, &f, 8.0, 8.0), 25.0);
}

#[test]
fn test24_deform_y_equals_interpolated_corners() {
    let mut m = NetworkModelInstance::create(&Grid, 7).expect("model");
    assert_eq!(m.rendered(), &m.flat[..]);
    m.deform(C);
    assert_eq!(m.corners, Some(C));
    for (v, f) in m.rendered().iter().zip(&m.flat) {
        let s = v.pos[0] / 16.0;
        let t = v.pos[2] / 16.0;
        let want = 10.0 * (1.0 - s) * (1.0 - t) + 20.0 * (1.0 - s) * t + 40.0 * s * t + 30.0 * s * (1.0 - t);
        assert!((v.pos[1] - want).abs() < 1e-4, "{:?}", v.pos);
        assert_eq!((v.pos[0], v.pos[2], v.color), (f.pos[0], f.pos[2], f.color));
    }
    // Deform starts from the flat model each time.
    m.deform([0.0; 4]);
    assert!(m.rendered().iter().all(|v| v.pos[1] == 0.0));
}

#[test]
fn deform_adds_to_model_relief() {
    let mut m = NetworkModelInstance::create(&Grid, 7).unwrap();
    m.flat[0].pos[1] = 0.5;
    m.deform([2.0; 4]);
    assert_eq!(m.rendered()[0].pos[1], 2.5);
}

#[test]
fn passes_delegate_with_flip_bit() {
    let mut m = NetworkModelInstance::create(&Grid, 7).unwrap();
    let mut r = Rend::default();
    m.flip_and_light(&mut r, 0x85);
    m.flip_and_light(&mut r, 0x05);
    m.clip(&mut r);
    m.prelight(&mut r);
    assert_eq!(
        r.calls,
        vec![
            ("flip_and_light", 25, true),
            ("flip_and_light", 25, false),
            ("clip", 25, false),
            ("prelight", 25, false)
        ]
    );
    assert!(flip_bit(0x80) && !flip_bit(0x7F));
}

#[test]
fn render_selects_model_or_fallback() {
    let quad = [Vertex::default(); 4];
    let tris: Vec<Triangle> = Vec::new();
    assert!(NetworkModelInstance::create(&Grid, 0).is_none());
    let none = PieceRender::select(None, &quad, &tris);
    assert!(matches!(none, PieceRender::Fallback { .. }));
    assert!(none.model_instance().is_none());
    let m = NetworkModelInstance::create(&Grid, 7).unwrap();
    let some = PieceRender::select(Some(&m), &quad, &tris);
    assert_eq!(some.model_instance().map(|i| i.key), Some(7));
}

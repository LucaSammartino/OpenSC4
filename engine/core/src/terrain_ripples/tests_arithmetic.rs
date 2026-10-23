// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

use super::*;
use crate::terrain_ripples::arithmetic::{self, Arithmetic, Portable, Spill};
use std::sync::Mutex;

#[derive(Default)]
struct Observed {
    stores: Mutex<Vec<(Spill, f64, f32)>>,
    angles: Mutex<Vec<f64>>,
}
macro_rules! binary {
    ($($method:ident),*) => {$(
        fn $method(&self, a: f64, b: f64) -> f64 { Portable.$method(a, b) }
    )*};
}
impl Arithmetic for Observed {
    type Register = f64;
    fn load(&self, value: f32) -> f64 {
        f64::from(value)
    }
    fn store(&self, at: Spill, value: f64) -> f32 {
        let result = value as f32;
        self.stores.lock().unwrap().push((at, value, result));
        result
    }
    binary!(add, sub, mul, div);
    fn sqrt(&self, a: f64) -> f64 {
        Portable.sqrt(a)
    }
    fn sin(&self, a: f64) -> f64 {
        Portable.sin(a)
    }
    fn acos(&self, a: f64) -> f64 {
        self.angles.lock().unwrap().push(a);
        Portable.acos(a)
    }
    fn less(&self, a: f64, b: f64) -> bool {
        a < b
    }
    fn greater(&self, a: f64, b: f64) -> bool {
        a > b
    }
}

fn parameters() -> mesh::Parameters {
    mesh::Parameters {
        step: 0.25,
        spacing_squared: 1.0,
        turn_radians: 1.0,
        advance: 0.15,
        outer: 26.0,
        inner: 1.0,
        wobble: 0.2,
        height: 0.3,
        alpha: [100; 2],
        fade: 6,
    }
}

#[test]
fn vector30_axis_spills_phase_and_vertices() {
    let a = Observed::default();
    let mut phase = 0.0;
    let v = arithmetic::candidate(
        &a,
        [point(0., 0.), point(16., 0.), point(32., 0.), point(48., 0.)],
        0.0,
        &mut phase,
        9.0,
        &parameters(),
    );
    assert_eq!(v, [[16., 9., -26.], [16., 9.3, 1.]]);
    assert_eq!(phase.to_bits(), 0x3DCC_CCCD);
    let stores = a.stores.lock().unwrap();
    assert_eq!(
        stores[..10].iter().map(|s| s.0).collect::<Vec<_>>(),
        [
            Spill::W1,
            Spill::W2,
            Spill::W3,
            Spill::D0,
            Spill::D1,
            Spill::D2,
            Spill::Px,
            Spill::Pz,
            Spill::X,
            Spill::Tx,
        ]
    );
    assert_eq!(
        stores[..6].iter().map(|s| s.2.to_bits()).collect::<Vec<_>>(),
        [0x3F2A_AAAB, 0x3E2A_AAAB, 0, 0xBF00_0000, 0x8000_0000, 0x3F00_0000]
    );
    assert_eq!(stores[6].2.to_bits(), 16.0_f32.to_bits());
    assert_eq!(stores[9].2, 1.0);
    // No w0/d3/Z/Tz/radius stores are permitted by this ledger.
    assert_eq!(stores.len(), 17);
}

#[test]
fn non_axis_fixture_retains_tangent_z_until_vertex_stores() {
    // Rational cubic weights at t=3/8, binary32 weight stores, exact rational
    // coordinate reductions, then a 70-decimal-digit square root provide these
    // independent fixture bits. This is not a native x87 capture/oracle.
    let points = [
        point(0.125, 3.25),
        point(12.75, -4.5),
        point(18.25, 18.125),
        point(45.5, -6.75),
    ];
    let a = Observed::default();
    let v = arithmetic::candidate(&a, points, 0.375, &mut 0.0, 9.0, &parameters());
    let stores = a.stores.lock().unwrap();
    assert_eq!(
        stores[6..10].iter().map(|s| s.2.to_bits()).collect::<Vec<_>>(),
        [0x416B_6B81, 0x4099_B101, 0x4106_BC00, 0x3F08_8F4D]
    );
    assert_eq!(
        [v[0][0], v[0][2], v[1][0], v[1][2]].map(f32::to_bits),
        [0x4212_D285, 0xC111_105D, 0x415D_E2F1, 0x40AA_C2EB]
    );
    drop(stores);
    let sample = arithmetic::spline(&Portable, points, 0.375);
    assert_ne!(sample.tz, f64::from(sample.tz as f32));
    assert_eq!(sample.tz as f32, 0.845_840_04);
}

#[test]
fn turn_distance_direction_spills_and_unclamped_register_dot() {
    let a = Observed::default();
    // f32 squared-distance arithmetic would round this to exactly 1.0.
    let squared = arithmetic::distance_squared(&a, [0.; 3], [1., 0., 0.0001]);
    assert!(squared > 1.0);
    assert!(a.stores.lock().unwrap().is_empty());
    let direction = arithmetic::direction(&a, [0.; 3], [3., 0., 4.]);
    assert_eq!(direction, [0.6, 0.8]);
    assert_eq!(
        a.stores.lock().unwrap().iter().map(|s| s.0).collect::<Vec<_>>(),
        [Spill::DirectionX, Spill::DirectionZ]
    );
    // The retained dot exceeds 1 for these stored directions; it must not be
    // clamped or spilled before the library boundary. NaN > limit is false.
    assert!(!arithmetic::turn_exceeds(&a, direction, direction, -1.0));
    assert!(a.angles.lock().unwrap()[0] > 1.0);
    assert!(!arithmetic::turn_exceeds(
        &a,
        [1., 0.],
        [0., 1.],
        std::f32::consts::FRAC_PI_2
    ));
    assert!(arithmetic::turn_exceeds(&a, [1., 0.], [-1., 0.], 3.0));
}

#[test]
fn zero_derivative_reaches_math_boundary_without_a_guard_or_fake_tangent() {
    let sample = arithmetic::spline(&Portable, [point(0., 0.); 4], 0.0);
    assert!(sample.tx.is_nan());
    assert!(sample.tz.is_nan());
}

#[test]
fn owner_uses_primitive_backend_for_mesh_and_turns() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static STORES: AtomicUsize = AtomicUsize::new(0);
    #[derive(Default)]
    struct Counted;
    impl Arithmetic for Counted {
        type Register = f64;
        fn load(&self, a: f32) -> f64 {
            f64::from(a)
        }
        fn store(&self, _: Spill, a: f64) -> f32 {
            STORES.fetch_add(1, Ordering::SeqCst);
            a as f32
        }
        binary!(add, sub, mul, div);
        fn sqrt(&self, a: f64) -> f64 {
            a.sqrt()
        }
        fn sin(&self, a: f64) -> f64 {
            a.sin()
        }
        fn acos(&self, a: f64) -> f64 {
            Portable.acos(a)
        }
        fn less(&self, a: f64, b: f64) -> bool {
            a < b
        }
        fn greater(&self, a: f64, b: f64) -> bool {
            a > b
        }
    }
    let mut w = RippleOwner::with_arithmetic(Counted);
    let mut ground = Ground::new([200, 20]);
    for x in 1..=190 {
        ground.flags[10 * 201 + x] = 0x10;
    }
    let shared = process();
    shared.lock().unwrap().random = Some(mesh::NativeRandom::from_effective_seed(0));
    w.init(
        Arc::new(ground),
        Arc::new(Factory::default()),
        &Source::default(),
        shared,
        arithmetic(),
    )
    .unwrap();
    // Even if a failed texture request prevents preparation, traversal must
    // have called only primitive operations on the caller's backend.
    let _ = w.update(zoom(0), true);
    assert!(STORES.load(Ordering::SeqCst) > 0);
}

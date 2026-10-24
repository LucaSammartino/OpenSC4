// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::effects::{directory::IDENTITY_PLACEMENT, shipped::BRUSHES, BrushEffect, GameEffect, GameRecord};
use crate::rng::{FixedSeeds, RzRandom};

fn placed(translation: [f32; 3]) -> Placement {
    Placement {
        flags: 1,
        transform: ParentTransform {
            translation,
            ..ParentTransform::default()
        },
        ..Placement::default()
    }
}

#[test]
fn ef19_1_2_decode_revision_flags_and_float_bits() {
    let words = [1f32, 2., 3., 4., 5., 6., 7., 8., 9., 10., 11., 12., 2.];
    let mut bytes = [0; 56];
    for (out, word) in bytes.chunks_exact_mut(4).zip(words) {
        out.copy_from_slice(&word.to_le_bytes());
    }
    bytes[52..].copy_from_slice(&0x123456ABu32.to_le_bytes());
    let mut decoded = Placement::decode(&bytes);
    assert_eq!(decoded.transform.rotation, [[1., 2., 3.], [4., 5., 6.], [7., 8., 9.]]);
    assert_eq!(decoded.transform.translation, [10., 11., 12.]);
    assert_eq!(
        (decoded.flags, decoded.revision, decoded.transform.scale),
        (7, 0xAB, 2.)
    );
    assert_eq!(&decoded.encode()[..52], &bytes[..52]);
    assert_eq!(&decoded.encode()[52..], &0xABu32.to_le_bytes());
    decoded.flags = 0xFA;
    decoded.load(&IDENTITY_PLACEMENT);
    assert_eq!((decoded.flags, decoded.revision), (0xFD, 0));
    assert_eq!(Placement::decode(&IDENTITY_PLACEMENT).flags, 5);
    for bits in [0x80000000u32, 0x7FC12345, 0x7F800000] {
        let mut bytes = IDENTITY_PLACEMENT;
        bytes[36..40].copy_from_slice(&bits.to_le_bytes());
        bytes[48..52].copy_from_slice(&bits.to_le_bytes());
        let p = Placement::decode(&bytes);
        assert_eq!(p.transform.translation[0].to_bits(), bits);
        assert_eq!(p.flags, 7);
        assert_eq!(p.encode(), bytes);
    }
}

#[test]
fn ef19_3_both_component_owners_use_rows_and_translation_only() {
    let mut local = placed([1., 2., 3.]);
    local.transform.scale = 99.;
    local.transform.rotation = [[f32::NAN; 3]; 3];
    let parent = Placement {
        flags: 7,
        transform: ParentTransform {
            rotation: [[1., 2., 3.], [4., 5., 6.], [7., 8., 9.]],
            scale: 2.,
            translation: [10., 20., 30.],
        },
        ..Placement::default()
    };
    let mut rng = RzRandom::new(
        1,
        &FixedSeeds {
            wall_clock: 1,
            process_clock: 2,
        },
    );
    let mut brush = BrushEffect::new(&BRUSHES[0], &mut rng);
    let mut game = GameEffect::new(&GameRecord::default());
    brush.set_placement(&local, &parent, Precision::Single);
    game.set_placement(&local, &parent, Precision::Single);
    assert_eq!(brush.cur, (38., 130.));
    assert_eq!(game.position, [38., 84., 130.]);
    assert_eq!(game.cell, (2, 8));
}

#[test]
fn ef19_4_5_6_flags_precedence_cancellation_and_zero_sign() {
    let mut local = placed([-0., 2., -0.]);
    let mut parent = placed([7., 8., 9.]);
    parent.flags = 0;
    assert_eq!(
        local.world_position(&parent, true, Precision::Single).map(f32::to_bits),
        local.transform.translation.map(f32::to_bits)
    );
    local.flags = 0;
    parent.transform.translation = [-0., -0., -0.];
    assert_eq!(
        local.world_position(&parent, true, Precision::Single).map(f32::to_bits),
        [(-0f32).to_bits(); 3]
    );
    local = placed([1.; 3]);
    parent = placed([0.; 3]);
    parent.transform.rotation[0] = [1., -16777216., 16777216.];
    assert_eq!(local.world_position(&parent, true, Precision::Single)[0], 0.);
    assert_eq!(local.world_position(&parent, false, Precision::Single)[0], 0.);
    assert_eq!(local.world_position(&parent, true, Precision::Double)[0], 1.);
    local.transform.translation = [0.; 3];
    parent.transform.rotation[0] = [0.; 3];
    parent.transform.translation[0] = -0.;
    assert_eq!(local.world_position(&parent, true, Precision::Single)[0].to_bits(), 0);
    parent.transform.rotation[0][2] = f32::NAN;
    assert!(local.world_position(&parent, false, Precision::Single)[0].is_nan());
}

#[test]
fn extended_exponent_and_double_precision_store_boundaries() {
    let local = placed([2., 0., 0.]);
    let mut parent = placed([0.; 3]);
    parent.transform.rotation[0][0] = f32::MAX;
    parent.transform.scale = 0.25;
    assert_eq!(
        local.world_position(&parent, true, Precision::Single)[0],
        f32::MAX * 0.5
    );
    // In double precision the game's z dot spills before scaling; brush spills
    // after scaling. Choose a half-ULP dot and a scale exposing that distinction.
    let local = placed([1., 1., 1.]);
    parent.transform.rotation[2] = [1., f32::from_bits(0x3380_0000), 0.];
    parent.transform.scale = 3.;
    let brush = local.world_position(&parent, false, Precision::Double);
    let game = local.world_position(&parent, true, Precision::Double);
    assert_eq!(brush[2].to_bits(), 3f32.to_bits() + 1);
    assert_eq!(game[2], 3.);
}

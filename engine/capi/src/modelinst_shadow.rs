// SPDX-License-Identifier: GPL-3.0-or-later
//! Private fixture and live destination-counter transport, not a live-state reconstruction.
use fc4_core::model_instances::{DrawAttributes, StaticInstance, Transform};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeTransform {
    counter: u8,
    kind: u8,
    reserved: [u8; 2],
    words: [f32; 13],
}

fn decode(t: NativeTransform) -> Transform {
    Transform {
        change_counter: t.counter,
        kind: t.kind,
        rotation: [
            [t.words[0], t.words[1], t.words[2]],
            [t.words[3], t.words[4], t.words[5]],
            [t.words[6], t.words[7], t.words[8]],
        ],
        translation: [t.words[9], t.words[10], t.words[11]],
        scale: t.words[12],
    }
}

fn encode(t: Transform, reserved: [u8; 2]) -> NativeTransform {
    NativeTransform {
        counter: t.change_counter,
        kind: t.kind,
        reserved,
        words: [
            t.rotation[0][0],
            t.rotation[0][1],
            t.rotation[0][2],
            t.rotation[1][0],
            t.rotation[1][1],
            t.rotation[1][2],
            t.rotation[2][0],
            t.rotation[2][1],
            t.rotation[2][2],
            t.translation[0],
            t.translation[1],
            t.translation[2],
            t.scale,
        ],
    }
}

// Fixture inputs are specified here independently of the native outputs.
fn input(probe: u32) -> Transform {
    let mut t = Transform::identity();
    if probe != 0 {
        t.kind = 7;
        t.rotation = [[0., -1., 0.], [1., 0., 0.], [0., 0., 1.]];
        t.translation = [4., -8., 16.];
        t.scale = -2.;
        t.change_counter = 255;
    }
    t
}

#[no_mangle]
unsafe extern "C" fn fc4_modelinst_shadow_fixture(
    probe: u32,
    supplied: *mut NativeTransform,
    expected: *mut NativeTransform,
    bounds: *mut f32,
) -> bool {
    if probe > 1 || supplied.is_null() || expected.is_null() || bounds.is_null() {
        return false;
    }
    let mut core = StaticInstance::new(DrawAttributes::default());
    let t = input(probe);
    if probe != 0 {
        core.set_transform(&t);
    }
    // Caller counter wraps; input record counter is not copied by GetTransform.
    let mut output = Transform {
        change_counter: 255,
        ..Transform::identity()
    };
    core.get_transform(&mut output);
    let world = core.world_bounds();
    unsafe {
        supplied.write(encode(t, [0xA5, 0x5A]));
        expected.write(encode(output, [0xA5, 0x5A]));
        for (i, v) in world.min.into_iter().chain(world.max).enumerate() {
            bounds.add(i).write(v);
        }
    }
    true
}

// MI14: compare only channels with an independent baseline captured before the
// game's own static GetTransform. Source words/kind are deliberately unmeasured;
// no source state or additional getter is needed to predict these three bytes.
#[no_mangle]
unsafe extern "C" fn fc4_modelinst_shadow_destination(
    before: *const NativeTransform,
    after: *const NativeTransform,
) -> u32 {
    if before.is_null() || after.is_null() {
        return 1;
    }
    let (before, after) = unsafe { (before.read(), after.read()) };
    let mut expected = decode(before);
    StaticInstance::new(DrawAttributes::default()).get_transform(&mut expected);
    u32::from(expected.change_counter != after.counter)
        + u32::from(before.reserved[0] != after.reserved[0])
        + u32::from(before.reserved[1] != after.reserved[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_layout_and_fixture_counter_are_independent_of_input() {
        assert_eq!(std::mem::size_of::<NativeTransform>(), 0x38);
        let mut supplied = encode(Transform::identity(), [0; 2]);
        let mut expected = supplied;
        let mut bounds = [0.; 6];
        assert!(unsafe { fc4_modelinst_shadow_fixture(1, &mut supplied, &mut expected, bounds.as_mut_ptr()) });
        assert_eq!(decode(supplied), input(1));
        assert_eq!(expected.counter, 0);
        assert_eq!(expected.kind, 7);
        assert_eq!(bounds, [4., -10., 14., 6., -8., 16.]);
        assert!(!unsafe { fc4_modelinst_shadow_fixture(2, &mut supplied, &mut expected, bounds.as_mut_ptr()) });
    }

    #[test]
    fn live_destination_detects_counter_and_padding_damage_without_source_words() {
        let before = encode(input(1), [0xA5, 0x5A]);
        let mut after = encode(Transform::identity(), before.reserved);
        // The independent destination counter wraps even though the source has
        // different kind/words; those unknown source channels are not compared.
        after.counter = 0;
        assert_eq!(unsafe { fc4_modelinst_shadow_destination(&before, &after) }, 0);
        after.counter = 255;
        after.reserved = [0, 0];
        assert_eq!(unsafe { fc4_modelinst_shadow_destination(&before, &after) }, 3);
    }
}

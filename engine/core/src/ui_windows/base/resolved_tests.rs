// SPDX-License-Identifier: GPL-3.0-or-later
//! Worked cases for the resolved WT1-WT3, WT6 and WT13 answers.
use super::*;

#[derive(Default)]
struct RecordingHost(Vec<NativeOp>);
impl Host for RecordingHost {
    fn event(&mut self, event: NativeOp) {
        self.0.push(event);
    }
}

#[test]
fn wt1_callable_inventory_and_constant_stubs() {
    assert_eq!(BASE_SLOT_COUNT, 151);
    assert!((0..BASE_SLOT_COUNT).all(|slot| base_slot(slot).is_some()));
    assert_eq!(base_slot(148), Some(BaseSlot::DeletingDestructor));
    assert_eq!(base_slot(149), Some(BaseSlot::PointTransparent));
    assert_eq!(base_slot(150), Some(BaseSlot::CreatePrivateBuffer));
    assert_eq!(base_slot(151), None);
    let mut wins = Windows::default();
    let mut host = RecordingHost::default();
    let w = wins.create(&mut host, 1, None);
    let before = wins.get(w).clone();
    host.0.clear();
    assert!(host.handler(&mut wins, w, 88, &[]));
    assert!(!host.handler(&mut wins, w, 143, &[0x12345678, u32::MAX]));
    for (slots, bytes) in [
        (&[129, 133, 141, 142][..], 4),
        (&[130, 131, 140, 143][..], 8),
        (&[134, 135, 136, 137, 138][..], 12),
        (&[139][..], 16),
    ] {
        for &slot in slots {
            assert_eq!(
                base_slot(slot),
                Some(BaseSlot::Constant {
                    byte: 0,
                    argument_bytes: bytes
                })
            );
            assert!(!host.handler(&mut wins, w, slot, &[u32::MAX; 4]));
        }
    }
    assert_eq!(wins.get(w), &before);
    assert!(host.0.is_empty());
}

#[test]
fn wt2_untouched_fields_and_mouse_offset_writer() {
    let mut wins = Windows::default();
    let mut host = RecordingHost::default();
    let w = wins.create(&mut host, 1, None);
    assert_eq!(wins.get(w).draw_origin, None);
    assert_eq!(wins.get(w).mouse_down_offset, None);
    wins.get_mut(w).set_mouse_down_offset(4, 6);
    wins.set_area(&mut host, w, Rect(0, 0, 20, 30));
    assert!(wins.get(w).draw_origin.is_some());
    wins.init(&mut host, w);
    wins.shutdown(&mut host, w);
    assert_eq!(wins.get(w).mouse_down_offset, Some((4, 6)));
}

#[test]
fn wt3_default_consumer_flags_have_no_base_side_effects() {
    let mut wins = Windows::default();
    let mut host = RecordingHost::default();
    let w = wins.create(&mut host, 1, None);
    assert_eq!(wins.get(w).flags, 0x8903);
    wins.get_mut(w).set_mouse_down_offset(4, 6);
    let before = wins.get(w).clone();
    host.0.clear();
    for mask in [0x100, 0x8000] {
        assert!(wins.get(w).flag(mask));
        assert_eq!(wins.set_flag(&mut host, w, mask, false), 0x8903 & !mask);
        assert!(!wins.get(w).flag(mask));
        // The dialog/manager consume this query; the base does not drag or change focus here.
        assert_eq!(wins.set_flag(&mut host, w, mask, true), 0x8903);
    }
    assert_eq!(wins.get(w), &before);
    assert!(host.0.is_empty());
    assert_eq!(wins.focus, None);
    assert_eq!(wins.capture, None);
}

#[test]
fn wt6_fresh_manager_first_registration_is_one() {
    let mut wins = Windows::default();
    let mut host = RecordingHost::default();
    let first = wins.create_default(&mut host);
    let second = wins.create(&mut host, 0, None);
    assert_eq!((wins.get(first).instance_id, wins.get(second).instance_id), (1, 2));
    assert_eq!(host.0, [NativeOp::Register(first), NativeOp::Register(second)]);
}

#[test]
fn wt13_fade_shared_deleting_wrapper_and_parameter_lifetime() {
    for flags in [0, 3, 2, 1] {
        for initialized in [false, true] {
            for already_shutdown in [false, true] {
                let mut wins = Windows::default();
                let mut host = RecordingHost::default();
                let w = wins.create_fade(&mut host, 1, None);
                assert!(wins.get(w).params().contains_key(&FADE_TEMP));
                wins.set_param(&mut host, w, FADE_TEMP, Some(Param::Object(77)));
                if initialized {
                    wins.init(&mut host, w);
                }
                host.0.clear();
                if already_shutdown {
                    wins.shutdown(&mut host, w);
                }
                assert_eq!(wins.deleting_destructor(&mut host, w, flags), w);
                assert_eq!(
                    host.0.iter().filter(|e| **e == NativeOp::Release(77)).count(),
                    usize::from(initialized)
                );
                assert_eq!(host.0.iter().filter(|e| **e == NativeOp::Unregister(w)).count(), 1);
                assert!(!host.0.iter().any(|e| matches!(e, NativeOp::AddRef(_))));
                assert_eq!(wins.windows.contains_key(&w), flags & 1 == 0);
                assert!(!wins.valid(w));
                if flags & 1 == 0 {
                    assert!(wins.get(w).params().is_empty());
                }
            }
        }
    }
}

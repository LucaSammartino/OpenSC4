// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Default)]
struct Controls {
    calls: Vec<Call>,
    missing: Vec<u32>,
    auto: Option<[u8; 0x29]>,
}
impl Host for Controls {
    fn call(&mut self, call: &Call) -> Result<Reply, Unsupported> {
        self.calls.push(call.clone());
        Ok(match call {
            Call::Root(_) => Reply::Object(Some(Object(3))),
            Call::Global(Global::App) => Reply::Object(Some(Object(4))),
            Call::Global(Global::ModalService) => Reply::Object(Some(Object(12))),
            Call::CreateScript { .. } => Reply::Lookup {
                al: 1,
                output: Some(Object(2)),
            },
            Call::Pause(_) => Reply::Object(Some(Object(10))),
            Call::Framework => Reply::Object(Some(Object(11))),
            Call::Service { .. } => Reply::Lookup {
                al: 0,
                output: Some(Object(13)),
            },
            Call::WindowManager => Reply::Object(Some(Object(13))),
            Call::Slot {
                object: Object(13),
                slot: 0x90,
                ..
            } => Reply::U32(123),
            Call::Slot {
                object: Object(13),
                slot: 0x60,
                ..
            } => Reply::Al(1),
            Call::Slot {
                object: Object(3),
                slot: 0x18,
                ..
            } => Reply::Object(None),
            Call::Slot {
                object: Object(3),
                slot: 0x2c,
                ..
            } => Reply::Object(Some(Object(6))),
            Call::Slot {
                object: Object(6),
                slot: 0xa4,
                ..
            } => Reply::U32(1280),
            Call::Slot {
                object: Object(6),
                slot: 0xa8,
                ..
            } => Reply::U32(720),
            Call::Slot { slot: 0x8c, args, .. } => match args.as_slice() {
                [Arg::U32(id)] => Reply::Object(Some(Object(u64::from(*id)))),
                _ => panic!("child ID expected"),
            },
            Call::Lookup { id, .. } => Reply::Lookup {
                al: u8::from(!self.missing.contains(id)),
                output: (!self.missing.contains(id)).then_some(Object(u64::from(*id))),
            },
            _ => Reply::Ignored,
        })
    }
    fn preferences(&mut self, call: &Call, preferences: &mut Preferences) -> Result<Reply, Unsupported> {
        if matches!(call, Call::Slot { slot: 0x50, .. }) {
            if let Some(raw) = self.auto {
                *preferences.raw_mut() = raw;
            }
        }
        self.call(call)
    }
}
fn panel(raw: [u8; 0x29]) -> Owner {
    Owner::from_raw(
        RawState {
            owner: Object(1),
            host: Some(Object(2)),
            preferences: Some(Preferences::from_raw(Object(9), raw)),
            baselines: Baselines {
                b10: 99,
                b12: 99,
                b14: 99,
                b16: 99,
                b18: 99,
                b1c: 99,
            },
            marker: 0,
            first_redraw: 1,
            guard: 0,
            initialized: 1,
        },
        Domain {
            valid_objects: true,
            unaliased_storage: true,
            no_foreign_reentry: true,
        },
    )
    .unwrap()
}
fn redraw_bytes() -> [u8; 0x29] {
    let mut raw = [0; 0x29];
    raw[0x16] = 2;
    raw[0x15] = 1;
    raw[0x13] = 1;
    raw[0x1c..0x20].copy_from_slice(&1024u32.to_le_bytes());
    raw[0x20..0x24].copy_from_slice(&768u32.to_le_bytes());
    raw[0x24..0x28].copy_from_slice(&32u32.to_le_bytes());
    raw
}
fn requests(c: &Controls, ids: &[u32]) -> Vec<(u32, u16)> {
    c.calls
        .iter()
        .filter_map(|c| match c {
            Call::Slot {
                object: Object(id),
                slot: slot @ (0x18 | 0x1c),
                ..
            } if ids.contains(&(*id as u32)) => Some((*id as u32, *slot)),
            _ => None,
        })
        .collect()
}
#[test]
fn section9_case1_first_redraw_captures_only_once() {
    let mut owner = panel(redraw_bytes());
    let mut controls = Controls::default();
    owner.redraw(&mut controls).unwrap();
    let expected = Baselines {
        b10: 0,
        b12: 1,
        b14: 32,
        b16: 1,
        b18: 1,
        b1c: 0,
    };
    assert_eq!(owner.state.baselines, expected);
    assert_eq!(owner.state.first_redraw, 0);
    owner.state.preferences = Some(Preferences::from_raw(Object(9), [255; 0x29]));
    owner.redraw(&mut controls).unwrap();
    assert_eq!(owner.state.baselines, expected);
}
#[test]
fn section9_case2_unlisted_pair_clears_all_and_captures_zero() {
    let mut bytes = redraw_bytes();
    bytes[0x20..0x24].copy_from_slice(&600u32.to_le_bytes());
    let mut owner = panel(bytes);
    let mut controls = Controls::default();
    owner.redraw(&mut controls).unwrap();
    assert_eq!(owner.state.baselines.b18, 0);
    assert_eq!(
        requests(&controls, &[0x6a57da5a, 0x6a57da5b, 0x6a57da5c, 0x6a57da5d]),
        [
            (0x6a57da5a, 0x1c),
            (0x6a57da5b, 0x1c),
            (0x6a57da5c, 0x1c),
            (0x6a57da5d, 0x1c)
        ]
    );
}
#[test]
fn redraw_failed_checkboxes_do_not_capture_but_radio_still_does() {
    let mut owner = panel(redraw_bytes());
    let mut controls = Controls {
        missing: vec![0x6a57da56, 0x6a57da60, 0x2ab6c373, 0x2ab6c374, 0x2ab6c375],
        ..Controls::default()
    };
    owner.redraw(&mut controls).unwrap();
    assert_eq!(owner.state.baselines.b1c, 99);
    assert_eq!(owner.state.baselines.b10, 99);
    assert_eq!(owner.state.baselines.b16, 1);
    // Utility calls keep the previous lookup reference alive (§9.2).
    let at = controls
        .calls
        .iter()
        .position(|c| matches!(c, Call::SetToggle { id: 0x6a57da54, .. }))
        .unwrap();
    assert!(matches!(controls.calls[at - 1], Call::Root(_)));
}

#[test]
fn section9_case3_resolution_radio_and_self_toggling_checkbox() {
    let mut owner = panel(redraw_bytes());
    let before = owner.state().clone();
    let mut controls = Controls::default();
    assert_eq!(owner.action(&mut controls, 0x6a57da5c), Ok(true));
    assert_eq!(
        requests(&controls, &[0x6a57da5a, 0x6a57da5b, 0x6a57da5c, 0x6a57da5d]),
        [
            (0x6a57da5a, 0x1c),
            (0x6a57da5b, 0x1c),
            (0x6a57da5c, 0x18),
            (0x6a57da5d, 0x1c)
        ]
    );
    controls.calls.clear();
    assert_eq!(owner.action(&mut controls, 0x6a57da55), Ok(true));
    assert!(controls.calls.is_empty());
    assert_eq!(owner.state(), &before);
}

#[test]
fn auto_mutates_live_preferences_and_redraw_preserves_opening_baselines() {
    let mut owner = panel(redraw_bytes());
    let mut controls = Controls::default();
    owner.redraw(&mut controls).unwrap();
    let baseline = owner.state.baselines;
    let mut auto = redraw_bytes();
    auto[0x16] = 3;
    controls.auto = Some(auto);
    controls.calls.clear();
    owner.action(&mut controls, 0xea5e99d9).unwrap();
    assert_eq!(owner.state.preferences.as_ref().unwrap().byte(Byte::F16), 3);
    assert_eq!(owner.state.baselines, baseline);
    assert!(matches!(
        controls.calls[1],
        Call::Slot {
            object: Object(4),
            slot: 4,
            ..
        }
    ));
    assert!(matches!(
        controls.calls[2],
        Call::Slot {
            object: Object(4),
            slot: 0x50,
            ..
        }
    ));
    assert!(matches!(
        controls.calls.last(),
        Some(Call::Slot {
            object: Object(4),
            slot: 8,
            ..
        })
    ));
}

#[test]
fn section9_case4_construction_full_show_modal_and_free() {
    let mut owner = Owner::new(Object(1), DialogGlobals::default()).unwrap();
    assert_eq!(owner.state.first_redraw, 1);
    assert_eq!(owner.add_ref(), Ok(1));
    let mut controls = Controls::default();
    assert_eq!(
        owner.show_dialog(&mut controls, 0, Some(Preferences::from_raw(Object(9), redraw_bytes()))),
        Ok(false)
    );
    let bindings: Vec<_> = controls
        .calls
        .iter()
        .filter_map(|c| match c {
            Call::Slot {
                object,
                slot: 0x158,
                args,
            } => {
                assert_eq!(args, &[Arg::Object(Some(Object(3)))]);
                Some(object.0 as u32)
            }
            _ => None,
        })
        .collect();
    assert_eq!(bindings, BINDINGS);
    assert_eq!(
        BINDINGS.len(),
        BINDINGS.iter().filter(|&&id| Owner::admitted(id)).count()
    );
    assert_eq!(owner.do_modal(&mut controls), Ok(true));
    assert_eq!(
        owner.globals(),
        DialogGlobals {
            initialized: 1,
            guard: 0
        }
    );
    assert!(owner.state.host.is_none());
    assert_eq!(owner.release(&mut controls), Ok(0));
    assert!(matches!(controls.calls.last(), Some(Call::FreeOwner(Object(1)))));
    assert_eq!(owner.shutdown(), Err(Unsupported::FreedOwner));
}

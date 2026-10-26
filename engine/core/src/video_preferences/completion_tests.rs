// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Default)]
struct Controls {
    calls: Vec<Call>,
    missing: Vec<u32>,
}
impl Host for Controls {
    fn call(&mut self, call: &Call) -> Result<Reply, Unsupported> {
        self.calls.push(call.clone());
        Ok(match call {
            Call::Root(_) => Reply::Object(Some(Object(3))),
            Call::Lookup { id, .. } => Reply::Lookup {
                al: u8::from(!self.missing.contains(id)),
                output: (!self.missing.contains(id)).then_some(Object(u64::from(*id))),
            },
            _ => Reply::Ignored,
        })
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

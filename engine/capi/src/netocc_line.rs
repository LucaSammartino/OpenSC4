// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link transport for the power-line occupant factory.
//! The plugin serializes access and owns all host references.
use crate::guard;
use crate::props::{fc4_props, props_ref};
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};
use fc4_core::network_occupants::power_line::{read_power_line_record, write_power_line_record, PowerLine};
use fc4_core::stream::RefValue;
use std::ffi::c_void;

#[repr(C)]
#[derive(Default)]
pub struct LineState {
    removal: u16,
    highlight: u8,
    status: u8,
    flags: u8,
    rect: [u8; 4],
    preview: u8,
    pole_flag: u8,
    position: [f32; 3],
    cell: [i32; 2],
    poles: [u64; 2],
}

fn token(r: &RefValue) -> u64 {
    match r {
        RefValue::Host { token, .. } => *token,
        _ => 0,
    }
}

#[no_mangle]
pub extern "C" fn fc4_netline_create() -> *mut PowerLine {
    guard(std::ptr::null_mut(), || Box::into_raw(Box::new(PowerLine::new())))
}
#[no_mangle]
pub unsafe extern "C" fn fc4_netline_destroy(p: *mut PowerLine) {
    if !p.is_null() {
        guard((), || drop(Box::from_raw(p)));
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc4_netline_state(p: *mut PowerLine, s: *mut LineState, put: bool) -> bool {
    guard(false, || {
        let (Some(p), Some(s)) = (p.as_mut(), s.as_mut()) else {
            return false;
        };
        if put {
            p.base.removal = s.removal;
            p.base.highlight = s.highlight;
            p.base.status = s.status;
            p.base.flags = s.flags;
            p.base.cell_rect = s.rect;
            p.preview = s.preview;
            p.pole_flag = s.pole_flag;
            p.position = s.position;
            p.cell = (s.cell[0], s.cell[1]);
            p.poles = s.poles.map(|token| {
                if token == 0 {
                    RefValue::Null
                } else {
                    RefValue::Host {
                        token,
                        class_id: 0x09C0_5C6A,
                    }
                }
            });
        } else {
            *s = LineState {
                removal: p.base.removal,
                highlight: p.base.highlight,
                status: p.base.status,
                flags: p.base.flags,
                rect: p.base.cell_rect,
                preview: p.preview,
                pole_flag: p.pole_flag,
                position: p.position,
                cell: [p.cell.0, p.cell.1],
                poles: [token(&p.poles[0]), token(&p.poles[1])],
            };
        }
        true
    })
}
#[no_mangle]
pub unsafe extern "C" fn fc4_netline_position(p: *mut PowerLine, xyz: *const [f32; 3]) -> bool {
    guard(false, || {
        let (Some(p), Some(xyz)) = (p.as_mut(), xyz.as_ref()) else {
            return false;
        };
        p.set_position(*xyz);
        true
    })
}
#[no_mangle]
pub unsafe extern "C" fn fc4_netline_write(p: *mut PowerLine, props: *const fc4_props, s: *const fc4_ostream) -> bool {
    guard(false, || {
        let (Some(p), Some(s)) = (p.as_mut(), s.as_ref()) else {
            return false;
        };
        if !ostream_ok(s) {
            return false;
        }
        if let Some(props) = props_ref(props) {
            p.base.holder.own = props.clone();
        }
        write_power_line_record(&p.record(), &mut COut(s))
    })
}
#[no_mangle]
pub unsafe extern "C" fn fc4_netline_read(p: *mut PowerLine, s: *const fc4_istream) -> bool {
    guard(false, || {
        let (Some(p), Some(s)) = (p.as_mut(), s.as_ref()) else {
            return false;
        };
        if !istream_ok(s) {
            return false;
        }
        let mut input = CIn {
            stream: s,
            decode: |_, _| None,
        };
        let Some(record) = read_power_line_record(&mut input) else {
            return false;
        };
        *p = PowerLine::from_record(record);
        true
    })
}
#[no_mangle]
pub unsafe extern "C" fn fc4_netline_props(
    p: *const PowerLine,
    cb: crate::neighbors::fc4_neighbor_property_callback,
    ctx: *mut c_void,
) {
    guard((), || {
        let (Some(p), Some(cb)) = (p.as_ref(), cb) else {
            return;
        };
        for id in p.base.holder.own.ids() {
            if let Some(v) = p
                .base
                .holder
                .own
                .get(id)
                .and_then(crate::neighbors::connection_property)
            {
                cb(ctx, id, v.mem_type, v.is_array, v.bytes.as_ptr().cast(), v.count);
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn position_uses_core_truncation_and_preserves_loaded_cell_until_move() {
        unsafe {
            let p = fc4_netline_create();
            assert_eq!(std::mem::size_of::<LineState>(), 48);
            let mut state = LineState::default();
            assert!(fc4_netline_state(p, &mut state, false));
            assert_eq!(state.rect[0], 255);
            state.cell = [73, -91];
            assert!(fc4_netline_state(p, &mut state, true));
            assert!(fc4_netline_state(p, &mut state, false));
            assert_eq!(state.cell, [73, -91]);
            assert!(fc4_netline_position(p, &[-17.0, 8.0, -15.0]));
            assert!(fc4_netline_state(p, &mut state, false));
            assert_eq!(state.cell, [-1, 0]);
            fc4_netline_destroy(p);
        }
    }
}

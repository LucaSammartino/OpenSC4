// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link transport for the recording widget host. Not a DLL ABI.
use fc4_core::legacy_control::observation::recording::{Call, RecordingHost};
use fc4_core::legacy_control::observation::{replay_build, OwnerState};
use fc4_core::legacy_control::{interpret, interpret_common, FragmentCoverage, Properties, Property, Value};
use fc4_core::variant::{Elem, Variant};
use std::ffi::c_void;

#[repr(C)]
pub struct InputProperty {
    key: u32,
    kind: u32,
    number: u32,
    bytes: *const u8,
    length: u32,
}

type Exchange = unsafe extern "C" fn(*mut c_void, *const u32, u32, *mut u32, u32) -> u32;

fn text(out: &mut Vec<u32>, s: &str) {
    out.push(s.len() as u32);
    out.extend(s.bytes().map(u32::from));
}

fn encode(out: &mut Vec<u32>, value: &Value) -> bool {
    match value {
        Value::None => out.push(0),
        Value::U32(n) => out.extend([1, *n]),
        Value::I32(n) => out.extend([2, *n as u32]),
        Value::Handle(h) => out.extend([3, *h as u32]),
        Value::Text(s) => {
            out.push(4);
            text(out, s);
        }
        Value::Rect(r) => {
            out.push(5);
            out.extend(r.map(|n| n as u32));
        }
        Value::Colour(c) => {
            out.push(6);
            out.extend(c.map(u32::from));
        }
        Value::Pair(a, b) => out.extend([7, *a, *b]),
        Value::Words(words) => {
            out.extend([8, words.len() as u32]);
            out.extend(words);
        }
        Value::Key(k) => out.extend([9, k.kind, k.group, k.instance]),
        Value::Comments(comments) => {
            out.push(10);
            for s in comments {
                text(out, s);
            }
        }
        Value::Points(points) => {
            out.extend([11, points.len() as u32]);
            for p in points {
                out.extend(p.map(|v| v as u32));
            }
        }
        // Complex widget payload transport is not yet observed by this shadow.
        _ => return false,
    }
    true
}

fn decode(words: &[u32]) -> Option<Value> {
    let (&tag, tail) = words.split_first()?;
    Some(match tag {
        0 if tail.is_empty() => Value::None,
        1 if tail.len() == 1 => Value::U32(tail[0]),
        2 if tail.len() == 1 => Value::I32(tail[0] as i32),
        3 if tail.len() == 1 => Value::Handle(u64::from(tail[0])),
        4 if !tail.is_empty() && tail[0] as usize == tail.len() - 1 => {
            let bytes: Option<Vec<_>> = tail[1..].iter().map(|b| u8::try_from(*b).ok()).collect();
            Value::Text(String::from_utf8(bytes?).ok()?)
        }
        5 if tail.len() == 4 => Value::Rect(std::array::from_fn(|i| tail[i] as i32)),
        6 if tail.len() == 3 && tail.iter().all(|c| *c <= 255) => Value::Colour(std::array::from_fn(|i| tail[i] as u8)),
        7 if tail.len() == 2 => Value::Pair(tail[0], tail[1]),
        8 if !tail.is_empty() && tail[0] as usize == tail.len() - 1 => Value::Words(tail[1..].to_vec()),
        _ => return None,
    })
}

unsafe fn properties(input: *const InputProperty, count: u32) -> Option<Vec<Property>> {
    if count > 65536 || (count != 0 && input.is_null()) {
        return None;
    }
    let entries = if count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(input, count as usize) }
    };
    let mut result = Vec::with_capacity(entries.len());
    for p in entries {
        let value = match p.kind {
            0xffff => None,
            0 => Some(Variant::Empty),
            6 => Some(Variant::Scalar(Elem::U32(p.number))),
            7 => Some(Variant::Scalar(Elem::I32(p.number as i32))),
            0x800e if p.length <= 1048576 && (p.length == 0 || !p.bytes.is_null()) => {
                let bytes = if p.length == 0 {
                    &[]
                } else {
                    unsafe { std::slice::from_raw_parts(p.bytes, p.length as usize) }
                };
                Some(Variant::string(bytes))
            }
            // Preserve unsupported typed entries at capture; never coerce them
            // into missing properties or claim a completed replay.
            _ => return None,
        };
        result.push(Property { key: p.key, value });
    }
    Some(result)
}

/// Output: five owner words, the defined scalar, then four area out-words. The callback compares
/// predicted calls to captured native effects and supplies foreign results only.
/// Safety: plugin-owned arrays/payloads remain live throughout this call.
#[no_mangle]
unsafe extern "C" fn fc4_legacyctl_shadow_run(
    before: *const u32,
    op: u32,
    args: *const u32,
    input: *const InputProperty,
    count: u32,
    exchange: Exchange,
    context: *mut c_void,
    output: *mut u32,
) -> i32 {
    if before.is_null() || args.is_null() || output.is_null() {
        return 0;
    }
    let b = unsafe { std::slice::from_raw_parts(before, 5) };
    let a = unsafe { std::slice::from_raw_parts(args, 4) };
    let Some(p) = (unsafe { properties(input, count) }) else {
        return 0;
    };
    let identity = |n: u32| (n != 0).then_some(u64::from(n));
    let state = OwnerState {
        refs: b[0],
        control: identity(b[1]),
        parent: identity(b[2]),
        procedure: identity(b[3]),
        own: b[4] != 0,
    };
    let mut supported = b[4] <= 1;
    let mut host = RecordingHost::new(|call: &Call| {
        let mut request = vec![call.op, call.object as u32, call.slot, call.arguments.len() as u32];
        for v in &call.arguments {
            if !encode(&mut request, v) {
                supported = false;
                return Value::None;
            }
        }
        let mut response = vec![0; 16384];
        let n = unsafe {
            exchange(
                context,
                request.as_ptr(),
                request.len() as u32,
                response.as_mut_ptr(),
                response.len() as u32,
            )
        } as usize;
        if n > response.len() {
            supported = false;
            return Value::None;
        }
        match decode(&response[..n]) {
            Some(v) => v,
            None => {
                supported = false;
                Value::None
            }
        }
    });
    let mut owner = state.owner();
    let mut area = std::array::from_fn(|i| a[i] as i32);
    let scalar = match op {
        0 => match replay_build(state, Properties(&p), &mut host) {
            Ok((after, value)) => {
                owner = after.owner();
                u32::from(value)
            }
            Err(_) => return 0,
        },
        1 => u32::from(owner.set_parent(identity(a[0]), &mut host)),
        2 => {
            owner.force_control(identity(a[0]), &mut host);
            0
        }
        3 => {
            owner.use_win_proc(identity(a[0]), &mut host);
            0
        }
        4 => match owner.get_id(&mut host) {
            Ok(n) => n,
            Err(_) => return 0,
        },
        5 => {
            owner.set_id(a[0], &mut host);
            0
        }
        6 => u32::from(owner.set_area(std::array::from_fn(|i| a[i] as i32), &mut host)),
        7 => match owner.get_area(&mut area, &mut host) {
            Ok(n) => u32::from(n),
            Err(_) => return 0,
        },
        8 => {
            owner.do_exceptions(u64::from(a[0]), a[1] != 0, &mut host);
            0
        }
        9 => {
            let mut fragment = None;
            match owner.get_script_fragment(&mut fragment, &mut host) {
                Ok(value) => {
                    if fragment
                        .as_ref()
                        .is_some_and(|f| f.coverage != FragmentCoverage::CompleteKnownDomain)
                    {
                        return 0;
                    }
                    u32::from(value)
                }
                Err(_) => return 0,
            }
        }
        10 => {
            if interpret_common(u64::from(a[0]), Properties(&p), a[1] != 0, &mut host).is_err() {
                return 0;
            }
            0
        }
        11 => {
            let g = fc4_core::legacy_control::observation::build_globals();
            let Some(&kind) = g.interpreters.get(&a[2]) else {
                return 0;
            };
            if interpret(kind, u64::from(a[0]), Properties(&p), a[1] != 0, &mut host).is_err() {
                return 0;
            }
            0
        }
        _ => return 0,
    };
    if !supported {
        return 0;
    }
    let after = OwnerState::capture(&owner);
    unsafe { std::slice::from_raw_parts_mut(output, 6) }.copy_from_slice(&[
        after.refs,
        after.control.unwrap_or(0) as u32,
        after.parent.unwrap_or(0) as u32,
        after.procedure.unwrap_or(0) as u32,
        u32::from(after.own),
        scalar,
    ]);
    unsafe { std::slice::from_raw_parts_mut(output.add(6), 4) }.copy_from_slice(&area.map(|v| v as u32));
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wire_rejects_bad_utf8_and_preserves_wrong_type_duplicate_and_null() {
        assert_eq!(decode(&[4, 1, 255]), None);
        assert_eq!(decode(&[3, 0x1234]), Some(Value::Handle(0x1234)));
        let input = [
            InputProperty {
                key: 4,
                kind: 7,
                number: 5,
                bytes: std::ptr::null(),
                length: 0,
            },
            InputProperty {
                key: 4,
                kind: 6,
                number: 6,
                bytes: std::ptr::null(),
                length: 0,
            },
            InputProperty {
                key: 5,
                kind: 0xffff,
                number: 0,
                bytes: std::ptr::null(),
                length: 0,
            },
        ];
        let props = unsafe { properties(input.as_ptr(), input.len() as u32) }.unwrap();
        assert_eq!(Properties(&props).number(4), Some(6));
        assert!(Properties(&props).has(5));
        assert_eq!(props.len(), 3);
    }
}

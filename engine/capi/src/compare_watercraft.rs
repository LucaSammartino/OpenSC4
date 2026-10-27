// SPDX-License-Identifier: GPL-3.0-or-later
//! Complete value-only watercraft bindings (watercraft §§3.1/3.3/3.4).
//! These entries have no outgoing object calls. Windows retain every byte,
//! including the heading, transform flags/counter and supplied output vectors.

use super::{Buffer, ResultValue};
use fc4_core::watercraft::{quantize_heading, sign, Transform, Watercraft, WatercraftRecord, CLASS_ID};

unsafe fn output(buffers: &mut [Buffer], base: u32, size: usize) -> Option<&mut [u8]> {
    let buffer = buffers.iter_mut().find(|b| b.base == base && b.offset == 0)?;
    if buffer.size as usize != size || buffer.data.is_null() {
        return None;
    }
    Some(std::slice::from_raw_parts_mut(buffer.data, size))
}

fn boat() -> Watercraft {
    Watercraft::new(
        WatercraftRecord {
            base: fc4_core::automata::AutomatonBase::new(),
        },
        None,
        0.0,
    )
}

unsafe fn receiver(buffers: &mut [Buffer], size: usize) -> Option<&mut [u8]> {
    let buffer = buffers.iter_mut().find(|b| b.base == 0 && b.offset == 0)?;
    if buffer.size as usize != size || buffer.data.is_null() {
        return None;
    }
    Some(std::slice::from_raw_parts_mut(buffer.data, size))
}

fn vector(bytes: &[u8], offset: usize) -> [f32; 3] {
    std::array::from_fn(|i| f32::from_ne_bytes(bytes[offset + i * 4..offset + i * 4 + 4].try_into().unwrap()))
}

fn put_vector(bytes: &mut [u8], offset: usize, value: [f32; 3]) {
    for (i, value) in value.into_iter().enumerate() {
        bytes[offset + i * 4..offset + i * 4 + 4].copy_from_slice(&value.to_ne_bytes());
    }
}

pub(super) unsafe fn adapt(entry: &str, args: &[u32; 16], buffers: &mut [Buffer], result: &mut ResultValue) -> bool {
    match entry {
        "watercraft-sign" => result.float_bits = sign(f32::from_bits(args[0])).to_bits(),
        "watercraft-clsid" => result.eax = CLASS_ID,
        "watercraft-direction" => {
            let Some(bytes) = receiver(buffers, 0x318) else {
                return false;
            };
            result.eax = quantize_heading(vector(bytes, 0x30c));
        }
        "watercraft-straight" => {
            let Some(bytes) = receiver(buffers, 0x358) else {
                return false;
            };
            let mut boat = boat();
            boat.position = vector(bytes, 0x2f4);
            boat.heading = vector(bytes, 0x30c);
            boat.speed = f32::from_ne_bytes(bytes[0x2dc..0x2e0].try_into().unwrap());
            boat.drive.0[4] = f32::from_ne_bytes(bytes[0x354..0x358].try_into().unwrap());
            let Some(bytes) = output(buffers, 2, 0x38) else {
                return false;
            };
            let mut transform = Transform {
                revision: bytes[0],
                flags: u32::from(bytes[1]),
                ..Transform::default()
            };
            let position = boat.integrate_straight(f32::from_bits(args[0]), &mut transform);
            bytes[0] = transform.revision;
            bytes[1] = transform.flags as u8;
            put_vector(bytes, 0x28, transform.translation);
            let Some(bytes) = output(buffers, 3, 12) else {
                return false;
            };
            put_vector(bytes, 0, position);
        }
        "watercraft-angular" => {
            // SPEC-GAP: WC13 native heading writes differ; retain all mismatches and claim no exact credit.
            let Some(bytes) = receiver(buffers, 0x64) else {
                return false;
            };
            let mut boat = Watercraft::new(
                WatercraftRecord {
                    base: fc4_core::automata::AutomatonBase::new(),
                },
                None,
                0.0,
            );
            boat.inherited_heading = vector(bytes, 0x58);
            result.eax = u32::from(boat.angular_proximity(std::array::from_fn(|i| f32::from_bits(args[i]))));
            put_vector(bytes, 0x58, boat.inherited_heading);
        }
        _ => return false,
    }
    true
}

pub(super) unsafe fn fixture(entry: &str, sample: u32, args: &mut [u32; 16], buffers: &mut [Buffer]) -> bool {
    // Include signed zeros, zero-length headings, vertical and non-unit vectors;
    // natural calls remain unrestricted. No mismatch is filtered or resynced.
    let headings: [[f32; 3]; 16] = [
        [0.0, 0.0, 0.0],
        [-0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, 0.0, -1.0],
        [1.0, 0.0, 1.0],
        [1.0, 0.0, -1.0],
        [-1.0, 0.0, 1.0],
        [-1.0, 0.0, -1.0],
        [0.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [1.0, 2.0, 3.0],
        [0.999, 0.0, 0.035],
        [0.999, 0.0, 0.034],
        [0.4, -0.2, 0.7],
        [-2.0, 0.5, 1.0],
    ];
    let heading = headings[sample as usize % headings.len()];
    match entry {
        "watercraft-sign" => args[0] = heading[0].to_bits(),
        "watercraft-clsid" => {}
        "watercraft-straight" => {
            args[0] = [0.0_f32, 100.0, -100.0, 17.25][sample as usize % 4].to_bits();
            if !buffers.is_empty() {
                let Some(bytes) = receiver(buffers, 0x358) else {
                    return false;
                };
                put_vector(bytes, 0x2f4, [10.0, 2.0, -20.0]);
                put_vector(bytes, 0x30c, heading);
                bytes[0x2dc..0x2e0].copy_from_slice(&[0.0_f32, 0.1, -0.1, 0.3][sample as usize % 4].to_ne_bytes());
                bytes[0x354..0x358].copy_from_slice(&0.2_f32.to_ne_bytes());
                let Some(bytes) = output(buffers, 2, 0x38) else {
                    return false;
                };
                bytes.fill(0x5a);
                bytes[0] = 254_u8.wrapping_add(sample as u8);
                bytes[1] = 0xa4;
                for (i, column) in Transform::default().basis.into_iter().enumerate() {
                    put_vector(bytes, 4 + i * 12, column);
                }
                put_vector(bytes, 0x28, [-5.0, 6.0, 7.0]);
                bytes[0x34..0x38].copy_from_slice(&2.0_f32.to_ne_bytes());
            }
        }
        "watercraft-direction" => {
            if !buffers.is_empty() {
                let Some(bytes) = receiver(buffers, 0x318) else {
                    return false;
                };
                put_vector(bytes, 0x30c, heading);
            }
        }
        "watercraft-angular" => {
            for (i, value) in [1.0_f32, 0.0, 0.0].into_iter().enumerate() {
                args[i] = value.to_bits();
            }
            if !buffers.is_empty() {
                let Some(bytes) = receiver(buffers, 0x64) else {
                    return false;
                };
                put_vector(bytes, 0x58, heading);
            }
        }
        _ => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn straight_binding_compares_both_outputs_flags_wrap_and_preserved_bytes() {
        let mut owner = [0xa5; 0x358];
        put_vector(&mut owner, 0x2f4, [10.0, 2.0, 20.0]);
        put_vector(&mut owner, 0x30c, [3.0, 4.0, 0.0]);
        owner[0x2dc..0x2e0].copy_from_slice(&0.1_f32.to_ne_bytes());
        owner[0x354..0x358].copy_from_slice(&0.2_f32.to_ne_bytes());
        let before = owner;
        let mut transform = [0x5a; 0x38];
        transform[0] = 255;
        transform[1] = 0xa4;
        let mut position = [0x7f; 12];
        let mut buffers = [
            Buffer {
                data: owner.as_mut_ptr(),
                size: 0x358,
                base: 0,
                offset: 0,
            },
            Buffer {
                data: transform.as_mut_ptr(),
                size: 0x38,
                base: 2,
                offset: 0,
            },
            Buffer {
                data: position.as_mut_ptr(),
                size: 12,
                base: 3,
                offset: 0,
            },
        ];
        let mut args = [0; 16];
        args[0] = (-100.0_f32).to_bits();
        let mut result = ResultValue::default();
        assert!(unsafe { adapt("watercraft-straight", &args, &mut buffers, &mut result) });
        assert_eq!(vector(&position, 0), [8.0, 2.0, 20.0]);
        assert_eq!(vector(&transform, 0x28), [8.0, 2.0, 20.0]);
        assert_eq!(&transform[..2], &[0, 0xa5]);
        assert_eq!(&transform[2..0x28], &[0x5a; 38]);
        assert_eq!(&transform[0x34..], &[0x5a; 4]);
        assert_eq!(owner, before);
        buffers[2].size = 8;
        assert!(!unsafe { adapt("watercraft-straight", &args, &mut buffers, &mut result) });
    }

    #[test]
    fn angular_binding_preserves_unrelated_bytes_and_publishes_normalized_heading() {
        let mut bytes = [0xa5; 0x64];
        put_vector(&mut bytes, 0x58, [2.0, 0.0, 0.0]);
        let mut buffers = [Buffer {
            data: bytes.as_mut_ptr(),
            size: 0x64,
            base: 0,
            offset: 0,
        }];
        let mut args = [0; 16];
        args[0] = 1.0_f32.to_bits();
        let mut result = ResultValue::default();
        assert!(unsafe { adapt("watercraft-angular", &args, &mut buffers, &mut result) });
        assert_eq!(result.eax, 1);
        assert_eq!(&bytes[..0x58], &[0xa5; 0x58]);
        assert_eq!(vector(&bytes, 0x58), [1.0, 0.0, 0.0]);
        buffers[0].size -= 1;
        assert!(!unsafe { adapt("watercraft-angular", &args, &mut buffers, &mut result) });
    }

    #[test]
    fn all_fixture_windows_are_accepted_without_touching_direction_receiver() {
        for sample in 0..16 {
            let mut bytes = [0x5a; 0x318];
            let mut buffers = [Buffer {
                data: bytes.as_mut_ptr(),
                size: 0x318,
                base: 0,
                offset: 0,
            }];
            let mut args = [0; 16];
            assert!(unsafe { fixture("watercraft-direction", sample, &mut args, &mut buffers) });
            let before = bytes;
            let mut result = ResultValue::default();
            assert!(unsafe { adapt("watercraft-direction", &args, &mut buffers, &mut result) });
            assert!(result.eax < 8);
            assert_eq!(bytes, before);
        }
    }
}

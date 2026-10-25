// SPDX-License-Identifier: GPL-3.0-or-later
//! Private monthly differentials for gz-framework-core sections 4 and 6.
use fc4_core::gz_framework::buffer::{Allocator, Buffer};

struct Storage;
impl Allocator for Storage {
    fn allocate(&mut self, bytes: usize) -> Option<Vec<u8>> {
        Some(vec![0xa5; bytes])
    }
}

fn buffer_trace(width: u32, height: u32, kind: u32, bpp: u32) -> [u32; 28] {
    let mut b = Buffer::default();
    let mut key = 0x12345678;
    let before_lock = b.lock(0x8040);
    let before_unlock = b.unlock(0x0800);
    let before_key = b.colour_key(&mut key).unwrap();
    let zero = b.init(&mut Storage, 0, height, kind, bpp).unwrap();
    let init = b.init(&mut Storage, width, height, kind, bpp).unwrap();
    let duplicate = b.init(&mut Storage, width + 1, height, kind, bpp).unwrap();
    let lock_a = b.lock(0x8040);
    let lock_b = b.lock(0x0800);
    let unlock_b = b.unlock(0x0800);
    let unlock_a = b.unlock(0x8040);
    let mut face = None;
    let qi = b.query_interface(0x20732180, &mut face);
    let primary = face == Some(fc4_core::gz_framework::framework::Face::Primary);
    let miss = b.query_interface(0xffff_fffe, &mut face);
    [
        before_lock.into(),
        before_unlock.into(),
        before_key.into(),
        key,
        zero.into(),
        init.into(),
        duplicate.into(),
        b.width(),
        b.height(),
        b.stride(),
        b.bytes_per_pixel(),
        b.colour_type().0,
        b.colour_type().1,
        0,
        0,
        b.width(),
        b.height(),
        lock_a.into(),
        lock_b.into(),
        unlock_b.into(),
        unlock_a.into(),
        b.map_rgb(0, 0, 0).unwrap(),
        b.map_rgb(255, 255, 255).unwrap(),
        b.map_rgb(17, 129, 241).unwrap(),
        qi.into(),
        primary.into(),
        miss.into(),
        (face == Some(fc4_core::gz_framework::framework::Face::Primary)).into(),
    ]
}

// These are static-library symbols, deliberately absent from fc4.h / DLL exports.
#[no_mangle]
unsafe extern "C" fn fc4_gzfw_buffer_compare(
    width: u32,
    height: u32,
    kind: u32,
    bpp: u32,
    observed: *const u32,
) -> u32 {
    if observed.is_null()
        || width == 0
        || width > 64
        || height == 0
        || height > 64
        || !matches!((kind, bpp), (2, 8) | (4, 16) | (5, 16) | (7, 16) | (9, 32))
    {
        return u32::MAX;
    }
    let expected = buffer_trace(width, height, kind, bpp);
    // SAFETY: plugin supplies exactly 28 readable u32 observations.
    let observed = unsafe { std::slice::from_raw_parts(observed, 28) };
    expected.iter().zip(observed).filter(|(a, b)| a != b).count() as u32
}

const COMMANDS: [(u32, &[u8]); 4] = [
    (0xABAC4EE4, b"ListCommands"),
    (0x2BB40833, b"SetDebugLevel"),
    (0xABAC067C, b"ExecuteScript"),
    (0xABAC26B5, b"ExecuteCheat"),
];

fn command_expected(text: &[u8], parse: bool) -> u32 {
    use fc4_core::commands::{server::Server, CommandServer};
    let server = Server::new(std::rc::Rc::new(fc4_formats::command_values::PropertyConvert));
    for (id, name) in COMMANDS {
        assert!(server.register(id, Some(name), None));
    }
    if parse {
        server.get_command_id_from_string(Some(text), text.len() as u32)
    } else {
        server.get_command_id(text)
    }
}

#[no_mangle]
unsafe extern "C" fn fc4_gzfw_command_expected(text: *const u8, length: u32, parse: u32) -> u32 {
    if text.is_null() || length > 256 {
        return u32::MAX;
    }
    // SAFETY: plugin supplies a readable counted byte string.
    command_expected(unsafe { std::slice::from_raw_parts(text, length as usize) }, parse != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_trace_detects_each_corrupted_observation() {
        for (kind, bpp) in [(2, 8), (4, 16), (5, 16), (7, 16), (9, 32)] {
            let trace = buffer_trace(11, 3, kind, bpp);
            assert_eq!(
                trace[9],
                if bpp == 8 {
                    12
                } else if bpp == 16 {
                    24
                } else {
                    44
                }
            );
            assert_eq!(unsafe { fc4_gzfw_buffer_compare(11, 3, kind, bpp, trace.as_ptr()) }, 0);
            for i in 0..trace.len() {
                let mut bad = trace;
                bad[i] ^= 1;
                assert_eq!(unsafe { fc4_gzfw_buffer_compare(11, 3, kind, bpp, bad.as_ptr()) }, 1);
            }
        }
        assert_eq!(
            unsafe { fc4_gzfw_buffer_compare(0, 3, 2, 8, std::ptr::null()) },
            u32::MAX
        );
    }

    #[test]
    fn command_probe_exercises_native_one_word_quirk() {
        assert_eq!(command_expected(b"listcommands", false), 0xABAC4EE4);
        assert_eq!(command_expected(b"\"ListCommands\"", true), 0xABAC4EE4);
        assert_eq!(command_expected(b"ListCommands argument", true), 0);
        assert_eq!(command_expected(b"fc4_nonexistent_command", false), 0);
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link transport. Expectations come only from the production
//! core's registered defaults, never a native snapshot. Explicit C-locale domain.
use fc4_core::render_properties::RenderProperties;

#[no_mangle]
extern "C" fn fc4_renderprops_shadow_count(kind: u32) -> i32 {
    let core = RenderProperties::new();
    match kind {
        0 => core.num_bool_properties(),
        1 => core.num_int_properties(),
        2 => core.num_float_properties(),
        _ => 0,
    }
}

fn query(kind: u32, index: usize, probe: u32) -> Option<(Vec<u8>, i32)> {
    let core = RenderProperties::new();
    let (name, short) = match kind {
        0 => {
            let p = core.bool_properties().get(index)?;
            (p.name_bytes(), p.short_name_bytes())
        }
        1 => {
            let p = core.int_properties().get(index)?;
            (p.name_bytes(), p.short_name_bytes())
        }
        2 => {
            let p = core.float_properties().get(index)?;
            (p.name_bytes(), p.short_name_bytes())
        }
        _ => return None,
    };
    let bytes = match probe {
        0 => name.to_vec(),
        1 => short.to_vec(),
        2 => name.to_ascii_uppercase(),
        3 => [name, b"\0ignored"].concat(),
        4 => [b"k", name].concat(),
        5 => [name, b" "].concat(),
        _ => return None,
    };
    let expected = match kind {
        0 => core.find_bool(&bytes),
        1 => core.find_int(&bytes),
        _ => core.find_float(&bytes),
    };
    Some((bytes, expected))
}

// No public DLL export. Caller supplies capacity writable bytes, including NUL.
#[no_mangle]
unsafe extern "C" fn fc4_renderprops_shadow_query(
    kind: u32,
    index: u32,
    probe: u32,
    out: *mut u8,
    capacity: u32,
) -> i32 {
    let Some((bytes, expected)) = query(kind, index as usize, probe) else {
        return i32::MIN;
    };
    if out.is_null() || bytes.len() >= capacity as usize {
        return i32::MIN;
    }
    // SAFETY: the private caller provides capacity writable bytes; checked above.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len());
        out.add(bytes.len()).write(0);
    }
    expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_preserve_nul_and_first_match_collisions() {
        for kind in 0..3 {
            for index in 0..fc4_renderprops_shadow_count(kind) as usize {
                for probe in 0..6 {
                    let (bytes, result) = query(kind, index, probe).unwrap();
                    assert!(bytes.len() < 128);
                    if probe == 0 || probe == 2 || probe == 3 {
                        assert_eq!(result, index as i32);
                    }
                    if probe >= 4 {
                        assert_eq!(result, -1);
                    }
                    if probe == 3 {
                        assert!(bytes.ends_with(b"\0ignored"));
                    }
                }
            }
        }
        assert!(query(0, 38, 0).is_none());
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link transport. Expectations come only from the production
//! core's registered defaults and explicit fixture inputs, never a native snapshot.
//! Explicit C-locale domain; live configured values require the RP7 observer bridge.
use fc4_core::render_properties::{InterfaceView, RenderProperties, RenderPropertiesOwner};

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

// Private opaque fixture state. Only the synchronous shadow thread owns this
// handle; there is no getter RefCell or shared mutable global state.
#[no_mangle]
extern "C" fn fc4_renderprops_fixture_new() -> *mut RenderPropertiesOwner {
    Box::into_raw(Box::new(RenderPropertiesOwner::default()))
}

#[no_mangle]
unsafe extern "C" fn fc4_renderprops_fixture_drop(owner: *mut RenderPropertiesOwner) {
    if !owner.is_null() {
        // SAFETY: one handle returned by fixture_new, dropped exactly once.
        drop(unsafe { Box::from_raw(owner) });
    }
}

#[no_mangle]
unsafe extern "C" fn fc4_renderprops_fixture_value(owner: *const RenderPropertiesOwner, kind: u32, index: i32) -> u32 {
    // SAFETY: private caller retains the new handle until fixture_drop.
    let Some(properties) = (unsafe { owner.as_ref() }).and_then(RenderPropertiesOwner::properties) else {
        return u32::MAX;
    };
    match kind {
        0 => properties.get_bool(index).map(u32::from),
        1 => properties.get_int(index).map(|v| v as u32),
        2 => properties.get_float(index).map(f32::to_bits),
        _ => None,
    }
    .unwrap_or(u32::MAX)
}

#[no_mangle]
unsafe extern "C" fn fc4_renderprops_fixture_set(
    owner: *mut RenderPropertiesOwner,
    kind: u32,
    index: i32,
    bits: u32,
) -> bool {
    // SAFETY: exclusive private fixture handle, no concurrent operation.
    let Some(properties) = (unsafe { owner.as_mut() }).and_then(RenderPropertiesOwner::properties_mut) else {
        return false;
    };
    match kind {
        0 => properties.set_bool(index, bits as u8 != 0),
        1 => properties.set_int(index, bits as i32),
        2 => properties.set_float(index, f32::from_bits(bits)),
        _ => false,
    }
}

#[no_mangle]
unsafe extern "C" fn fc4_renderprops_fixture_ref(
    owner: *mut RenderPropertiesOwner,
    view: u32,
    operation: u32,
    argument: u32,
) -> u32 {
    // SAFETY: exclusive private fixture handle, no concurrent operation.
    let Some(owner) = (unsafe { owner.as_mut() }) else {
        return u32::MAX;
    };
    let view = if view == 0 {
        InterfaceView::RenderProperties
    } else {
        InterfaceView::Unknown
    };
    match operation {
        0 => owner.reference_count(),
        1 => owner.add_ref(view),
        2 => owner.release(view),
        3 => {
            let mut output = None;
            owner.query_interface(view, argument, &mut output);
            output.map(InterfaceView::offset)
        }
        4 => owner
            .deleting_destructor(view, argument)
            .map(|d| d.returned_view.offset()),
        _ => None,
    }
    .unwrap_or(u32::MAX)
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

    #[test]
    fn fixture_replay_preserves_raw_finite_inputs_and_independent_lifetimes() {
        // SAFETY: distinct live handles, all indices valid, each dropped once.
        unsafe {
            let a = fc4_renderprops_fixture_new();
            let b = fc4_renderprops_fixture_new();
            let original = fc4_renderprops_fixture_value(b, 1, 23);
            assert!(fc4_renderprops_fixture_set(a, 1, 23, (-17_i32) as u32));
            assert_eq!(fc4_renderprops_fixture_value(a, 1, 23), (-17_i32) as u32);
            assert_eq!(fc4_renderprops_fixture_value(b, 1, 23), original);
            for bits in [0x8000_0000, 0x3fc0_0000, 0xc18a_0000] {
                assert!(fc4_renderprops_fixture_set(a, 2, 2, bits));
                assert_eq!(fc4_renderprops_fixture_value(a, 2, 2), bits);
            }
            assert_eq!(fc4_renderprops_fixture_ref(a, 0, 0, 0), 0);
            assert_eq!(fc4_renderprops_fixture_ref(a, 0, 3, 1), 4);
            assert_eq!(fc4_renderprops_fixture_ref(a, 1, 3, 1), 4);
            assert_eq!(fc4_renderprops_fixture_ref(a, 0, 3, 0), u32::MAX);
            assert_eq!(fc4_renderprops_fixture_ref(a, 0, 0, 0), 2);
            assert_eq!(fc4_renderprops_fixture_ref(a, 1, 2, 0), 1);
            assert_eq!(fc4_renderprops_fixture_ref(a, 1, 4, 2), 0);
            assert_eq!(fc4_renderprops_fixture_value(a, 2, 2), u32::MAX);
            assert_eq!(fc4_renderprops_fixture_value(b, 1, 23), original);
            fc4_renderprops_fixture_drop(a);
            fc4_renderprops_fixture_drop(b);
        }
    }
}

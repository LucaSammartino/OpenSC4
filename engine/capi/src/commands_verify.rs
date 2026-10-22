// SPDX-License-Identifier: GPL-3.0-or-later
//! Private ORIGINAL comparison transport. Retains production sets; no native
//! result is imported. Not part of the DLL's public C ABI.
use fc4_core::commands::{CommandParameterSet, CommandServer, SetRef, StringCommandGenerator, ValueConvert};
use fc4_core::variant::{Elem, Variant};
use std::rc::Rc;

#[repr(C)]
struct Snapshot {
    count: u32,
    first: u32,
    next: u32,
    present: u32,
    kind: u32,
    elements: u32,
    bits: u32,
    len: u32,
    text: [u8; 512],
    logical_elements: u32,
}

// Safety: private caller FC4CommandsVerify.h supplies live Box<SetRef> handles,
// bounded buffers, a single city-load thread and exactly one release per handle.
#[no_mangle]
unsafe extern "C" fn fc4_commands_verify_new() -> *mut SetRef {
    Box::into_raw(Box::new(CommandParameterSet::new_owned()))
}

#[no_mangle]
unsafe extern "C" fn fc4_commands_verify_release(h: *mut SetRef) {
    if !h.is_null() {
        let set = unsafe { Box::from_raw(h) };
        set.borrow_mut().release();
    }
}

#[no_mangle]
unsafe extern "C" fn fc4_commands_verify_mutate(
    h: *mut SetRef,
    op: u32,
    id: u32,
    kind: u32,
    bits: u32,
    text: *const u8,
    len: u32,
    src: *mut SetRef,
) -> u32 {
    if h.is_null() || len > 512 || (len != 0 && text.is_null()) {
        return 0;
    }
    let bytes = if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(text, len as usize) }
    };
    let v = match kind {
        0 => Variant::Empty,
        1 => Variant::Scalar(Elem::Bool(bits != 0)),
        6 => Variant::Scalar(Elem::U32(bits)),
        7 => Variant::Scalar(Elem::I32(bits as i32)),
        10 => Variant::Scalar(Elem::F32(f32::from_bits(bits))),
        0x800e => Variant::string(bytes),
        _ => return 0,
    };
    let mut set = unsafe { &*h }.borrow_mut();
    match op {
        1 => set.set_parameter(id, &v),
        2 => set.append_parameter(&v),
        3 => set.set_parameter_count(bits),
        4 => set.remove_parameter(id),
        5 => set.set_status_parameter_value(bits as i32),
        6 => *set.get_or_fallback_mut(id) = v,
        7 if !src.is_null() && src != h => {
            let source = unsafe { &*src }.borrow();
            let Some(v) = source.get_parameter(0) else { return 0 };
            set.set_parameter(id, v);
        }
        _ => return 0,
    }
    1
}

#[no_mangle]
unsafe extern "C" fn fc4_commands_verify_snapshot(h: *mut SetRef, id: u32, fallback: u32, out: *mut Snapshot) -> u32 {
    if h.is_null() || out.is_null() {
        return 0;
    }
    let set = unsafe { &*h }.borrow();
    let mut result = Snapshot {
        count: set.get_parameter_count(),
        first: set.get_first_parameter_id(),
        next: set.get_next_parameter_id(id),
        present: 0,
        kind: u32::MAX,
        elements: 0,
        bits: 0,
        len: 0,
        text: [0; 512],
        logical_elements: 0,
    };
    let v = if fallback != 0 {
        Some(set.get_native_or_fallback(id))
    } else {
        set.get_native_parameter(id)
    };
    if let Some(v) = v {
        result.present = 1;
        result.kind = u32::from(v.get_type());
        result.elements = v.get_count() as u32;
        let v = v.variant();
        result.logical_elements = v.count() as u32;
        match v {
            Variant::Scalar(Elem::Bool(b)) => result.bits = u32::from(*b),
            Variant::Scalar(Elem::U32(b)) => result.bits = *b,
            Variant::Scalar(Elem::I32(b)) => result.bits = *b as u32,
            Variant::Scalar(Elem::F32(b)) => result.bits = b.to_bits(),
            _ => {}
        }
        if v.mem_type() == 0x800e {
            let bytes = v.bytes().unwrap_or_default();
            if bytes.len() > result.text.len() {
                return 0;
            }
            result.len = bytes.len() as u32;
            result.text[..bytes.len()].copy_from_slice(&bytes);
        }
    }
    unsafe { out.write(result) };
    1
}

// Parse-only operations never call converting getters or execute commands.
// Refuse those callbacks; do not invent a second conversion implementation.
struct ParsingOnly;
impl ValueConvert for ParsingOnly {
    fn get_val_string(&self, _v: &Variant, _out: &mut Vec<u8>) -> bool {
        false
    }
    fn get_val_sint32(&self, _v: &Variant) -> Option<i32> {
        None
    }
}

// Fixed independent fixture from command-system.md section 0. Native lookup
// outputs cannot seed it. The plugin checks the app's prerequisite separately.
struct DocumentedCommand;
impl CommandServer for DocumentedCommand {
    fn get_command_id(&self, name: &[u8]) -> u32 {
        if name.eq_ignore_ascii_case(b"SetDebugLevel") {
            0x2bb40833
        } else {
            0
        }
    }
    fn get_command_name(&self, _id: u32) -> Option<Vec<u8>> {
        None
    }
    fn execute_command_id(&self, _id: u32, _input: Option<&SetRef>, _output: Option<&SetRef>) -> i32 {
        2
    }
}

#[no_mangle]
unsafe extern "C" fn fc4_commands_verify_parse(
    text: *const u8,
    available: u32,
    length: u32,
    id: *mut u32,
    output: *mut *mut SetRef,
) -> u32 {
    if text.is_null() || id.is_null() || output.is_null() || available > 8192 {
        return 0;
    }
    let bytes = unsafe { std::slice::from_raw_parts(text, available as usize) };
    let mut generator = StringCommandGenerator::new(Rc::new(ParsingOnly));
    generator.init(Some(Rc::new(DocumentedCommand)));
    let mut result_id = unsafe { *id };
    let mut result_set = None;
    let ok = generator.convert_string_to_command(bytes, length, &mut result_id, &mut result_set);
    unsafe { *id = result_id };
    if let Some(set) = result_set {
        unsafe { *output = Box::into_raw(Box::new(set)) };
    }
    generator.shutdown();
    u32::from(ok)
}

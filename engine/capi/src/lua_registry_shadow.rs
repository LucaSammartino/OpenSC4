// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link collector/registration snapshot comparison, not VM replay.
//! lua-host.md §8; native callback addresses are opaque foreign identities.
use fc4_core::lua_script_server::collectors::{self, Function, RegistrationVm};
use std::collections::BTreeMap;

#[repr(C)]
#[derive(Clone, Copy)]
struct Entry {
    name: [u8; 80],
    function: u32,
    is_global: u32,
    registered: u32,
}

#[repr(C)]
#[derive(Default, Debug)]
struct Comparison {
    entries: u32,
    collector_diffs: u32,
    binding_diffs: u32,
    scratch_diffs: u32,
    stack_diffs: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    Sentinel,
    Table(bool),
    String(String),
    Function(u32),
}

struct Replay {
    bindings: BTreeMap<&'static str, u32>,
    globals: BTreeMap<String, Value>,
    game: BTreeMap<String, Value>,
    stack: Vec<Value>,
}

impl RegistrationVm for Replay {
    type Error = ();
    fn top(&mut self) -> Result<i32, ()> {
        Ok(self.stack.len() as i32)
    }
    fn get_globals(&mut self) -> Result<(), ()> {
        self.stack.push(Value::Table(true));
        Ok(())
    }
    fn get_global(&mut self, name: &str) -> Result<(), ()> {
        self.stack.push(self.globals.get(name).ok_or(())?.clone());
        Ok(())
    }
    fn register(&mut self, function: Function, name: &str) -> Result<(), ()> {
        let token = *self.bindings.get(function.name()).ok_or(())?;
        self.globals.insert(name.into(), Value::Function(token));
        Ok(())
    }
    fn push_string(&mut self, value: &str) -> Result<(), ()> {
        self.stack.push(Value::String(value.into()));
        Ok(())
    }
    fn set_table(&mut self, index: i32) -> Result<(), ()> {
        let at = if index < 0 {
            (self.stack.len() as i32 + index) as usize
        } else {
            index as usize - 1
        };
        let Value::Table(global) = self.stack.get(at).ok_or(())? else {
            return Err(());
        };
        let global = *global;
        let value = self.stack.pop().ok_or(())?;
        let Value::String(key) = self.stack.pop().ok_or(())? else {
            return Err(());
        };
        if global { &mut self.globals } else { &mut self.game }.insert(key, value);
        Ok(())
    }
    fn pop(&mut self, count: i32) -> Result<(), ()> {
        let len = self.stack.len().checked_sub(count as usize).ok_or(())?;
        self.stack.truncate(len);
        Ok(())
    }
    fn set_top(&mut self, top: i32) -> Result<(), ()> {
        if top < 0 || top as usize > self.stack.len() {
            return Err(());
        }
        self.stack.truncate(top as usize);
        Ok(())
    }
}

fn compare(entries: &[Entry], scratch: u32) -> Comparison {
    let expected = collectors::extensions();
    let mut result = Comparison {
        entries: entries.len() as u32,
        ..Comparison::default()
    };
    if entries.len() != expected.len() {
        result.collector_diffs += 1;
        return result;
    }
    let mut vm = Replay {
        bindings: BTreeMap::new(),
        globals: BTreeMap::from([("game".into(), Value::Table(false))]),
        game: BTreeMap::new(),
        stack: vec![Value::Sentinel],
    };
    for (original, entry) in entries.iter().zip(expected) {
        let end = original.name.iter().position(|b| *b == 0);
        result.collector_diffs += u32::from(end.map(|n| &original.name[..n]) != Some(entry.name.as_bytes()));
        result.collector_diffs += u32::from(original.is_global != u32::from(entry.is_global));
        result.collector_diffs += u32::from(original.function == 0);
        // Bind the core's symbolic callback to the recorded foreign identity.
        // A name/order/flag discrepancy is already a mismatch, never normalized away.
        vm.bindings.insert(entry.function.name(), original.function);
    }
    for global in [true, false] {
        if collectors::register_api(&mut vm, global) != Ok(true) {
            result.binding_diffs += 1;
            return result;
        }
        result.stack_diffs += u32::from(vm.stack != [Value::Sentinel]);
        // §8.6: each pass has its own scratch binding (print, then window_graph).
        // These are model stages; the native snapshot is after both passes.
        if let Some(last) = expected.iter().rfind(|e| e.is_global == global) {
            result.scratch_diffs +=
                u32::from(vm.globals.get("_tf_") != Some(&Value::Function(vm.bindings[last.function.name()])));
        }
    }
    for (original, entry) in entries.iter().zip(expected) {
        let table = if entry.is_global { &vm.globals } else { &vm.game };
        result.binding_diffs += u32::from(table.get(entry.name) != Some(&Value::Function(original.registered)));
    }
    result.scratch_diffs += u32::from(vm.globals.get("_tf_") != Some(&Value::Function(scratch)));
    result
}

// SAFETY: synchronous private caller supplies `len` initialized entries and one
// writable Comparison. Native pointers are numeric tokens, never dereferenced.
#[no_mangle]
unsafe extern "C" fn fc4_lua_registry_shadow_compare(
    entries: *const Entry,
    len: usize,
    scratch: u32,
    result: *mut Comparison,
) -> bool {
    if entries.is_null() || result.is_null() || len > 128 {
        return false;
    }
    let entries = unsafe { std::slice::from_raw_parts(entries, len) };
    unsafe {
        *result = compare(entries, scratch);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capture() -> Vec<Entry> {
        collectors::extensions()
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let mut entry = Entry {
                    name: [0; 80],
                    function: 100 + i as u32,
                    is_global: u32::from(e.is_global),
                    registered: 100 + i as u32,
                };
                entry.name[..e.name.len()].copy_from_slice(e.name.as_bytes());
                entry
            })
            .collect()
    }
    fn mismatches(r: Comparison) -> u32 {
        r.collector_diffs + r.binding_diffs + r.scratch_diffs + r.stack_diffs
    }
    #[test]
    fn original_collector_bindings_and_scratch_match_core_registration() {
        assert_eq!(mismatches(compare(&capture(), 146)), 0);
    }
    #[test]
    fn altered_binding_scratch_and_metadata_are_counted_independently() {
        let mut entries = capture();
        entries[0].name[0] = b'X';
        entries[1].is_global = 1;
        entries[2].registered = entries[3].function;
        let r = compare(&entries, 100);
        assert_eq!(r.collector_diffs, 2);
        assert_eq!(r.binding_diffs, 1);
        assert_eq!(r.scratch_diffs, 1);
        assert_eq!(r.stack_diffs, 0);
    }
    #[test]
    fn missing_entries_null_callbacks_and_unterminated_names_fail() {
        assert_ne!(mismatches(compare(&capture()[..46], 146)), 0);
        let mut entries = capture();
        entries[0].name.fill(b'X');
        entries[1].function = 0;
        assert_eq!(mismatches(compare(&entries, 146)), 3);
    }
    #[test]
    fn collector_order_and_missing_c_bindings_are_not_excluded() {
        let mut entries = capture();
        entries.swap(0, 1);
        assert_eq!(compare(&entries, 146).collector_diffs, 2);
        let mut entries = capture();
        entries[0].registered = 0;
        let r = compare(&entries, 0);
        assert_eq!(r.binding_diffs, 1);
        assert_eq!(r.scratch_diffs, 1);
    }
    #[test]
    fn actual_callback_identities_are_never_rebased_for_comparison() {
        let mut entries = capture();
        // Opaque same-process inputs can share an entry point; no invented
        // uniqueness or executable-address constraint is applied to them.
        for entry in &mut entries {
            entry.function = 0x0060_c890;
            entry.registered = entry.function;
        }
        assert_eq!(mismatches(compare(&entries, 0x0060_c890)), 0);
        // A preferred evidence label is not the actual pointer (LH7-G).
        entries[0].registered = 0x0040_c890;
        assert_eq!(compare(&entries, 0x0060_c890).binding_diffs, 1);
        assert_eq!(compare(&entries, 0x0040_c890).scratch_diffs, 1);
    }
    #[test]
    fn private_transport_rejects_invalid_pointers_and_lengths() {
        let entries = capture();
        let mut out = Comparison::default();
        unsafe {
            assert!(!fc4_lua_registry_shadow_compare(std::ptr::null(), 47, 0, &mut out));
            assert!(!fc4_lua_registry_shadow_compare(entries.as_ptr(), 129, 0, &mut out));
            assert!(!fc4_lua_registry_shadow_compare(
                entries.as_ptr(),
                47,
                0,
                std::ptr::null_mut()
            ));
            assert!(fc4_lua_registry_shadow_compare(entries.as_ptr(), 47, 146, &mut out));
        }
        assert_eq!(mismatches(out), 0);
    }
}

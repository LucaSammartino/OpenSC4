// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

struct Minimal;
impl Foreign for Minimal {
    fn invoke(&mut self, _: &mut ScriptServer, call: Primitive) -> Result<Value> {
        Ok(match call {
            Primitive::FindMessageService { .. } => Value::Handle(None),
            Primitive::ThreadId(_) => Value::Int(7),
            Primitive::FrameworkAvailable => Value::Bool(false),
            _ => Value::Unit,
        })
    }
}

fn seeded() -> ScriptServer {
    let mut s = ScriptServer::new();
    let mut r = Record::new(RecordToken(1), b"s", b"n");
    r.thread = Some(Handle(7));
    s.records.insert(7, r);
    s
}

#[test]
fn counters_use_32_bit_wrap_and_signed_pause_comparison() {
    let mut s = seeded();
    let r = s.records.get_mut(&7).unwrap();
    r.pause_count = i32::MAX;
    assert_eq!(s.pause_script(&mut Minimal, 7), Ok(i32::MIN));
    assert_eq!(s.resume_script(&mut Minimal, 7), Ok(i32::MAX));
    let r = s.records.get_mut(&7).unwrap();
    r.pause_count = 1;
    r.resume_transitions = u32::MAX;
    assert_eq!(s.resume_script(&mut Minimal, 7), Ok(0));
    assert_eq!(s.records[&7].resume_transitions, 0);
    s.references = u32::MAX;
    assert_eq!(s.add_ref(), Ok(0));
}

#[test]
fn absent_thread_argument_setup_has_no_stack_effects() {
    let mut s = ScriptServer::new();
    let r = Record::new(RecordToken(1), b"", b"");
    assert_eq!(s.setup_thread_arguments(&mut Minimal, Some(&r)), Ok(false));
}

#[test]
fn null_argument_record_is_a_defined_false_result() {
    let mut s = ScriptServer::new();
    assert_eq!(s.setup_thread_arguments(&mut Minimal, None), Ok(false));
}

#[test]
fn factory_allocation_failure_and_zero_reference_construction() {
    assert!(ScriptServer::factory(false).is_none());
    let s = ScriptServer::factory(true).unwrap();
    assert_eq!(s.ref_count(), 0);
    assert_eq!(s.get_last_created_script_id(), -1);
    assert_eq!(s.get_lua(), None);
    assert!(!s.init_latched());
}

#[test]
fn non_schedulable_record_is_not_even_status_checked() {
    let mut s = seeded();
    assert_eq!(s.on_tick(&mut Minimal), Ok(true));
    assert!(s.record(7).is_some());
}

#[test]
fn default_breakpoint_and_record_payloads() {
    assert_eq!(
        Breakpoint::default(),
        Breakpoint {
            id: -1,
            name: vec![],
            line: -1,
            enabled: 1,
            condition: vec![]
        }
    );
    let s = seeded();
    let r = s.record(7).unwrap();
    assert_eq!(
        (
            r.pause_count,
            r.resume_transitions,
            r.schedulable,
            r.auto_yield,
            r.last_line
        ),
        (0, 0, false, false, -1)
    );
}

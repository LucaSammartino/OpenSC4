// SPDX-License-Identifier: GPL-3.0-or-later
//! The 31 independent public vectors in lua-host.md §6.3.3/.5/.7, in table order.
//! Inputs are foreign responses; expectations are literal public-contract results.
use fc4_core::lua_script_server::*;
#[path = "support/lua_script_server.rs"]
mod support;
use support::Mock;

fn previous_attempts(s: &mut ScriptServer, h: &mut Mock, count: u32) {
    for _ in 0..count { h.create(s, 40); assert_eq!(s.kill_script(h, 40), Ok(true)); }
    h.clear();
}

#[test]
fn v01_init_failure_latches_without_factory_retry() {
    let mut s = ScriptServer::new();
    let mut h = Mock::default();
    h.lua_results.push_back(None);
    assert_eq!(s.init(&mut h), Ok(false));
    assert_eq!(s.init(&mut h), Ok(false));
    assert!(s.init_latched());
    assert_eq!(h.count(|c| matches!(c, Primitive::CreateLua { .. })), 1);
    assert_eq!(h.calls.len(), 1);
}

#[test]
fn v02_shutdown_allows_ordered_init_retry() {
    let mut s = ScriptServer::new();
    let mut h = Mock::default();
    h.lua_results.push_back(None);
    assert_eq!(s.init(&mut h), Ok(false));
    assert_eq!(s.shutdown(&mut h), Ok(true));
    h.clear();
    assert_eq!(s.init(&mut h), Ok(true));
    let open = h.position(|c| matches!(c, Primitive::Open { libraries: 0x7e, .. }));
    let generator = h.position(|c| matches!(c, Primitive::InitGenerator { global: "sc4app", .. }));
    let register = h.position(|c| matches!(c, Primitive::RegisterService));
    let args = h.position(|c| matches!(c, Primitive::Stack { op: StackOp::NewUserdata(1), .. }));
    assert!(open < generator && generator < register && register < args);
}

#[test]
fn v03_resume_from_zero_is_negative_without_notification() {
    let (mut s, mut h) = Mock::with_script(7);
    assert_eq!(s.resume_script(&mut h, 7), Ok(-1));
    assert_eq!(s.record(7).unwrap().resume_transitions, 0);
    assert!(h.calls.is_empty());
}

#[test]
fn v04_pause_notifies_only_transition_to_one() {
    let (mut s, mut h) = Mock::with_script(7);
    s.resume_script(&mut h, 7).unwrap();
    assert_eq!(s.pause_script(&mut h, 7), Ok(0));
    assert_eq!(s.pause_script(&mut h, 7), Ok(1));
    assert_eq!(s.pause_script(&mut h, 7), Ok(2));
    assert_eq!(h.sent, [(PAUSE_CHANGED, 7, Some(1))]);
    assert_eq!(h.observations[0].2, Some(1));
}

#[test]
fn v05_resume_notification_precedes_transition_counter_increment() {
    let (mut s, mut h) = Mock::with_script(7);
    for _ in 0..4 { s.pause_script(&mut h, 7).unwrap(); s.resume_script(&mut h, 7).unwrap(); }
    s.pause_script(&mut h, 7).unwrap(); s.pause_script(&mut h, 7).unwrap();
    h.clear();
    assert_eq!(s.resume_script(&mut h, 7), Ok(1));
    assert_eq!(s.resume_script(&mut h, 7), Ok(0));
    assert_eq!(h.sent, [(PAUSE_CHANGED, 7, Some(0))]);
    assert_eq!(h.observations, [(PAUSE_CHANGED, 7, Some(0), Some(4))]);
    assert_eq!(s.record(7).unwrap().resume_transitions, 5);
}

#[test]
fn v06_enumeration_appends_in_signed_order() {
    let (mut s, mut h) = Mock::initialized();
    h.create(&mut s, 8); h.create(&mut s, 2);
    let mut ids = vec![99];
    s.enumerate_script_ids(&mut ids);
    assert_eq!(ids, [99, 2, 8]);
}

#[test]
fn v07_missing_info_preserves_every_optional_output() {
    let (mut s, mut h) = Mock::initialized();
    let (mut t, mut src, mut name, mut p, mut r, mut eligible) = (Some(Handle(999)), vec![1], vec![2], 3, 4, true);
    assert_eq!(s.get_script_info(&mut h, 7, InfoOutputs {
        thread: Some(&mut t), source: Some(&mut src), name: Some(&mut name),
        pause_count: Some(&mut p), resume_transitions: Some(&mut r), schedulable: Some(&mut eligible)
    }), Ok(false));
    assert_eq!((t, src, name, p, r, eligible), (Some(Handle(999)), vec![1], vec![2], 3, 4, true));
    assert!(!s.get_script_auto_yield_enabled(7));
    assert!(!s.set_script_auto_yield_enabled(7, true));
    assert!(h.calls.is_empty());
}

#[test]
fn v08_lua_is_borrowed_and_info_thread_is_acquired() {
    let (mut s, mut h) = Mock::with_script(7);
    assert_eq!(s.get_lua(), Some(Handle(1)));
    assert!(h.calls.is_empty());
    let mut t = None;
    assert_eq!(s.get_script_info(&mut h, 7, InfoOutputs { thread: Some(&mut t), ..InfoOutputs::default() }), Ok(true));
    assert_eq!(t, Some(Handle(1007)));
    assert_eq!(h.calls, [Primitive::AddRef { interface: Interface::Thread, handle: Handle(1007) }]);
}

#[test]
fn v09_compile_failure_consumes_display_serial_only() {
    let (mut s, mut h) = Mock::initialized();
    previous_attempts(&mut s, &mut h, 8);
    h.compile.push_back(3);
    assert_eq!(s.run_script_string(&mut h, b"bad", b"job", b""), Ok(-1));
    assert_eq!(s.attempt_serial(), 9);
    assert_eq!(s.get_last_created_script_id(), 40);
    assert_eq!(h.stack_ops(Handle(10)), [StackOp::Pop(1)]);
    assert_eq!(h.count(|c| matches!(c, Primitive::CreateThread { .. } | Primitive::FindMessageService { .. })), 0);
}

#[test]
fn v10_success_uses_vm_id_and_installs_hook_before_notification() {
    let (mut s, mut h) = Mock::initialized();
    previous_attempts(&mut s, &mut h, 9);
    h.created.push_back((71, Some(Handle(171))));
    h.hook(|c| matches!(c, Primitive::InstallLineHook { .. }), |_, owner| {
        let r = owner.record(71).unwrap();
        assert!(r.schedulable);
        assert_eq!(owner.get_last_created_script_id(), 71);
    });
    assert_eq!(s.run_script_string(&mut h, b"source", b"job", b""), Ok(71));
    assert_eq!(s.attempt_serial(), 10);
    assert_eq!(s.get_last_created_script_id(), 71);
    assert_eq!(h.stack_ops(Handle(10)), [StackOp::PushString(b"Main - \"job\" - 10".to_vec()), StackOp::PushValue(-2), StackOp::RawSet(GLOBALS)]);
    assert!(h.position(|c| matches!(c, Primitive::SetUserContext { .. })) < h.position(|c| matches!(c, Primitive::InstallLineHook { mask: 4, count: 0, .. })));
    assert!(h.position(|c| matches!(c, Primitive::InstallLineHook { .. })) < h.position(|c| matches!(c, Primitive::SendMessage { .. })));
    assert_eq!(h.sent, [(CREATED, 71, Some(0))]);
}

#[test]
fn v11_file_read_failure_returns_zero_without_attempt() {
    let (mut s, mut h) = Mock::with_script(40);
    assert_eq!(s.run_script_file(&mut h, b"missing", b"args"), Ok(0));
    assert_eq!(s.attempt_serial(), 1);
    assert_eq!(s.get_last_created_script_id(), 40);
    assert_eq!(h.calls, [Primitive::ReadFile { path: b"missing".to_vec(), limit: 1_000_000 }]);
}

#[test]
fn v12_negative_pause_runs_and_resume_result_is_ignored() {
    let (mut s, mut h) = Mock::with_script(7);
    s.resume_script(&mut h, 7).unwrap(); s.resume_script(&mut h, 7).unwrap();
    h.status(&s, 7, &[1, 1]); h.resume_result = false;
    h.hook(|c| matches!(c, Primitive::ResumeThread { .. }), |host, owner| {
        assert_eq!(owner.get_current_running_script_id(host), Ok(7));
    });
    assert_eq!(s.on_tick(&mut h), Ok(true));
    assert!(s.record(7).is_some());
    assert_eq!(h.count(|c| matches!(c, Primitive::ResumeThread { lua: Handle(1), id: 7, count: 0, arguments: None })), 1);
    assert_eq!(s.get_current_running_script_id(&mut h), Ok(-1));
}

#[test]
fn v13_tick_terminates_zero_status_and_skips_paused_record() {
    let (mut s, mut h) = Mock::with_script(2);
    h.create(&mut s, 8); s.pause_script(&mut h, 8).unwrap();
    h.clear(); h.status(&s, 2, &[0, 0]);
    assert_eq!(s.on_tick(&mut h), Ok(true));
    assert_eq!(h.sent, [(TERMINATED, 2, Some(0))]);
    assert!(h.position(|c| matches!(c, Primitive::SendMessage { .. })) < h.position(|c| matches!(c, Primitive::DestroyThread { id: 2, .. })));
    assert!(h.position(|c| matches!(c, Primitive::DestroyThread { .. })) < h.position(|c| matches!(c, Primitive::Release { interface: Interface::Thread, .. })));
    assert_eq!(h.count(|c| matches!(c, Primitive::Status(Handle(1008)))), 0);
    assert!(s.record(2).is_none()); assert!(s.record(8).is_some());
}

#[test]
fn v14_nonzero_terminal_status_does_not_destroy_vm_thread() {
    let (mut s, mut h) = Mock::with_script(7);
    h.status(&s, 7, &[2, 2]);
    assert_eq!(s.on_tick(&mut h), Ok(true));
    assert_eq!(h.sent, [(TERMINATED, 7, Some(0))]);
    assert_eq!(h.count(|c| matches!(c, Primitive::DestroyThread { .. })), 0);
    assert!(s.record(7).is_none());
}

#[test]
fn v15_nested_tick_performs_no_scheduling() {
    let (mut s, mut h) = Mock::with_script(7);
    h.hook(|c| matches!(c, Primitive::ResumeThread { .. }), |host, owner| {
        assert_eq!(owner.tick_depth(), 1);
        let before = host.calls.len();
        assert_eq!(owner.on_tick(host), Ok(true));
        assert_eq!(owner.tick_depth(), 1);
        assert_eq!(host.calls.len(), before);
    });
    assert_eq!(s.on_tick(&mut h), Ok(true));
    assert_eq!(s.tick_depth(), 0);
    assert_eq!(h.count(|c| matches!(c, Primitive::Status(_))), 2);
    assert_eq!(h.count(|c| matches!(c, Primitive::ResumeThread { .. })), 1);
}

#[test]
fn v16_argument_table_uses_original_tokens_and_next_index_count() {
    let (mut s, mut h) = Mock::initialized();
    h.tokens = ["--fast", "/n:12", ":x", "---", "count:9"].iter().map(|x| x.as_bytes().to_vec()).collect();
    h.created.push_back((7, Some(Handle(107))));
    assert_eq!(s.run_script_string(&mut h, b"", b"job", b"foreign parser input"), Ok(7));
    use StackOp::*;
    assert_eq!(h.stack_ops(Handle(107)), [
        PushLightUserdata(Handle(107)), NewTable,
        PushNumber(0.0), PushString(b"job".to_vec()), RawSet(-3),
        PushString(b"fast".to_vec()), PushBoolean(true), RawSet(-3),
        PushNumber(1.0), PushString(b"--fast".to_vec()), RawSet(-3),
        PushString(b"n".to_vec()), PushString(b"12".to_vec()), RawSet(-3),
        PushNumber(2.0), PushString(b"/n:12".to_vec()), RawSet(-3),
        PushString(vec![]), PushString(b"x".to_vec()), RawSet(-3),
        PushNumber(3.0), PushString(b":x".to_vec()), RawSet(-3),
        PushString(b"count".to_vec()), PushString(b"9".to_vec()), RawSet(-3),
        PushNumber(4.0), PushString(b"count:9".to_vec()), RawSet(-3),
        PushString(b"count".to_vec()), PushNumber(5.0), RawSet(-3), RawSet(REGISTRY)
    ]);
}

#[test]
fn v17_auto_yield_only_on_changed_line() {
    let (mut s, mut h) = Mock::with_script(7);
    assert!(s.set_script_auto_yield_enabled(7, true));
    h.lines.push_back(4); assert_eq!(s.line_hook(&mut h, Handle(77)), Ok(true));
    h.clear(); h.lines.extend([4, 5, 5, 4]);
    let mut results = vec![]; let mut saved = vec![];
    for _ in 0..4 { results.push(s.line_hook(&mut h, Handle(77)).unwrap()); saved.push(s.record(7).unwrap().last_line); }
    assert_eq!(results, [false, true, false, true]);
    assert_eq!(saved, [4, 5, 5, 4]);
    assert_eq!(h.count(|c| matches!(c, Primitive::Yield { results: 0, .. })), 2);
}

#[test]
fn v18_create_exists_twice_notifies_once() {
    let (mut s, mut h) = Mock::with_script(7);
    for _ in 0..2 { assert_eq!(s.set_breakpoint_exists(&mut h, 7, b"name", 10, false), Ok(true)); }
    assert_eq!(s.record(7).unwrap().breakpoints.len(), 1);
    assert_eq!(s.record(7).unwrap().breakpoints[0].enabled, 1);
    assert_eq!(h.sent, [(BREAKPOINT_CHANGED, 7, None)]);
    assert_eq!(h.count(|c| matches!(c, Primitive::MessagePayload { .. })), 0);
}

#[test]
fn v19_exists_enables_existing_entry_silently() {
    let (mut s, mut h) = Mock::with_script(7);
    s.set_breakpoint_enabled(&mut h, 7, b"name", 10, 0).unwrap(); h.clear();
    assert_eq!(s.set_breakpoint_exists(&mut h, 7, b"NAME", 10, false), Ok(true));
    assert_eq!(s.record(7).unwrap().breakpoints[0].enabled, 1);
    assert!(h.sent.is_empty());
}

#[test]
fn v20_remove_exists_twice_notifies_only_first() {
    let (mut s, mut h) = Mock::with_script(7);
    s.set_breakpoint_exists(&mut h, 7, b"name", 10, false).unwrap(); h.clear();
    for _ in 0..2 { assert_eq!(s.set_breakpoint_exists(&mut h, 7, b"name", 10, true), Ok(true)); }
    assert!(s.record(7).unwrap().breakpoints.is_empty());
    assert_eq!(h.sent, [(BREAKPOINT_CHANGED, 7, None)]);
}

#[test]
fn v21_setting_disabled_creates_entry_and_notifies_once() {
    let (mut s, mut h) = Mock::with_script(7);
    assert_eq!(s.set_breakpoint_enabled(&mut h, 7, b"name", 10, 0), Ok(true));
    assert_eq!(s.get_breakpoint_enabled(&mut h, 7, b"name", 10), Ok(false));
    assert_eq!(s.get_breakpoint_exists(&mut h, 7, b"name", 10), Ok(true));
    assert_eq!(h.sent, [(BREAKPOINT_CHANGED, 7, None)]);
}

#[test]
fn v22_disabled_breakpoint_still_blocks_scheduler_without_control_notification() {
    let (mut s, mut h) = Mock::with_script(7);
    s.set_breakpoint_enabled(&mut h, 7, b"JOB", 10, 0).unwrap(); h.clear();
    h.debug = (b"job".to_vec(), 10);
    assert_eq!(s.should_break(&mut h, 7), Ok(true));
    h.clear(); assert_eq!(s.on_tick(&mut h), Ok(true));
    assert_eq!(h.count(|c| matches!(c, Primitive::ResumeThread { .. })), 0);
    assert!(h.sent.is_empty());
    assert_eq!(s.record(7).unwrap().pause_count, 0);
}

#[test]
fn v23_unequal_condition_on_existing_entry_is_ignored() {
    let (mut s, mut h) = Mock::with_script(7);
    s.set_breakpoint_exists(&mut h, 7, b"name", 10, false).unwrap(); h.clear();
    assert_eq!(s.set_breakpoint_condition(&mut h, 7, b"name", 10, b"x > 0"), Ok(true));
    assert!(s.record(7).unwrap().breakpoints[0].condition.is_empty());
    assert!(h.sent.is_empty());
}

#[test]
fn v24_unequal_condition_on_new_entry_only_notifies_creation() {
    let (mut s, mut h) = Mock::with_script(7);
    assert_eq!(s.set_breakpoint_condition(&mut h, 7, b"name", 10, b"x > 0"), Ok(true));
    assert!(s.record(7).unwrap().breakpoints[0].condition.is_empty());
    assert_eq!(s.record(7).unwrap().breakpoints[0].enabled, 1);
    assert_eq!(h.sent, [(BREAKPOINT_CHANGED, 7, None)]);
}

#[test]
fn v25_equal_empty_condition_copies_and_notifies() {
    let (mut s, mut h) = Mock::with_script(7);
    s.set_breakpoint_exists(&mut h, 7, b"name", 10, false).unwrap(); h.clear();
    assert_eq!(s.set_breakpoint_condition(&mut h, 7, b"name", 10, b""), Ok(true));
    assert_eq!(h.sent, [(BREAKPOINT_CHANGED, 7, None)]);
}

#[test]
fn v26_condition_output_is_cleared_on_either_miss() {
    let (mut s, mut h) = Mock::with_script(7);
    for id in [99, 7] {
        let mut output = b"old".to_vec();
        assert_eq!(s.get_breakpoint_condition(&mut h, id, b"name", 10, &mut output), Ok(false));
        assert!(output.is_empty());
    }
}

#[test]
fn v27_list_output_is_cleared_on_missing_record() {
    let (mut s, mut h) = Mock::initialized();
    let mut output = vec![Breakpoint { condition: b"owned old".to_vec(), ..Breakpoint::default() }];
    assert_eq!(s.get_breakpoint_entry_list(&mut h, 7, b"name", &mut output), Ok(false));
    assert!(output.is_empty());
}

#[test]
fn v28_list_filter_keeps_insertion_order_and_deep_copies() {
    let (mut s, mut h) = Mock::with_script(7);
    for (name, line) in [(b"A", 30), (b"a", 10), (b"B", 20)] { s.set_breakpoint_exists(&mut h, 7, name, line, false).unwrap(); }
    let mut output = vec![];
    assert_eq!(s.get_breakpoint_entry_list(&mut h, 7, b"a", &mut output), Ok(true));
    assert_eq!(output.iter().map(|b| b.line).collect::<Vec<_>>(), [30, 10]);
    assert_eq!(output[0].id, 7);
    output[0].name.push(b'!'); output[0].condition.push(b'!');
    assert_eq!(s.record(7).unwrap().breakpoints[0].name, b"A");
    assert!(s.record(7).unwrap().breakpoints[0].condition.is_empty());
}

#[test]
fn v29_missing_message_service_preserves_mutations_without_allocating() {
    let (mut s, mut h) = Mock::initialized(); h.service = false;
    h.create(&mut s, 7);
    assert_eq!(s.set_breakpoint_exists(&mut h, 7, b"job", 10, false), Ok(true));
    assert_eq!(s.set_breakpoint_exists(&mut h, 7, b"job", 10, true), Ok(true));
    assert_eq!(s.pause_script(&mut h, 7), Ok(1));
    assert_eq!(s.record(7).unwrap().pause_count, 1);
    assert_eq!(h.count(|c| matches!(c, Primitive::AllocateMessage { .. } | Primitive::SendMessage { .. })), 0);
}

#[test]
fn v30_duplicate_live_id_is_an_explicit_invalid_foreign_outcome() {
    let (mut s, mut h) = Mock::with_script(7);
    let original = s.record(7).unwrap().token();
    h.created.push_back((7, Some(Handle(777))));
    assert_eq!(s.run_script_string(&mut h, b"new", b"new", b""), Err(Failure::DuplicateLiveId(7)));
    assert_eq!(s.record(7).unwrap().token(), original);
    assert_eq!(h.count(|c| matches!(c, Primitive::Release { interface: Interface::Thread, handle: Handle(777) })), 1);
    assert_eq!(h.count(|c| matches!(c, Primitive::InstallLineHook { .. } | Primitive::SendMessage { .. })), 0);
}

#[test]
fn v31_kill_leaves_last_id_and_current_is_only_tick_resume_scoped() {
    let (mut s, mut h) = Mock::with_script(71);
    assert_eq!(s.kill_script(&mut h, 71), Ok(true));
    assert_eq!(s.get_last_created_script_id(), 71);
    assert_eq!(s.get_current_running_script_id(&mut h), Ok(-1));
    assert!(h.position(|c| matches!(c, Primitive::DestroyThread { id: 71, .. })) < h.position(|c| matches!(c, Primitive::SendMessage { .. })));
    assert_eq!(h.count(|c| matches!(c, Primitive::Stack { .. })), 0);
}

// SPDX-License-Identifier: GPL-3.0-or-later
//! Additional ownership, ordering, retry and non-invalidating reentry boundaries.
use fc4_core::lua_script_server::*;
#[path = "support/lua_script_server.rs"]
mod support;
use support::Mock;

#[test]
fn qi_primary_adjusted_views_share_refcount_and_unknown_ids_delegate() {
    let mut s = ScriptServer::new();
    let mut h = Mock::default();
    assert_eq!(s.ref_count(), 0);
    assert_eq!(s.query_interface(&mut h, 1), Ok(Some(InterfaceView::Primary)));
    assert_eq!(
        s.query_interface(&mut h, SCRIPT_SERVER_IID),
        Ok(Some(InterfaceView::Primary))
    );
    assert_eq!(s.ref_count(), 2);
    assert!(h.calls.is_empty());
    assert_eq!(
        s.query_interface(&mut h, 0x1234),
        Ok(Some(InterfaceView::Base(Handle(88))))
    );
    assert_eq!(s.ref_count(), 2);
    assert_eq!(h.calls, [Primitive::BaseQuery(0x1234)]);
    assert_eq!(s.release(&mut h), Ok(1));
    assert!(!s.is_destroyed());
}

#[test]
fn release_at_zero_or_one_dispatches_shutdown_once_and_deletes() {
    for refs in [0, 1] {
        let (mut s, mut h) = Mock::with_script(7);
        for _ in 0..refs {
            s.add_ref().unwrap();
        }
        assert_eq!(s.release(&mut h), Ok(0));
        assert!(s.is_destroyed());
        assert_eq!(h.count(|c| matches!(c, Primitive::UnregisterService)), 1);
        assert_eq!(h.count(|c| matches!(c, Primitive::DestroyThread { .. })), 1);
        assert_eq!(h.count(|c| matches!(c, Primitive::Close(_))), 1);
        assert_eq!(h.count(|c| matches!(c, Primitive::BaseDestroy)), 1);
        assert_eq!(h.count(|c| matches!(c, Primitive::FreeAllocation)), 1);
        assert_eq!(s.release(&mut h), Err(Failure::Destroyed));
    }
}

#[test]
fn plain_destructor_does_not_invent_shutdown_or_record_thread_release() {
    let (mut s, mut h) = Mock::with_script(7);
    assert_eq!(s.deleting_destructor(&mut h, 2), Ok(()));
    assert_eq!(
        h.calls,
        [
            Primitive::Release {
                interface: Interface::Generator,
                handle: Handle(2)
            },
            Primitive::Release {
                interface: Interface::Lua,
                handle: Handle(1)
            },
            Primitive::BaseDestroy,
        ]
    );
    assert!(s.is_destroyed());
}

#[test]
fn init_failure_paths_clear_owned_pointers_and_require_shutdown_to_retry() {
    for which in 0..3 {
        let mut s = ScriptServer::new();
        let mut h = Mock::default();
        match which {
            0 => h.open = false,
            1 => h.generator = None,
            _ => h.generator_init = false,
        }
        h.hook(
            |c| {
                matches!(
                    c,
                    Primitive::Release {
                        interface: Interface::Lua,
                        ..
                    }
                )
            },
            |_, owner| {
                assert!(owner.get_lua().is_none());
            },
        );
        assert_eq!(s.init(&mut h), Ok(false));
        assert!(s.init_latched());
        assert_eq!(s.get_lua(), None);
        assert_eq!(
            h.count(|c| matches!(
                c,
                Primitive::Release {
                    interface: Interface::Lua,
                    ..
                }
            )),
            1
        );
        assert_eq!(h.count(|c| matches!(c, Primitive::Close(_))), usize::from(which != 0));
        assert_eq!(
            h.count(|c| matches!(
                c,
                Primitive::Release {
                    interface: Interface::Generator,
                    ..
                }
            )),
            usize::from(which == 2)
        );
        h.open = true;
        h.generator = Some(Handle(2));
        h.generator_init = true;
        h.clear();
        assert_eq!(s.init(&mut h), Ok(false));
        assert!(h.calls.is_empty());
        s.shutdown(&mut h).unwrap();
        assert_eq!(s.init(&mut h), Ok(true));
    }
}

#[test]
fn missing_framework_is_successful_and_repeated_init_replaces_userdata_only() {
    let mut s = ScriptServer::new();
    let mut h = Mock {
        framework: false,
        ..Mock::default()
    };
    assert_eq!(s.init(&mut h), Ok(true));
    assert_eq!(h.count(|c| matches!(c, Primitive::RegisterService)), 0);
    h.clear();
    assert_eq!(s.init(&mut h), Ok(true));
    assert_eq!(
        h.count(|c| matches!(
            c,
            Primitive::CreateLua { .. } | Primitive::CreateGenerator { .. } | Primitive::RegisterService
        )),
        0
    );
    assert_eq!(
        h.stack_ops(Handle(10)),
        [
            StackOp::PushString(b"arg".to_vec()),
            StackOp::NewUserdata(1),
            StackOp::PushValue(-1),
            StackOp::Insert(-3),
            StackOp::RawSet(GLOBALS),
            StackOp::NewTable,
            StackOp::PushArgIndexCallback { upvalues: 0 },
            StackOp::SetField {
                index: -2,
                name: "__index"
            },
            StackOp::SetMetatable(-2),
            StackOp::SetTop(3),
        ]
    );
}

#[test]
fn shutdown_kills_in_signed_order_then_disposes_generator_and_lua() {
    let (mut s, mut h) = Mock::with_script(8);
    h.create(&mut s, -7);
    h.create(&mut s, 2);
    h.clear();
    h.hook(
        |c| {
            matches!(
                c,
                Primitive::Release {
                    interface: Interface::Lua,
                    ..
                }
            )
        },
        |_, owner| {
            assert_eq!(owner.get_lua(), None);
            assert!(!owner.init_latched());
        },
    );
    assert_eq!(s.shutdown(&mut h), Ok(true));
    let ids: Vec<i32> = h
        .calls
        .iter()
        .filter_map(|c| {
            if let Primitive::DestroyThread { id, .. } = c {
                Some(*id)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(ids, [-7, 2, 8]);
    let order: Vec<&str> = h
        .calls
        .iter()
        .filter_map(|c| match c {
            Primitive::UnregisterService => Some("unregister"),
            Primitive::GeneratorTarget(_) => Some("target"),
            Primitive::ShutdownGeneratorTarget(_) => Some("shutdown target"),
            Primitive::Release {
                interface: Interface::Generator,
                ..
            } => Some("release generator"),
            Primitive::Close(_) => Some("close"),
            Primitive::Release {
                interface: Interface::Lua,
                ..
            } => Some("release lua"),
            _ => None,
        })
        .collect();
    assert_eq!(
        order,
        [
            "unregister",
            "target",
            "shutdown target",
            "release generator",
            "close",
            "release lua"
        ]
    );
    assert_eq!(s.attempt_serial(), 3);
    assert_eq!(s.get_last_created_script_id(), 2);
    h.clear();
    assert_eq!(s.shutdown(&mut h), Ok(true));
    assert!(h.calls.is_empty());
}

#[test]
fn failed_thread_creation_releases_returned_reference_without_global_rollback() {
    let (mut s, mut h) = Mock::initialized();
    h.created.push_back((-1, Some(Handle(300))));
    assert_eq!(s.run_script_string(&mut h, b"compiled", b"job", b""), Ok(-1));
    assert_eq!(s.attempt_serial(), 1);
    assert_eq!(s.get_last_created_script_id(), -1);
    assert!(
        h.position(|c| matches!(
            c,
            Primitive::Stack {
                op: StackOp::RawSet(GLOBALS),
                ..
            }
        )) < h.position(|c| matches!(
            c,
            Primitive::Release {
                interface: Interface::Thread,
                ..
            }
        ))
    );
    assert_eq!(
        h.stack_ops(Handle(10)),
        [
            StackOp::PushString(b"Main - \"job\" - 1".to_vec()),
            StackOp::PushValue(-2),
            StackOp::RawSet(GLOBALS)
        ]
    );
    assert_eq!(
        h.count(|c| matches!(
            c,
            Primitive::Release {
                interface: Interface::Thread,
                handle: Handle(300)
            }
        )),
        1
    );
    assert_eq!(
        h.count(|c| matches!(c, Primitive::SetUserContext { .. } | Primitive::InstallLineHook { .. })),
        0
    );
    assert!(h.sent.is_empty());
}

#[test]
fn null_successful_thread_is_explicit_precondition_failure_and_no_auto_init() {
    let mut s = ScriptServer::new();
    let mut h = Mock::default();
    assert_eq!(s.run_script_string(&mut h, b"", b"", b""), Err(Failure::MissingLua));
    assert!(h.calls.is_empty());
    let (mut s, mut h) = Mock::initialized();
    h.created.push_back((7, None));
    assert_eq!(s.run_script_string(&mut h, b"", b"", b""), Err(Failure::MissingThread));
    assert!(s.record(7).is_none());
}

#[test]
fn successful_file_read_uses_original_name_and_path_component_for_display() {
    let (mut s, mut h) = Mock::initialized();
    h.file = Some(b"file bytes".to_vec());
    h.file_exists = true;
    h.path_component = b"part.lua".to_vec();
    h.created.push_back((0, Some(Handle(100))));
    assert_eq!(s.run_script_file(&mut h, b"dir/part.lua", b"options"), Ok(0));
    assert_eq!(s.record(0).unwrap().source, b"file bytes");
    assert_eq!(s.record(0).unwrap().name, b"dir/part.lua");
    assert_eq!(s.record(0).unwrap().arguments, b"options");
    assert!(h.calls.contains(&Primitive::PathName {
        path: b"dir/part.lua".to_vec(),
        flags: 0
    }));
    assert_eq!(
        h.stack_ops(Handle(10))[0],
        StackOp::PushString(b"Main - \"part.lua\" - 1".to_vec())
    );
}

#[test]
fn control_uses_snapshot_but_tick_visits_later_insertions_live() {
    let (mut s, mut h) = Mock::with_script(2);
    h.create(&mut s, 8);
    h.clear();
    h.hook(
        |c| matches!(c, Primitive::SendMessage { .. }),
        |host, owner| {
            host.create(owner, 5);
        },
    );
    assert_eq!(s.pause_script(&mut h, -1), Ok(1));
    assert_eq!(s.record(2).unwrap().pause_count, 1);
    assert_eq!(s.record(5).unwrap().pause_count, 0);
    assert_eq!(s.record(8).unwrap().pause_count, 1);
    s.resume_script(&mut h, 2).unwrap();
    s.resume_script(&mut h, 8).unwrap();
    h.clear();
    h.hook(
        |c| matches!(c, Primitive::ResumeThread { id: 2, .. }),
        |host, owner| {
            host.create(owner, 9);
            host.create(owner, -2);
        },
    );
    assert_eq!(s.on_tick(&mut h), Ok(true));
    let resumed: Vec<i32> = h
        .calls
        .iter()
        .filter_map(|c| {
            if let Primitive::ResumeThread { id, .. } = c {
                Some(*id)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(resumed, [2, 5, 8, 9]);
    assert!(s.record(-2).is_some());
}

#[test]
fn control_returns_post_callback_count_and_all_selectors_succeed_when_empty() {
    let (mut s, mut h) = Mock::with_script(7);
    h.hook(
        |c| matches!(c, Primitive::SendMessage { .. }),
        |host, owner| {
            assert_eq!(owner.pause_script(host, 7), Ok(2));
        },
    );
    assert_eq!(s.pause_script(&mut h, 7), Ok(2));
    assert_eq!(s.kill_script(&mut h, -1), Ok(false));
    s.kill_script(&mut h, 7).unwrap();
    assert_eq!(s.pause_script(&mut h, -1), Ok(1));
    assert_eq!(s.resume_script(&mut h, -1), Ok(1));
    assert_eq!(s.pause_script(&mut h, 7), Ok(0));
    assert_eq!(s.resume_script(&mut h, 7), Ok(0));
}

#[test]
fn status_is_read_independently_and_paused_dead_script_is_retained() {
    let (mut s, mut h) = Mock::with_script(7);
    h.status(&s, 7, &[0, 1]);
    assert_eq!(s.on_tick(&mut h), Ok(true));
    assert!(s.record(7).is_some());
    assert_eq!(
        h.count(|c| matches!(c, Primitive::ResumeThread { .. } | Primitive::DestroyThread { .. })),
        0
    );
    s.pause_script(&mut h, 7).unwrap();
    h.clear();
    h.status(&s, 7, &[0, 0]);
    assert_eq!(s.on_tick(&mut h), Ok(true));
    assert!(h.calls.is_empty());
    s.resume_script(&mut h, 7).unwrap();
    h.clear();
    assert_eq!(s.on_tick(&mut h), Ok(true));
    assert!(s.record(7).is_none());
}

#[test]
fn invalidating_resume_callback_is_failure_without_deferred_kill_promise() {
    let (mut s, mut h) = Mock::with_script(7);
    h.hook(
        |c| matches!(c, Primitive::ResumeThread { .. }),
        |host, owner| {
            assert_eq!(owner.kill_script(host, 7), Ok(true));
            assert!(owner.record(7).is_none());
        },
    );
    assert_eq!(s.on_tick(&mut h), Err(Failure::InvalidatingCallback));
    assert_eq!(s.tick_depth(), 0);
    assert_eq!(s.get_current_running_script_id(&mut h), Ok(-1));
    assert_eq!(h.count(|c| matches!(c, Primitive::Status(_))), 1);
}

#[test]
fn invalidating_control_notification_does_not_return_a_fabricated_count() {
    let (mut s, mut h) = Mock::with_script(7);
    h.hook(
        |c| matches!(c, Primitive::SendMessage { .. }),
        |host, owner| {
            owner.kill_script(host, 7).unwrap();
        },
    );
    assert_eq!(s.pause_script(&mut h, 7), Err(Failure::InvalidatingCallback));
    assert!(s.record(7).is_none());
}

#[test]
fn shutdown_during_resume_is_typed_lifecycle_invalidation() {
    let (mut s, mut h) = Mock::with_script(7);
    h.hook(
        |c| matches!(c, Primitive::ResumeThread { .. }),
        |host, owner| {
            owner.shutdown(host).unwrap();
        },
    );
    assert_eq!(s.on_tick(&mut h), Err(Failure::InvalidatingCallback));
    assert_eq!(s.get_lua(), None);
    assert_eq!(s.tick_depth(), 0);
}

#[test]
fn info_outputs_are_optional_copies_and_last_byte_is_schedulable() {
    let (mut s, mut h) = Mock::with_script(7);
    assert!(!s.get_script_auto_yield_enabled(7));
    let (mut src, mut name, mut pause, mut transitions, mut schedulable) = (vec![], vec![], 99, 99, false);
    assert_eq!(
        s.get_script_info(
            &mut h,
            7,
            InfoOutputs {
                source: Some(&mut src),
                name: Some(&mut name),
                pause_count: Some(&mut pause),
                resume_transitions: Some(&mut transitions),
                schedulable: Some(&mut schedulable),
                ..InfoOutputs::default()
            }
        ),
        Ok(true)
    );
    assert_eq!(
        (src.as_slice(), name.as_slice(), pause, transitions, schedulable),
        (b"source".as_slice(), b"job".as_slice(), 0, 0, true)
    );
    src.push(b'!');
    name.push(b'!');
    assert_eq!(s.record(7).unwrap().name, b"job");
    assert!(h.calls.is_empty());
    s.set_script_auto_yield_enabled(7, true);
    assert!(s.get_script_auto_yield_enabled(7));
    assert!(s.record(7).unwrap().schedulable);
    assert!(h.calls.is_empty());
}

#[test]
fn breakpoint_toggle_byte_values_and_no_path_normalization() {
    let (mut s, mut h) = Mock::with_script(7);
    assert_eq!(s.toggle_breakpoint_exists(&mut h, 7, b"Dir\\Job", -9), Ok(true));
    assert_eq!(s.get_breakpoint_exists(&mut h, 7, b"dir\\job", -9), Ok(true));
    assert_eq!(s.get_breakpoint_exists(&mut h, 7, b"dir/job", -9), Ok(false));
    assert_eq!(s.get_breakpoint_exists(&mut h, 7, b"Job", -9), Ok(false));
    assert_eq!(s.set_breakpoint_enabled(&mut h, 7, b"Dir\\Job", -9, 0x80), Ok(true));
    assert_eq!(s.get_breakpoint_enabled(&mut h, 7, b"Dir\\Job", -9), Ok(true));
    h.clear();
    s.set_breakpoint_enabled(&mut h, 7, b"Dir\\Job", -9, 0x80).unwrap();
    assert!(h.sent.is_empty());
    assert_eq!(s.toggle_breakpoint_exists(&mut h, 7, b"DIR\\JOB", -9), Ok(true));
    assert_eq!(s.get_breakpoint_exists(&mut h, 7, b"Dir\\Job", -9), Ok(false));
    assert_eq!(h.sent, [(BREAKPOINT_CHANGED, 7, None)]);
}

#[test]
fn all_eight_breakpoint_methods_reject_missing_single_id() {
    let (mut s, mut h) = Mock::with_script(7);
    for id in [-1, 99] {
        assert_eq!(s.get_breakpoint_exists(&mut h, id, b"job", 10), Ok(false));
        assert_eq!(s.set_breakpoint_exists(&mut h, id, b"job", 10, false), Ok(false));
        assert_eq!(s.toggle_breakpoint_exists(&mut h, id, b"job", 10), Ok(false));
        assert_eq!(s.get_breakpoint_enabled(&mut h, id, b"job", 10), Ok(false));
        assert_eq!(s.set_breakpoint_enabled(&mut h, id, b"job", 10, 1), Ok(false));
        let mut condition = b"old".to_vec();
        assert_eq!(
            s.get_breakpoint_condition(&mut h, id, b"job", 10, &mut condition),
            Ok(false)
        );
        assert!(condition.is_empty());
        assert_eq!(s.set_breakpoint_condition(&mut h, id, b"job", 10, b""), Ok(false));
        let mut list = vec![Breakpoint::default()];
        assert_eq!(s.get_breakpoint_entry_list(&mut h, id, b"job", &mut list), Ok(false));
        assert!(list.is_empty());
    }
    assert!(h.calls.is_empty());
}

#[test]
fn removing_breakpoint_allows_resume_but_transition_count_does_not_bypass_it() {
    let (mut s, mut h) = Mock::with_script(7);
    s.set_breakpoint_exists(&mut h, 7, b"job", 10, false).unwrap();
    s.pause_script(&mut h, 7).unwrap();
    s.resume_script(&mut h, 7).unwrap();
    h.clear();
    assert_eq!(s.on_tick(&mut h), Ok(true));
    assert_eq!(h.count(|c| matches!(c, Primitive::ResumeThread { .. })), 0);
    s.set_breakpoint_exists(&mut h, 7, b"job", 10, true).unwrap();
    h.clear();
    assert_eq!(s.on_tick(&mut h), Ok(true));
    assert_eq!(h.count(|c| matches!(c, Primitive::ResumeThread { .. })), 1);
}

#[test]
fn thread_release_observes_cleared_pointer_and_termination_observes_live_record() {
    let (mut s, mut h) = Mock::with_script(7);
    h.hook(
        |c| matches!(c, Primitive::SendMessage { .. }),
        |host, owner| {
            assert!(owner.record(7).unwrap().thread().is_some());
            assert_eq!(owner.get_current_running_script_id(host), Ok(-1));
        },
    );
    h.hook(
        |c| {
            matches!(
                c,
                Primitive::Release {
                    interface: Interface::Thread,
                    ..
                }
            )
        },
        |_, owner| {
            assert_eq!(owner.record(7).unwrap().thread(), None);
        },
    );
    assert_eq!(s.kill_script(&mut h, 7), Ok(true));
    assert!(h.hooks.is_empty());
}

#[test]
fn notification_wire_protocol_balances_only_owned_message_and_service_refs() {
    let (mut s, mut h) = Mock::with_script(7);
    h.send_result = false;
    assert_eq!(s.pause_script(&mut h, 7), Ok(1));
    assert_eq!(
        h.calls,
        [
            Primitive::FindMessageService {
                service: 0x04fa845b,
                iid: 0x652294c7
            },
            Primitive::AllocateMessage { bytes: 0x2c },
            Primitive::AddRef {
                interface: Interface::Message,
                handle: Handle(91)
            },
            Primitive::MessageType {
                message: Handle(91),
                value: PAUSE_CHANGED
            },
            Primitive::MessageId {
                message: Handle(91),
                value: 7
            },
            Primitive::MessagePayload {
                message: Handle(91),
                value: 1
            },
            Primitive::SendMessage {
                service: Handle(90),
                message: Handle(91),
                flags: 0
            },
            Primitive::Release {
                interface: Interface::Message,
                handle: Handle(91)
            },
            Primitive::Release {
                interface: Interface::MessageService,
                handle: Handle(90)
            },
        ]
    );
    h.clear();
    h.message_storage = false;
    assert_eq!(s.resume_script(&mut h, 7), Err(Failure::StorageRequired));
    assert_eq!(s.record(7).unwrap().pause_count, 0);
    assert_eq!(s.record(7).unwrap().resume_transitions, 0);
}

#[test]
fn argument_proxy_never_falls_back_to_another_thread_and_leaves_temporaries() {
    let (mut s, mut h) = Mock::with_script(7);
    assert_eq!(s.argument_index(&mut h, Handle(77)), Ok(1));
    assert_eq!(
        h.stack_ops(Handle(1007)),
        [
            StackOp::PushLightUserdata(Handle(1007)),
            StackOp::RawGet(REGISTRY),
            StackOp::PushValue(-2),
            StackOp::RawGet(-2),
        ]
    );
    h.clear();
    h.registry_type = 0;
    assert_eq!(s.argument_index(&mut h, Handle(77)), Ok(1));
    assert_eq!(
        h.stack_ops(Handle(1007)),
        [
            StackOp::PushLightUserdata(Handle(1007)),
            StackOp::RawGet(REGISTRY),
            StackOp::PushNil
        ]
    );
    h.clear();
    h.top = 0;
    assert_eq!(s.argument_index(&mut h, Handle(77)), Ok(0));
    assert!(h.stack_ops(Handle(1007)).is_empty());
    h.clear();
    h.wrapper = None;
    assert_eq!(s.argument_index(&mut h, Handle(77)), Ok(0));
    assert_eq!(h.calls, [Primitive::WrapperFromState(Handle(77))]);
}

#[test]
fn empty_and_repeated_argument_tokens_use_independent_registry_keys() {
    let (mut s, mut h) = Mock::initialized();
    h.create(&mut s, 7);
    assert_eq!(
        h.stack_ops(Handle(1007)),
        [
            StackOp::PushLightUserdata(Handle(1007)),
            StackOp::NewTable,
            StackOp::PushNumber(0.0),
            StackOp::PushString(b"job".to_vec()),
            StackOp::RawSet(-3),
            StackOp::PushString(b"count".to_vec()),
            StackOp::PushNumber(1.0),
            StackOp::RawSet(-3),
            StackOp::RawSet(REGISTRY),
        ]
    );
    h.clear();
    h.tokens = ["", "///", "-a:", "/a:2"]
        .iter()
        .map(|x| x.as_bytes().to_vec())
        .collect();
    h.created.push_back((8, Some(Handle(1008))));
    s.run_script_string(&mut h, b"", b"job", b"parse me").unwrap();
    assert_eq!(
        h.stack_ops(Handle(1008)),
        [
            StackOp::PushLightUserdata(Handle(1008)),
            StackOp::NewTable,
            StackOp::PushNumber(0.0),
            StackOp::PushString(b"job".to_vec()),
            StackOp::RawSet(-3),
            StackOp::PushString(b"a".to_vec()),
            StackOp::PushString(vec![]),
            StackOp::RawSet(-3),
            StackOp::PushNumber(1.0),
            StackOp::PushString(b"-a:".to_vec()),
            StackOp::RawSet(-3),
            StackOp::PushString(b"a".to_vec()),
            StackOp::PushString(b"2".to_vec()),
            StackOp::RawSet(-3),
            StackOp::PushNumber(2.0),
            StackOp::PushString(b"/a:2".to_vec()),
            StackOp::RawSet(-3),
            StackOp::PushString(b"count".to_vec()),
            StackOp::PushNumber(3.0),
            StackOp::RawSet(-3),
            StackOp::RawSet(REGISTRY),
        ]
    );
    h.clear();
    s.kill_script(&mut h, 7).unwrap();
    assert_eq!(h.count(|c| matches!(c, Primitive::Stack { .. })), 0);
}

#[test]
fn line_hook_gates_on_wrapper_context_full_u16_and_auto_yield_without_reset() {
    let (mut s, mut h) = Mock::with_script(7);
    assert_eq!(s.line_hook(&mut h, Handle(77)), Ok(false));
    assert!(h.lines.is_empty());
    s.set_script_auto_yield_enabled(7, true);
    h.raw_field = 0x100;
    assert_eq!(s.line_hook(&mut h, Handle(77)), Ok(false));
    h.raw_field = 0;
    h.lines.push_back(5);
    assert_eq!(s.line_hook(&mut h, Handle(77)), Ok(true));
    s.set_script_auto_yield_enabled(7, false);
    assert_eq!(s.line_hook(&mut h, Handle(77)), Ok(false));
    s.set_script_auto_yield_enabled(7, true);
    h.lines.push_back(5);
    assert_eq!(s.line_hook(&mut h, Handle(77)), Ok(false));
    h.contexts.clear();
    assert_eq!(s.line_hook(&mut h, Handle(77)), Ok(false));
    h.wrapper = None;
    assert_eq!(s.line_hook(&mut h, Handle(77)), Ok(false));
    assert_eq!(h.count(|c| matches!(c, Primitive::Yield { .. })), 1);
}

#[test]
fn malformed_primitive_replies_and_foreign_failures_are_typed() {
    struct Wrong;
    impl Foreign for Wrong {
        fn invoke(&mut self, _: &mut ScriptServer, _: Primitive) -> Result<Value, Failure> {
            Ok(Value::Unit)
        }
    }
    let mut s = ScriptServer::new();
    assert_eq!(s.init(&mut Wrong), Err(Failure::UnexpectedReply));
    assert!(s.init_latched());
    struct Unavailable;
    impl Foreign for Unavailable {
        fn invoke(&mut self, _: &mut ScriptServer, _: Primitive) -> Result<Value, Failure> {
            Err(Failure::Foreign("unavailable"))
        }
    }
    assert_eq!(
        ScriptServer::new().init(&mut Unavailable),
        Err(Failure::Foreign("unavailable"))
    );
}

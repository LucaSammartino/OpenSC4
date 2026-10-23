// SPDX-License-Identifier: GPL-3.0-or-later
// Included by lua_vm.rs to share its foreign primitive fixture.

fn opened() -> (LuaHost, Api) {
    let mut a = Api::default();
    let mut h = LuaHost::new(Handle(10));
    assert!(h.open(&mut a, 0).unwrap());
    (h, a)
}
fn child(h: &mut LuaHost, a: &mut Api, flags: u32) -> (i32, Handle) {
    h.main(a).unwrap().push_string(b"F").unwrap();
    let mut out = None;
    let id = h.create_thread(a, flags, THREAD_CLASS, &mut out).unwrap();
    assert!(id >= 0);
    (id, out.unwrap())
}
fn stack_at(a: &mut Api, context: Handle) -> Vec<V> {
    a.invoke(Some(context), Op::GetTop).unwrap();
    a.stack.clone()
}

#[test]
fn lh4_a_b_c_e_main_child_roots_borrowing_and_monotonic_ids() {
    let mut a = Api::default();
    let fresh = Thread::new(Handle(42), ThreadKind::Standalone);
    let mut h = LuaHost::new(Handle(10));
    assert!(!fresh.is_valid(&mut a).unwrap());
    assert_eq!(h.status(&mut a, &fresh).unwrap(), -1);
    assert!(!fresh.destroy(&mut h, &mut a).unwrap());
    assert!(h.open(&mut a, 0).unwrap());
    assert_eq!(h.main_thread.get_id(), 0);
    assert_eq!(h.status(&mut a, &h.main_thread).unwrap(), 2);
    let refs = a.references.len();
    assert_eq!(h.get_thread(&mut a, 0).unwrap(), Some(Handle(10)));
    assert_eq!(refs, a.references.len());
    assert!(!h.open(&mut a, 0).unwrap());
    a.stack.push(V::String(b"lower".to_vec()));
    let (id, view) = child(&mut h, &mut a, 1);
    assert_eq!(id, 1);
    let t = h.thread_view(id).unwrap().clone();
    let context = t.context().unwrap();
    assert_eq!(t.identity, view);
    assert_eq!(stack_at(&mut a, context), [V::String(b"F".to_vec())]);
    assert_eq!(h.status(&mut a, &t).unwrap(), 1);
    assert!(t.is_valid(&mut a).unwrap());
    assert_eq!(a.tables[1].get(&format!("p:{}", context.0)), Some(&V::Pointer(context)));
    assert_eq!(stack_at(&mut a, Handle(20)), [V::String(b"lower".to_vec())]);
    assert!(h.destroy_thread(&mut a, id).unwrap());
    assert!(!t.is_valid(&mut a).unwrap()); // retained view sees cleared backpointer
    assert_eq!(h.status(&mut a, &t).unwrap(), -1);
    assert_eq!(stack_at(&mut a, context), [V::String(b"F".to_vec())]);
    assert_eq!(child(&mut h, &mut a, 1).0, 2);
    h.close(&mut a).unwrap();
    assert!(h.open(&mut a, 0).unwrap());
    assert_eq!(h.main_thread.get_id(), 3);
}

#[test]
fn lh4_d_f_failed_factory_and_hooks_do_not_rollback_or_advance_id() {
    let (mut h, mut a) = opened();
    for factory in [false, true] {
        a.factory = factory;
        a.reserve = false;
        h.main(&mut a).unwrap().push_string(b"F").unwrap();
        let mut out = Some(Handle(999));
        assert_eq!(h.create_thread(&mut a, 1, THREAD_CLASS, &mut out).unwrap(), -1);
        assert_eq!(out, None);
        assert_eq!(stack_at(&mut a, Handle(20)), []);
        assert!(h.record(1).is_none());
        assert!(a.backpointers.keys().all(|c| *c == Handle(20)));
        if factory {
            assert!(a.references.contains(&(false, Handle(a.next_context + 1000))));
        }
        assert!(a.destroy_hooks.is_empty());
    }
    a.reserve = true;
    assert_eq!(child(&mut h, &mut a, 1).0, 1);
    let mut h = LuaHost::new(Handle(90));
    let mut a = Api {
        reserve: false,
        ..Api::default()
    };
    assert!(!h.open(&mut a, 0).unwrap());
    assert_eq!(h.main_thread.context(), Some(Handle(20)));
    assert_eq!(h.main_thread.owner(), Some(Handle(90)));
    assert_eq!(h.main_thread.get_id(), 0);
    assert!(h.record(0).is_none());
    assert!(!a.operations().contains(&Op::Close));
    assert_eq!(a.references, [(true, Handle(90)), (true, Handle(90))]);
    assert!(!h.open(&mut a, 0).unwrap());
}

#[test]
fn lh4_g_h_i_j_resume_results_yield_completion_rejection_and_reporting() {
    let (mut h, mut a) = opened();
    let (id, _) = child(&mut h, &mut a, 1);
    let context = h.thread_view(id).unwrap().context().unwrap();
    h.main(&mut a).unwrap().set_top(0).unwrap();
    a.stack = vec![V::String(b"lower".to_vec()), V::Number(1.0), V::Number(2.0)];
    a.resume_values = vec![V::String(b"A".to_vec()), V::String(b"B".to_vec())];
    let mut count = -1;
    assert!(h.resume_thread(&mut a, id, 2, Some(&mut count)).unwrap());
    assert_eq!(count, 2);
    assert_eq!(
        a.stack,
        [
            V::String(b"lower".to_vec()),
            V::String(b"A".to_vec()),
            V::String(b"B".to_vec())
        ]
    );
    assert_eq!(stack_at(&mut a, context), []);
    let t = h.thread_view(id).unwrap().clone();
    assert_eq!(h.status(&mut a, &t).unwrap(), 0);
    h.main(&mut a).unwrap().set_top(1).unwrap();
    assert!(h.resume_thread(&mut a, id, 0, None).unwrap());
    assert_eq!(a.stack, [V::String(b"lower".to_vec())]);
    h.set_error_reporter(Some(Handle(99)), false);
    a.stack.extend([V::Number(1.0), V::Number(2.0)]);
    assert!(!h.resume_thread(&mut a, 999, 2, Some(&mut count)).unwrap());
    assert_eq!(count, 3);
    assert_eq!(
        a.reports.last(),
        Some(&(
            Handle(99),
            Handle(0),
            1,
            Some(b"Resuming Lua thread with invalid ID: 999.".to_vec())
        ))
    );
    a.stack.pop();
    assert!(!h.resume_thread(&mut a, 999, 2, None).unwrap());
    assert_eq!(a.stack, [V::String(b"lower".to_vec())]);
    let (id, view) = child(&mut h, &mut a, 0);
    a.resume_values = vec![V::Number(7.0)];
    a.resume_frame = true;
    assert!(h.resume_thread(&mut a, id, 0, Some(&mut count)).unwrap());
    let t = h.thread_view(id).unwrap().clone();
    assert_eq!(h.status(&mut a, &t).unwrap(), 1);
    assert_eq!(count, 1);
    a.resume_frame = false;
    a.runtime_status = 3;
    assert!(!h.resume_thread(&mut a, id, 0, Some(&mut count)).unwrap());
    assert!(h.get_thread(&mut a, id).unwrap().is_none());
    assert_ne!(view, Handle(0));
    assert_eq!(a.reports.last().unwrap().1, Handle(0)); // auto-destroy precedes reporter
    assert_eq!(a.reports.last().unwrap().2, 3);
    assert!(!t.is_valid(&mut a).unwrap());
}

#[test]
fn lh4_k_l_hooks_parent_restrictions_recursive_destroy_and_pause() {
    let (mut h, mut a) = opened();
    let (id, _) = child(&mut h, &mut a, 0x111);
    let t = h.thread_view(id).unwrap().clone();
    let context = t.context().unwrap();
    assert!(!h.destroy_from(&mut a, id, Some(Handle(999))).unwrap());
    let mut count = 0;
    assert!(!h.resume_thread(&mut a, 0, 0, Some(&mut count)).unwrap());
    assert!(a.stack.last() == Some(&V::String(b"Lua thread 0 is already running.".to_vec())));
    assert_eq!(t.pause(&mut a, 2).unwrap(), 2);
    let mut out = None;
    // Source child has its initial value; creation attaches a grandchild.
    let grandchild = h.create_from(&mut a, context, 0x11, THREAD_CLASS, &mut out).unwrap();
    assert!(grandchild > id);
    assert_eq!(h.record(id).unwrap().children.len(), 1);
    assert!(h.destroy_from(&mut a, id, None).unwrap());
    assert!(h.record(grandchild).is_none());
    assert!(h.record(id).is_none());
    let (id, _) = child(&mut h, &mut a, 0x100);
    assert!(!h.resume_thread(&mut a, id, 0, Some(&mut count)).unwrap());
    assert!(
        a.stack.last()
            == Some(&V::String(
                format!("Lua thread {id} is only resumable by its parent.").into_bytes()
            ))
    );
    // Hook alone invalidates its view but does not remove registry bookkeeping.
    let mut t = h.thread_view(id).unwrap().clone();
    t.set_user_context(Some(Handle(777)));
    assert!(t.on_destroy(&mut a).unwrap());
    assert_eq!(t.context(), None);
    assert_eq!(t.owner(), None);
    assert_eq!(t.get_id(), -1);
    assert_eq!(t.user_context(), Some(Handle(777)));
    assert!(h.record(id).is_some());
    assert_eq!(h.get_thread(&mut a, id).unwrap(), Some(t.identity));
}

fn generator(a: &mut Api) -> CommandGenerator {
    let mut g = CommandGenerator::new(Handle(40));
    assert!(!g.generic_init().unwrap());
    assert!(g.init(a, Some(Handle(2)), Some(Handle(1)), b"cmd").unwrap());
    g
}
fn binding(a: &mut Api, name: &[u8]) -> V {
    a.session().get_global(b"cmd").unwrap();
    a.session().push_string(name).unwrap();
    a.session().execute(Op::GetTable(-2)).unwrap();
    let v = a.stack.pop().unwrap();
    a.stack.pop();
    v
}
fn invoke_binding(g: &CommandGenerator, a: &mut Api, binding: &V) -> i32 {
    let V::Closure(Callback::Command, captures) = binding else {
        panic!("not command closure")
    };
    let [V::Number(index), V::Pointer(generator), V::Pointer(main)] = captures.as_slice() else {
        panic!("bad captures")
    };
    g.callback(a, &thread(), Some(*generator), *main, *index).unwrap()
}

#[test]
fn lh5_a_b_c_d_reinitialization_null_sets_and_global_identity() {
    let mut a = Api::default();
    let mut g = CommandGenerator::new(Handle(40));
    assert_eq!(g.records().len(), 500);
    assert!(!g.generic_init().unwrap());
    assert!(a.generator_trace.is_empty());
    a.stack.push(V::Number(42.0));
    assert!(g.init(&mut a, Some(Handle(2)), Some(Handle(1)), b"cmd").unwrap());
    assert!(g.records().is_empty());
    assert_eq!(a.stack, [V::Number(42.0)]);
    let table = a.tables[0]["s:cmd"].clone();
    assert!(g.command_registered(&mut a, 7, b"x", b"desc").unwrap());
    a.generator_trace.clear();
    a.allocate_parameters = false;
    assert!(g.init(&mut a, Some(Handle(3)), Some(Handle(1)), b"cmd").unwrap());
    assert_eq!(a.tables[0]["s:cmd"], table);
    assert!(g.records().is_empty());
    assert_eq!(g.retained_server(), Some(Handle(3)));
    assert!(!a.generator_trace.iter().any(|s| s == "unregister"));
    a.global(b"bad", V::Number(1.0));
    assert!(!g.init(&mut a, Some(Handle(4)), Some(Handle(1)), b"bad").unwrap());
    assert_eq!(g.retained_server(), Some(Handle(3)));
    assert_eq!(g.name(), b"cmd");
    assert_eq!(a.stack, [V::Number(42.0)]);
    assert_eq!(a.generator_trace.last().unwrap(), "unregister");
}

#[test]
fn lh5_e_f_g_registration_collisions_saved_closures_and_index_reuse() {
    let mut a = Api::default();
    let mut g = generator(&mut a);
    assert!(g.command_registered(&mut a, 7, b"x", b"first").unwrap());
    let saved = binding(&mut a, b"x");
    assert_eq!(g.command_index(7), Some(0));
    assert_eq!(g.records().len(), 20);
    assert!(g.command_registered(&mut a, 7, b"x", b"changed").unwrap());
    assert!(!g.command_registered(&mut a, 7, b"y", b"changed").unwrap());
    assert_eq!(g.records()[0].description, b"first");
    assert_eq!(binding(&mut a, b"x"), saved);
    assert!(g.command_registered(&mut a, 8, b"x", b"collision").unwrap());
    assert!(g.command_unregistered(&mut a, 7).unwrap());
    assert_eq!(binding(&mut a, b"x"), V::Nil);
    assert_eq!(g.command_index(8), Some(1));
    assert_eq!(invoke_binding(&g, &mut a, &saved), 1);
    assert_eq!(a.command_ids, [7]);
    a.stack.clear();
    assert!(g.command_registered(&mut a, 9, b"z", b"reused").unwrap());
    assert_eq!(g.command_index(9), Some(0));
    assert_eq!(invoke_binding(&g, &mut a, &saved), 1);
    assert_eq!(a.command_ids, [7, 9]);
    a.stack.clear();
    a.global(b"cmd", V::Number(0.0));
    assert!(!g.command_registered(&mut a, 10, b"lost", b"record survives").unwrap());
    assert_eq!(g.command_index(10), Some(2));
    assert!(a.stack.is_empty());
    assert!(g.command_registered(&mut a, 10, b"lost", b"ignored").unwrap());
    assert!(g.command_unregistered(&mut a, 10).unwrap());
    assert!(!g.command_unregistered(&mut a, 10).unwrap());
}

#[test]
fn lh5_h_i_j_parameter_types_order_reserved_key_status_and_validation() {
    let mut a = Api::default();
    let mut g = generator(&mut a);
    g.command_registered(&mut a, 7, b"x", b"").unwrap();
    let saved = binding(&mut a, b"x");
    let table = a.new_table();
    a.stack = vec![V::Bool(true), V::Number(1.25), V::String(b"x".to_vec()), V::Nil, table];
    a.command_status = -7;
    a.output_values = vec![
        (9, Some((1, Parameter::Bool(false)))),
        (0x2b999d2b, Some((3, Parameter::Number(99.0)))),
        (2, None),
    ];
    assert_eq!(invoke_binding(&g, &mut a, &saved), 3);
    assert_eq!(
        a.params[&Handle(500)],
        [
            (0, Parameter::Bool(true)),
            (1, Parameter::Number(1.25)),
            (2, Parameter::String(b"x".to_vec())),
            (3, Parameter::Number(0.0)),
            (4, Parameter::Number(0.0))
        ]
    );
    assert_eq!(&a.stack[5..], [V::Number(-7.0), V::Bool(false), V::Nil]);
    a.stack.clear();
    a.output_values = (2..=14)
        .map(|kind| (kind, Some((kind, Parameter::Number(1.0 / 3.0)))))
        .collect();
    a.output_values
        .push((15, Some((15, Parameter::String(b"converted".to_vec())))));
    assert_eq!(invoke_binding(&g, &mut a, &saved), 15);
    assert!(a.stack[1..14].iter().all(|v| *v == V::Number(1.0 / 3.0)));
    assert_eq!(a.stack.last(), Some(&V::String(b"converted".to_vec())));
    a.stack.clear();
    let before = a.params.clone();
    let commands = a.command_ids.len();
    for (generator, main, index) in [
        (Some(Handle(40)), Handle(999), 0.0),
        (None, Handle(10), 0.0),
        (Some(Handle(40)), Handle(10), -1.0),
        (Some(Handle(40)), Handle(10), 1000.0),
    ] {
        assert_eq!(g.callback(&mut a, &thread(), generator, main, index).unwrap(), 1);
        assert_eq!(a.stack.pop(), Some(V::Number(2.0)));
    }
    assert_eq!(a.command_ids.len(), commands);
    assert_eq!(a.params, before);
    // A valid closure can be invoked from a different thread: captured main is
    // compared to the generator's retained view, not the invoking context.
    let child = Thread::from_foreign_view(Handle(77), ThreadKind::Standalone, Handle(200), 3, Some(Handle(10)));
    a.backpointers.insert(Handle(200), Handle(77));
    a.output_values.clear();
    assert_eq!(g.callback(&mut a, &child, Some(Handle(40)), Handle(10), 0.0).unwrap(), 1);
    assert_eq!(stack_at(&mut a, Handle(200)), [V::Number(-7.0)]);
    assert_eq!(a.command_ids.len(), commands + 1);
    assert_eq!(a.precision, 24);
}

#[test]
fn lh5_k_l_shutdown_failure_repeat_and_destructor_do_not_remove_bindings() {
    let mut a = Api::default();
    let mut g = generator(&mut a);
    g.command_registered(&mut a, 7, b"x", b"").unwrap();
    let saved = binding(&mut a, b"x");
    a.unregister = false;
    a.generator_trace.clear();
    assert!(!g.shutdown(&mut a));
    assert_eq!(g.name(), b"cmd");
    assert!(g.records().is_empty());
    assert_eq!(
        a.generator_trace,
        [
            "unregister",
            "release CommandServer",
            "release MainThread",
            "release ParameterSet",
            "release ParameterSet"
        ]
    );
    a.generator_trace.clear();
    assert!(g.shutdown(&mut a));
    assert!(a.generator_trace.is_empty());
    assert_eq!(binding(&mut a, b"x"), saved);
    assert_eq!(invoke_binding(&g, &mut a, &saved), 1);
    assert_eq!(a.stack.pop(), Some(V::Number(2.0)));
    let mut g = generator(&mut a);
    a.generator_trace.clear();
    g.destroy(&mut a);
    assert_eq!(
        a.generator_trace,
        [
            "release CommandServer",
            "release ParameterSet",
            "release ParameterSet",
            "release MainThread"
        ]
    );
    assert_eq!(binding(&mut a, b"x"), saved);
}

#[test]
fn shared_unknown_counts_owner_changes_and_destructors() {
    let mut a = Api::default();
    let mut t = Thread::new(Handle(80), ThreadKind::Standalone);
    assert_eq!(t.release(&mut a), 0); // zero also deletes
    assert_eq!(t.add_ref(), 1);
    let view = t.clone();
    assert_eq!(view.add_ref(), 2);
    assert_eq!(t.release(&mut a), 1);
    assert_eq!(view.references(), 1);
    assert_eq!(t.query_interface(&mut a, THREAD_IID), Some(Handle(80)));
    assert_eq!(view.references(), 2);
    assert_eq!(t.query_interface(&mut a, LUA_IID), None);
    let owner = Thread::new(Handle(90), ThreadKind::Main);
    owner.add_ref();
    t.on_create(&mut a, &owner, Handle(20), 7).unwrap();
    assert_eq!(owner.references(), 2);
    let refs = a.references.len();
    t.on_create(&mut a, &owner, Handle(20), 8).unwrap();
    assert_eq!(a.references.len(), refs); // unchanged owner is not reacquired
    let other = Thread::new(Handle(91), ThreadKind::Main);
    other.add_ref();
    t.on_create(&mut a, &other, Handle(20), 9).unwrap();
    assert_eq!(owner.references(), 1);
    assert_eq!(other.references(), 2);
    assert_eq!(
        &a.references[a.references.len() - 2..],
        [(true, Handle(91)), (false, Handle(90))]
    );
    assert_eq!(view.get_id(), 9);
    assert_eq!(t.release(&mut a), 1);
    assert_eq!(view.release(&mut a), 0);
    assert_eq!(other.references(), 1);
    assert_eq!(view.owner(), None);
    assert_eq!(view.context(), Some(Handle(20))); // destructor does not OnDestroy
    assert!(a.destroy_hooks.is_empty());
    let (mut h, mut a) = opened();
    assert_eq!(h.main_thread.references(), 3); // registry owner, main owner, root
    assert_eq!(h.query_interface(&mut a, LUA_IID), Some(Handle(10)));
    assert_eq!(h.as_thread().references(), 4);
    assert_eq!(h.release(&mut a).unwrap(), 3);
    assert!(!a.operations().contains(&Op::Close));
    h.destroy(&mut a).unwrap();
    assert!(a.operations().contains(&Op::Close));
    assert_eq!(h.as_thread().references(), 0);
    let mut g = generator(&mut a);
    assert_eq!(g.query_interface(&mut a, GENERATOR_IID), Some(Handle(40)));
    assert_eq!(g.add_ref(), 2);
    assert_eq!(g.release(&mut a), 1);
    a.generator_trace.clear();
    assert_eq!(g.release(&mut a), 0);
    assert!(!a.generator_trace.iter().any(|s| s == "unregister"));
}

#[test]
fn thread_hook_failures_parent_validation_and_destroy_order() {
    let mut a = Api {
        reserve: false,
        create_hook_writes: false,
        ..Api::default()
    };
    let mut h = LuaHost::new(Handle(10));
    assert!(!h.open(&mut a, 0).unwrap());
    assert_eq!(h.main_thread.context(), None);
    assert_eq!(h.main_thread.references(), 1); // retained registry owner survives
    a.reserve = true;
    a.create_hook_writes = true;
    assert!(h.open(&mut a, 0).unwrap());
    assert_eq!(h.main_thread.get_id(), 0); // failed hook did not consume an ID
    let (id, _) = child(&mut h, &mut a, 1);
    let t = h.thread_view(id).unwrap().clone();
    let context = t.context().unwrap();
    // Remove only the creating context's backpointer: required-parent creation
    // consumes F but does not register or root the proposed child.
    a.invoke(Some(context), Op::ClearExtraOwner).unwrap();
    let mut out = Some(Handle(999));
    assert_eq!(
        h.create_from(&mut a, context, 0x10, THREAD_CLASS, &mut out).unwrap(),
        -1
    );
    assert_eq!(out, None);
    assert!(h.record(2).is_none());
    assert_eq!(stack_at(&mut a, context), []);
    a.invoke(Some(context), Op::SetExtraOwner(t.identity)).unwrap();
    for _ in 0..3 {
        a.invoke(Some(context), Op::PushString(b"F".to_vec())).unwrap();
        h.create_from(&mut a, context, 0x11, THREAD_CLASS, &mut out).unwrap();
    }
    let children = h.record(id).unwrap().children.clone();
    let middle_id = 3;
    assert!(h.destroy_from(&mut a, middle_id, Some(context)).unwrap());
    assert_eq!(h.record(id).unwrap().children, [children[0], children[2]]); // swap-remove
    a.destroy_hooks.clear();
    a.deny_destroy = Some(context);
    assert!(!h.destroy_from(&mut a, id, None).unwrap());
    assert_eq!(a.destroy_hooks, [children[2], children[0], context]);
    assert!(h.record(id).unwrap().children.is_empty());
    assert!(t.is_valid(&mut a).unwrap());
    a.deny_destroy = None;
    assert!(h.destroy_from(&mut a, id, None).unwrap());
    assert_eq!(t.context(), None);
    assert_eq!(t.owner(), None);
    assert_eq!(t.get_id(), -1);
    assert!(!t.is_valid(&mut a).unwrap());
    h.close(&mut a).unwrap();
}

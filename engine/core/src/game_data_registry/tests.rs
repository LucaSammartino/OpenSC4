// SPDX-License-Identifier: GPL-3.0-or-later
//! Literal §6 ledgers fixed before production in registry-literal-ledgers.md.
use super::*;

fn id(word: u32) -> EntryId {
    EntryId::new(word).unwrap()
}
fn obj(word: u32) -> Object {
    Object::new(word).unwrap()
}
fn target(word: u32) -> Target {
    Target::new(word).unwrap()
}
fn object_ref(word: u32) -> Reference {
    Reference::Object(obj(word))
}
fn target_ref(kind: TargetKind, word: u32) -> Reference {
    Reference::Target(kind, target(word))
}
fn create(token: u32, parent: u32, name: &str) -> Create<'_> {
    Create {
        allocation: Allocation(id(token)),
        name: Some(name),
        parent: Some(id(parent)),
        flags: 0,
        context: 0,
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Effect {
    Add(Reference),
    Release(Reference),
    Query(u32),
    Framework(bool),
    Typed(u32, u32, Option<u32>),
    AllocateMessage,
    MessageType(u32, u32),
    MessageRegistry(u32, u32),
    MessageCode(u32, u32),
    MessageEntry(u32, u32),
    Deliver(u32, u32),
    Call(u32, u32, u32, u32),
    Exchange(u32, u32, u32, bool),
    Destroy(bool),
}
#[derive(Default)]
struct Host {
    effects: Vec<Effect>,
    refs: BTreeMap<ReferenceKey, i32>,
    qi: VecDeque<QueryOutcome>,
    callback_result: bool,
    observe: bool,
    observations: Vec<(u32, [bool; 3], bool)>,
    shutdown_seen: Vec<(bool, bool, bool)>,
    fail_message: bool,
}
// Test-only ledger key. Reference itself deliberately has no registry ordering role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ReferenceKey(u8, u32);
fn ref_key(reference: Reference) -> ReferenceKey {
    match reference {
        Reference::Object(key) => ReferenceKey(0, key.word()),
        Reference::Target(TargetKind::Typed, key) => ReferenceKey(1, key.word()),
        Reference::Target(TargetKind::Message, key) => ReferenceKey(2, key.word()),
        Reference::Message(key) => ReferenceKey(3, key.word()),
    }
}
impl Host {
    fn count(&self, reference: Reference) -> i32 {
        *self.refs.get(&ref_key(reference)).unwrap_or(&0)
    }
    fn typed(&self) -> Vec<(u32, u32, Option<u32>)> {
        self.effects
            .iter()
            .filter_map(|e| match e {
                Effect::Typed(t, c, id) => Some((*t, *c, *id)),
                _ => None,
            })
            .collect()
    }
    fn releases(&self) -> Vec<u32> {
        self.effects
            .iter()
            .filter_map(|e| match e {
                Effect::Release(Reference::Object(k)) => Some(k.word()),
                _ => None,
            })
            .collect()
    }
}
impl Primitives for Host {
    fn add_ref(&mut self, reference: Reference) {
        self.effects.push(Effect::Add(reference));
        *self.refs.entry(ref_key(reference)).or_default() += 1;
    }
    fn release(&mut self, reference: Reference) {
        self.effects.push(Effect::Release(reference));
        *self.refs.entry(ref_key(reference)).or_default() -= 1;
    }
    fn query_unknown(&mut self, object: Object) -> QueryOutcome {
        self.effects.push(Effect::Query(object.word()));
        let out = self.qi.pop_front().unwrap_or(QueryOutcome {
            succeeded: false,
            output: None,
        });
        // QI primitive owns acquiring its output; core must release it, even on false.
        if let Some(key) = out.output {
            *self.refs.entry(ref_key(Reference::Object(key))).or_default() += 1;
        }
        out
    }
    fn framework_register(&mut self, _: ServiceIdentity) {
        self.effects.push(Effect::Framework(true));
    }
    fn framework_unregister(&mut self, _: ServiceIdentity) {
        self.effects.push(Effect::Framework(false));
    }
    fn typed_event(&mut self, target: Target, event: Event, registry: &Registry) -> bool {
        self.effects
            .push(Effect::Typed(target.word(), event.code, event.entry.map(EntryId::word)));
        if self.observe && event.code == 2 {
            let victim = event.entry.unwrap();
            let membership = [
                registry.is_valid(Some(id(103))).unwrap(),
                registry.is_valid(Some(id(104))).unwrap(),
                registry.is_valid(Some(id(101))).unwrap(),
            ];
            // Earlier descendants are unindexed but remain allocated/readable.
            self.observations.push((
                victim.word(),
                membership,
                registry.name(Some(id(103))).unwrap() == Some("X"),
            ));
            assert_eq!(
                registry.parent(Some(victim)),
                Ok(Some(if victim == id(101) { id(100) } else { id(101) }))
            );
        }
        if event.code == 1 {
            self.shutdown_seen.push((
                registry.is_valid(Some(id(100))).unwrap(),
                registry.is_valid(Some(id(101))).unwrap(),
                registry.is_valid(Some(id(105))).unwrap(),
            ));
        }
        self.callback_result
    }
    fn allocate_message(&mut self) -> Domain<Message> {
        self.effects.push(Effect::AllocateMessage);
        if self.fail_message {
            Err(Unsupported::FailedAllocation)
        } else {
            *self.refs.entry(ReferenceKey(3, 800)).or_default() += 1;
            Ok(Message::new(800).unwrap())
        }
    }
    fn message_type(&mut self, message: Message, value: u32) {
        self.effects.push(Effect::MessageType(message.word(), value));
    }
    fn message_registry(&mut self, message: Message, registry: ServiceIdentity) {
        self.effects
            .push(Effect::MessageRegistry(message.word(), registry.word()));
    }
    fn message_code(&mut self, message: Message, code: u32) {
        self.effects.push(Effect::MessageCode(message.word(), code));
    }
    fn message_entry(&mut self, message: Message, entry: EntryId) {
        self.effects.push(Effect::MessageEntry(message.word(), entry.word()));
    }
    fn message_deliver(&mut self, target: Target, message: Message, _: &Registry) -> bool {
        self.effects.push(Effect::Deliver(target.word(), message.word()));
        self.callback_result
    }
    fn value_call(&mut self, callback: Callback, key: u32, input: u32, output: u32, _: &Registry) -> bool {
        self.effects.push(Effect::Call(callback.word(), key, input, output));
        self.callback_result
    }
    fn value_exchange(&mut self, callback: Callback, key: u32, variant: u32, set: bool, _: &Registry) -> bool {
        self.effects.push(Effect::Exchange(callback.word(), key, variant, set));
        self.callback_result
    }
    fn service_destroyed(&mut self, _: ServiceIdentity, free: bool) {
        self.effects.push(Effect::Destroy(free));
    }
}
fn registry(host: &mut Host) -> Registry {
    Registry::new(ServiceIdentity::new(700).unwrap(), Allocation(id(100)), obj(900), host)
}
fn tree(host: &mut Host) -> Registry {
    let mut r = registry(host);
    assert_eq!(r.add_node(Some(obj(11)), create(101, 100, "A"), host), Ok(id(101)));
    assert_eq!(r.add_node(Some(obj(12)), create(102, 100, "B"), host), Ok(id(102)));
    assert_eq!(r.add_node(Some(obj(13)), create(103, 101, "X"), host), Ok(id(103)));
    assert_eq!(r.add_node(Some(obj(14)), create(104, 101, "Y"), host), Ok(id(104)));
    host.effects.clear();
    r
}

#[test]
fn g1_g2_separate_keyspaces_duplicate_unchanged_and_copied_names() {
    let mut h = Host::default();
    let mut r = registry(&mut h);
    let mut name = String::from("same.Name");
    let mut c = create(101, 100, &name);
    c.flags = 0x8000_0000;
    c.context = 19;
    assert_eq!(r.add_node(Some(obj(11)), c, &mut h), Ok(id(101)));
    name.clear();
    assert_eq!(
        r.add_leaf(11, None, LeafKind::Call, create(102, 100, "same.Name"), &mut h),
        Ok(Some(id(102)))
    );
    assert_eq!(r.lookup_object(Some(obj(11)), &mut h), Ok(Some(id(101))));
    assert_eq!(r.lookup_raw(11), Ok(Some(id(102))));
    r.subscribe(TargetKind::Typed, Some(target(70)), &mut h).unwrap();
    h.effects.clear();
    let mut duplicate = create(199, 102, "changed");
    duplicate.context = 999;
    duplicate.flags = 7;
    assert_eq!(r.add_node(Some(obj(11)), duplicate, &mut h), Ok(id(101)));
    assert_eq!(r.add_leaf(11, None, LeafKind::Exchange, duplicate, &mut h), Ok(None));
    assert_eq!(r.name(Some(id(101))), Ok(Some("same.Name")));
    assert_eq!(r.context(Some(id(101))), Ok(Some(19)));
    assert_eq!(r.parent(Some(id(101))), Ok(Some(id(100))));
    assert_eq!(r.check_flags(Some(id(101)), 0x8000_0000), Ok(true));
    assert_eq!(r.check_flags(Some(id(101)), 7), Ok(false));
    assert!(h.effects.is_empty());
}

#[test]
fn g3_anonymous_key_uses_record_token_no_zero_insertion() {
    let mut h = Host::default();
    let mut r = registry(&mut h);
    assert_eq!(
        r.add_leaf(0, None, LeafKind::Exchange, create(105, 100, "anon"), &mut h),
        Ok(Some(id(105)))
    );
    assert_eq!(r.value(Some(id(105))), Ok(Some(105)));
    assert_eq!(r.lookup_raw(0), Ok(None));
    assert_eq!(r.lookup_raw(105), Ok(Some(id(105))));
    assert_eq!(r.lookup_raw(123), Ok(None));
    assert_eq!(r.leaves.len(), 1);
    assert_eq!(
        r.add_leaf(105, None, LeafKind::Call, create(106, 100, "collision"), &mut h),
        Ok(None)
    );
}

#[test]
fn g4_g5_three_orders_and_false_visitors() {
    let mut h = Host::default();
    let mut r = tree(&mut h);
    let mut visits = Vec::new();
    assert_eq!(
        r.visit_immediate(Some(id(100)), 77, |id, ctx, _| {
            assert_eq!(ctx, 77);
            visits.push(id.word());
            true
        }),
        Ok(true)
    );
    assert_eq!(visits, vec![101, 102]);
    visits.clear();
    assert_eq!(
        r.visit_breadth_first(Some(id(100)), 0, |id, _, _| {
            visits.push(id.word());
            true
        }),
        Ok(true)
    );
    assert_eq!(visits, vec![100, 101, 102, 103, 104]);
    visits.clear();
    assert_eq!(
        r.visit_breadth_first(Some(id(100)), 0, |id, _, _| {
            visits.push(id.word());
            id.word() != 101
        }),
        Ok(false)
    );
    assert_eq!(visits, vec![100, 101]);
    visits.clear();
    assert_eq!(
        r.visit_immediate(Some(id(100)), 0, |id, _, _| {
            visits.push(id.word());
            false
        }),
        Ok(false)
    );
    assert_eq!(visits, vec![101]);
    assert_eq!(r.subscribe(TargetKind::Typed, Some(target(70)), &mut h), Ok(true));
    assert_eq!(
        h.typed(),
        vec![
            (70, 3, Some(100)),
            (70, 3, Some(102)),
            (70, 3, Some(101)),
            (70, 3, Some(104)),
            (70, 3, Some(103))
        ]
    );
    // Host defaults false: replay adapter ignores false, unlike either visitor.
    assert!(!h.callback_result);
}

#[test]
fn g6_g7_full_width_flags_context_and_raw_publisher_dispatch() {
    let mut h = Host::default();
    let mut r = registry(&mut h);
    let mut c = create(105, 100, "exchange");
    c.flags = u32::MAX;
    c.context = 0x3333_0000;
    let cb = Callback::new(44);
    r.add_leaf(0x2222_0000, cb, LeafKind::Exchange, c, &mut h).unwrap();
    r.add_leaf(55, cb, LeafKind::Call, create(106, 100, "call"), &mut h)
        .unwrap();
    assert_eq!(r.view(Some(id(105))).unwrap().unwrap().flags, 0xFFFF_FFFE);
    assert_eq!(r.view(Some(id(106))).unwrap().unwrap().flags, 1);
    assert_eq!(r.check_flags(Some(id(105)), 0x8000_0000), Ok(true));
    assert_eq!(r.check_flags(Some(id(105)), u32::MAX), Ok(false));
    assert_eq!(r.check_flags(Some(id(100)), 0), Ok(true));
    h.effects.clear();
    assert_eq!(r.value_exchange(Some(id(105)), 99, true, &mut h), Ok(false));
    assert!(h.effects.is_empty());
    h.callback_result = true;
    assert_eq!(r.value_exchange(Some(id(105)), 99, false, &mut h), Ok(true));
    assert_eq!(r.value_call(Some(id(105)), 21, 22, &mut h), Ok(true));
    assert_eq!(
        h.effects,
        vec![
            Effect::Exchange(44, 0x2222_0000, 99, false),
            Effect::Call(44, 0x2222_0000, 21, 22)
        ]
    );
    assert_eq!(r.context(Some(id(105))), Ok(Some(0x3333_0000)));
    assert_eq!(r.set_context(Some(id(105)), 0), Ok(true));
    assert_eq!(r.context(Some(id(105))), Ok(Some(0)));
    h.callback_result = false;
    assert_eq!(r.value_call(Some(id(106)), 23, 24, &mut h), Ok(false));
    assert_eq!(h.effects.last(), Some(&Effect::Call(44, 55, 23, 24)));
}

#[test]
fn g8_null_missing_leaf_visits_and_borrowed_outputs() {
    let mut h = Host::default();
    let mut r = registry(&mut h);
    h.effects.clear();
    assert_eq!(r.lookup_raw(11), Ok(None));
    assert_eq!(r.lookup_object(None, &mut h), Ok(None));
    assert_eq!(r.name(None), Ok(None));
    assert_eq!(r.node(None), Ok(None));
    assert_eq!(r.value(None), Ok(None));
    assert_eq!(r.parent(None), Ok(None));
    assert_eq!(r.context(None), Ok(None));
    assert_eq!(r.is_node(None), Ok(false));
    assert_eq!(r.set_context(None, 0), Ok(false));
    assert_eq!(r.check_flags(None, 0), Ok(false));
    assert_eq!(r.is_valid(None), Ok(false));
    assert_eq!(r.visit_immediate(None, 0, |_, _, _| panic!("empty")), Ok(true));
    assert_eq!(r.visit_breadth_first(None, 0, |_, _, _| panic!("empty")), Ok(true));
    assert_eq!(r.value_call(None, 0, 0, &mut h), Ok(false));
    assert_eq!(r.value_exchange(None, 0, false, &mut h), Ok(false));
    assert!(h.effects.is_empty());
    r.add_leaf(21, None, LeafKind::Call, create(105, 100, "leaf"), &mut h)
        .unwrap();
    h.effects.clear();
    assert_eq!(
        r.visit_immediate(Some(id(105)), 0, |_, _, _| panic!("leaf has no children")),
        Ok(true)
    );
    let mut visited = Vec::new();
    assert_eq!(
        r.visit_breadth_first(Some(id(105)), 0, |e, _, _| {
            visited.push(e.word());
            true
        }),
        Ok(true)
    );
    assert_eq!(visited, vec![105]);
    assert_eq!(r.root(), Ok(id(100)));
    assert_eq!(r.node(Some(id(100))), Ok(Some(obj(900))));
    assert_eq!(r.name(Some(id(100))), Ok(Some("__root__")));
    assert_eq!(r.parent(Some(id(100))), Ok(None));
    assert_eq!(r.value(Some(id(100))), Ok(None));
    assert_eq!(r.node(Some(id(105))), Ok(None));
    assert_eq!(r.value_call(Some(id(105)), 1, 2, &mut h), Ok(false));
    assert!(h.effects.is_empty());
}

#[test]
fn lookup_exact_then_qi_core_search_and_release_before_return() {
    let mut h = Host::default();
    let mut r = tree(&mut h);
    assert_eq!(r.lookup_object(Some(obj(11)), &mut h), Ok(Some(id(101))));
    assert!(h.effects.is_empty());
    h.qi.push_back(QueryOutcome {
        succeeded: true,
        output: Some(obj(11)),
    });
    assert_eq!(r.lookup_object(Some(obj(51)), &mut h), Ok(Some(id(101))));
    assert_eq!(h.effects, vec![Effect::Query(51), Effect::Release(object_ref(11))]);
    h.effects.clear();
    h.qi.push_back(QueryOutcome {
        succeeded: false,
        output: Some(obj(11)),
    });
    assert_eq!(r.lookup_object(Some(obj(52)), &mut h), Ok(None));
    assert_eq!(h.effects, vec![Effect::Query(52), Effect::Release(object_ref(11))]);
    h.effects.clear();
    h.qi.push_back(QueryOutcome {
        succeeded: true,
        output: Some(obj(99)),
    });
    assert_eq!(r.lookup_object(Some(obj(53)), &mut h), Ok(None));
    assert_eq!(h.effects, vec![Effect::Query(53), Effect::Release(object_ref(99))]);
    // Creation keeps supplied alias54, not canonical99, when fallback misses.
    h.qi.push_back(QueryOutcome {
        succeeded: true,
        output: Some(obj(99)),
    });
    assert_eq!(
        r.add_node(Some(obj(54)), create(105, 100, "alias"), &mut h),
        Ok(id(105))
    );
    assert_eq!(r.node(Some(id(105))), Ok(Some(obj(54))));
    assert!(!r.nodes.contains_key(&99));
    h.effects.clear();
    h.qi.push_back(QueryOutcome {
        succeeded: true,
        output: Some(obj(11)),
    });
    assert_eq!(
        r.add_node(Some(obj(55)), create(106, 100, "duplicate alias"), &mut h),
        Ok(id(101))
    );
    assert_eq!(h.effects, vec![Effect::Query(55), Effect::Release(object_ref(11))]);
    assert_eq!(h.count(object_ref(11)), 1);
    assert_eq!(h.count(object_ref(99)), 0);
}

#[test]
fn g9_remove_notifications_before_each_erase_then_parent_first_destroy() {
    let mut h = Host::default();
    let mut r = tree(&mut h);
    r.subscribe(TargetKind::Typed, Some(target(70)), &mut h).unwrap();
    h.effects.clear();
    h.observe = true;
    assert_eq!(r.remove_object(Some(obj(11)), &mut h), Ok(true));
    assert_eq!(
        h.typed(),
        vec![(70, 2, Some(103)), (70, 2, Some(104)), (70, 2, Some(101))]
    );
    assert_eq!(
        h.observations,
        vec![
            (103, [true, true, true], true),
            (104, [false, true, true], true),
            (101, [false, false, true], true)
        ]
    );
    assert_eq!(
        h.effects,
        vec![
            Effect::Typed(70, 2, Some(103)),
            Effect::Typed(70, 2, Some(104)),
            Effect::Typed(70, 2, Some(101)),
            Effect::Release(object_ref(11)),
            Effect::Release(object_ref(13)),
            Effect::Release(object_ref(14))
        ]
    );
    assert_eq!(r.view(Some(id(100))).unwrap().unwrap().children, &[id(102)]);
    assert_eq!(r.is_valid(Some(id(101))), Ok(false));
    assert_eq!(r.is_valid(Some(id(103))), Ok(false));
    assert_eq!(r.is_valid(Some(id(104))), Ok(false));
    assert_eq!(r.name(Some(id(103))), Err(Unsupported::InvalidRecord));
    h.effects.clear();
    assert_eq!(r.remove_object(Some(obj(900)), &mut h), Ok(false));
    assert!(h.effects.is_empty());
    assert_eq!(r.remove_raw(999, &mut h), Ok(false));
}

#[test]
fn g10_registration_holds_duplicate_no_replay_first_equal_unsubscribe() {
    let mut h = Host::default();
    let mut r = registry(&mut h);
    h.effects.clear();
    let tr = target_ref(TargetKind::Typed, 70);
    assert_eq!(r.subscribe(TargetKind::Typed, Some(target(70)), &mut h), Ok(true));
    assert_eq!(
        h.effects,
        vec![
            Effect::Add(tr),
            Effect::Add(tr),
            Effect::Release(tr),
            Effect::Typed(70, 3, Some(100))
        ]
    );
    assert_eq!(h.count(tr), 1);
    h.effects.clear();
    assert_eq!(r.subscribe(TargetKind::Typed, Some(target(70)), &mut h), Ok(false));
    assert_eq!(h.effects, vec![Effect::Add(tr), Effect::Release(tr)]);
    assert_eq!(h.count(tr), 1);
    h.effects.clear();
    assert_eq!(r.unsubscribe(TargetKind::Typed, Some(target(70)), &mut h), Ok(true));
    assert_eq!(h.effects, vec![Effect::Release(tr)]);
    assert_eq!(h.count(tr), 0);
    h.effects.clear();
    assert_eq!(r.unsubscribe(TargetKind::Typed, Some(target(70)), &mut h), Ok(false));
    assert!(h.effects.is_empty());
    // Structural fixture for the first-equal rule; duplicate registration itself
    // cannot construct this list in the supported nonmutating public domain.
    r.typed = vec![target(70), target(71), target(70)];
    assert_eq!(r.unsubscribe(TargetKind::Typed, Some(target(70)), &mut h), Ok(true));
    assert_eq!(r.typed, vec![target(71), target(70)]);
}

#[test]
fn g11_message_replay_primitive_ledger_and_ordinary_silence() {
    let mut h = Host::default();
    let mut r = registry(&mut h);
    h.effects.clear();
    let mr = target_ref(TargetKind::Message, 80);
    assert_eq!(r.subscribe(TargetKind::Message, Some(target(80)), &mut h), Ok(true));
    assert_eq!(
        h.effects,
        vec![
            Effect::Add(mr),
            Effect::Add(mr),
            Effect::Release(mr),
            Effect::AllocateMessage,
            Effect::MessageType(800, 0x2BDF_E414),
            Effect::MessageRegistry(800, 700),
            Effect::MessageCode(800, 3),
            Effect::MessageEntry(800, 100),
            Effect::Deliver(80, 800),
            Effect::Release(Reference::Message(Message::new(800).unwrap()))
        ]
    );
    assert_eq!(h.count(Reference::Message(Message::new(800).unwrap())), 0);
    h.effects.clear();
    r.add_leaf(21, None, LeafKind::Call, create(105, 100, "Z"), &mut h)
        .unwrap();
    assert!(h.effects.is_empty()); // empty typed list still skips message broadcast
    r.remove_raw(21, &mut h).unwrap();
    assert!(h.effects.is_empty());
    r.subscribe(TargetKind::Typed, Some(target(70)), &mut h).unwrap();
    h.effects.clear();
    r.add_leaf(22, None, LeafKind::Call, create(106, 100, "Z2"), &mut h)
        .unwrap();
    assert_eq!(h.effects, vec![Effect::Typed(70, 3, Some(106))]);
    h.effects.clear();
    r.remove_raw(22, &mut h).unwrap();
    assert_eq!(h.effects, vec![Effect::Typed(70, 2, Some(106))]);
    h.effects.clear();
    r.shutdown(&mut h).unwrap();
    assert_eq!(
        h.effects,
        vec![
            Effect::Framework(false),
            Effect::Typed(70, 1, None),
            Effect::Release(target_ref(TargetKind::Typed, 70)),
            Effect::Release(mr)
        ]
    );
}

#[test]
fn each_new_target_only_gets_own_replay_typed_order_ignores_false() {
    let mut h = Host::default();
    let mut r = tree(&mut h);
    r.subscribe(TargetKind::Typed, Some(target(70)), &mut h).unwrap();
    h.effects.clear();
    r.subscribe(TargetKind::Typed, Some(target(71)), &mut h).unwrap();
    assert_eq!(
        h.typed(),
        vec![
            (71, 3, Some(100)),
            (71, 3, Some(102)),
            (71, 3, Some(101)),
            (71, 3, Some(104)),
            (71, 3, Some(103))
        ]
    );
    h.effects.clear();
    r.subscribe(TargetKind::Message, Some(target(70)), &mut h).unwrap();
    let entries: Vec<_> = h
        .effects
        .iter()
        .filter_map(|e| match e {
            Effect::MessageEntry(_, id) => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(entries, vec![100, 102, 101, 104, 103]);
    assert!(h.typed().is_empty());
    h.effects.clear();
    r.add_leaf(55, None, LeafKind::Call, create(105, 100, "Z"), &mut h)
        .unwrap();
    assert_eq!(
        h.effects,
        vec![Effect::Typed(70, 3, Some(105)), Effect::Typed(71, 3, Some(105))]
    );
    h.effects.clear();
    r.unsubscribe(TargetKind::Message, Some(target(70)), &mut h).unwrap();
    assert_eq!(h.effects, vec![Effect::Release(target_ref(TargetKind::Message, 70))]);
    assert_eq!(h.count(target_ref(TargetKind::Typed, 70)), 1);
}

#[test]
fn g12_shutdown_preserves_root_record_but_not_membership() {
    let mut h = Host::default();
    let mut r = tree(&mut h);
    r.add_leaf(55, None, LeafKind::Call, create(105, 100, "Z"), &mut h)
        .unwrap();
    r.subscribe(TargetKind::Typed, Some(target(70)), &mut h).unwrap();
    r.subscribe(TargetKind::Message, Some(target(80)), &mut h).unwrap();
    h.effects.clear();
    assert_eq!(r.shutdown(&mut h), Ok(true));
    assert_eq!(h.shutdown_seen, vec![(true, true, true)]);
    assert_eq!(
        h.effects,
        vec![
            Effect::Framework(false),
            Effect::Typed(70, 1, None),
            Effect::Release(object_ref(11)),
            Effect::Release(object_ref(13)),
            Effect::Release(object_ref(14)),
            Effect::Release(object_ref(12)),
            Effect::Release(target_ref(TargetKind::Typed, 70)),
            Effect::Release(target_ref(TargetKind::Message, 80))
        ]
    );
    assert_eq!(r.root(), Ok(id(100)));
    assert_eq!(r.is_valid(Some(id(100))), Ok(false));
    assert_eq!(r.node(Some(id(100))), Ok(Some(obj(900))));
    assert_eq!(r.name(Some(id(100))), Ok(Some("__root__")));
    assert!(r.view(Some(id(100))).unwrap().unwrap().children.is_empty());
    assert_eq!(r.lookup_raw(55), Ok(None));
    assert!(r.nodes.is_empty());
    assert!(r.leaves.is_empty());
    assert_eq!(h.count(object_ref(900)), 1);
    h.effects.clear();
    r.destroy(false, &mut h).unwrap();
    assert_eq!(
        h.effects,
        vec![Effect::Release(object_ref(900)), Effect::Destroy(false)]
    );
}

#[test]
fn g13_node_hold_four_five_five_four_borrowed_getters_no_addref() {
    let mut h = Host::default();
    h.refs.insert(ref_key(object_ref(11)), 4);
    let mut r = registry(&mut h);
    r.add_node(Some(obj(11)), create(101, 100, "N"), &mut h).unwrap();
    assert_eq!(h.count(object_ref(11)), 5);
    h.effects.clear();
    assert_eq!(
        r.add_node(Some(obj(11)), create(102, 100, "duplicate"), &mut h),
        Ok(id(101))
    );
    assert_eq!(h.count(object_ref(11)), 5);
    assert_eq!(r.node(Some(id(101))), Ok(Some(obj(11))));
    assert_eq!(r.root(), Ok(id(100)));
    assert_eq!(r.name(Some(id(101))), Ok(Some("N")));
    assert_eq!(r.lookup_object(Some(obj(11)), &mut h), Ok(Some(id(101))));
    assert!(h.effects.is_empty());
    r.remove_object(Some(obj(11)), &mut h).unwrap();
    assert_eq!(h.count(object_ref(11)), 4);
    assert_eq!(h.effects, vec![Effect::Release(object_ref(11))]);
    assert_eq!(r.is_valid(Some(id(101))), Ok(false));
}

#[test]
fn g14_service_shared_count_views_and_zero_one_greater_release() {
    let mut h = Host::default();
    let mut r = registry(&mut h);
    assert_eq!(r.ref_count(), Ok(0));
    assert_eq!(r.query_interface(0xE898_ED03), Ok(Some(Face::Registry)));
    assert_eq!(r.query_interface(0x287F_B697), Ok(Some(Face::SystemService)));
    assert_eq!(r.query_interface(1), Ok(Some(Face::Unknown)));
    assert_eq!(r.ref_count(), Ok(3));
    assert_eq!(r.query_interface(0xFFFF_FFFF), Ok(None));
    assert_eq!(r.ref_count(), Ok(3));
    h.effects.clear();
    assert_eq!(r.release(&mut h), Ok(2));
    assert_eq!(r.release(&mut h), Ok(1));
    assert!(h.effects.is_empty());
    assert_eq!(r.release(&mut h), Ok(0));
    assert_eq!(h.effects, vec![Effect::Release(object_ref(900)), Effect::Destroy(true)]);
    assert_eq!(r.root(), Err(Unsupported::Destroyed));
    let mut zero = registry(&mut h);
    h.effects.clear();
    assert_eq!(zero.release(&mut h), Ok(0));
    assert_eq!(h.effects, vec![Effect::Release(object_ref(900)), Effect::Destroy(true)]);
}

#[test]
fn destructor_does_not_synthesize_shutdown_and_node_release_is_preorder() {
    let mut h = Host::default();
    let mut r = tree(&mut h);
    r.subscribe(TargetKind::Typed, Some(target(70)), &mut h).unwrap();
    h.effects.clear();
    assert_eq!(r.init(&mut h), Ok(true));
    assert_eq!(h.effects, vec![Effect::Framework(true)]);
    h.effects.clear();
    r.destroy(true, &mut h).unwrap();
    assert_eq!(h.releases(), vec![900, 11, 13, 14, 12]);
    assert_eq!(
        h.effects,
        vec![
            Effect::Release(object_ref(900)),
            Effect::Release(object_ref(11)),
            Effect::Release(object_ref(13)),
            Effect::Release(object_ref(14)),
            Effect::Release(object_ref(12)),
            Effect::Release(target_ref(TargetKind::Typed, 70)),
            Effect::Destroy(true)
        ]
    );
}

#[test]
fn tagged_unsupported_inputs_distinct_from_defined_negatives() {
    let mut h = Host::default();
    let mut r = registry(&mut h);
    assert_eq!(r.name(Some(id(999))), Err(Unsupported::InvalidRecord));
    assert_eq!(r.is_valid(Some(id(999))), Ok(false));
    assert_eq!(
        r.add_node(None, create(101, 100, "bad"), &mut h),
        Err(Unsupported::NullNodeKey)
    );
    let mut c = create(101, 100, "bad");
    c.parent = None;
    assert_eq!(r.add_node(Some(obj(11)), c, &mut h), Err(Unsupported::NullNodeParent));
    c = create(101, 100, "bad");
    c.name = None;
    assert_eq!(r.add_node(Some(obj(11)), c, &mut h), Err(Unsupported::NullName));
    assert_eq!(
        r.add_node(Some(obj(11)), create(100, 100, "bad"), &mut h),
        Err(Unsupported::ReusedAllocation)
    );
    assert_eq!(
        r.value_call(Some(id(100)), 0, 0, &mut h),
        Err(Unsupported::CallbackSignature)
    );
    assert_eq!(r.debug_dump(None), Ok(()));
    assert_eq!(r.debug_dump(Some(id(100))), Err(Unsupported::DiagnosticOutput));
    r.add_leaf(21, None, LeafKind::Exchange, create(105, 100, "leaf"), &mut h)
        .unwrap();
    assert_eq!(
        r.add_leaf(22, None, LeafKind::Exchange, create(106, 105, "leaf child"), &mut h),
        Ok(None)
    );
    assert_eq!(
        r.add_node(Some(obj(11)), create(106, 105, "node child"), &mut h),
        Err(Unsupported::NodeParentIsLeaf)
    );
    assert_eq!(
        r.add_leaf(0, None, LeafKind::Call, create(21, 100, "anon collision"), &mut h),
        Err(Unsupported::AnonymousKeyCollision)
    );
    assert_eq!(
        r.subscribe(TargetKind::Typed, None, &mut h),
        Err(Unsupported::InvalidRecord)
    );
    h.fail_message = true;
    assert_eq!(
        r.subscribe(TargetKind::Message, Some(target(80)), &mut h),
        Err(Unsupported::FailedAllocation)
    );
    assert_eq!(h.count(target_ref(TargetKind::Message, 80)), 1); // no fabricated rollback
    r.shutdown(&mut h).unwrap();
    assert_eq!(
        r.add_leaf(23, None, LeafKind::Call, create(106, 100, "restart"), &mut h),
        Err(Unsupported::AfterShutdown)
    );
}

#[test]
fn leaves_retain_nothing_and_delete_wrapper_uses_only_low_bit() {
    let mut h = Host::default();
    let mut r = registry(&mut h);
    h.effects.clear();
    let mut c = create(105, 100, "raw");
    c.context = 0x3333_0000;
    r.add_leaf(0x2222_0000, Callback::new(44), LeafKind::Call, c, &mut h)
        .unwrap();
    assert!(h.effects.is_empty());
    r.remove_raw(0x2222_0000, &mut h).unwrap();
    assert!(h.effects.is_empty());
    r.delete_wrapper(0xFFFF_FFFE, &mut h).unwrap();
    assert_eq!(
        h.effects,
        vec![Effect::Release(object_ref(900)), Effect::Destroy(false)]
    );
    let mut r = registry(&mut h);
    h.effects.clear();
    r.delete_wrapper(0x8000_0001, &mut h).unwrap();
    assert_eq!(h.effects, vec![Effect::Release(object_ref(900)), Effect::Destroy(true)]);
}

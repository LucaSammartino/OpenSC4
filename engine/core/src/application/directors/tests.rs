// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::cell::Cell;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Call {
    Heap(Token),
    Init(Token, Option<Token>, Vec<u8>),
    Class(Token, u32, u32),
    Enum(Token),
    Unload(Token),
    Factory(Token),
    Query(Token, u32),
    AddRef(Token),
    Release(Token),
    Callback(Token, u32),
    Create(Token, u32, u32),
    Service(Token, Token),
    Update(Token, Token),
    Hooks(Token, Identity),
    Manager(Token, u32, u32),
    Bind(Token, u32, u32, Option<Token>),
    GlobalCom,
    Subordinate(Subordinate),
    Start(Token, Option<Token>),
    Free(Token),
    Exit(Owner),
}

struct Mock {
    calls: Vec<Call>,
    child_edges: BTreeMap<Token, Vec<Token>>,
    child_winners: BTreeSet<Token>,
    child_enums: BTreeMap<Token, Vec<u32>>,
    unload: bool,
    factory_object: Option<Token>,
    query_ok: bool,
    callback_ok: bool,
    create_ok: bool,
    service: Option<Token>,
    framework: Option<Token>,
    manager: Option<Token>,
}

impl Default for Mock {
    fn default() -> Self {
        Self {
            calls: vec![],
            child_edges: BTreeMap::new(),
            child_winners: BTreeSet::new(),
            child_enums: BTreeMap::new(),
            unload: false,
            factory_object: Some(800),
            query_ok: false,
            callback_ok: false,
            create_ok: false,
            service: None,
            framework: Some(900),
            manager: None,
        }
    }
}

impl Host for Mock {
    fn heap(&mut self, com: Token) -> Option<Token> {
        self.calls.push(Call::Heap(com));
        Some(com + 100)
    }
    fn child_initialize(&mut self, child: Token, com: Option<Token>, path: &[u8]) -> bool {
        self.calls.push(Call::Init(child, com, path.to_vec()));
        false
    }
    fn child_children(&mut self, child: Token) -> Vec<Token> {
        self.child_edges.get(&child).cloned().unwrap_or_default()
    }
    fn child_class(&mut self, child: Token, class: u32, iid: u32, out: &mut Option<Token>) -> bool {
        self.calls.push(Call::Class(child, class, iid));
        if self.child_winners.contains(&child) {
            *out = Some(child + 1000);
            true
        } else {
            false
        }
    }
    fn child_enum(&mut self, child: Token, callback: &mut dyn FnMut(u32, u32, Token) -> bool, context: Token) {
        self.calls.push(Call::Enum(child));
        if let Some(classes) = self.child_enums.get(&child) {
            for &class in classes {
                callback(class, 0, context);
            }
        }
    }
    fn child_unload(&mut self, child: Token) -> bool {
        self.calls.push(Call::Unload(child));
        self.unload
    }
    fn factory(&mut self, factory: Token) -> Option<Token> {
        self.calls.push(Call::Factory(factory));
        self.factory_object
    }
    fn query(&mut self, object: Token, iid: u32, out: &mut Option<Token>) -> bool {
        self.calls.push(Call::Query(object, iid));
        if self.query_ok {
            *out = Some(object + 4);
        }
        self.query_ok
    }
    fn add_ref(&mut self, object: Token) {
        self.calls.push(Call::AddRef(object));
    }
    fn release(&mut self, object: Token) {
        self.calls.push(Call::Release(object));
    }
    fn callback(&mut self, callback: Token, iid: u32, out: &mut Option<Token>) -> bool {
        self.calls.push(Call::Callback(callback, iid));
        *out = Some(456);
        self.callback_ok
    }
    fn create(&mut self, com: Token, class: u32, iid: u32, out: &mut Option<Token>) -> bool {
        assert_eq!(*out, None, "Start must initialize its local output to null");
        self.calls.push(Call::Create(com, class, iid));
        *out = self.service;
        self.create_ok
    }
    fn framework(&mut self) -> Option<Token> {
        self.framework
    }
    fn register_service(&mut self, framework: Token, service: Token) -> bool {
        self.calls.push(Call::Service(framework, service));
        false
    }
    fn register_update(&mut self, framework: Token, service: Token) -> bool {
        self.calls.push(Call::Update(framework, service));
        false
    }
    fn register_hooks(&mut self, framework: Token, hooks: Identity) -> bool {
        self.calls.push(Call::Hooks(framework, hooks));
        false
    }
    fn manager(&mut self, framework: Token, service: u32, iid: u32, out: &mut Option<Token>) -> bool {
        assert_eq!(*out, None);
        self.calls.push(Call::Manager(framework, service, iid));
        *out = self.manager;
        false
    }
    fn register_factory(&mut self, manager: Token, class: u32, resource_type: u32, factory: Option<Token>) -> bool {
        self.calls.push(Call::Bind(manager, class, resource_type, factory));
        false
    }
    fn global_com(&mut self) -> Option<Token> {
        self.calls.push(Call::GlobalCom);
        Some(77)
    }
    fn subordinate(&mut self, which: Subordinate) -> Token {
        self.calls.push(Call::Subordinate(which));
        SUBORDINATES.iter().position(|&(s, _)| s == which).unwrap() as u32 + 10
    }
    fn child_start(&mut self, child: Token, com: Option<Token>) -> bool {
        self.calls.push(Call::Start(child, com));
        false
    }
    fn free_allocation(&mut self, allocation: Token) {
        self.calls.push(Call::Free(allocation));
    }
    fn register_exit(&mut self, owner: Owner) -> bool {
        self.calls.push(Call::Exit(owner));
        false
    }
}

const OWNERS: [Owner; 4] = [Owner::Tools, Owner::ResourceLib, Owner::Game, Owner::ResMan];

fn fresh(owner: Owner) -> Director {
    Director::new(owner, 50, false)
}

#[test]
fn ad_v1_v2_v3_shared_faces_and_non_deleting_lifetime() {
    for owner in OWNERS {
        let mut d = fresh(owner);
        assert_eq!(d.count(), 0);
        assert_eq!(d.release(), 0);
        assert_eq!(d.add_ref(), 1);
        assert_eq!(d.release(), 0);
        let mut out = None;
        assert!(d.query_interface(HOOKS_IID, &mut out));
        assert_eq!(
            out,
            Some(Identity {
                object: 50,
                face: Face::Hooks
            })
        );
        // QI on either face forwards to the same primary receiver.
        assert!(d.query_interface(1, &mut out));
        assert_eq!(
            out,
            Some(Identity {
                object: 50,
                face: Face::Primary
            })
        );
        assert_eq!(d.count(), 2);
        let sentinel = out;
        assert!(!d.query_interface(u32::MAX, &mut out));
        assert_eq!(out, sentinel);
        assert_eq!(d.count(), 2);
        assert!(d.query_interface(DIRECTOR_IID, &mut out));
        assert_eq!(d.count(), 3);
        d.count = u32::MAX;
        assert_eq!(d.add_ref(), 0);
        assert!(!d.classes.is_empty(), "zero count never destroys the table");
    }
}

#[test]
fn ad_v4_v5_v6_initialization_and_descendant_first_flattening() {
    for owner in OWNERS {
        let mut h = Mock::default();
        let mut d = fresh(owner);
        assert_eq!(
            (d.com(), d.heap(), d.path(), d.children()),
            (None, None, &[][..], &[][..])
        );
        assert!(d.initialize(Some(1), b"old", &mut h));
        h.child_edges.insert(10, vec![11]);
        d.add_director(10, &mut h);
        assert_eq!(d.children(), &[11, 10]);
        assert_eq!(
            h.calls,
            vec![
                Call::Heap(1),
                Call::Init(10, Some(1), b"old".to_vec()),
                Call::Init(11, Some(1), b"old".to_vec())
            ]
        );
        h.calls.clear();
        assert!(!d.initialize(None, b"new", &mut h));
        assert_eq!((d.com(), d.heap(), d.path()), (Some(1), Some(101), &b"old"[..]));
        assert!(h.calls.is_empty());
        assert!(d.initialize(Some(2), b"x", &mut h));
        assert_eq!(
            h.calls,
            vec![
                Call::Heap(2),
                Call::Init(11, Some(2), b"x".to_vec()),
                Call::Init(10, Some(2), b"x".to_vec())
            ]
        );
        assert_eq!((d.com(), d.heap(), d.path()), (Some(2), Some(102), &b"x"[..]));
        d.add_director(10, &mut h);
        assert_eq!(d.children(), &[11, 10, 11, 10]);
    }
}

#[test]
fn ad_v7_v8_child_first_lookup_and_preserved_outputs() {
    let mut h = Mock::default();
    let mut d = fresh(Owner::Tools);
    d.add_director(10, &mut h);
    d.add_director(11, &mut h);
    d.add_class(1234, Registration::Factory(99));
    h.child_winners.extend([10, 11]);
    h.calls.clear();
    let mut out = Some(42);
    assert_eq!(d.get_class_object(1234, 9, &mut out, &mut h), Ok(true));
    assert_eq!(out, Some(1010));
    assert_eq!(h.calls, vec![Call::Class(10, 1234, 9)]);
    h.child_winners.clear();
    h.calls.clear();
    out = Some(42);
    assert_eq!(d.get_class_object(5678, 9, &mut out, &mut h), Ok(false));
    assert_eq!(out, Some(42));
    assert_eq!(h.calls, vec![Call::Class(10, 5678, 9), Call::Class(11, 5678, 9)]);
}

#[test]
fn ad_v9_factory_qi_failure_success_callback_and_null_fault() {
    let mut h = Mock::default();
    let mut d = fresh(Owner::Tools);
    assert!(d.add_class(1234, Registration::Factory(99)));
    assert!(!d.add_class(1234, Registration::Factory(88)));
    let mut out = Some(42);
    assert_eq!(d.get_class_object(1234, 7, &mut out, &mut h), Ok(false));
    assert_eq!(out, Some(42));
    assert_eq!(
        h.calls,
        vec![
            Call::Factory(99),
            Call::Query(800, 7),
            Call::AddRef(800),
            Call::Release(800)
        ]
    );
    h.calls.clear();
    h.query_ok = true;
    assert_eq!(d.get_class_object(1234, 7, &mut out, &mut h), Ok(true));
    assert_eq!(out, Some(804));
    assert_eq!(h.calls, vec![Call::Factory(99), Call::Query(800, 7)]);
    h.calls.clear();
    h.factory_object = None;
    assert_eq!(
        d.get_class_object(1234, 7, &mut out, &mut h),
        Err(Boundary::NullFactoryObject)
    );
    assert_eq!(h.calls, vec![Call::Factory(99)]);
    d.add_class(4321, Registration::Callback(77));
    h.calls.clear();
    assert_eq!(d.get_class_object(4321, 7, &mut out, &mut h), Ok(false));
    assert_eq!(out, Some(456), "callback controls output even on false");
    assert_eq!(h.calls, vec![Call::Callback(77, 7)]);
    h.callback_ok = true;
    assert_eq!(d.get_class_object(4321, 7, &mut out, &mut h), Ok(true));
    d.add_class(99, Registration::Unknown(3));
    h.calls.clear();
    out = Some(42);
    assert_eq!(d.get_class_object(99, 7, &mut out, &mut h), Ok(false));
    assert_eq!(out, Some(42));
    assert!(h.calls.is_empty());
}

#[test]
fn ad_v10_unload_count_short_circuits_and_children_stop_at_false() {
    let mut h = Mock::default();
    let mut d = fresh(Owner::ResMan);
    d.add_director(10, &mut h);
    d.add_director(11, &mut h);
    d.add_ref();
    h.calls.clear();
    assert!(d.can_unload(&mut h));
    assert!(h.calls.is_empty());
    d.release();
    assert!(!d.can_unload(&mut h));
    assert_eq!(h.calls, vec![Call::Unload(10)]);
    h.calls.clear();
    h.unload = true;
    assert!(d.can_unload(&mut h));
    assert_eq!(h.calls, vec![Call::Unload(10), Call::Unload(11)]);
}

#[test]
fn ad_v11_enumeration_children_then_local_duplicates_and_false_callbacks() {
    let mut h = Mock::default();
    let mut d = fresh(Owner::Tools);
    d.add_director(10, &mut h);
    d.add_director(11, &mut h);
    h.child_enums.insert(10, vec![1234, 1234]);
    h.child_enums.insert(11, vec![1234]);
    d.add_class(1234, Registration::Factory(99));
    h.calls.clear();
    let mut seen = vec![];
    d.enum_classes(
        &mut |class, zero, context| {
            seen.push((class, zero, context));
            false
        },
        42,
        &mut h,
    );
    assert_eq!(&seen[..3], &[(1234, 0, 42); 3]);
    assert_eq!(seen.len(), 6);
    assert!(seen.iter().all(|&(_, zero, context)| zero == 0 && context == 42));
    assert_eq!(seen.iter().filter(|&&(class, _, _)| class == 1234).count(), 4);
    assert_eq!(h.calls, vec![Call::Enum(10), Call::Enum(11)]);
}

#[test]
fn ad_v12_all_valid_noop_hooks_and_start() {
    let mut h = Mock::default();
    for owner in OWNERS {
        let d = fresh(owner);
        for slot in 3..=10 {
            if owner == Owner::ResourceLib && slot == 4 {
                continue;
            }
            assert_eq!(d.hook(slot, &mut h), Ok(true));
        }
        assert_eq!(d.hook(11, &mut h), Err(Boundary::InvalidHooksSlot));
        assert_eq!(d.hook(2, &mut h), Err(Boundary::InvalidHooksSlot));
        assert!(d.primary_success_hook());
        if matches!(owner, Owner::Tools | Owner::Game) {
            assert_eq!(d.start(None, &mut h), Ok(true));
        }
    }
    assert!(h.calls.is_empty());
}

#[test]
fn ad_v13_v14_resource_start_stored_com_pointer_test_and_repeat() {
    let mut h = Mock::default();
    let mut d = fresh(Owner::ResourceLib);
    d.initialize(Some(1), b"", &mut h);
    h.calls.clear();
    h.create_ok = true;
    assert_eq!(d.start(Some(2), &mut h), Ok(true));
    assert_eq!(
        h.calls,
        vec![
            Call::Create(1, 0x252EA0DF, 0x287FB697),
            Call::Hooks(900, d.identity(Face::Hooks))
        ]
    );
    h.calls.clear();
    h.create_ok = false;
    h.service = Some(700);
    let expected = vec![
        Call::Create(1, 0x252EA0DF, 0x287FB697),
        Call::Service(900, 700),
        Call::Release(700),
        Call::Hooks(900, d.identity(Face::Hooks)),
    ];
    assert_eq!(d.start(Some(2), &mut h), Ok(true));
    assert_eq!(h.calls, expected);
    h.calls.clear();
    assert_eq!(d.start(Some(2), &mut h), Ok(true));
    assert_eq!(h.calls, expected);
}

#[test]
fn ad_v15_v16_resource_binding_missing_manager_and_false_results() {
    let d = fresh(Owner::ResourceLib);
    let mut h = Mock {
        framework: None,
        ..Mock::default()
    };
    assert_eq!(d.hook(4, &mut h), Ok(false));
    assert!(h.calls.is_empty());
    h.framework = Some(900);
    assert_eq!(d.hook(4, &mut h), Ok(false));
    assert_eq!(h.calls, vec![Call::Manager(900, 0x056B906E, 0x656B8EFC)]);
    h.calls.clear();
    h.manager = Some(600);
    let expected = vec![
        Call::Manager(900, 0x056B906E, 0x656B8EFC),
        Call::Bind(600, 0x453429B3, 0x6534284A, None),
        Call::Bind(600, 0x053429C8, 0x05342861, None),
        Call::Release(600),
    ];
    assert_eq!(d.hook(4, &mut h), Ok(true));
    assert_eq!(h.calls, expected);
    h.calls.clear();
    assert_eq!(d.hook(4, &mut h), Ok(true));
    assert_eq!(h.calls, expected);
}

#[test]
fn ad_v17_false_hook_does_not_cancel_framework_phase() {
    let d = fresh(Owner::ResourceLib);
    let mut h = Mock {
        framework: None,
        ..Mock::default()
    };
    let later = Cell::new(false);
    let mut first = || d.hook(4, &mut h).unwrap();
    let mut second = || {
        later.set(true);
        false
    };
    assert!(dispatch_hooks(&mut [&mut first, &mut second]));
    assert!(later.get());
}

#[test]
fn ad_v18_v19_resman_supplied_com_and_null_service() {
    let mut h = Mock {
        service: Some(700),
        ..Mock::default()
    };
    let mut d = fresh(Owner::ResMan);
    d.initialize(Some(1), b"", &mut h);
    h.calls.clear();
    let expected = vec![
        Call::Create(2, 0x656B8F27, 0x287FB697),
        Call::Service(900, 700),
        Call::Update(900, 700),
        Call::Release(700),
    ];
    assert_eq!(d.start(Some(2), &mut h), Ok(true));
    assert_eq!(h.calls, expected);
    h.calls.clear();
    assert_eq!(d.start(Some(2), &mut h), Ok(true));
    assert_eq!(h.calls, expected);
    h.calls.clear();
    h.service = None;
    h.create_ok = true;
    h.framework = None;
    assert_eq!(d.start(Some(2), &mut h), Ok(true));
    assert_eq!(h.calls, vec![Call::Create(2, 0x656B8F27, 0x287FB697)]);
}

#[test]
fn ad_v20_v21_game_wires_global_root_fourteen_in_order_ten_starts() {
    let mut h = Mock::default();
    let d = Director::construct_game(50, false, None, &mut h);
    assert_eq!(d.classes.len(), 190);
    assert!(d.children().is_empty());
    assert!(h.calls.is_empty());
    let mut root = fresh(Owner::Tools);
    root.initialize(Some(1), b"root", &mut h);
    h.calls.clear();
    let d = Director::construct_game(50, false, Some(&mut root), &mut h);
    let mut expected = vec![Call::GlobalCom];
    // Independent literal fixture, rather than using SUBORDINATES to predict itself.
    let sequence = [
        (Subordinate::ScriptedWindow, true),
        (Subordinate::GDrivers, true),
        (Subordinate::Fonts, false),
        (Subordinate::Drawing, true),
        (Subordinate::Graphics, false),
        (Subordinate::ResMan, false),
        (Subordinate::WindowsResource, true),
        (Subordinate::Window, false),
        (Subordinate::Image, true),
        (Subordinate::ResourceLib, true),
        (Subordinate::Script, true),
        (Subordinate::UiScript, true),
        (Subordinate::Automata, true),
        (Subordinate::Lua, true),
    ];
    for (index, (which, start)) in sequence.into_iter().enumerate() {
        let child = index as u32 + 10;
        expected.push(Call::Subordinate(which));
        expected.push(Call::Init(child, Some(1), b"root".to_vec()));
        if start {
            expected.push(Call::Start(child, Some(77)));
        }
    }
    assert_eq!(h.calls, expected);
    assert_eq!(h.calls.iter().filter(|c| matches!(c, Call::Start(..))).count(), 10);
    assert_eq!(root.children(), &(10..24).collect::<Vec<_>>());
    assert!(d.children().is_empty());
    assert_eq!(d.classes.len(), 190);
}

#[test]
fn ad_v22_complete_class_sets_dispatch_foreign_qi_faces() {
    let mut h = Mock {
        query_ok: true,
        ..Mock::default()
    };
    for (owner, size) in [
        (Owner::Tools, 2),
        (Owner::ResourceLib, 25),
        (Owner::ResMan, 16),
        (Owner::Game, 190),
    ] {
        let d = fresh(owner);
        let mut seen = BTreeSet::new();
        d.enum_classes(
            &mut |class, _, _| {
                seen.insert(class);
                false
            },
            0,
            &mut h,
        );
        assert_eq!(seen.len(), size);
        for &(class, factory) in owner.registrations() {
            h.calls.clear();
            let mut out = None;
            assert_eq!(d.get_class_object(class, 7, &mut out, &mut h), Ok(true));
            assert_eq!(out, Some(804));
            assert_eq!(h.calls, vec![Call::Factory(factory), Call::Query(800, 7)]);
        }
    }
    // Discriminating literal identities: adjusted-face factories and endpoints.
    assert_eq!(RESOURCE_CLASSES[0], (0x453429B3, 0x00664240));
    assert!(RESOURCE_CLASSES.contains(&(0x071E971B, 0x006648F0)));
    assert!(RESOURCE_CLASSES.contains(&(0x252EA0DF, 0x00664220)));
    assert_eq!(RESOURCE_CLASSES[24], (0x5CF9459A, 0x00681CA0));
    assert_eq!(GAME_CLASSES[0], (0x252294E9, 0x0092EBB7));
    assert_eq!(GAME_CLASSES[189], (0xC2BF1DC5, 0x0080EE60));
}

#[test]
fn registration_inventories_match_published_spec_independently() {
    // Compile-time spec fixtures, not production data loading or native evidence.
    let spec = include_str!("../../../../../docs/specs/app-directors.md");
    let appendix = spec.split("## Appendix A.").nth(1);
    // The heading is "Appendix A" in the current contract; anchor the table.
    let appendix = appendix.unwrap_or_else(|| spec.split("| Ordinals | Class").nth(1).unwrap());
    let mut parsed = vec![];
    for fragment in appendix.split('`') {
        if let Some((class, factory)) = fragment.split_once('→') {
            if let (Ok(class), Ok(factory)) = (
                u32::from_str_radix(class.trim(), 16),
                u32::from_str_radix(factory.trim(), 16),
            ) {
                parsed.push((class, factory));
            }
        }
    }
    assert_eq!(GAME_CLASSES, parsed);

    fn table_pairs(section: &str) -> Vec<(u32, u32)> {
        section
            .lines()
            .filter_map(|line| {
                let mut columns = line.strip_prefix('|')?.split('|');
                let class = columns.next()?.trim().trim_matches('`').trim_start_matches("0x");
                let factory = columns
                    .next()?
                    .split_whitespace()
                    .next()?
                    .trim_matches('`')
                    .trim_start_matches("0x");
                Some((
                    u32::from_str_radix(class, 16).ok()?,
                    u32::from_str_radix(factory, 16).ok()?,
                ))
            })
            .collect()
    }
    let resman = spec.split("### 6.1").nth(1).unwrap().split("### 6.2").next().unwrap();
    assert_eq!(RESMAN_CLASSES, table_pairs(resman));
    let resource = include_str!("../../../../../docs/specs/resource-exemplar-editor.md");
    let editors = resource.split("## 8.").nth(1).unwrap().split("### 8.1").next().unwrap();
    assert_eq!(&RESOURCE_CLASSES[3..23], table_pairs(editors));
    assert_eq!(TOOLS_CLASSES, &[(0x3CEC2B47, 0x0094AB94), (0x9CEC2B58, 0x0094A4A6)]);
    assert_eq!(
        &RESOURCE_CLASSES[..3],
        &[
            (0x453429B3, 0x00664240),
            (0x053429C8, 0x00664260),
            (0x252EA0DF, 0x00664220)
        ]
    );
    assert_eq!(
        &RESOURCE_CLASSES[23..],
        &[(0x69B6F01C, 0x00664510), (0x5CF9459A, 0x00681CA0)]
    );
}

#[test]
fn invalid_start_preconditions_are_faults_and_balance_acquired_service() {
    let mut h = Mock::default();
    let mut resource = fresh(Owner::ResourceLib);
    let resman = fresh(Owner::ResMan);
    assert_eq!(resource.start(Some(1), &mut h), Err(Boundary::RequiredCom));
    assert_eq!(resman.start(None, &mut h), Err(Boundary::RequiredCom));
    assert!(h.calls.is_empty());
    resource.initialize(Some(1), b"", &mut h);
    h.calls.clear();
    h.framework = None;
    assert_eq!(resource.start(Some(2), &mut h), Err(Boundary::RequiredFramework));
    assert_eq!(h.calls, vec![Call::Create(1, 0x252EA0DF, SERVICE_IID)]);
    h.service = Some(700);
    for d in [resource, resman] {
        h.calls.clear();
        assert_eq!(d.start(Some(2), &mut h), Err(Boundary::RequiredFramework));
        assert_eq!(h.calls.len(), 2);
        assert_eq!(h.calls[1], Call::Release(700));
    }
}

#[test]
fn ad_v25_library_path_heap_size_and_owner_ids() {
    let mut h = Mock::default();
    for (owner, id) in [
        (Owner::Tools, 0x1CEC20BE),
        (Owner::ResourceLib, 0x84FE3294),
        (Owner::Game, 0x63CC0AF8),
        (Owner::ResMan, 0xC3CAEC3B),
    ] {
        let mut d = fresh(owner);
        assert_eq!(d.owner().id(), id);
        d.initialize(Some(1), b"abc", &mut h);
        let mut out = b"sentinel".to_vec();
        h.calls.clear();
        assert!(d.library_path(&mut out));
        assert_eq!(out, b"abc");
        assert_eq!(d.heap_allocated_size(), 0);
        assert!(h.calls.is_empty());
    }
}

#[test]
fn ad_v26_getters_borrow_static_identity_once_and_allow_recursive_access() {
    let mut h = Mock::default();
    for owner in OWNERS {
        let mut slot = Singleton::new(owner, 50);
        assert_eq!(
            slot.begin_get(),
            Get {
                identity: 50,
                construct: true
            }
        );
        assert_eq!(
            slot.begin_get(),
            Get {
                identity: 50,
                construct: false
            }
        );
        assert!(
            slot.director().is_none(),
            "recursive access does not wait for construction"
        );
        slot.complete_get(None, &mut h);
        assert_eq!(slot.get(None, &mut h), 50);
        assert_eq!(slot.get(None, &mut h), 50);
        assert_eq!(slot.director().unwrap().count(), 0);
        assert_eq!(h.calls, vec![Call::Exit(owner)]);
        h.calls.clear();
        assert_eq!(slot.exit(&mut h), Ok(()));
        assert!(h.calls.is_empty());
        assert!(slot.director().unwrap().classes.is_empty());
    }
}

#[test]
fn tools_unwind_retry_and_other_getter_unwind_open_adir2() {
    let mut h = Mock::default();
    let mut slot = Singleton::new(Owner::Tools, 50);
    assert!(slot.begin_get().construct);
    let mut base = fresh(Owner::Tools);
    base.initialize(Some(1), b"x", &mut h);
    base.add_director(10, &mut h);
    h.calls.clear();
    assert_eq!(slot.construction_failed(Some(&mut base), &mut h), Ok(()));
    assert!(base.classes.is_empty());
    assert!(base.path().is_empty());
    assert!(base.children().is_empty());
    assert!(h.calls.is_empty());
    assert!(slot.begin_get().construct);
    for owner in [Owner::Game, Owner::ResourceLib, Owner::ResMan] {
        let mut slot = Singleton::new(owner, 50);
        slot.begin_get();
        assert_eq!(
            slot.construction_failed(None, &mut h),
            Err(Boundary::ExceptionalSingletonConstruction)
        );
    }
}

#[test]
fn ad_v27_deleting_flags_free_only_low_bit_never_release_borrowed_refs() {
    for owner in OWNERS {
        for flags in [0, 1, 2, 3] {
            let mut h = Mock::default();
            let mut d = fresh(owner);
            d.initialize(Some(1), b"abc", &mut h);
            d.add_director(10, &mut h);
            h.calls.clear();
            assert_eq!(d.destroy(flags, &mut h), Ok(50));
            assert!(d.classes.is_empty());
            assert!(d.children().is_empty());
            assert!(d.path().is_empty());
            assert_eq!(h.calls, if flags & 1 == 0 { vec![] } else { vec![Call::Free(50)] });
        }
        let mut h = Mock::default();
        let mut d = Director::new(owner, 50, true);
        assert_eq!(d.destroy(1, &mut h), Err(Boundary::StaticStorageFree));
        assert!(h.calls.is_empty());
        assert!(!d.classes.is_empty());
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
//! Traces from system-service-pointer.md §5. Rows that need a caller owner's own
//! algorithm (D05-D07, R04/R05) are outside this utility and are not run here.

use super::*;
use std::collections::BTreeMap;

const P: u32 = 0x0100_1000;
const Q: u32 = 0x0100_2000;

fn p(word: u32) -> Option<Pointer> {
    Pointer::new(word)
}

/// What the lookup does to its output field.
#[derive(Clone, Copy)]
enum Write {
    Untouched,
    Null,
    Some(u32),
}

struct Lookup {
    status: bool,
    write: Write,
    calls: Vec<(u32, u32, Option<Pointer>)>,
}

impl Lookup {
    fn new(status: bool, write: Write) -> Self {
        Self {
            status,
            write,
            calls: Vec::new(),
        }
    }
}

impl Framework for Lookup {
    fn lookup(&mut self, service: u32, interface: u32, output: &mut Option<Pointer>) -> bool {
        self.calls.push((service, interface, *output));
        match self.write {
            Write::Untouched => {}
            Write::Null => *output = None,
            Write::Some(word) => *output = p(word),
        }
        self.status
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Event {
    AddRef(u32, Option<Pointer>),
    Release(u32, Option<Pointer>),
}

/// Records reference calls and, as the spec's synthetic counter fixture,
/// keeps a count per pointer (inputs, not a foreign algorithm).
#[derive(Default)]
struct Recorder {
    events: Vec<Event>,
    counts: BTreeMap<u32, i32>,
}

impl Recorder {
    fn count(&self, word: u32) -> i32 {
        self.counts.get(&word).copied().unwrap_or(0)
    }
}

impl References for Recorder {
    fn add_ref(&mut self, target: Pointer, field: Option<Pointer>) {
        self.events.push(Event::AddRef(target.word(), field));
        *self.counts.entry(target.word()).or_default() += 1;
    }
    fn release(&mut self, target: Pointer, field: Option<Pointer>) {
        self.events.push(Event::Release(target.word(), field));
        *self.counts.entry(target.word()).or_default() -= 1;
    }
}

fn snd_sys() -> Identity {
    stored_identity("cIGZSndSys").unwrap()
}

#[test]
fn s01_null_framework_stores_null_without_lookup() {
    let w = SysServPtr::construct(snd_sys(), None);
    assert_eq!(w.get(), None);
    let mut refs = Recorder::default();
    w.destroy(&mut refs);
    assert!(refs.events.is_empty());
}

#[test]
fn s02_to_s06_output_and_status_are_independent() {
    // (status, write, stored field, releases at destruction)
    let rows = [
        (false, Write::Null, None, false),
        (true, Write::Null, None, false),
        (false, Write::Some(P), p(P), true),
        (true, Write::Some(P), p(P), true),
        (false, Write::Untouched, None, false),
    ];
    for (status, write, stored, releases) in rows {
        let mut fw = Lookup::new(status, write);
        let w = SysServPtr::construct(snd_sys(), Some(&mut fw));
        // Exact identity, and the lookup observes the cleared field.
        assert_eq!(fw.calls, [(0x23CE_CBA0, 0x1060_77D2, None)]);
        assert_eq!(w.get(), stored);
        let mut refs = Recorder::default();
        w.destroy(&mut refs);
        let expected: Vec<Event> = if releases {
            vec![Event::Release(P, p(P))]
        } else {
            vec![]
        };
        assert_eq!(
            refs.events, expected,
            "no AddRef ever; Release observes the field still P"
        );
    }
}

#[test]
fn construction_overwrites_poisoned_storage_without_releasing_it() {
    let mut w = SysServPtr::from_field(p(0xDEAD_BEEF));
    let mut fw = Lookup::new(true, Write::Some(P));
    w.construct_in_place(snd_sys(), Some(&mut fw));
    assert_eq!(fw.calls[0].2, None, "poison is cleared before the lookup");
    assert_eq!(w.get(), p(P));
}

#[test]
fn s05_registry_and_short_forms_use_their_own_words() {
    let registry = stored_identity("cIGZRegistry").unwrap();
    assert_eq!((registry.service, registry.interface), (0x2507_6B9E, 0xA237_613C));
    for (label, service, interface) in [
        ("cIGZRegistry", 0x2507_6B9E, 0xA237_613C),
        ("cIGZWWWSystem", 0x449B_1939, 0x449B_1912),
        ("cIGZWinIMEServer", 0x2AC4_5447, 0x2AC4_4AE7),
    ] {
        for (status, write, stored) in [(true, Write::Null, None), (false, Write::Some(P), p(P))] {
            let mut fw = Lookup::new(status, write);
            let w = SysServPtr::construct(stored_identity(label).unwrap(), Some(&mut fw));
            assert_eq!(fw.calls, [(service, interface, None)]);
            assert_eq!(w.get(), stored);
        }
    }
}

#[test]
fn identity_table_matches_the_section_2_2_forms() {
    assert_eq!(STORED.iter().filter(|s| s.form == Form::C40).count(), 19);
    assert_eq!(STORED.iter().filter(|s| s.form == Form::C37).count(), 3);
    let mut seen = std::collections::BTreeSet::new();
    for s in STORED {
        assert!(seen.insert((s.identity.service, s.identity.interface)), "{}", s.label);
    }
}

#[test]
fn a01_a05_assignment_adds_then_releases_then_stores() {
    struct Row {
        initial: Option<Pointer>,
        new: Option<Pointer>,
        events: Vec<Event>,
    }
    let rows = [
        Row {
            initial: None,
            new: p(P),
            events: vec![Event::AddRef(P, None)],
        },
        Row {
            initial: p(P),
            new: p(Q),
            events: vec![Event::AddRef(Q, p(P)), Event::Release(P, p(P))],
        },
        Row {
            initial: p(P),
            new: p(P),
            events: vec![Event::AddRef(P, p(P)), Event::Release(P, p(P))],
        },
        Row {
            initial: p(P),
            new: None,
            events: vec![Event::Release(P, p(P))],
        },
        Row {
            initial: None,
            new: None,
            events: vec![],
        },
    ];
    for row in rows {
        let mut w = SysServPtr::from_field(row.initial);
        let mut refs = Recorder::default();
        let returned = w.assign(row.new, &mut refs).get();
        assert_eq!(returned, row.new, "returns the destination wrapper");
        assert_eq!(w.get(), row.new);
        assert_eq!(refs.events, row.events);
    }
}

#[test]
fn reference_counter_fixture_balances() {
    // S05: lookup grants +1, constructor +0, destructor -1.
    let mut refs = Recorder::default();
    refs.counts.insert(P, 1);
    let mut fw = Lookup::new(true, Write::Some(P));
    SysServPtr::construct(snd_sys(), Some(&mut fw)).destroy(&mut refs);
    assert_eq!(refs.count(P), 0);

    // A02: Q goes 1 -> 2 before P goes 1 -> 0; destroying W takes Q 2 -> 1.
    let mut refs = Recorder::default();
    refs.counts.insert(P, 1);
    refs.counts.insert(Q, 1);
    let mut w = SysServPtr::from_field(p(P));
    w.assign(p(Q), &mut refs);
    assert_eq!((refs.count(P), refs.count(Q)), (0, 2));
    w.destroy(&mut refs);
    assert_eq!(refs.count(Q), 1);

    // A03: 1 -> 2 -> 1, then destruction 1 -> 0.
    let mut refs = Recorder::default();
    refs.counts.insert(P, 1);
    let mut w = SysServPtr::from_field(p(P));
    w.assign(p(P), &mut refs);
    assert_eq!(refs.count(P), 1);
    w.destroy(&mut refs);
    assert_eq!(refs.count(P), 0);
}

#[test]
fn l01_release_keeps_the_field() {
    let mut w = SysServPtr::from_field(p(P));
    let mut refs = Recorder::default();
    w.release(&mut refs);
    assert_eq!(refs.events, [Event::Release(P, p(P))]);
    assert_eq!(w.get(), p(P), "no wrapper store");
}

#[test]
fn l02_l03_clear_and_release_clears_first() {
    let mut w = SysServPtr::from_field(p(P));
    let mut refs = Recorder::default();
    assert_eq!(*w.clear_and_release(&mut refs), None);
    assert_eq!(refs.events, [Event::Release(P, None)], "field is null during Release");

    let mut w = SysServPtr::default();
    let mut refs = Recorder::default();
    w.clear_and_release(&mut refs);
    assert!(refs.events.is_empty());
    assert_eq!(w.get(), None);
}

#[test]
fn l04_l07_temporary_lookups_release_before_returning() {
    for (identity, status, word) in [
        (TICK_TEMPORARY, true, P),
        (TICK_TEMPORARY, false, P),
        (AEMS_EVENTS_TEMPORARY, true, Q),
    ] {
        let mut fw = Lookup::new(status, Write::Some(word));
        let mut refs = Recorder::default();
        refs.counts.insert(word, 1);
        let result = lookup_temporary(identity, Some(&mut fw), &mut refs);
        assert_eq!(fw.calls, [(identity.service, identity.interface, None)]);
        assert_eq!(result, p(word));
        assert_eq!(refs.events, [Event::Release(word, p(word))], "no AddRef");
        assert_eq!(refs.count(word), 0, "net zero before return");
    }
    assert_eq!(
        (TICK_TEMPORARY.service, TICK_TEMPORARY.interface),
        (0x86AD_10EC, 0x86AD_10ED)
    );
    assert_eq!(
        (AEMS_EVENTS_TEMPORARY.service, AEMS_EVENTS_TEMPORARY.interface),
        (0xAC1F_AE31, 0x8C1F_AEB7)
    );
}

#[test]
fn l06_temporary_with_null_output_or_framework() {
    let mut fw = Lookup::new(true, Write::Null);
    let mut refs = Recorder::default();
    assert_eq!(lookup_temporary(TICK_TEMPORARY, Some(&mut fw), &mut refs), None);
    assert!(refs.events.is_empty());

    let mut refs = Recorder::default();
    assert_eq!(lookup_temporary(TICK_TEMPORARY, None, &mut refs), None);
    assert!(refs.events.is_empty());
}

#[test]
fn l08_l09_successive_lifetimes_do_not_cache() {
    let mut refs = Recorder::default();
    for word in [P, Q] {
        let mut fw = Lookup::new(true, Write::Some(word));
        SysServPtr::construct(snd_sys(), Some(&mut fw)).destroy(&mut refs);
        assert_eq!(fw.calls.len(), 1);
    }
    assert_eq!(refs.events, [Event::Release(P, p(P)), Event::Release(Q, p(Q))]);

    let mut refs = Recorder::default();
    for word in [P, Q] {
        let mut fw = Lookup::new(true, Write::Some(word));
        assert_eq!(lookup_temporary(TICK_TEMPORARY, Some(&mut fw), &mut refs), p(word));
        assert_eq!(fw.calls.len(), 1);
    }
    assert_eq!(refs.events, [Event::Release(P, p(P)), Event::Release(Q, p(Q))]);
}

#[test]
fn d01_d04_draw_cleanup_releases_without_clearing() {
    let draw = stored_identity("cIGZDrawSystem").unwrap();
    assert_eq!((draw.service, draw.interface), (0xEB2E_A3CB, 0x4B2E_A3B3));
    let rows = [
        (Some((false, Write::Some(P))), p(P)),
        (Some((true, Write::Some(Q))), p(Q)),
        (Some((true, Write::Null)), None),
        (Some((false, Write::Untouched)), None),
        (None, None),
    ];
    for (lookup, stored) in rows {
        let mut fw = lookup.map(|(status, write)| Lookup::new(status, write));
        let mut w = SysServPtr::from_field(p(0xDEAD_BEEF));
        w.construct_in_place(draw, fw.as_mut().map(|f| f as &mut dyn Framework));
        if let Some(fw) = &fw {
            assert_eq!(fw.calls, [(draw.service, draw.interface, None)]);
        }
        assert_eq!(w.get(), stored);
        let mut refs = Recorder::default();
        w.release(&mut refs);
        let expected: Vec<Event> = stored.map(|s| Event::Release(s.word(), Some(s))).into_iter().collect();
        assert_eq!(refs.events, expected);
        assert_eq!(w.get(), stored, "W keeps its word after cleanup");
    }
}

#[test]
fn r01_r03_prepared_field_receives_a_foreign_output() {
    // R01: release P with the field already null, then a foreign write of Q.
    let mut w = SysServPtr::from_field(p(P));
    let mut refs = Recorder::default();
    let output = w.clear_and_release(&mut refs);
    assert_eq!(*output, None, "foreign output sees null");
    *output = p(Q);
    assert_eq!(refs.events, [Event::Release(P, None)]);
    assert_eq!(w.get(), p(Q));

    // R02: null field, no reference call, foreign write of P.
    let mut w = SysServPtr::default();
    let mut refs = Recorder::default();
    *w.clear_and_release(&mut refs) = p(P);
    assert!(refs.events.is_empty());
    assert_eq!(w.get(), p(P));

    // R03: no output write leaves the field null.
    let mut w = SysServPtr::from_field(p(Q));
    let mut refs = Recorder::default();
    w.clear_and_release(&mut refs);
    assert_eq!(refs.events, [Event::Release(Q, None)]);
    assert_eq!(w.get(), None);
}

// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub(super) fn tokens(values: &[&str]) -> Vec<ScriptToken> {
    values
        .iter()
        .enumerate()
        .map(|(i, s)| ScriptToken::new(i as u32, s))
        .collect()
}
#[test]
fn sw1_through_sw8_reader_records_and_failure_prefix() {
    let mut s = vec![99];
    let mut t = tokens(&["cmd", "-scale", "2", "-scale", "3"]);
    for (at, value) in [(1, 2), (3, 4)] {
        assert!(find_switch(&mut t, b"scale", 1, &mut s, &mut Ascii).unwrap());
        assert_eq!(s, [value]);
        assert_eq!(t[at].bytes, b"+scale");
        assert_eq!(t[at].index, at as u32);
    }
    t = tokens(&["cmd", "-Scale", "2", "--scale", "3", "-scale=4"]);
    let before = t.clone();
    assert!(!find_switch(&mut t, b"scale", 1, &mut s, &mut Ascii).unwrap());
    assert!(s.is_empty());
    assert_eq!(t, before);
    t = tokens(&["cmd", "-k", "-2", "+3", "--", "-", "+", "", "-next", "9"]);
    assert!(find_switch(&mut t, b"k", 1, &mut s, &mut Ascii).unwrap());
    assert_eq!(s, [2, 3, 4, 5, 6, 7]);
    t = tokens(&["cmd", "-k", "+next"]);
    assert!(find_switch(&mut t, b"k", 1, &mut s, &mut Ascii).is_err());
    assert!(s.is_empty());
    assert_eq!(t[1].bytes, b"+k");
    t = tokens(&["cmd", "-k", ""]);
    assert!(find_switch(&mut t, b"k", 1, &mut s, &mut Ascii).unwrap());
    assert_eq!(s, [2]);
    t = tokens(&["cmd", "-k", "1", "2", "3"]);
    assert!(find_switch(&mut t, b"k", 0, &mut s, &mut Ascii).unwrap());
    assert_eq!(s, [2, 3, 4]);
    t = tokens(&["cmd", "first", "-unknown", "value", "later"]);
    find_positional(&t, 1, &mut s, &mut Ascii).unwrap();
    assert_eq!(s, [1]);
    assert!(check_switches(&t, &mut Ascii).is_err());
    t = tokens(&["cmd", "-k", "1"]);
    assert!(!find_switch(&mut t, b"missing", 4, &mut s, &mut Ascii).unwrap());
    assert!(s.is_empty());
    find_positional(&t, 1, &mut s, &mut Ascii).unwrap_err();
    assert!(s.is_empty());
}

#[derive(Default)]
struct Hooks {
    calls: Vec<String>,
    fail_end: bool,
}
impl DescriptorHooks for Hooks {
    fn register(&mut self, d: Descriptor) {
        self.calls.push(format!("register:{}", d.0));
    }
    fn destroy(&mut self, d: Descriptor) {
        self.calls.push(format!("destroy:{}", d.0));
    }
    fn command(&mut self, d: Descriptor, _: &mut [ScriptToken]) -> Result<()> {
        self.calls.push(format!("command:{}", d.0));
        Ok(())
    }
    fn end(&mut self, d: Descriptor) -> Result<()> {
        self.calls.push(format!("end:{}", d.0));
        if self.fail_end {
            Err(Error::Foreign("end".into()))
        } else {
            Ok(())
        }
    }
}
#[test]
fn dr1_dr2_dr3_duplicate_ownership_and_case() {
    let mut d = Dictionary::default();
    let mut h = Hooks::default();
    d.root(b"x", Descriptor(1), &mut h);
    d.root(b"x", Descriptor(2), &mut h);
    d.root(b"X", Descriptor(3), &mut h);
    for (name, expected) in [("x", "command:2"), ("X", "command:3")] {
        d.dispatch(&mut tokens(&[name]), &mut Ascii, &mut h).unwrap();
        assert_eq!(h.calls.last().unwrap(), expected);
    }
    d.child(Descriptor(4), b"x", Descriptor(5), &mut h);
    d.child(Descriptor(4), b"x", Descriptor(6), &mut h);
    d.start(Descriptor(4));
    d.dispatch(&mut tokens(&["x"]), &mut Ascii, &mut h).unwrap();
    assert_eq!(h.calls.last().unwrap(), "command:6");
    d.destroy_block(Descriptor(4), &mut h);
    d.destroy_root(&mut h);
    assert_eq!(
        &h.calls[h.calls.len() - 4..],
        ["destroy:6", "destroy:1", "destroy:2", "destroy:3"]
    );
    assert!(!h.calls.iter().any(|c| c == "destroy:5"));
}
#[test]
fn dr4_dr5_dr6_nested_precedence_throwing_end_and_switch_asymmetry() {
    let mut d = Dictionary::default();
    let mut h = Hooks::default();
    d.root(b"y", Descriptor(1), &mut h);
    d.child(Descriptor(2), b"x", Descriptor(3), &mut h);
    d.child(Descriptor(4), b"x", Descriptor(5), &mut h);
    d.start(Descriptor(2));
    d.start(Descriptor(4));
    d.dispatch(&mut tokens(&["x"]), &mut Ascii, &mut h).unwrap();
    assert_eq!(h.calls.last().unwrap(), "command:5");
    assert!(d.dispatch(&mut tokens(&["x", "-unknown"]), &mut Ascii, &mut h).is_err());
    assert_eq!(h.calls.last().unwrap(), "command:5");
    d.dispatch(&mut tokens(&["y", "-unknown"]), &mut Ascii, &mut h).unwrap();
    assert_eq!(h.calls.last().unwrap(), "command:1");
    assert!(d.dispatch(&mut tokens(&["missing"]), &mut Ascii, &mut h).is_err());
    h.fail_end = true;
    assert!(d
        .dispatch(&mut tokens(&["end", "surplus"]), &mut Ascii, &mut h)
        .is_err());
    assert_eq!(d.stack, [Descriptor(2), Descriptor(4)]);
    assert_eq!(h.calls.last().unwrap(), "end:4");
    h.fail_end = false;
    d.dispatch(&mut tokens(&["end", "surplus"]), &mut Ascii, &mut h)
        .unwrap();
    assert_eq!(d.stack, [Descriptor(2)]);
}

#[derive(Default)]
struct Refs(Vec<String>);
impl ObjectReferences for Refs {
    fn retain(&mut self, id: Identity) {
        self.0.push(format!("retain:{}", id.0));
    }
    fn release(&mut self, id: Identity) {
        self.0.push(format!("release:{}", id.0));
    }
}
#[test]
fn di1_through_di5_type_names_replacement_and_anonymous_identity() {
    let mut d = Directory::default();
    let mut refs = Refs::default();
    for id in 1..=2 {
        d.register(6, None, Identity(id), &mut Ascii, &mut refs);
    }
    assert_eq!(d.register(6, Some(b"Mixed"), Identity(3), &mut Ascii, &mut refs), 2);
    refs.0.clear();
    assert_eq!(d.register(6, Some(b"mIXED"), Identity(4), &mut Ascii, &mut refs), 2);
    assert_eq!(refs.0, ["retain:4", "release:3"]);
    refs.0.clear();
    assert!(d.lookup(6, b"MIXED", None, &mut Ascii, &mut refs));
    assert!(!d.lookup(7, b"MIXED", None, &mut Ascii, &mut refs));
    assert!(refs.0.is_empty());
    assert_eq!(d.register(6, Some(b"Mixed"), Identity(4), &mut Ascii, &mut refs), 2);
    assert!(refs.0.is_empty());
    assert_eq!(d.register(6, None, Identity(4), &mut Ascii, &mut refs), 3);
    assert_eq!(d.register(6, None, Identity(4), &mut Ascii, &mut refs), 4);
    let mut out = Some(Identity(9));
    refs.0.clear();
    assert!(!d.lookup(6, b"missing", Some(&mut out), &mut Ascii, &mut refs));
    assert_eq!(out, Some(Identity(9)));
    assert!(refs.0.is_empty());
    assert!(d.lookup(6, b"MIXED", Some(&mut out), &mut Ascii, &mut refs));
    assert_eq!(out, Some(Identity(4)));
    assert_eq!(refs.0, ["retain:4"]);
}
impl Destruction for Refs {
    fn dispose_names_and_temporaries(&mut self) {
        self.0.push("names".into());
    }
    fn release_component(&mut self, id: Identity) {
        self.0.push(format!("component:{}", id.0));
    }
    fn dispose_descriptions_and_id_maps(&mut self) {
        self.0.push("descriptions".into());
    }
    fn release_interface(&mut self, id: Identity) {
        self.0.push(format!("interface:{}", id.0));
    }
    fn base_destroy(&mut self) {
        self.0.push("base".into());
    }
    fn free_allocation(&mut self) {
        self.0.push("free".into());
    }
}
#[test]
fn lt1_lt2_lt3_final_release_order_and_aliased_held_slots() {
    let mut life = Lifetime {
        count: 3,
        component: Some(Identity(1)),
        interfaces: [
            Some(Identity(2)),
            Some(Identity(3)),
            Some(Identity(3)),
            Some(Identity(4)),
        ],
    };
    let mut refs = Refs::default();
    assert_eq!(life.release(&mut refs), 2);
    assert!(refs.0.is_empty());
    assert_eq!(life.release(&mut refs), 1);
    assert_eq!(life.release(&mut refs), 0);
    assert_eq!(life.count, 1);
    assert_eq!(
        refs.0,
        [
            "names",
            "component:1",
            "descriptions",
            "interface:2",
            "interface:3",
            "interface:3",
            "interface:4",
            "base",
            "free"
        ]
    );
}

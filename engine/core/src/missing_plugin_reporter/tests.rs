// SPDX-License-Identifier: GPL-3.0-or-later
// Literal public MPR-T01–15 expectations recorded before production code.
// No expected trace is computed by a reporter/verifier implementation.
const T01: &[&str] = &["selector"];
const T02: &[&str] = &[
    "selector",
    "dispatch R31",
    "set zero",
    "set init 100",
    "context",
    "manager",
    "set clear",
    "set buckets free",
];
const T03: &[&str] = &[
    "set zero",
    "set init 100",
    "context",
    "read C12",
    "read C16",
    "write F12 73",
    "write F16 A6",
    "manager",
    "set clear",
    "set buckets free",
];
const T04: &[&str] = &[
    "set zero",
    "set init 100",
    "context",
    "manager",
    "cache load",
    "alloc A17 12",
    "reset N7",
    "query M63 F51 max",
    "next N7",
    "next N7", // Cleanup independently reads the empty list's first link.
    "reset N7",
    "cache load",
    "free A17 N7",
    "set clear",
    "set buckets free",
];
const T05: &[&str] = &[
    "occupant N23",
    "holder O17",
    "lookup H117 6A871B82",
    "release O17",
    "next N23",
];
const T06: &[&str] = &[
    "occupant N61",
    "holder O92",
    "lookup H192 6A871B82",
    "variant P512",
    "convert V612",
    "release O92",
    "next N61",
];
const T07: &[&str] = &[
    "occupant N23",
    "holder O17",
    "lookup H117 6A871B82",
    "variant P256",
    "convert V356",
    "capacity 1",
    "insert A15E0037",
    "release O17",
    "next N23",
    "permission",
];
const T08: &[&str] = &[
    "capacity 1",
    "insert 0",
    "release O92",
    "next N61",
    "permission",
    "dialog alloc 8",
    "dialog init D83 vptr A8ECE0 ref 0",
    "addref D83",
    "dialog D83",
    "save AL 00",
    "release D83",
];
const T09: &[&str] = &[
    "capacity 1",
    "insert FFFFFFFF",
    "release O17",
    "next N23",
    "permission",
    "dialog alloc 8",
    "dialog init D83 vptr A8ECE0 ref 0",
    "addref D83",
    "dialog D83",
    "save AL 01",
    "release D83",
];
const T10: &[&str] = &[
    "occupant N61",
    "holder O92",
    "lookup H192 6A871B82",
    "variant P512",
    "convert V612",
    "capacity 1",
    "insert A15E0037",
    "release O92",
    "next N61",
    "occupant N23",
    "holder O17",
    "lookup H117 6A871B82",
    "variant P256",
    "convert V356",
    "capacity 2",
    "insert D209",
    "release O17",
    "next N23",
    "permission",
    "dialog alloc 8",
    "dialog init D83 vptr A8ECE0 ref 0",
    "addref D83",
    "dialog D83",
    "save AL 80",
    "release D83",
];
const T11: &[&str] = &[
    "capacity 1",
    "insert A15E0037",
    "release O17",
    "next N23",
    "occupant N61",
    "holder O92",
    "lookup H192 6A871B82",
    "release O92",
    "next N61",
    "occupant N89",
    "holder O44",
    "lookup H144 6A871B82",
    "variant P768",
    "convert V868",
    "capacity 2",
    "insert A15E0037",
    "release O44",
    "next N89",
    "permission",
];
const T12: &[&str] = &[
    "save AL 00",
    "release D83",
    "next N7",
    "next N61",
    "cache load",
    "free A17 N61",
    "reset N7",
    "cache load",
    "free A17 N7",
    "set node free D209",
    "set clear",
    "set buckets free",
    "selector",
    "dispatch R31",
    "set zero",
    "set init 100",
];
const T13: &[&str] = &[
    "cache load",
    "framework",
    "lookup W39 3AE4BEA3 3AE4BEAB",
    "cache store A17",
    "release A17",
    "cache load",
    "alloc A17 12",
];
const T14_NO_FRAMEWORK: &[&str] = &[
    "set zero",
    "set init 100",
    "context",
    "manager",
    "cache load",
    "framework",
    "cache store null",
    "cache load",
];
const T14_NULL_OUTPUT: &[&str] = &[
    "set zero",
    "set init 100",
    "context",
    "manager",
    "cache load",
    "framework",
    "lookup W39 3AE4BEA3 3AE4BEAB",
    "cache store null",
    "cache load",
];
const T15: &[&str] = &[
    "capacity 1",
    "insert D209",
    "release O92",
    "next N61",
    "permission",
    "dialog alloc 8",
];

use super::*;
use std::collections::BTreeMap;

const HEAD: &[&str] = &[
    "set zero",
    "set init 100",
    "context",
    "manager",
    "cache load",
    "alloc A17 12",
    "reset N7",
    "query M63 F51 max",
    "next N7",
];
const P17: &[&str] = &[
    "occupant N23",
    "holder O17",
    "lookup H117 6A871B82",
    "variant P256",
    "convert V356",
];
const P92: &[&str] = &[
    "occupant N61",
    "holder O92",
    "lookup H192 6A871B82",
    "variant P512",
    "convert V612",
];
const C17: &[&str] = &[
    "next N7",
    "next N23",
    "cache load",
    "free A17 N23",
    "reset N7",
    "cache load",
    "free A17 N7",
];
const C92: &[&str] = &[
    "next N7",
    "next N61",
    "cache load",
    "free A17 N61",
    "reset N7",
    "cache load",
    "free A17 N7",
];
const EMPTY_SET: &[&str] = &["set clear", "set buckets free"];

#[derive(Clone)]
struct Item {
    node: Id,
    occupant: Id,
    holder: Id,
    property: Option<Id>,
    variant: Option<Id>,
    bits: u32,
    output: Option<u32>,
}
impl Item {
    fn o17(output: u32) -> Self {
        Self {
            node: Id(23),
            occupant: Id(17),
            holder: Id(117),
            property: Some(Id(256)),
            variant: Some(Id(356)),
            bits: 1,
            output: Some(output),
        }
    }
    fn o92(output: u32) -> Self {
        Self {
            node: Id(61),
            occupant: Id(92),
            holder: Id(192),
            property: Some(Id(512)),
            variant: Some(Id(612)),
            bits: 1,
            output: Some(output),
        }
    }
    fn o44(output: u32) -> Self {
        Self {
            node: Id(89),
            occupant: Id(44),
            holder: Id(144),
            property: Some(Id(768)),
            variant: Some(Id(868)),
            bits: 1,
            output: Some(output),
        }
    }
}

struct Fixture {
    trace: Vec<String>,
    selector: u32,
    manager: bool,
    context: Option<(u32, u32)>,
    filter: [u32; 2],
    cache: Option<Id>,
    framework: bool,
    service_output: Option<Id>,
    service_bits: u32,
    query_bits: u32,
    items: Vec<Item>,
    permission_bits: u32,
    dialog_bits: u32,
    dialog: Option<Id>,
    dialog_members: Vec<u32>,
    owned: BTreeMap<u32, i32>,
    live_nodes: Vec<Id>,
    last_next: Option<Id>,
    clear_cache_on_next: Option<Id>,
    null_sentinel: bool,
    invalid_holder: bool,
}
impl Fixture {
    fn new(items: Vec<Item>) -> Self {
        Self {
            trace: Vec::new(),
            selector: POST_CITY_INIT,
            manager: true,
            context: None,
            filter: [0x19, 0xD2],
            cache: Some(Id(217)),
            framework: true,
            service_output: Some(Id(217)),
            service_bits: 1,
            query_bits: 1,
            items,
            permission_bits: 1,
            dialog_bits: 1,
            dialog: Some(Id(83)),
            dialog_members: Vec::new(),
            owned: BTreeMap::new(),
            live_nodes: Vec::new(),
            last_next: None,
            clear_cache_on_next: None,
            null_sentinel: false,
            invalid_holder: false,
        }
    }
    fn record(&mut self, value: impl Into<String>) {
        self.trace.push(value.into());
    }
    fn assert_trace(&self, pieces: &[&[&str]]) {
        let expected: Vec<&str> = pieces.iter().flat_map(|p| p.iter().copied()).collect();
        assert_eq!(self.trace, expected);
    }
    fn balanced(&self) {
        assert!(self.owned.values().all(|&refs| refs == 0), "{:?}", self.owned);
        assert!(self.live_nodes.is_empty());
    }
}
fn reporter() -> Reporter {
    Reporter {
        owner: Id(31),
        filter: Id(51),
    }
}

impl PrimitiveHost for Fixture {
    fn observe(&mut self, effect: Effect) {
        self.record(match effect {
            Effect::Dispatch(Id(owner)) => format!("dispatch R{owner}"),
            Effect::SetZero => "set zero".into(),
            Effect::SetInit(hint) => format!("set init {hint}"),
            Effect::Capacity(count) => format!("capacity {count}"),
            Effect::Insert(value) => format!("insert {value:X}"),
            Effect::SaveAl(value) => format!("save AL {value:02X}"),
            Effect::SetNodeFree(value) => format!("set node free {value:X}"),
            Effect::SetClear => "set clear".into(),
            Effect::SetBucketsFree => "set buckets free".into(),
        });
    }
    fn selector(&mut self, message: Id) -> Result<u32> {
        assert_eq!(message, Id(97));
        self.record("selector");
        Ok(self.selector)
    }
    fn context(&mut self) -> Result<Option<Id>> {
        self.record("context");
        Ok(self.context.map(|_| Id(41)))
    }
    fn context_word(&mut self, context: Id, offset: u32) -> Result<u32> {
        assert_eq!(context, Id(41));
        self.record(format!("read C{offset}"));
        let (a, b) = self.context.unwrap();
        match offset {
            12 => Ok(a),
            16 => Ok(b),
            _ => panic!("wrong context word"),
        }
    }
    fn filter_word(&mut self, filter: Id, offset: u32, value: u32) -> Result<()> {
        assert_eq!(filter, Id(51));
        self.record(format!("write F{offset} {value:X}"));
        self.filter[match offset {
            12 => 0,
            16 => 1,
            _ => panic!("wrong filter word"),
        }] = value;
        Ok(())
    }
    fn manager(&mut self) -> Result<Option<Id>> {
        self.record("manager");
        Ok(self.manager.then_some(Id(63)))
    }
    fn cache_load(&mut self) -> Result<Option<Id>> {
        self.record("cache load");
        Ok(self.cache)
    }
    fn cache_store(&mut self, value: Option<Id>) -> Result<()> {
        self.record(match value {
            Some(id) => {
                assert_eq!(id, Id(217));
                "cache store A17"
            }
            None => "cache store null",
        });
        self.cache = value;
        Ok(())
    }
    fn framework(&mut self) -> Result<Option<Id>> {
        self.record("framework");
        Ok(self.framework.then_some(Id(39)))
    }
    fn service_lookup(&mut self, framework: Id, service: u32, iid: u32, output: &mut Option<Id>) -> Result<u32> {
        assert_eq!((framework, service, iid), (Id(39), 0x3AE4_BEA3, 0x3AE4_BEAB));
        assert_eq!(*output, None);
        self.record("lookup W39 3AE4BEA3 3AE4BEAB");
        *output = self.service_output;
        if let Some(id) = self.service_output {
            *self.owned.entry(id.0 + 1000).or_default() += 1;
        }
        Ok(self.service_bits)
    }
    fn allocate(&mut self, allocator: Id, bytes: u32) -> Result<Option<Id>> {
        assert_eq!((allocator, bytes), (Id(217), 12));
        self.record("alloc A17 12");
        if self.null_sentinel {
            return Ok(None);
        }
        self.live_nodes.push(Id(7));
        Ok(Some(Id(7)))
    }
    fn reset_sentinel(&mut self, sentinel: Id) -> Result<()> {
        assert_eq!(sentinel, Id(7));
        assert!(self.live_nodes.contains(&sentinel));
        self.record("reset N7");
        Ok(())
    }
    fn query(
        &mut self,
        manager: Id,
        head: &mut Id,
        min: Option<Id>,
        max: Option<Id>,
        filter: Id,
        max_count: u32,
    ) -> Result<u32> {
        assert_eq!(
            (manager, *head, min, max, filter, max_count),
            (Id(63), Id(7), None, None, Id(51), u32::MAX)
        );
        self.record("query M63 F51 max");
        // One foreign output operation supplies node links and owned references.
        for item in &self.items {
            self.live_nodes.push(item.node);
            *self.owned.entry(item.occupant.0).or_default() += 1;
        }
        Ok(self.query_bits)
    }
    fn next(&mut self, node: Id) -> Result<Id> {
        assert!(self.live_nodes.contains(&node));
        self.record(format!("next N{}", node.0));
        self.last_next = Some(node);
        if self.clear_cache_on_next == Some(node) {
            self.cache = None;
            self.clear_cache_on_next = None;
        }
        if node == Id(7) {
            return Ok(self.items.first().map_or(Id(7), |item| item.node));
        }
        let i = self.items.iter().position(|item| item.node == node).unwrap();
        assert_eq!(self.owned[&self.items[i].occupant.0], 0, "Release must precede next");
        Ok(self.items.get(i + 1).map_or(Id(7), |item| item.node))
    }
    fn occupant(&mut self, node: Id) -> Result<Id> {
        self.record(format!("occupant N{}", node.0));
        Ok(self.items.iter().find(|item| item.node == node).unwrap().occupant)
    }
    fn holder(&mut self, occupant: Id) -> Result<Option<Id>> {
        self.record(format!("holder O{}", occupant.0));
        if self.invalid_holder {
            return Ok(None);
        }
        Ok(Some(
            self.items.iter().find(|item| item.occupant == occupant).unwrap().holder,
        ))
    }
    fn property(&mut self, holder: Id, property: u32) -> Result<Option<Id>> {
        assert_eq!(property, 0x6A87_1B82);
        self.record(format!("lookup H{} {property:X}", holder.0));
        Ok(self.items.iter().find(|item| item.holder == holder).unwrap().property)
    }
    fn variant(&mut self, property: Id) -> Result<Option<Id>> {
        self.record(format!("variant P{}", property.0));
        Ok(self
            .items
            .iter()
            .find(|item| item.property == Some(property))
            .unwrap()
            .variant)
    }
    fn convert(&mut self, variant: Id, output: &mut Option<u32>) -> Result<u32> {
        // No initialization imposed by the native caller; portable marker only.
        assert_eq!(*output, None);
        self.record(format!("convert V{}", variant.0));
        let item = self.items.iter().find(|item| item.variant == Some(variant)).unwrap();
        *output = item.output;
        Ok(item.bits)
    }
    fn release(&mut self, reference: Id) -> Result<()> {
        let (name, key) = if reference == Id(83) {
            ("D", 83)
        } else if self.cache == Some(reference) && self.owned.get(&(reference.0 + 1000)).copied().unwrap_or(0) > 0 {
            ("A", reference.0 + 1000)
        } else {
            ("O", reference.0)
        };
        let refs = self
            .owned
            .get_mut(&key)
            .expect("Release must consume a supplied reference");
        assert!(*refs > 0);
        *refs -= 1;
        let label = if name == "A" { 17 } else { reference.0 };
        self.record(format!("release {name}{label}"));
        Ok(())
    }
    fn permission(&mut self) -> Result<u32> {
        assert!(self.items.iter().all(|item| self.owned[&item.occupant.0] == 0));
        self.record("permission");
        Ok(self.permission_bits)
    }
    fn dialog_allocate(&mut self, bytes: u32) -> Result<Option<Id>> {
        assert_eq!(bytes, 8);
        self.record("dialog alloc 8");
        Ok(self.dialog)
    }
    fn dialog_initialize(&mut self, dialog: Id, vptr: u32, refs: u32) -> Result<()> {
        assert_eq!((dialog, vptr, refs), (Id(83), 0x00A8_ECE0, 0));
        self.owned.insert(83, 0);
        self.record("dialog init D83 vptr A8ECE0 ref 0");
        Ok(())
    }
    fn add_ref(&mut self, reference: Id) -> Result<u32> {
        assert_eq!(reference, Id(83));
        *self.owned.get_mut(&83).unwrap() += 1;
        self.record("addref D83");
        Ok(0xF000_0000)
    }
    fn do_dialog(&mut self, dialog: Id, packs: &PackSet) -> Result<u32> {
        assert_eq!(dialog, Id(83));
        assert_eq!(packs.len(), self.dialog_members.len() as u32);
        for &member in &self.dialog_members {
            assert!(packs.contains(member));
        }
        assert_eq!(packs.is_empty(), self.dialog_members.is_empty());
        assert_eq!(self.owned[&83], 1);
        assert_eq!(self.live_nodes.len(), self.items.len() + 1, "list survives dialog");
        self.record("dialog D83");
        Ok(self.dialog_bits)
    }
    fn free(&mut self, allocator: Id, node: Id) -> Result<()> {
        assert_eq!(allocator, Id(217));
        if node != Id(7) {
            assert_eq!(self.last_next, Some(node), "read next before free");
        }
        let position = self.live_nodes.iter().position(|&id| id == node).unwrap();
        self.live_nodes.remove(position);
        self.record(format!("free A17 N{}", node.0));
        Ok(())
    }
}

#[test]
fn mpr_t01_unrelated_selector_full_word() {
    let mut h = Fixture::new(vec![]);
    h.selector = 0xEA8A_E29B;
    assert_eq!(reporter().listener().do_message(&mut h, Id(97)), Ok(1));
    h.assert_trace(&[T01]);
    // Same low byte is insufficient; the whole selector must match.
    h.trace.clear();
    h.selector = 0x1234_569A;
    assert_eq!(reporter().listener().do_message(&mut h, Id(97)), Ok(1));
    h.assert_trace(&[T01]);
}
#[test]
fn mpr_t02_matching_null_manager_message_al_one() {
    let mut h = Fixture::new(vec![]);
    h.manager = false;
    assert_eq!(reporter().post_init(&mut h), Ok(0));
    h.trace.clear();
    assert_eq!(reporter().listener().do_message(&mut h, Id(97)), Ok(1));
    h.assert_trace(&[T02]);
    h.balanced();
}
#[test]
fn mpr_t03_both_source_words_before_stores_before_false() {
    let mut h = Fixture::new(vec![]);
    h.manager = false;
    h.context = Some((0x73, 0xA6));
    assert_eq!(reporter().post_init(&mut h), Ok(0));
    assert_eq!(h.filter, [0x73, 0xA6]);
    h.assert_trace(&[T03]);
    h.balanced();
}
#[test]
fn mpr_t04_false_empty_list_is_normal_completion() {
    let mut h = Fixture::new(vec![]);
    h.query_bits = 0;
    assert_eq!(reporter().post_init(&mut h), Ok(1));
    assert_eq!(h.filter, [0x19, 0xD2]);
    h.assert_trace(&[T04]);
    h.balanced();
}
#[test]
fn mpr_t05_null_full_property_pointer_no_output_use() {
    let mut item = Item::o17(0xB16B_00B5);
    item.property = None;
    let mut h = Fixture::new(vec![item]);
    assert_eq!(reporter().post_init(&mut h), Ok(1));
    h.assert_trace(&[HEAD, T05, C17, EMPTY_SET]);
    h.balanced();
}
#[test]
fn mpr_t06_conversion_false_written_scalar_ignored() {
    let mut item = Item::o92(0xA15E_0037);
    item.bits = 0x1234_5600;
    let mut h = Fixture::new(vec![item]);
    assert_eq!(reporter().post_init(&mut h), Ok(1));
    h.assert_trace(&[HEAD, T06, C92, EMPTY_SET]);
    h.balanced();
}
#[test]
fn mpr_t07_permission_full_eax_nonzero_al_zero() {
    let mut h = Fixture::new(vec![Item::o17(0xA15E_0037)]);
    h.permission_bits = 0x7F00_0100;
    assert_eq!(reporter().post_init(&mut h), Ok(1));
    h.assert_trace(&[HEAD, T07, C17, &["set node free A15E0037"], EMPTY_SET]);
    h.balanced();
}
#[test]
fn mpr_t08_zero_id_dialog_zero_not_latched() {
    let mut h = Fixture::new(vec![Item::o92(0)]);
    h.dialog_bits = 0x7F00_0100;
    h.dialog_members = vec![0];
    assert_eq!(reporter().post_init(&mut h), Ok(0));
    h.assert_trace(&[HEAD, P92, T08, C92, &["set node free 0"], EMPTY_SET]);
    h.balanced();
}
#[test]
fn mpr_t09_maximum_id_full_property_pointer_low_byte_zero() {
    let mut item = Item::o17(u32::MAX);
    item.bits = 0x7F00_0080;
    let mut h = Fixture::new(vec![item]);
    h.dialog_members = vec![u32::MAX];
    assert_eq!(reporter().post_init(&mut h), Ok(1));
    h.assert_trace(&[HEAD, P17, T09, C17, &["set node free FFFFFFFF"], EMPTY_SET]);
    h.balanced();
}
#[test]
fn mpr_t10_false_partial_list_and_raw_dialog_al_80() {
    let mut h = Fixture::new(vec![Item::o92(0xA15E_0037), Item::o17(0xD209)]);
    h.query_bits = 0;
    h.dialog_bits = 0x57AB_CD80;
    h.dialog_members = vec![0xD209, 0xA15E_0037];
    assert_eq!(reporter().post_init(&mut h), Ok(0x80));
    h.assert_trace(&[
        HEAD,
        T10,
        &[
            "next N7",
            "next N61",
            "cache load",
            "free A17 N61",
            "next N23",
            "cache load",
            "free A17 N23",
            "reset N7",
            "cache load",
            "free A17 N7",
            "set node free A15E0037",
            "set node free D209",
        ],
        EMPTY_SET,
    ]);
    h.balanced();
}
#[test]
fn mpr_t11_dedup_count_and_release_every_occupant() {
    let mut missing = Item::o92(0xB16B_00B5);
    missing.property = None;
    let mut h = Fixture::new(vec![Item::o17(0xA15E_0037), missing, Item::o44(0xA15E_0037)]);
    h.dialog_members = vec![0xA15E_0037];
    assert_eq!(reporter().post_init(&mut h), Ok(1));
    h.assert_trace(&[
        HEAD,
        P17,
        T11,
        &[
            "dialog alloc 8",
            "dialog init D83 vptr A8ECE0 ref 0",
            "addref D83",
            "dialog D83",
            "save AL 01",
            "release D83",
            "next N7",
            "next N23",
            "cache load",
            "free A17 N23",
            "next N61",
            "cache load",
            "free A17 N61",
            "next N89",
            "cache load",
            "free A17 N89",
            "reset N7",
            "cache load",
            "free A17 N7",
            "set node free A15E0037",
        ],
        EMPTY_SET,
    ]);
    h.balanced();
}
#[test]
fn mpr_t12_repeated_matching_messages_fresh_collection() {
    let r = reporter();
    let listener = r.listener();
    let mut h = Fixture::new(vec![Item::o92(0xD209)]);
    h.dialog_members = vec![0xD209];
    h.dialog_bits = 0;
    assert_eq!(listener.do_message(&mut h, Id(97)), Ok(1));
    h.balanced();
    h.dialog_bits = 1;
    assert_eq!(listener.do_message(&mut h, Id(97)), Ok(1));
    h.balanced();
    // Complete literal primitive traces for both invocations, in addition to
    // the independently recorded boundary witness T12 and exact call counts.
    h.assert_trace(&[
        &["selector", "dispatch R31"],
        HEAD,
        P92,
        &[
            "capacity 1",
            "insert D209",
            "release O92",
            "next N61",
            "permission",
            "dialog alloc 8",
            "dialog init D83 vptr A8ECE0 ref 0",
            "addref D83",
            "dialog D83",
            "save AL 00",
            "release D83",
        ],
        C92,
        &["set node free D209"],
        EMPTY_SET,
        &["selector", "dispatch R31"],
        HEAD,
        P92,
        &[
            "capacity 1",
            "insert D209",
            "release O92",
            "next N61",
            "permission",
            "dialog alloc 8",
            "dialog init D83 vptr A8ECE0 ref 0",
            "addref D83",
            "dialog D83",
            "save AL 01",
            "release D83",
        ],
        C92,
        &["set node free D209"],
        EMPTY_SET,
    ]);
    assert!(h
        .trace
        .windows(T12.len())
        .any(|window| window.iter().map(String::as_str).eq(T12.iter().copied())));
    let saves: Vec<&str> = h
        .trace
        .iter()
        .map(String::as_str)
        .filter(|s| s.starts_with("save AL"))
        .collect();
    assert_eq!(saves, ["save AL 00", "save AL 01"]);
    for event in [
        "selector",
        "dispatch R31",
        "set init 100",
        "query M63 F51 max",
        "permission",
        "dialog D83",
        "release D83",
        "free A17 N7",
        "set node free D209",
        "set buckets free",
    ] {
        assert_eq!(h.trace.iter().filter(|s| s.as_str() == event).count(), 2, "{event}");
    }
}
#[test]
fn mpr_t13_service_false_with_live_out_cache_release_reload() {
    let mut h = Fixture::new(vec![]);
    h.cache = None;
    h.service_bits = 0;
    assert_eq!(reporter().post_init(&mut h), Ok(1));
    h.assert_trace(&[
        &["set zero", "set init 100", "context", "manager"],
        T13,
        &[
            "reset N7",
            "query M63 F51 max",
            "next N7",
            "next N7", // Separate list-cleanup entry read, even when empty.
            "reset N7",
            "cache load",
            "free A17 N7",
        ],
        EMPTY_SET,
    ]);
    h.balanced();
}
#[test]
fn mpr_t14_missing_allocator_unsupported_prefix_native_return_unknown() {
    let mut h = Fixture::new(vec![]);
    h.cache = None;
    h.framework = false;
    assert_eq!(reporter().post_init(&mut h), Err(Unsupported::Allocator));
    h.assert_trace(&[T14_NO_FRAMEWORK]);
    h.trace.clear();
    h.framework = true;
    h.service_output = None;
    for bits in [0, 1] {
        h.service_bits = bits;
        h.trace.clear();
        assert_eq!(reporter().post_init(&mut h), Err(Unsupported::Allocator));
        h.assert_trace(&[T14_NULL_OUTPUT]);
    }
}
#[test]
fn mpr_t15_null_dialog_unsupported_prefix_native_call_return_unknown() {
    let mut h = Fixture::new(vec![Item::o92(0xD209)]);
    h.dialog = None;
    assert_eq!(reporter().post_init(&mut h), Err(Unsupported::NullDialog));
    h.assert_trace(&[HEAD, P92, T15]);
    assert_eq!(h.live_nodes, [Id(7), Id(61)], "no invented rollback/cleanup");
    assert_eq!(h.owned[&92], 0);
}

#[test]
fn cleanup_reacquires_weak_cache_at_node_and_sentinel_sites() {
    let mut h = Fixture::new(vec![Item::o17(0xA15E_0037)]);
    h.permission_bits = 0;
    h.service_bits = 0;
    h.clear_cache_on_next = Some(Id(23));
    assert_eq!(reporter().post_init(&mut h), Ok(1));
    assert!(h.trace.windows(7).any(|w| w.iter().map(String::as_str).eq([
        "cache load",
        "framework",
        "lookup W39 3AE4BEA3 3AE4BEAB",
        "cache store A17",
        "release A17",
        "cache load",
        "free A17 N23"
    ])));
    h.balanced();
    // Empty list: next(sentinel) invalidates only the weak cache, with a new
    // external live service output. Sentinel free must independently reacquire.
    let mut h = Fixture::new(vec![]);
    h.clear_cache_on_next = Some(Id(7));
    assert_eq!(reporter().post_init(&mut h), Ok(1));
    assert!(h.trace.windows(7).any(|w| w.iter().map(String::as_str).eq([
        "cache load",
        "framework",
        "lookup W39 3AE4BEA3 3AE4BEAB",
        "cache store A17",
        "release A17",
        "cache load",
        "free A17 N7"
    ])));
    h.balanced();
}
#[test]
fn unsupported_views_and_unwritten_success_are_not_ordinary_false() {
    let mut h = Fixture::new(vec![Item::o17(3)]);
    h.invalid_holder = true;
    assert_eq!(reporter().post_init(&mut h), Err(Unsupported::InvalidView));
    assert_eq!(h.trace.last().unwrap(), "holder O17");
    assert_eq!(h.owned[&17], 1); // no inferred unwind Release
    let mut item = Item::o17(3);
    item.variant = None;
    let mut h = Fixture::new(vec![item]);
    assert_eq!(reporter().post_init(&mut h), Err(Unsupported::InvalidView));
    assert_eq!(h.trace.last().unwrap(), "variant P256");
    let mut item = Item::o17(3);
    item.output = None;
    let mut h = Fixture::new(vec![item]);
    assert_eq!(reporter().post_init(&mut h), Err(Unsupported::SuccessWithoutOutput));
    assert_eq!(h.trace.last().unwrap(), "convert V356");
    let mut h = Fixture::new(vec![]);
    h.null_sentinel = true;
    assert_eq!(reporter().post_init(&mut h), Err(Unsupported::Allocation));
    assert_eq!(h.trace.last().unwrap(), "alloc A17 12");
}

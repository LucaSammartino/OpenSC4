// SPDX-License-Identifier: GPL-3.0-or-later
// Literal conditional fixtures transcribed from html-ui-hooks §7 before the
// production implementation. These are raw primitive fixtures, not an oracle
// implementing the hook, database, URL codec, or native lifecycle.
use super::*;

#[derive(Default)]
struct Hex(Vec<(Vec<u8>, u32)>);
impl HexPrimitive for Hex {
    fn convert(&mut self, input: &[u8]) -> Result<u32, Unsupported> {
        let (expected, result) = self.0.remove(0);
        assert_eq!(input, expected);
        Ok(result)
    }
}

#[test]
fn section_7_1_fourteen_groups() {
    for scheme in ["sc4", "SC4", "sC4"] {
        assert_eq!(test_url(scheme.as_bytes()), Ok(true));
    }
    for scheme in ["", "sc", "sc4x", "http", "file"] {
        assert_eq!(test_url(scheme.as_bytes()), Ok(false));
    }
    for url in ["", "sc4:/html/12345678", "SC4://html/12345678"] {
        assert_eq!(select_key(url.as_bytes(), &mut Hex::default()), Ok(None));
    }
    assert_eq!(test_url(b"SC4"), Ok(true));
    assert_eq!(select_key(b"SC4://html/12345678", &mut Hex::default()), Ok(None));
    for url in ["sc4://", "sc4:////"] {
        assert_eq!(select_key(url.as_bytes(), &mut Hex::default()), Ok(None));
    }
    // Rows 6–14: the expected keys and converter inputs are independent literals.
    let rows: &[(&str, u32, u32, &[(&str, u32)])] = &[
        ("sc4://HTML/12345678", 0x0A133FC0, 0x12345678, &[("12345678", 0x12345678)]),
        ("sc4://image/87654321", 0xCA133ECB, 0x87654321, &[("87654321", 0x87654321)]),
        ("sc4://other/12345678", 0, 0x12345678, &[("12345678", 0x12345678)]),
        ("sc4://html/1234567", 0x0A133FC0, 0, &[]),
        ("sc4://html", 0x0A133FC0, 0, &[]),
        ("sc4://anything/11223344/55667788", 0x11223344, 0x55667788,
            &[("11223344", 0x11223344), ("55667788", 0x55667788)]),
        ("sc4://image/11223344/55", 0x11223344, 0, &[("11223344", 0x11223344)]),
        // Conditional raw conversion outcomes, not claims about malformed hex.
        ("sc4://html/12345678?x", 0x0A133FC0, 0x76543210, &[("12345678?x", 0x76543210)]),
        ("sc4://html/../12345678", 0x11221122, 0x12345678,
            &[("..", 0x11221122), ("12345678", 0x12345678)]),
    ];
    for &(url, group, instance, calls) in rows {
        let mut hex = Hex(calls.iter().map(|(s, v)| (s.as_bytes().to_vec(), *v)).collect());
        assert_eq!(select_key(url.as_bytes(), &mut hex),
            Ok(Some(ResourceKey { resource_type: 0x856DDBAC, group, instance })), "{url}");
        assert!(hex.0.is_empty());
    }
}

#[derive(Debug, PartialEq)]
enum LinkEvent { Action(Vec<u8>, Option<Vec<u8>>, Vec<u8>), Destroy, Advisor(Vec<u8>) }
#[derive(Default)]
struct Links { events: Vec<LinkEvent>, eax: u32 }
impl LinkPrimitives for Links {
    fn action_temporary(&mut self, erased: &[u8], token: Option<&[u8]>, remaining: &[u8]) {
        self.events.push(LinkEvent::Action(erased.to_vec(), token.map(<[u8]>::to_vec), remaining.to_vec()));
    }
    fn destroy_action_temporaries(&mut self) { self.events.push(LinkEvent::Destroy); }
    fn advisor_do_link(&mut self, payload: &[u8]) -> Register {
        self.events.push(LinkEvent::Advisor(payload.to_vec()));
        Register(self.eax)
    }
}

#[test]
fn section_7_2_nine_groups_and_full_al() {
    for input in ["", "action", "sc4://advice", "sc4://ADVICE/x", "adviceadvice/payload", "sc4://adviceXpayload", "http://example"] {
        let mut f = Links::default();
        assert_eq!(do_url(input.as_bytes(), &mut f), Ok(0));
        assert!(f.events.is_empty());
    }
    for (input, expected) in [
        ("sc4://action/run?arg", vec![LinkEvent::Action(b"run?arg".to_vec(), Some(b"run".to_vec()), b"arg".to_vec()), LinkEvent::Destroy]),
        ("sc4://action/", vec![LinkEvent::Action(vec![], None, vec![]), LinkEvent::Destroy]),
    ] {
        let mut f = Links::default();
        assert_eq!(do_url(input.as_bytes(), &mut f), Ok(0));
        assert_eq!(f.events, expected);
    }
    for (input, payload) in [("sc4://advice/1234?x", "1234?x"), ("sc4://advice/", ""), ("xxxxxxadvice/payload", "payload")] {
        for (eax, al) in [(0xFEED0000, 0), (0xFEED0001, 1), (0xABCD0080, 0x80)] {
            let mut f = Links { eax, ..Links::default() };
            assert_eq!(do_url(input.as_bytes(), &mut f), Ok(al));
            assert_eq!(f.events, vec![LinkEvent::Advisor(payload.as_bytes().to_vec())]);
        }
    }
}

const KEY: ResourceKey = ResourceKey { resource_type: 0x856DDBAC, group: 0x0A133FC0, instance: 0x12345678 };
#[derive(Debug, PartialEq)]
enum ReadEvent { Acquire(Service, ResourceKey), Size(Segment, u32), Read(Segment, u32, Vec<u8>), Release(Segment) }
struct Reader {
    acquired: Register, segment: Option<Segment>, first_eax: u32,
    first_size: u32, second_eax: u32, second_size: u32, bytes: Vec<u8>, events: Vec<ReadEvent>,
}
impl Default for Reader {
    fn default() -> Self {
        Self { acquired: Register(0x11110001), segment: Some(Segment(8)), first_eax: 3,
            first_size: 0, second_eax: 0xAAAA0000, second_size: 3, bytes: vec![], events: vec![] }
    }
}
impl ResourcePrimitives for Reader {
    fn acquire(&mut self, service: Service, key: ResourceKey) -> Acquisition {
        self.events.push(ReadEvent::Acquire(service, key));
        Acquisition { result: self.acquired, segment: self.segment }
    }
    fn read_size(&mut self, segment: Segment, key: ResourceKey, size: &mut u32) -> u32 {
        assert_eq!(key, KEY);
        self.events.push(ReadEvent::Size(segment, *size));
        *size = self.first_size;
        self.first_eax
    }
    fn read_bytes(&mut self, segment: Segment, key: ResourceKey, output: &mut [u8], size: &mut u32) -> u32 {
        assert_eq!(key, KEY);
        self.events.push(ReadEvent::Read(segment, *size, output.to_vec()));
        output[..self.bytes.len()].copy_from_slice(&self.bytes);
        *size = self.second_size;
        self.second_eax
    }
    fn release_segment(&mut self, segment: Segment) { self.events.push(ReadEvent::Release(segment)); }
}

#[test]
fn section_7_3_seven_groups() {
    let a = ReadEvent::Acquire(Service(7), KEY);
    let mut f = Reader { acquired: Register(0xF0000000), ..Reader::default() };
    let mut out = b"old".to_vec();
    assert_eq!(read_resource(Some(Service(7)), KEY, &mut out, &mut f), Ok(false));
    assert_eq!(out, b"old");
    assert_eq!(f.events, vec![a]); // Non-null foreign output is deliberately not released.
    let mut f = Reader { first_eax: u32::MAX, ..Reader::default() };
    assert_eq!(read_resource(Some(Service(7)), KEY, &mut out, &mut f), Ok(false));
    assert_eq!(out, b"old");
    assert_eq!(f.events, vec![ReadEvent::Acquire(Service(7), KEY), ReadEvent::Size(Segment(8), 0), ReadEvent::Release(Segment(8))]);
    // Ordinary write; out-size vs EAX; failed read; zero size; changed second size.
    for (first_size, first_eax, bytes, second_size, initial_read, expected) in [
        (0, 3, b"ABC".as_slice(), 3, b"old".as_slice(), b"ABC".as_slice()),
        (9, 3, b"".as_slice(), 3, b"old".as_slice(), b"old".as_slice()),
        (0, 3, b"".as_slice(), 3, b"old".as_slice(), b"old".as_slice()),
        (0, 0, b"".as_slice(), 0, b"".as_slice(), b"".as_slice()),
        (0, 3, b"A".as_slice(), 1, b"old".as_slice(), b"Ald".as_slice()),
    ] {
        let mut f = Reader { first_size, first_eax, bytes: bytes.to_vec(), second_size, ..Reader::default() };
        let mut out = b"old".to_vec();
        assert_eq!(read_resource(Some(Service(7)), KEY, &mut out, &mut f), Ok(true));
        assert_eq!(out, expected);
        assert_eq!(f.events, vec![ReadEvent::Acquire(Service(7), KEY), ReadEvent::Size(Segment(8), 0), ReadEvent::Read(Segment(8), first_eax, initial_read.to_vec()), ReadEvent::Release(Segment(8))]);
    }
}

#[derive(Default)]
struct Life { lookup: Option<Service>, lookup_eax: u32, events: Vec<LifeEvent> }
impl LifecyclePrimitives for Life {
    fn lookup(&mut self, framework: Framework, class: u32, iid: u32) -> Lookup {
        assert_eq!((framework, class, iid), (Framework(3), 0x056B906E, 0x656B8EFC));
        Lookup { result: Register(self.lookup_eax), service: self.lookup }
    }
    fn event(&mut self, event: LifeEvent) { self.events.push(event); }
}
fn initialized() -> (Hook, Life) {
    let mut h = Hook::new(Allocation(10));
    let mut f = Life { lookup: Some(Service(1)), ..Life::default() };
    assert_eq!(h.init(Some(Framework(3)), &mut f), Ok(true));
    f.events.clear();
    (h, f)
}

#[test]
fn section_7_4_six_lifecycle_traces() {
    let mut h = Hook::new(Allocation(10));
    let mut f = Life::default();
    assert_eq!(h.count(), 0);
    assert_eq!(h.retained(), None);
    assert_eq!(h.add_ref(), Ok(1));
    let sentinel = View { allocation: Allocation(99), kind: ViewKind::Base };
    let mut output = sentinel;
    assert_eq!(h.query_interface(1, &mut output), Ok(true));
    assert_eq!(output, View { allocation: Allocation(10), kind: ViewKind::Hook });
    assert_eq!(h.count(), 2);
    assert_eq!(h.query_interface(42, &mut output), Ok(false));
    assert_eq!(output, View { allocation: Allocation(10), kind: ViewKind::Hook });
    assert_eq!(h.count(), 2);
    assert_eq!(h.release(&mut f), Ok(1));
    assert_eq!(h.release(&mut f), Ok(0));
    assert_eq!(f.events, vec![LifeEvent::RestoreTables, LifeEvent::BaseDestroy(Allocation(10)), LifeEvent::Free(Allocation(10))]);

    let (mut h, mut f) = initialized();
    f.lookup = Some(Service(2)); // false lookup AL still has a usable pointer.
    assert_eq!(h.init(Some(Framework(3)), &mut f), Ok(true));
    assert_eq!(f.events, vec![LifeEvent::AddRef(Service(2)), LifeEvent::Store(Some(Service(2))), LifeEvent::Release { service: Service(1), retained: Some(Service(2)) }, LifeEvent::Release { service: Service(2), retained: Some(Service(2)) }]);

    let (mut h, mut f) = initialized();
    assert_eq!(h.init(Some(Framework(3)), &mut f), Ok(true));
    assert_eq!(f.events, vec![LifeEvent::Release { service: Service(1), retained: Some(Service(1)) }]);

    let (mut h, mut f) = initialized();
    assert_eq!(h.init(None, &mut f), Ok(true));
    assert_eq!(f.events, vec![LifeEvent::Store(None), LifeEvent::Release { service: Service(1), retained: None }]);

    let (mut h, mut f) = initialized();
    assert_eq!(h.shutdown(&mut f), Ok(true));
    assert_eq!(h.shutdown(&mut f), Ok(true));
    h.destroy(1, &mut f).unwrap();
    assert_eq!(f.events, vec![LifeEvent::Store(None), LifeEvent::Release { service: Service(1), retained: None }, LifeEvent::RestoreTables, LifeEvent::BaseDestroy(Allocation(10)), LifeEvent::Free(Allocation(10))]);

    let (mut h, mut f) = initialized();
    assert_eq!(h.destroy(2, &mut f), Ok(View { allocation: Allocation(10), kind: ViewKind::Base }));
    assert_eq!(f.events, vec![LifeEvent::RestoreTables, LifeEvent::Release { service: Service(1), retained: Some(Service(1)) }, LifeEvent::BaseDestroy(Allocation(10))]);
}

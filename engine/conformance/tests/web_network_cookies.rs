//! web-network section 3: cookie vectors with deterministic clock/INI replies.
use fc4_core::rzurl::RzUrl;
use fc4_core::www::{cookies::*, Contract};
use std::collections::BTreeMap;
#[derive(Default)]
struct Io {
    now: u32,
    exists: bool,
    entries: BTreeMap<Vec<u8>, Vec<u8>>,
    reads: Vec<Vec<u8>>,
    writes: Vec<(Vec<u8>, Vec<u8>)>,
    clears: usize,
}
impl Host for Io {
    fn now(&mut self) -> u32 {
        self.now
    }
    fn date_seconds(&mut self, text: &[u8]) -> Contract<u32> {
        Ok(if text == b"valid" { 123 } else { 0 })
    }
    fn exists(&mut self, _: &[u8]) -> bool {
        self.exists
    }
    fn clear(&mut self, _: &[u8]) {
        self.clears += 1;
    }
    fn binary_marker(&mut self, _: &[u8]) -> bool {
        false
    }
    fn write_binary_marker(&mut self, _: &[u8]) {}
    fn read_text(&mut self, _: &[u8], key: &[u8]) -> Option<Vec<u8>> {
        self.reads.push(key.to_vec());
        self.entries.get(key).cloned()
    }
    fn write_text(&mut self, _: &[u8], key: &[u8], text: &[u8]) {
        self.writes.push((key.to_vec(), text.to_vec()));
    }
}
fn url(text: &[u8]) -> RzUrl {
    RzUrl::from_text(text)
}
#[test]
fn parse_defaults_attributes_and_literal_port_rule() {
    let mut io = Io {
        now: 100,
        ..Io::default()
    };
    let u = url(b"http://www.simcity.com/a/b.html");
    let c = Cookie::parse(b"a=1", Some(&u), &mut io).unwrap();
    assert_eq!(c.expires, 2);
    assert_eq!(c.domain, b".simcity.com");
    assert_eq!(c.path, b"/a/b.html");
    assert_eq!(c.text, b"a=1;  domain=.simcity.com; path=/a/b.html");
    assert_eq!(c.pairs, [(b"a".to_vec(), b"1".to_vec())]);
    let c = Cookie::parse(
        b"a=1; Comment=x; CommentURL=y; domain=simcity.com; PATH=/; Version=-2x; Expires=bad; Port=80,81,82; Secure",
        Some(&u),
        &mut io,
    )
    .unwrap();
    assert_eq!(c.expires, 86500);
    assert_eq!(c.version, -2);
    assert!(c.secure);
    assert_eq!(c.comment, b"x");
    assert_eq!(c.comment_url, b"y");
    assert_eq!(&c.ports[..4], &[80, 81, 0, 0]);
    for (text, expires) in [
        (b"a=1; Max-Age=50; Discard".as_slice(), 1),
        (b"a=1; Discard; Expires=valid; Max-Age=-10", 90),
        (b"a=1; Expires=valid", 123),
    ] {
        assert_eq!(Cookie::parse(text, None, &mut io).unwrap().expires, expires);
    }
    assert_eq!(Cookie::parse(b"a=1; Port", None, &mut io).unwrap().ports[0], 80);
    assert_eq!(Cookie::parse(b"a=1; Port=80", None, &mut io).unwrap().ports[0], 0);
    assert_eq!(
        Cookie::parse(b"a=1", Some(&url(b"http://localhost/x")), &mut io)
            .unwrap()
            .domain,
        b""
    );
    let long = std::iter::repeat("a=1").take(45).collect::<Vec<_>>().join(";");
    assert_eq!(Cookie::parse(long.as_bytes(), None, &mut io).unwrap().pairs.len(), 40);
}
#[test]
fn admission_replacement_expiration_and_lookup() {
    let mut io = Io {
        now: 100,
        ..Io::default()
    };
    let mut m = Manager::new(vec![]);
    let u = url(b"http://www.simcity.com/x/y");
    assert!(m.add_text(b"a=1; Path=/x", Some(&u), &mut io).unwrap());
    assert!(!m.add_text(b"a=2; Path=/z", Some(&u), &mut io).unwrap());
    assert!(!m.add_text(b"b=1; Domain=.com; Path=/", Some(&u), &mut io).unwrap());
    assert!(!m
        .add_text(
            b"b=1; Domain=.simcity.com; Path=/",
            Some(&url(b"http://a.b.simcity.com/x")),
            &mut io
        )
        .unwrap());
    assert!(m.lookup(&u, 0, 100).is_some());
    assert!(m.lookup(&url(b"http://www.SIMCITY.com/x/y"), 0, 100).is_none());
    assert!(m.add_text(b"a=2; Path=/x; Max-Age=0", Some(&u), &mut io).is_ok());
    assert!(m.records.is_empty());
    assert!(m.add_text(b"s=1; Secure; Path=/", Some(&u), &mut io).unwrap());
    assert!(m.lookup(&url(b"https://www.simcity.com/"), 0, 100).is_none());
    assert!(m.add_text(b"p=1; Path=/; Port=81,82", Some(&u), &mut io) == Ok(false));
    m.add_text(b"test=v; Domain=.a; Path=/", None, &mut io).unwrap();
    assert!(m.lookup(&url(b"http://a.a.a/"), 0, 100).is_none()); // first occurrence is not the suffix
    assert!(m.lookup(&url(b"http://x/"), 0, 100).is_none()); // domain longer than server
    assert!(!m.add_text(b"", None, &mut io).unwrap());
}
#[test]
fn header_version_quirk_and_eviction_order() {
    let mut io = Io {
        now: 100,
        ..Io::default()
    };
    let mut m = Manager::new(vec![]);
    let u = url(b"http://www.simcity.com/x/y");
    let mut out = vec![99];
    for text in [b"a=1; Version=1; Path=/".as_slice(), b"b=2; Path=/x"] {
        assert!(m.add_text(text, Some(&u), &mut io).unwrap());
    }
    assert!(m.header(&u, 100, &mut out));
    assert_eq!(out, b"$Version=1; a=1; $Path=/; b=2; $Path=/x");
    assert!(m.add_text(b"c=3; Version=2; Path=/", Some(&u), &mut io).unwrap());
    m.header_text(b"http://www.simcity.com/x/y", 100, &mut out);
    assert!(out.starts_with(b"$Version=2; $Version=1;"));
    m.remove_all();
    for n in 0..6 {
        let text = format!("k{n}=v; Max-Age={}", n + 1);
        m.add_text(text.as_bytes(), None, &mut io).unwrap();
    }
    assert_eq!(m.evict(102), 4);
    assert_eq!(m.records[0].expires, 105);
    m.capacity = 2;
    m.add_text(b"new=v", None, &mut io).unwrap();
    assert_eq!(m.records.len(), 1);
}
#[test]
fn ck01_through_ck06_and_lifecycle() {
    let mut io = Io {
        now: 100,
        ..Io::default()
    };
    let mut m = Manager::new(vec![]);
    assert_eq!(m.capacity, 16384);
    assert!(!m.binary);
    assert!(!m.initialized);
    m.add_text(b"a=1", None, &mut io).unwrap();
    assert_eq!(m.read(&mut io), Ok(true));
    assert!(m.records.is_empty()); // CK01
    for (text, expiry) in [(b"a=1".as_slice(), 2), (b"b=2", 100), (b"c=3", 101)] {
        let mut c = Cookie::parse(text, None, &mut io).unwrap();
        c.expires = expiry;
        m.records.push(c);
    }
    assert_eq!(m.write(&mut io), Ok(true));
    assert_eq!(io.writes, [(b"CookieData00000".to_vec(), b"c=3; path=/".to_vec())]); // CK02
    let mut count = 2;
    assert_eq!(m.enumerate(&mut count).len(), 2);
    assert_eq!(count, 2); // CK04
    count = 5;
    assert_eq!(m.enumerate(&mut count).len(), 3);
    assert_eq!(count, 3); // CK05
    io.exists = true;
    io.entries.insert(b"CookieData00000".to_vec(), b"loaded=v".to_vec());
    io.entries.insert(b"CookieData00002".to_vec(), b"gap=v".to_vec());
    m.read(&mut io).unwrap();
    assert_eq!(m.records.len(), 1);
    assert_eq!(io.reads, [b"CookieData00000".to_vec(), b"CookieData00001".to_vec()]); // CK03
    m.remove_all();
    let clears = io.clears;
    m.write(&mut io).unwrap();
    assert_eq!(io.clears, clears + 1); // CK06
    m.init(&mut io);
    let reads = io.reads.len();
    m.init(&mut io);
    assert_eq!(io.reads.len(), reads);
    m.shutdown(&mut io);
    assert!(!m.initialized);
    assert!(m.records.is_empty());
    m.binary = true;
    m.add_text(b"x=v; Max-Age=20", None, &mut io).unwrap();
    assert!(m.write(&mut io).is_err()); // Q3 stays explicit
}

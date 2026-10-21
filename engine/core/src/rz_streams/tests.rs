// SPDX-License-Identifier: GPL-3.0-or-later
//! Conformance tests of docs/specs/output-streams.md §9 (numbered as there)
//! plus boundary, short-read and partial-transfer cases.

use super::*;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Recording binary sink with scripted results (true when the script is out).
#[derive(Default)]
struct Rec {
    calls: Vec<Vec<u8>>,
    script: VecDeque<bool>,
    flushes: u32,
}
impl ByteSink for Rec {
    fn flush(&mut self) {
        self.flushes += 1;
    }
    fn set_void(&mut self, p: &[u8]) -> bool {
        self.calls.push(p.to_vec());
        self.script.pop_front().unwrap_or(true)
    }
    fn get_error(&self) -> bool {
        false
    }
}
impl TextSink for Rec {
    fn flush(&mut self) {
        self.flushes += 1;
    }
    fn set_rz_char_str(&mut self, s: &[u8]) -> bool {
        self.calls.push(s.to_vec());
        self.script.pop_front().unwrap_or(true)
    }
}

fn bin(le: bool) -> RzOStream<Rec> {
    RzOStream::new(le, Rec::default())
}
fn text() -> RzTextOStream<Rec> {
    RzTextOStream::new(true, Rec::default())
}
fn calls<S: ByteSink>(s: &RzOStream<S>) -> &S {
    s.sink()
}
fn texts(s: &RzTextOStream<Rec>) -> Vec<String> {
    s.sink()
        .calls
        .iter()
        .map(|c| String::from_utf8(c.clone()).unwrap())
        .collect()
}

#[test]
fn t1_little_endian() {
    let mut s = bin(true);
    s.set_uint8(0xAB);
    s.set_sint8(-1);
    s.set_uint16(0x1234);
    s.set_sint16(-2);
    s.set_uint32(0x1234_5678);
    s.set_sint64(-2);
    s.set_uint64(0x0102_0304_0506_0708);
    let c = &calls(&s).calls;
    assert_eq!(
        c,
        &vec![
            vec![0xAB],
            vec![0xFF],
            vec![0x34, 0x12],
            vec![0xFE, 0xFF],
            vec![0x78, 0x56, 0x34, 0x12],
            vec![0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            vec![8, 7, 6, 5, 4, 3, 2, 1],
        ]
    );
}

#[test]
fn t2_big_endian() {
    let mut s = bin(false);
    s.set_uint8(0xAB);
    s.set_uint16(0x1234);
    s.set_sint16(-2);
    s.set_uint32(0x1234_5678);
    s.set_uint64(0x0102_0304_0506_0708);
    s.set_sint32(-2);
    let c = &calls(&s).calls;
    assert_eq!(c[0], vec![0xAB]);
    assert_eq!(c[1], vec![0x12, 0x34]);
    assert_eq!(c[2], vec![0xFF, 0xFE]);
    assert_eq!(c[3], vec![0x12, 0x34, 0x56, 0x78]);
    assert_eq!(c[4], vec![1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(c[5], vec![0xFF, 0xFF, 0xFF, 0xFE]);
}

#[test]
fn t3_floats_and_init() {
    for le in [true, false] {
        let mut s = bin(le);
        s.set_float32(1.0);
        s.set_float64(1.0);
        s.set_float32(-2.5);
        let c = &calls(&s).calls;
        assert_eq!(c[0], vec![0, 0, 0x80, 0x3F]);
        assert_eq!(c[1], vec![0, 0, 0, 0, 0, 0, 0xF0, 0x3F]);
        assert_eq!(c[2], vec![0, 0, 0x20, 0xC0]);
    }
    let mut s = bin(true);
    assert!(s.core_mut().init(false));
    s.set_uint16(0x1234);
    assert!(s.core_mut().init(true));
    s.set_uint16(0x1234);
    assert!(s.core_mut().shutdown());
    assert_eq!(calls(&s).calls, vec![vec![0x12, 0x34], vec![0x34, 0x12]]);
}

#[test]
fn t4_base_strings() {
    let mut s = bin(true);
    s.set_rz_char_str(b"");
    s.set_rz_char_str(b"abc\0ignored");
    assert_eq!(calls(&s).calls, vec![vec![0], vec![3], b"abc".to_vec()]);
    for (n, prefix) in [
        (128usize, vec![0x80u8, 0x01]),
        (300, vec![0xAC, 0x02]),
        (16384, vec![0x80, 0x80, 0x01]),
    ] {
        let mut s = bin(false);
        let body = vec![b'x'; n];
        s.set_rz_char_str(&body);
        let c = &calls(&s).calls;
        assert_eq!(c.len(), prefix.len() + 1);
        for (i, p) in prefix.iter().enumerate() {
            assert_eq!(c[i], vec![*p]);
        }
        assert_eq!(c[prefix.len()], body);
    }
    let mut s = bin(true);
    s.set_gz_str(b"a\0b");
    assert_eq!(calls(&s).calls, vec![vec![3], vec![b'a', 0, b'b']]);
}

#[test]
fn t4_length_max() {
    let mut s = bin(true);
    super::ostream::encode_string_length(&mut s, u32::MAX);
    let c: Vec<u8> = calls(&s).calls.iter().map(|v| v[0]).collect();
    assert_eq!(c, vec![0xFF, 0xFF, 0xFF, 0xFF, 0x0F]);
}

#[test]
fn t5_string_results() {
    let mut s = bin(true);
    s.sink_mut().script = VecDeque::from(vec![false; 10]);
    assert!(s.set_rz_char_str(b""));
    assert!(!s.set_rz_char_str(b"abc"));
    let mut s = bin(true);
    s.sink_mut().script = VecDeque::from(vec![false]);
    assert!(s.set_rz_char_str(b"abc"));
    // a failing sink is passed through for numbers, without stickiness
    let mut s = bin(true);
    s.sink_mut().script = VecDeque::from(vec![false, true]);
    assert!(!s.set_uint32(1));
    assert!(s.set_uint32(1));
}

struct Ser {
    writes: u32,
    reads: u32,
    result: bool,
}
impl GzSerializable for Ser {
    fn write(&mut self, stream: &mut dyn GzOStream) -> bool {
        self.writes += 1;
        let _ = stream.get_error();
        self.result
    }
    fn read(&mut self, stream: &mut dyn GzIStream) -> bool {
        self.reads += 1;
        let _ = stream.get_error();
        self.result
    }
}

#[test]
fn t6_serializable() {
    let mut s = bin(true);
    let mut o = Ser {
        writes: 0,
        reads: 0,
        result: false,
    };
    assert!(!s.set_gz_serializable(&mut o));
    o.result = true;
    assert!(s.set_gz_serializable(&mut o));
    assert_eq!(o.writes, 2);
    assert!(calls(&s).calls.is_empty());
    let mut n = RzNullOStream::new_null();
    assert!(n.set_gz_serializable(&mut o));
    assert_eq!(o.writes, 3);
    let mut i = RzNullIStream::new_null();
    assert!(i.get_gz_serializable(&mut o));
    assert_eq!(o.reads, 1);
}

struct Ud(AtomicU32);
impl CountedRef for Ud {
    fn add_ref(&self) -> u32 {
        self.0.fetch_add(1, Ordering::SeqCst) + 1
    }
    fn release(&self) -> u32 {
        self.0.fetch_sub(1, Ordering::SeqCst) - 1
    }
}

#[test]
fn t7_counting_interfaces_user_data() {
    let mut s = bin(true);
    let c = s.core_mut();
    assert_eq!(c.ref_count(), 0);
    assert_eq!(c.add_ref(), 1);
    assert_eq!(c.add_ref(), 2);
    assert_eq!(c.release(), ReleaseOutcome::Remaining(1));
    assert_eq!(c.release(), ReleaseOutcome::Deleted);
    assert_eq!(c.ref_count(), 1, "deleted without decrementing");
    let mut z = StreamCore::new(true, IID_GZ_OSTREAM);
    assert_eq!(z.release().value(), 0, "count 0 also deletes");
    assert!(c.query_interface(IID_GZ_UNKNOWN));
    assert!(c.query_interface(IID_GZ_OSTREAM));
    assert_eq!(c.ref_count(), 3);
    assert!(!c.query_interface(IID_GZ_ISTREAM));
    assert_eq!(c.ref_count(), 3);
    let mut i = RzNullIStream::new_null();
    assert!(i.core_mut().query_interface(IID_GZ_ISTREAM));
    assert!(!i.core_mut().query_interface(IID_GZ_OSTREAM));

    let v: Arc<Ud> = Arc::new(Ud(AtomicU32::new(1)));
    let w: Arc<Ud> = Arc::new(Ud(AtomicU32::new(1)));
    let s2 = {
        let mut s2 = RzNullOStream::new_null();
        s2.core_mut().set_user_data(Some(v.clone()));
        assert_eq!(v.0.load(Ordering::SeqCst), 2);
        s2.core_mut().set_user_data(Some(v.clone()));
        assert_eq!(v.0.load(Ordering::SeqCst), 2, "same pointer changes nothing");
        s2.core_mut().set_user_data(Some(w.clone()));
        assert_eq!(v.0.load(Ordering::SeqCst), 1);
        assert_eq!(w.0.load(Ordering::SeqCst), 2);
        let got = s2.core().user_data().unwrap();
        assert!(std::ptr::addr_eq(Arc::as_ptr(got), Arc::as_ptr(&w)));
        assert_eq!(w.0.load(Ordering::SeqCst), 2, "GetUserData adds no reference");
        s2
    };
    drop(s2);
    assert_eq!(w.0.load(Ordering::SeqCst), 1, "destructor releases the user data");
}

#[test]
fn t8_text_numbers() {
    let mut s = text();
    s.set_sint8(-5);
    s.set_uint8(200);
    s.set_sint16(-32768);
    s.set_uint16(65535);
    s.set_sint32(i32::MIN);
    s.set_uint32(u32::MAX);
    s.set_sint64(i64::MIN);
    s.set_uint64(u64::MAX);
    s.set_uint32(1);
    s.set_uint32(2);
    assert_eq!(
        texts(&s),
        vec![
            "-5",
            "200",
            "-32768",
            "65535",
            "-2147483648",
            "4294967295",
            "-9223372036854775808",
            "18446744073709551615",
            "1",
            "2"
        ]
    );
    assert!(s.get_error(), "cRZTextOStream::GetError is AL 1");
    s.sink_mut().script = VecDeque::from(vec![false]);
    assert!(!s.set_uint8(1));
}

#[test]
fn t9_text_floats() {
    let mut s = text();
    s.set_float32(0.5);
    s.set_float32(0.1);
    for d in [-2.5, 0.0, 100000.0, 1000000.0, 1234567.0, 0.0001, 0.00001, 1e100] {
        s.set_float64(d);
    }
    assert_eq!(
        texts(&s),
        vec![
            "0.5",
            "0.1",
            "-2.5",
            "0",
            "100000",
            "1e+006",
            "1.23457e+006",
            "0.0001",
            "1e-005",
            "1e+100"
        ]
    );
}

#[test]
fn format_g_edges() {
    assert_eq!(format_g(999999.5, b'.'), "1e+006");
    assert_eq!(format_g(999999.0, b'.'), "999999");
    assert_eq!(format_g(0.000123456789, b'.'), "0.000123457");
    assert_eq!(format_g(1e-300, b'.'), "1e-300");
    assert_eq!(format_g(-0.0, b'.'), "-0");
    assert_eq!(format_g(1.5, b','), "1,5");
    assert_eq!(format_g(f64::INFINITY, b'.'), "1.#INF");
    assert_eq!(format_g(f64::NEG_INFINITY, b'.'), "-1.#INF");
    assert_eq!(format_g(f64::from_bits(0x7FF8_0000_0000_0000), b'.'), "1.#QNAN");
    assert_eq!(format_g(f64::from_bits(0xFFF8_0000_0000_0000), b'.'), "-1.#IND");
    assert_eq!(format_g(f64::from_bits(0x7FF0_0000_0000_0001), b'.'), "1.#SNAN");
    assert_eq!(format_g(f64::MAX, b'.'), "1.79769e+308");
    assert_eq!(format_g(123456.0, b'.'), "123456");
    assert_eq!(format_g(12.5, b'.'), "12.5");
}

#[test]
fn t10_text_strings() {
    let mut s = text();
    s.set_gz_str(b"ab");
    s.set_gz_str(b"a\0b");
    s.set_rz_char_str(b"x\0y");
    assert_eq!(texts(&s), vec!["ab", "a", "x"]);
}

#[test]
fn t11_hex_dump() {
    let mut s = text();
    assert!(s.set_void(&[0x00, 0x7F, 0x80, 0xFF]));
    assert_eq!(
        texts(&s),
        vec!["\nHEXDUMP:\n", "00 ", "7f ", "ffffff80 ", "ffffffff ", "\n"]
    );
    let mut s = text();
    assert!(s.set_void(&[]));
    assert_eq!(texts(&s), vec!["\nHEXDUMP:\n", "\n"]);
    let mut s = text();
    s.set_void(&[0; 25]);
    let t = texts(&s);
    assert_eq!(t.len(), 28);
    assert!(t[1..26].iter().all(|p| p == "00 "));
    assert_eq!(&t[26..], &["\n", "\n"]);
    let mut s = text();
    s.set_void(&[0; 26]);
    let t = texts(&s);
    assert_eq!(t.len(), 29);
    assert_eq!(&t[26..], &["\n", "00 ", "\n"]);
    let mut s = text();
    s.sink_mut().script = VecDeque::from(vec![true, true, false]);
    assert!(!s.set_void(&[0x00, 0x7F, 0x80, 0xFF]));
    assert_eq!(s.sink().calls.len(), 3);
}

#[test]
fn t12_null_stream() {
    let mut s = RzNullOStream::new_null();
    assert!(!s.core().little_endian());
    assert!(s.set_uint8(1) && s.set_sint8(1) && s.set_uint16(1) && s.set_sint16(1));
    assert!(s.set_uint32(1) && s.set_sint32(1) && s.set_uint64(1) && s.set_sint64(1));
    assert!(s.set_float32(1.0) && s.set_float64(1.0));
    assert!(s.set_rz_char_str(b"") && s.set_rz_char_str(b"abc") && s.set_gz_str(b"a\0b"));
    assert!(s.set_void(&[1, 2, 3]));
    s.flush();
    assert!(!s.get_error());
}

fn temp_path(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("fc4-rz-streams-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[test]
fn t13_file_stream() {
    let p = temp_path("t13.bin");
    let ps = p.to_str().unwrap().as_bytes().to_vec();
    {
        let mut f = RzOFStream::open(&mut StdFileOpener, &ps, true, false, false, false);
        assert!(!f.get_error());
        assert!(f.set_uint32(1));
        assert!(f.set_rz_char_str(b"hi\n"));
        assert!(f.set_float32(1.0));
    }
    assert_eq!(
        std::fs::read(&p).unwrap(),
        vec![0, 0, 0, 1, 0x68, 0x69, 0x0A, 0, 0, 0x80, 0x3F]
    );
    {
        let mut f = RzOFStream::open(&mut StdFileOpener, &ps, true, false, false, true);
        assert!(f.set_uint8(7));
    }
    assert_eq!(std::fs::read(&p).unwrap().last(), Some(&7));
    assert_eq!(std::fs::read(&p).unwrap().len(), 12);
    {
        let mut f = RzOFStream::open(&mut StdFileOpener, &ps, true, false, false, false);
        assert!(f.set_gz_str(b"a\0b"));
    }
    assert_eq!(std::fs::read(&p).unwrap(), b"a".to_vec());
    let bad = temp_path("no-such-dir").join("x").join("y.bin");
    let mut f = RzOFStream::open(
        &mut StdFileOpener,
        bad.to_str().unwrap().as_bytes(),
        true,
        false,
        false,
        false,
    );
    assert!(f.get_error());
    assert!(!f.set_uint32(1) && !f.set_rz_char_str(b"x") && !f.set_void(&[]));
    f.flush();
    assert!(f.get_error());
    let _ = std::fs::remove_file(&p);
}

/// A scripted provider file: takes at most `cap` bytes per write.
#[derive(Default)]
struct MemFile {
    data: Vec<u8>,
    cap: Option<usize>,
    syncs: u32,
    sync_ok: bool,
}
impl OfFile for MemFile {
    fn write(&mut self, p: &[u8]) -> usize {
        let n = self.cap.map_or(p.len(), |c| c.min(p.len()));
        self.data.extend_from_slice(&p[..n]);
        n
    }
    fn sync(&mut self) -> bool {
        self.syncs += 1;
        self.sync_ok
    }
}
struct MemOpener {
    cap: Option<usize>,
    modes: Vec<(Vec<u8>, OfOpenMode)>,
}
impl OfFileOpener for MemOpener {
    type File = MemFile;
    fn open(&mut self, path: &[u8], mode: OfOpenMode) -> Option<MemFile> {
        self.modes.push((path.to_vec(), mode));
        Some(MemFile {
            cap: self.cap,
            sync_ok: true,
            ..Default::default()
        })
    }
}

#[test]
fn file_partial_transfer_is_sticky() {
    let mut o = MemOpener {
        cap: Some(2),
        modes: vec![],
    };
    let mut f = RzOFStream::open(&mut o, b"q.txt\0junk", true, true, false, true);
    assert_eq!(
        o.modes,
        vec![(
            b"q.txt".to_vec(),
            OfOpenMode {
                binary: true,
                append: true
            }
        )]
    );
    assert!(f.set_uint16(0x1234));
    assert!(!f.set_uint32(0x0A0B_0C0D), "short write sets badbit");
    assert!(f.get_error());
    assert!(!f.set_uint8(1), "sticky");
    assert_eq!(f.file().unwrap().data, vec![0x34, 0x12, 0x0D, 0x0C]);
    assert_eq!(f.state() & file::BADBIT, file::BADBIT);
}

#[test]
fn file_auto_flush_and_text_mode() {
    let mut o = MemOpener {
        cap: None,
        modes: vec![],
    };
    let mut f = RzOFStream::open(&mut o, b"log", true, false, true, false);
    f.set_uint32(5);
    assert_eq!(f.file().unwrap().syncs, 0, "binary numbers do not flush");
    f.set_rz_char_str(b"x");
    f.set_gz_str(b"y");
    assert_eq!(f.file().unwrap().syncs, 2);
    let mut t = RzOFStream::open(&mut o, b"t", false, true, true, false);
    t.set_sint32(-7);
    t.set_float64(1234567.0);
    t.set_rz_char_str(b"z\n");
    t.set_void(&[0xFF]);
    assert_eq!(t.file().unwrap().data, b"-71.23457e+006z\n\xFF".to_vec());
    assert_eq!(
        t.file().unwrap().syncs,
        3,
        "text values and strings flush, SetVoid does not"
    );
    // a failed sync sets badbit but is not the write's result
    let mut u = RzOFStream::open(&mut o, b"u", true, true, true, false);
    let mut u2 = RzOFStream::open(&mut BadSync, b"u", true, true, true, false);
    assert!(u2.set_rz_char_str(b"a"));
    assert!(u2.get_error());
    assert!(!u2.set_rz_char_str(b"a"));
    assert!(u.set_rz_char_str(b"a") && !u.get_error());
}

struct BadSync;
impl OfFileOpener for BadSync {
    type File = MemFile;
    fn open(&mut self, _path: &[u8], _mode: OfOpenMode) -> Option<MemFile> {
        Some(MemFile::default())
    }
}

/// An atomic source over bytes, with an error flag; optional short reads
/// that transfer what is left and then fail.
struct Src {
    data: Vec<u8>,
    pos: usize,
    err: bool,
    partial: bool,
    skips: Vec<u32>,
}
impl Src {
    fn new(data: Vec<u8>) -> Self {
        Src {
            data,
            pos: 0,
            err: false,
            partial: false,
            skips: vec![],
        }
    }
}
impl ByteSource for Src {
    fn skip(&mut self, n: u32) -> bool {
        self.skips.push(n);
        if self.data.len() - self.pos < n as usize {
            self.err = true;
            return false;
        }
        self.pos += n as usize;
        true
    }
    fn get_void(&mut self, out: &mut [u8]) -> bool {
        let left = self.data.len() - self.pos;
        if left < out.len() {
            if self.partial {
                out[..left].copy_from_slice(&self.data[self.pos..]);
                self.pos = self.data.len();
            }
            self.err = true;
            return false;
        }
        out.copy_from_slice(&self.data[self.pos..self.pos + out.len()]);
        self.pos += out.len();
        true
    }
    fn get_error(&self) -> bool {
        self.err
    }
}

#[derive(Default)]
struct Collect(Vec<u8>);
impl ByteSink for Collect {
    fn flush(&mut self) {}
    fn set_void(&mut self, p: &[u8]) -> bool {
        self.0.extend_from_slice(p);
        true
    }
    fn get_error(&self) -> bool {
        false
    }
}

#[test]
fn t14_reader_round_trip() {
    for le in [true, false] {
        let mut w = RzOStream::new(le, Collect::default());
        w.set_uint8(0xAB);
        w.set_sint8(-1);
        w.set_uint16(0x1234);
        w.set_sint16(-2);
        w.set_uint32(0x1234_5678);
        w.set_sint32(-3);
        w.set_sint64(-2);
        w.set_uint64(0x0102_0304_0506_0708);
        w.set_float32(-2.5);
        w.set_float64(1.0);
        w.set_rz_char_str(b"abc");
        w.set_gz_str(&[b'z'; 300]);
        let mut r = RzIStream::new(le, Src::new(w.sink().0.clone()));
        let (mut a, mut b, mut c, mut d) = (0u8, 0i8, 0u16, 0i16);
        let (mut e, mut f, mut g, mut h) = (0u32, 0i32, 0i64, 0u64);
        let (mut x, mut y) = (0f32, 0f64);
        assert!(r.get_uint8(&mut a) && r.get_sint8(&mut b) && r.get_uint16(&mut c) && r.get_sint16(&mut d));
        assert!(r.get_uint32(&mut e) && r.get_sint32(&mut f) && r.get_sint64(&mut g) && r.get_uint64(&mut h));
        assert!(r.get_float32(&mut x) && r.get_float64(&mut y));
        assert_eq!(
            (a, b, c, d, e, f, g, h),
            (0xAB, -1, 0x1234, -2, 0x1234_5678, -3, -2, 0x0102_0304_0506_0708)
        );
        assert_eq!((x, y), (-2.5, 1.0));
        let mut buf = [0xEEu8; 8];
        assert!(r.get_rz_char_str(&mut buf, 7));
        assert_eq!(&buf[..4], b"abc\0");
        let mut s = vec![1, 2];
        assert!(r.get_gz_str(&mut s));
        assert_eq!(s, vec![b'z'; 300]);
        assert!(!r.get_error());
    }
}

#[test]
fn t14_truncated_char_str() {
    let mut w = RzOStream::new(true, Collect::default());
    w.set_rz_char_str(b"abcde");
    w.set_uint8(0x55);
    let mut r = RzIStream::new(true, Src::new(w.sink().0.clone()));
    let mut buf = [0xEEu8; 4];
    assert!(r.get_rz_char_str(&mut buf, 2));
    assert_eq!(buf, [b'a', b'b', 0, 0xEE], "terminator at buf[max], 3 bytes written");
    assert_eq!(r.source().skips, vec![3]);
    let mut n = 0u8;
    assert!(r.get_uint8(&mut n) && n == 0x55, "all 6 bytes consumed");
    // Exact fit still calls Skip(0) (literal reading, OS6)
    let mut r = RzIStream::new(true, Src::new(vec![2, b'h', b'i']));
    let mut buf = [0xEEu8; 3];
    assert!(r.get_rz_char_str(&mut buf, 5));
    assert_eq!(buf, [b'h', b'i', 0]);
    assert_eq!(r.source().skips, vec![0]);
    // Empty string
    let mut r = RzIStream::new(true, Src::new(vec![0]));
    let mut buf = [0xEEu8; 2];
    assert!(r.get_rz_char_str(&mut buf, 5));
    assert_eq!(buf, [0, 0xEE]);
}

#[test]
fn short_reads() {
    // integers: failure leaves what GetVoid left, unreversed
    let mut r = RzIStream::new(false, Src::new(vec![0x12, 0x34]));
    r.source_mut().partial = true;
    let mut v = 0xAABB_CCDDu32;
    assert!(!r.get_uint32(&mut v));
    assert_eq!(v.to_le_bytes(), [0x12, 0x34, 0xBB, 0xAA]);
    // atomic source: output unchanged
    let mut r = RzIStream::new(true, Src::new(vec![1]));
    let mut w = 7u16;
    assert!(!r.get_uint16(&mut w));
    assert_eq!(w, 7);
    // string body short: error reported, buf terminated at k
    let mut r = RzIStream::new(true, Src::new(vec![5, b'a', b'b']));
    r.source_mut().partial = true;
    let mut buf = [0xEEu8; 8];
    assert!(!r.get_rz_char_str(&mut buf, 8));
    assert_eq!(&buf[..6], &[b'a', b'b', 0xEE, 0xEE, 0xEE, 0]);
    assert!(r.source().skips.is_empty(), "no Skip after an error");
    // GetGZStr: a failed chunk is not appended
    let mut src = vec![0xAC, 0x02];
    src.extend(vec![b'q'; 260]);
    let mut r = RzIStream::new(true, Src::new(src));
    let mut s = vec![9];
    assert!(!r.get_gz_str(&mut s));
    assert_eq!(s, vec![b'q'; 255]);
    // length truncated mid-varint: the failed byte reads as 0 and ends it
    let mut r = RzIStream::new(true, Src::new(vec![0x85]));
    assert_eq!(istream::decode_string_length(&mut r), 5);
    assert!(r.get_error());
}

#[test]
fn long_varint_is_consumed() {
    let mut data = vec![0xFF; 10];
    data.push(0x01);
    data.push(0x42);
    let mut r = RzIStream::new(true, Src::new(data));
    let _ = istream::decode_string_length(&mut r);
    let mut n = 0u8;
    assert!(r.get_uint8(&mut n) && n == 0x42);
}

#[test]
fn null_istream() {
    let mut r = RzNullIStream::new_null();
    assert!(!r.get_error());
    let mut v = 0x1234u16;
    assert!(!r.get_uint16(&mut v));
    assert_eq!(v, 0x1234);
    assert!(r.get_error());
    let mut r = RzNullIStream::new_null();
    let mut buf = [0xEEu8; 2];
    assert!(!r.get_rz_char_str(&mut buf, 1));
    assert_eq!(buf, [0, 0xEE]);
    let mut s = b"keep".to_vec();
    assert!(!r.get_gz_str(&mut s));
    assert!(s.is_empty(), "FromChar(\"\") first");
    assert!(!r.skip(1));
}

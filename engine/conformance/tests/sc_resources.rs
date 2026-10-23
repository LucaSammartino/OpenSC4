// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent executable vectors SCR-V01–30, docs/specs/sc-resources.md §9.
use fc4_core::{
    persistence_runtime::{Buffer, BufferHandle, BufferLayout, Format, Key, RawStream, SerialRecord},
    rz_streams::{GzOStream, RzTextOStream, TextSink},
    sc_resources::{self as sc, ascii::*, command::*, files::*, image::*, lexical::*, token::*},
};
use std::{
    collections::{BTreeMap, VecDeque},
    io::Cursor,
    sync::{Arc, Mutex},
};

fn parser() -> CommandParser {
    let mut p = CommandParser::default();
    p.init();
    p
}
fn serial(bytes: &[u8]) -> SerialRecord<Cursor<Vec<u8>>> {
    SerialRecord::new(Cursor::new(bytes.to_vec()), true)
}
fn text_record(bytes: &[u8]) -> SerialRecord<Cursor<Vec<u8>>> {
    let mut record = serial(&[b"TKT1".as_slice(), bytes, b"!"].concat());
    record.raw_mut().seek(4);
    record
}

#[test]
fn scr_v01_grouped_tokens() {
    let v = split(b"cmd 'a b' [c d]").unwrap();
    assert_eq!(
        v.iter().map(|t| t.bytes.as_slice()).collect::<Vec<_>>(),
        [b"cmd".as_slice(), b"a b", b"c d"]
    );
    assert_eq!(count_arguments(b"cmd 'a b' [c d]").unwrap(), 3);
    assert_eq!(v.iter().map(|t| t.index).collect::<Vec<_>>(), [0, 1, 2]);
}
#[test]
fn scr_v02_unterminated_delimiters() {
    for (s, expected) in [(b"cmd 'a".as_slice(), 5), (b"cmd [a".as_slice(), 6)] {
        assert!(matches!(split(s), Err(sc::Error::Parse { code, .. }) if code == expected));
    }
}
#[test]
fn scr_v03_numeric_expression() {
    assert_eq!(parse_float(b"2+3*4").unwrap(), 14.);
    assert!(parse_float(b"2+3junk").is_err());
    assert!(parse_float(b"").is_err());
    assert_eq!(parse_float(b"18/3-2*2").unwrap(), 2.);
    assert_eq!(parse_int(b"2.5").unwrap(), 2);
    assert_eq!(parse_int(b"3.5").unwrap(), 4);
    assert_eq!(parse_uint(b"-1").unwrap(), u32::MAX);
    assert_eq!(EXPRESSION_PRECISION.load(std::sync::atomic::Ordering::SeqCst), 53);
}
#[test]
fn scr_v04_boolean_spellings() {
    for (s, b) in [(b"ON".as_slice(), true), (b"false", false), (b"", false), (b"2", true)] {
        assert_eq!(parse_bool(s).unwrap(), b);
    }
    assert!(parse_bool(b"garbage").is_err());
}
#[test]
fn scr_v05_latest_registration() {
    let trace = Arc::new(Mutex::new(Vec::new()));
    let mut p = parser();
    for id in [1, 2] {
        let trace = trace.clone();
        p.register(
            b"cmd",
            Arc::new(move |_: &mut CommandParser, _: &mut [sc::ScriptToken]| {
                trace.lock().unwrap().push(id);
                Ok(())
            }),
        );
    }
    p.parse(b"cmd").unwrap();
    assert_eq!(*trace.lock().unwrap(), [2]);
    assert_eq!(p.registration_count(), 17);
}
#[test]
fn scr_v06_command_failures() {
    let mut p = parser();
    assert!(p.parse(b"unknown").is_err());
    assert!(p.parse(b"enddef").is_err());
    p.parse(b"define macro [x]").unwrap();
    assert!(p.parse(b"define nested [y]").is_err());
}
#[test]
fn scr_v07_macro_count() {
    let mut p = parser();
    p.parse(b"define macro [x y]").unwrap();
    p.parse(b"unknown %x").unwrap();
    p.parse(b"enddef").unwrap();
    assert!(p.parse(b"create macro [one]").is_err());
}

#[derive(Default)]
struct MemoryFiles {
    files: BTreeMap<String, Vec<Vec<u8>>>,
    trace: Vec<String>,
    stopped: Arc<Mutex<Vec<u64>>>,
}
struct Watch(Arc<Mutex<Vec<u64>>>);
impl Watcher for Watch {
    fn shutdown(&mut self) {
        self.0.lock().unwrap().push(9);
    }
}
struct MemoryLines(VecDeque<Vec<u8>>);
impl Lines for MemoryLines {
    fn read_line(&mut self) -> Option<Vec<u8>> {
        self.0.pop_front()
    }
}
impl Files for MemoryFiles {
    fn open(&mut self, path: &str) -> Option<Box<dyn Lines>> {
        self.trace.push(format!("open:{path}"));
        self.files
            .get(path)
            .cloned()
            .map(|v| Box::new(MemoryLines(v.into())) as Box<dyn Lines>)
    }
    fn watch(&mut self, path: &str, class: u32, iid: u32) -> Option<Box<dyn Watcher>> {
        assert_eq!((class, iid), (0xe953_da68, 0xe953_da64));
        self.trace.push(format!("watch:{path}"));
        Some(Box::new(Watch(self.stopped.clone())))
    }
}
#[derive(Default)]
struct Hooks(Vec<&'static str>);
impl ParseHooks for Hooks {
    fn begin(&mut self, _: &mut CommandParser) -> sc::Result<()> {
        self.0.push("begin");
        Ok(())
    }
    fn end(&mut self, _: &mut CommandParser) -> sc::Result<()> {
        self.0.push("end");
        Ok(())
    }
    fn finish(&mut self, _: &mut CommandParser) {
        self.0.push("finish");
    }
}
#[test]
fn scr_v08_file_modified() {
    let mut p = FileParser::default();
    assert!(!p.modified());
    p.add_input("a", 0, &mut MemoryFiles::default());
    assert!(p.modified());
    assert_eq!(p.num_inputs(), 1);
    assert_eq!(p.input_path(0), Some("a"));
}
#[test]
fn scr_v09_token_attachment() {
    let s = Arc::new(Mutex::new(serial(b"")));
    let mut t = TokenRecord::default();
    assert!(t.init_write(s.clone(), true));
    assert_eq!(s.lock().unwrap().record.reference_count(), 2);
    assert!(!t.init_write(s.clone(), true));
    assert_eq!(s.lock().unwrap().record.reference_count(), 2);
    t.shutdown();
    assert_eq!(s.lock().unwrap().record.reference_count(), 1);
    assert_eq!(t.cookie(), 0);
}
#[test]
fn scr_v10_missing_token() {
    let mut t: TokenRecord<Cursor<Vec<u8>>> = TokenRecord::default();
    assert!(t
        .get(Descriptor {
            id: 23,
            ty: 3,
            count: 1
        })
        .is_none());
    assert!(!t.set_token_string(23, b"name"));
}
fn fragments(record: &SerialRecord<Cursor<Vec<u8>>>) -> Vec<Vec<u8>> {
    let bytes = record.raw().get_ref();
    let mut at = 4;
    let mut result = Vec::new();
    while at < bytes.len() {
        let n = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        at += 4;
        result.push(bytes[at..at + n].to_vec());
        at += n;
    }
    result
}
#[test]
fn scr_v11_sorted_write_and_terminator() {
    let s = Arc::new(Mutex::new(serial(b"")));
    let mut t = TokenRecord::default();
    t.init_write(s.clone(), true);
    t.set(9, Values::U32(vec![90])).unwrap();
    t.set(2, Values::U32(vec![20])).unwrap();
    t.shutdown();
    let s = s.lock().unwrap();
    assert_eq!(&s.raw().get_ref()[..4], b"TKT1");
    assert_eq!(
        fragments(&s),
        [
            b"\r\n[ 2, 3, 1 ]= ".to_vec(),
            b"20, ".to_vec(),
            b"\r\n[ 9, 3, 1 ]= ".to_vec(),
            b"90, ".to_vec(),
            b"\r\nTK_TKR_End[ 1, 3, 1 ]= ".to_vec(),
            b"0, ".to_vec()
        ]
    );
}
#[test]
fn scr_v12_stop_before_end_payload() {
    let mut s = text_record(b"[ 1, 3, 1 ]= 0, ");
    s.raw_mut().seek(0);
    let s = Arc::new(Mutex::new(s));
    let mut t = TokenRecord::default();
    assert_eq!(t.init_read(s.clone(), None), Ok(true));
    let s = s.lock().unwrap();
    assert_eq!(&s.raw().get_ref()[s.raw().position() as usize..], b" 0, !");
}
#[test]
fn scr_v13_text_descriptor() {
    let mut s = text_record(b"[ 23, 3, 1 ]= 42, ");
    let mut io = TextIo::new(&mut s);
    let mut d = END;
    let mut v = 0u32;
    assert!(io.read_definition(&mut d));
    assert_eq!(
        d,
        Descriptor {
            id: 23,
            ty: 3,
            count: 1
        }
    );
    assert!(io.read_number(&mut v));
    assert_eq!(v, 42);
}
#[test]
fn scr_v14_numeric_prefix_and_zeroing() {
    let mut s = text_record(b"12suffix, suffix, ");
    let mut io = TextIo::new(&mut s);
    let mut v = 99u32;
    assert!(io.read_number(&mut v));
    assert_eq!(v, 12);
    assert!(!io.read_number(&mut v));
    assert_eq!(v, 0);
    v = 99;
    assert!(!io.read_number(&mut v));
    assert_eq!(v, 99);
}
#[test]
fn scr_v15_key_preservation() {
    let mut s = text_record(b"1, 2, ");
    let mut io = TextIo::new(&mut s);
    let mut k = Key::new(9, 8, 7);
    assert!(!io.read_key(&mut k));
    assert_eq!(k, Key::new(9, 8, 7));
}

#[derive(Default)]
struct BufferState {
    trace: Vec<String>,
    pixels: Vec<u32>,
    width: u32,
    height: u32,
    flags: u32,
}
struct TestBuffer {
    state: Arc<Mutex<BufferState>>,
    layout: BufferLayout,
    bytes: Vec<u8>,
}
impl Buffer for TestBuffer {
    fn add_ref(&mut self) {
        self.state.lock().unwrap().trace.push("retain".into());
    }
    fn release(&mut self) {
        self.state.lock().unwrap().trace.push("release".into());
    }
    fn query_interface(&mut self, iid: u32) -> bool {
        if iid == 0x2073_2180 {
            self.add_ref();
            true
        } else {
            false
        }
    }
    fn initialize(&mut self, w: u32, h: u32, f: Format) -> bool {
        self.layout = BufferLayout {
            width: w,
            height: h,
            stride: w as usize * 4,
            format: f,
        };
        let mut s = self.state.lock().unwrap();
        s.width = w;
        s.height = h;
        s.pixels = vec![0; (w * h) as usize];
        true
    }
    fn initialize_with_flags(&mut self, w: u32, h: u32, f: Format, flags: u32) -> bool {
        self.state.lock().unwrap().flags = flags;
        self.initialize(w, h, f)
    }
    fn layout(&self) -> BufferLayout {
        self.layout
    }
    fn lock(&mut self, _: bool) -> bool {
        self.state.lock().unwrap().trace.push("lock".into());
        true
    }
    fn unlock(&mut self) {
        self.state.lock().unwrap().trace.push("unlock".into());
    }
    fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }
    fn pixel32(&mut self, x: u32, y: u32, p: u32) -> bool {
        self.state.lock().unwrap().pixels[(y * self.layout.width + x) as usize] = p;
        true
    }
    fn rgb(&mut self, x: u32, y: u32, rgb: [u8; 3], _: Option<u8>) -> bool {
        self.pixel32(
            x,
            y,
            0xff00_0000 | (u32::from(rgb[0]) << 16) | (u32::from(rgb[1]) << 8) | u32::from(rgb[2]),
        )
    }
    fn rgb16(&self, _: u16) -> Option<[u8; 3]> {
        None
    }
    fn convert(&mut self, _: Format) -> bool {
        false
    }
}
struct Provider {
    buffer: BufferHandle,
    calls: Vec<u32>,
    fallback: bool,
}
impl BufferProvider for Provider {
    fn create(&mut self, class: u32, iid: u32, _: u32) -> Option<CreatedBuffer> {
        assert_eq!(iid, 0x2073_2180);
        self.calls.push(class);
        if self.fallback && class == 0xc470_d326 {
            None
        } else {
            Some(CreatedBuffer {
                buffer: self.buffer.clone(),
                aggregator: None,
            })
        }
    }
}
fn image(bits: u32) -> (PersistBufferResource, Arc<Mutex<BufferState>>) {
    let state = Arc::new(Mutex::new(BufferState::default()));
    let buffer: BufferHandle = Arc::new(Mutex::new(TestBuffer {
        state: state.clone(),
        layout: BufferLayout {
            width: 0,
            height: 0,
            stride: 0,
            format: Format::new(8, bits),
        },
        bytes: Vec::new(),
    }));
    let mut provider = Provider {
        buffer,
        calls: Vec::new(),
        fallback: true,
    };
    let image = PersistBufferResource::new(Format::new(8, bits), 0, &mut provider);
    assert_eq!(provider.calls, [0xc470_d326, 0xc470_d325]);
    (image, state)
}
#[test]
fn scr_v16_self_set_buffer() {
    let (mut image, state) = image(32);
    let b = image.buffer().unwrap();
    image.set_buffer(Some(b.clone()));
    b.lock().unwrap().release();
    assert_eq!(state.lock().unwrap().trace, ["retain", "retain", "release", "release"]);
}
struct FailedSeek {
    reads: usize,
    seeks: usize,
}
impl RawStream for FailedSeek {
    fn read(&mut self, _: &mut [u8]) -> bool {
        self.reads += 1;
        false
    }
    fn write(&mut self, _: &[u8]) -> bool {
        false
    }
    fn position(&self) -> u64 {
        0
    }
    fn size(&self) -> u64 {
        0
    }
    fn seek(&mut self, _: u64) -> bool {
        self.seeks += 1;
        false
    }
    fn close(&mut self) -> bool {
        true
    }
}
#[test]
fn scr_v17_seek_failure_aborts_fallback() {
    let (mut image, _) = image(32);
    let mut raw = FailedSeek { reads: 0, seeks: 0 };
    assert_eq!(image.load_record(&mut raw), Ok(false));
    assert_eq!((raw.reads, raw.seeks), (1, 1));
}
#[test]
fn scr_v18_bmp_conversion_pairs() {
    assert!(!bmp_translation(24, 8));
    assert!(bmp_translation(8, 32));
    for target in [16, 24, 32] {
        assert!(bmp_translation(24, target));
    }
    assert_eq!(bmp_stride(7, 4, true), Some(4));
    assert_eq!(bmp_stride(7, 4, false), Some(0));
}
fn tga(descriptor: u8, depth: u8, kind: u8, pixels: &[u8]) -> Cursor<Vec<u8>> {
    let mut h = vec![0u8; 18];
    h[2] = kind;
    h[12] = 1;
    h[14] = 2;
    h[16] = depth;
    h[17] = descriptor;
    h.extend(pixels);
    Cursor::new(h)
}
#[test]
fn scr_v19_tga_vertical_origin() {
    let (mut image, state) = image(32);
    assert_eq!(image.load_record(&mut tga(0, 24, 2, &[0, 0, 255, 255, 0, 0])), Ok(true));
    assert_eq!(state.lock().unwrap().pixels, [0xff00_00ff, 0xffff_0000]);
    assert_eq!(
        image.load_record(&mut tga(0x20, 24, 2, &[0, 0, 255, 255, 0, 0])),
        Ok(true)
    );
    assert_eq!(state.lock().unwrap().pixels, [0xffff_0000, 0xff00_00ff]);
    assert_eq!(state.lock().unwrap().flags, 0x8080);
}
#[test]
fn scr_v20_hex_and_key_labels() {
    assert_eq!(hex(0xab), "0x000000ab");
    assert_eq!(key(Key::new(1, 2, 3)), "G:0x00000002 I:0x00000003 T:0x00000001");
}

#[derive(Clone, Default)]
struct Sink(Arc<Mutex<String>>, Arc<Mutex<Vec<String>>>);
impl TextSink for Sink {
    fn flush(&mut self) {
        self.1.lock().unwrap().push("flush".into());
    }
    fn set_rz_char_str(&mut self, s: &[u8]) -> bool {
        self.0.lock().unwrap().push_str(std::str::from_utf8(s).unwrap());
        true
    }
}
struct Survey {
    graph: Graph,
    stream: RzTextOStream<Sink>,
    trace: Arc<Mutex<Vec<String>>>,
}
impl Surveyor for Survey {
    fn graph(&self) -> Graph {
        self.graph.clone()
    }
    fn stream(&mut self) -> &mut dyn GzOStream {
        &mut self.stream
    }
    fn release(&mut self) {
        self.trace.lock().unwrap().push("release-surveyor".into());
    }
}
struct Factory(Option<Survey>);
impl SurveyorFactory for Factory {
    fn create(&mut self, class: u32, iid: u32, _: u64) -> Option<Box<dyn Surveyor>> {
        assert_eq!((class, iid), (0xbb19_4c78, 0x9b19_4c66));
        self.0.take().map(|s| Box::new(s) as Box<dyn Surveyor>)
    }
}
struct Complete(Arc<Mutex<Vec<String>>>);
impl Dependency for Complete {
    fn complete(&self) {
        self.0.lock().unwrap().push("dependency-complete".into());
    }
}
type Body = dyn Fn(&mut AsciiWriter) -> sc::Result<()> + Send + Sync;
struct Object(Arc<Body>);
impl Serializable for Object {
    fn class(&self) -> u32 {
        0xab
    }
    fn write(&self, w: &mut AsciiWriter) -> sc::Result<()> {
        self.0(w)
    }
}
fn graph_write(
    body: impl Fn(&mut AsciiWriter) -> sc::Result<()> + Send + Sync + 'static,
    two: bool,
) -> (String, Vec<String>) {
    let sink = Sink::default();
    let root = ResourceInfo {
        identity: 1,
        name: "root".into(),
        key: Key::new(1, 2, 3),
    };
    let object = Arc::new(Object(Arc::new(body)));
    let mut objects = vec![ObjectInfo {
        identity: 11,
        index: 7,
        type_name: "First".into(),
        object: object.clone(),
    }];
    if two {
        objects.push(ObjectInfo {
            identity: 12,
            index: 8,
            type_name: "Second".into(),
            object,
        });
    }
    let graph = Graph {
        root,
        dependencies: vec![DependencyInfo {
            index: 4,
            resource: ResourceInfo {
                identity: 2,
                name: "dep".into(),
                key: Key::new(4, 5, 6),
            },
            completion: Arc::new(Complete(sink.1.clone())),
        }],
        objects,
    };
    let mut factory = Factory(Some(Survey {
        graph,
        stream: RzTextOStream::new(true, sink.clone()),
        trace: sink.1.clone(),
    }));
    AsciiWriter::default().write_root(1, &mut factory).unwrap();
    let output = sink.0.lock().unwrap().clone();
    let trace = sink.1.lock().unwrap().clone();
    (output, trace)
}
#[test]
fn scr_v21_ascii_graph_order() {
    let (s, trace) = graph_write(|w| w.scalar("n", Scalar::U32(9)), true);
    assert_eq!(
        s,
        concat!(
            "Resource root G:0x00000002 I:0x00000003 T:0x00000001\n",
            "   Dep. Resource #4 dep G:0x00000005 I:0x00000006 T:0x00000004\n",
            "   Object #7 First\n   Object #8 Second\n\n",
            "   Object #7 First\n   {\n      Uint32 n = 9\n   }\n\n",
            "   Object #8 Second\n   {\n      Uint32 n = 9\n   }\n\n\n"
        )
    );
    assert_eq!(trace, ["flush", "dependency-complete", "release-surveyor"]);
}
#[test]
fn scr_v22_unclamped_color_alpha() {
    assert_eq!(color_alpha(b"2 -1 0.5").unwrap(), [2., -1., 0.5, 1.]);
    assert!(color(b"1 2").is_err());
}
#[test]
fn scr_v23_missing_vector_coordinates() {
    assert_eq!(vector3(b"7").unwrap(), [7., 0., 0.]);
    assert_eq!(vector2(b"").unwrap(), [0., 0.]);
    assert!(vector2(b"1 2 3").is_err());
}
#[test]
fn scr_v24_enum_precedence() {
    assert_eq!(parse_enum(b"name", &[(b"name", 1)], &[(b"name", 2)]), Ok(1));
    assert!(matches!(
        parse_enum(b"unknown", &[], &[]),
        Err(sc::Error::Parse { code: 4, .. })
    ));
}
#[test]
fn scr_v25_file_messages() {
    let mut p = FileParser::default();
    assert!(p.message(0x6ae7_6406));
    assert!(p.modified());
    assert!(!p.message(42));
    assert!(p.modified());
}
#[test]
fn scr_v26_final_physical_byte_excluded() {
    let mut s = serial(b"TKT1ab");
    s.raw_mut().seek(4);
    let mut io = TextIo::new(&mut s);
    let mut b = 0;
    assert!(io.character(&mut b));
    assert_eq!(b, b'a');
    assert!(!io.character(&mut b));
    assert_eq!(io.record.raw().position(), 5);
    assert_eq!(io.counter, 6);
}
#[test]
fn scr_v27_descriptor_count_must_match() {
    let mut t: TokenRecord<Cursor<Vec<u8>>> = TokenRecord::default();
    t.set(23, Values::U32(vec![42])).unwrap();
    assert!(t
        .field(Descriptor {
            id: 23,
            ty: 3,
            count: 2
        })
        .is_none());
    assert_eq!(
        t.create_field(Descriptor {
            id: 23,
            ty: 3,
            count: 1
        }),
        Ok(false)
    );
}
#[test]
fn scr_v28_ascii_bool_array() {
    let (s, _) = graph_write(
        |w| w.array("flags", Scalar::Bool(false), &[Scalar::Bool(true), Scalar::Bool(false)]),
        false,
    );
    assert!(s.contains("      bool flags[2] = true, false, \n"));
}
#[test]
fn scr_v29_ascii_empty_array() {
    let (s, _) = graph_write(|w| w.array("flags", Scalar::Bool(false), &[]), false);
    assert!(s.contains("      bool flags[0] = \n"));
}
#[test]
fn scr_v30_ascii_null_inline_object() {
    let (s, _) = graph_write(|w| w.object("ignored", None), false);
    assert!(s.contains("      Object GZCLSID: 0x00000000\n   }"));
    assert!(!s.contains("ignored"));
}

#[test]
fn file_required_optional_order_and_hooks() {
    let mut files = MemoryFiles::default();
    files.files.insert("bad".into(), vec![b"unknown".to_vec()]);
    let mut p = FileParser::default();
    p.set_parser(Some(Arc::new(Mutex::new(parser()))));
    p.add_input("missing", 1, &mut files);
    p.add_input("bad", 2, &mut files);
    let mut hooks = Hooks::default();
    assert_eq!(p.parse_all(&mut files, &mut hooks), Ok(false));
    assert_eq!(hooks.0, ["begin", "finish"]);
    assert_eq!(p.errors().len(), 2);
    assert_eq!(files.trace, ["watch:bad", "open:missing", "open:bad"]);
    p.shutdown();
    assert_eq!(*files.stopped.lock().unwrap(), [9]);
    let mut p = FileParser::default();
    p.set_parser(Some(Arc::new(Mutex::new(parser()))));
    p.add_input("bad", 0, &mut files);
    p.add_input("missing", 0, &mut files);
    let mut hooks = Hooks::default();
    assert_eq!(p.parse_all(&mut files, &mut hooks), Ok(true));
    assert_eq!(p.errors().len(), 1);
    assert_eq!(hooks.0, ["begin", "end", "finish"]);
}
#[test]
fn include_suppresses_only_open_error() {
    let mut files = MemoryFiles::default();
    files.files.insert("root".into(), vec![b"sinclude /absent".to_vec()]);
    let mut p = FileParser::default();
    p.set_parser(Some(Arc::new(Mutex::new(parser()))));
    p.add_input("root", 1, &mut files);
    assert_eq!(p.parse_all(&mut files, &mut Hooks::default()), Ok(true));
    files.files.insert("/absent".into(), vec![b"unknown".to_vec()]);
    assert_eq!(p.parse_all(&mut files, &mut Hooks::default()), Ok(false));
}
#[test]
fn macro_substitution_and_nested_switch_validation() {
    let trace = Arc::new(Mutex::new(Vec::new()));
    let mut p = parser();
    let sink = trace.clone();
    p.register(
        b"out",
        Arc::new(move |_: &mut CommandParser, t: &mut [sc::ScriptToken]| {
            sink.lock().unwrap().push(t[1].bytes.clone());
            Ok(())
        }),
    );
    p.parse(b"define m [x]").unwrap();
    p.parse(b"out %x").unwrap();
    p.parse(b"enddef").unwrap();
    p.parse(b"create m [value]").unwrap();
    assert_eq!(*trace.lock().unwrap(), [b"value".to_vec()]);
    p.set_variable(b"command", b"out");
    p.set_variable(b"v", b"expanded");
    p.parse(b"$command ${v}").unwrap();
    assert_eq!(trace.lock().unwrap()[1], b"expanded");
    let mut block = BTreeMap::new();
    block.insert(
        b"out".to_vec(),
        Arc::new(|_: &mut CommandParser, _: &mut [sc::ScriptToken]| Ok(())) as Arc<dyn Command>,
    );
    p.push_block(block);
    assert!(p.parse(b"out -unused").is_err());
    assert!(p.pop_block());
}
#[test]
fn text_comments_narrowing_and_all_value_writers() {
    let mut s = text_record(b"ignored; comment\r\n[23, 65539, 65537]= \t hello, ");
    let mut io = TextIo::new(&mut s);
    let mut d = END;
    assert!(io.read_definition(&mut d));
    assert_eq!(
        d,
        Descriptor {
            id: 23,
            ty: 3,
            count: 1
        }
    );
    let mut text = Vec::new();
    assert!(io.read_string(&mut text));
    assert_eq!(text, b"hello");
    let mut s = serial(b"TKT1");
    s.raw_mut().seek(4);
    let mut io = TextIo::new(&mut s);
    assert!(io.write_values(&Values::I8(vec![-1])));
    assert!(io.write_values(&Values::U8(vec![255])));
    assert!(io.write_values(&Values::I16(vec![-2])));
    assert!(io.write_values(&Values::U16(vec![65535])));
    assert!(io.write_values(&Values::I32(vec![-3])));
    assert!(io.write_values(&Values::U32(vec![4])));
    assert!(io.write_values(&Values::I64(vec![i64::MIN])));
    assert!(io.write_values(&Values::U64(vec![u64::MAX])));
    assert!(io.write_values(&Values::F32(vec![1.5])));
    assert!(io.write_values(&Values::F64(vec![2.5])));
    assert!(io.write_values(&Values::String(vec![b"raw".to_vec()])));
    assert!(io.write_values(&Values::Key(vec![Key::new(1, 2, 3)])));
    assert_eq!(
        fragments(&s),
        [
            "-1, ",
            "255, ",
            "-2, ",
            "65535, ",
            "-3, ",
            "4, ",
            "-9223372036854775808, ",
            "18446744073709551615, ",
            "1.500000, ",
            "2.5, ",
            "raw, ",
            "1, ",
            "2, ",
            "3, "
        ]
        .map(|s| s.as_bytes().to_vec())
    );
}
#[test]
fn tga_rle_alpha_and_unsupported_depth() {
    let (mut image, state) = image(32);
    assert_eq!(
        image.load_record(&mut tga(0x28, 32, 10, &[0x81, 1, 2, 3, 128])),
        Ok(true)
    );
    assert_eq!(state.lock().unwrap().pixels, [0x8003_0201; 2]);
    assert_eq!(image.load_file_tga(&mut tga(0x21, 16, 2, &[0, 0x80, 31, 0])), Ok(true));
    assert_eq!(state.lock().unwrap().pixels, [0xff00_0000, 0x0000_00ff]);
    assert_eq!(state.lock().unwrap().flags, 0x8040);
    assert_eq!(image.load_record(&mut tga(0, 8, 2, &[])), Ok(true));
    let state = state.lock().unwrap();
    assert_eq!(
        state.trace.iter().filter(|s| *s == "lock").count(),
        state.trace.iter().filter(|s| *s == "unlock").count()
    );
}
#[test]
fn ascii_references_identity_and_invalid_context() {
    assert!(AsciiWriter::default().scalar("bad", Scalar::Bool(true)).is_err());
    let (s, _) = graph_write(
        |w| {
            w.reference("none", None)?;
            w.reference(
                "local",
                Some(&Referent {
                    resource_identity: 1,
                    object_identity: 11,
                    id: 99,
                    type_name: "Local".into(),
                    class: 0xab,
                }),
            )?;
            w.reference(
                "external",
                Some(&Referent {
                    resource_identity: 2,
                    object_identity: 11,
                    id: 99,
                    type_name: "External".into(),
                    class: 0xab,
                }),
            )?;
            w.type_info("Type", 0xab, 2)
        },
        false,
    );
    assert!(s.contains("      cIGZPersistSerializableReferent* none = 0\n"));
    assert!(s.contains("local = Object #7 Local GZCLSID: 0x000000ab\n"));
    assert!(s.contains("external = Object ID 99 External in dep. resource #4 G:0x00000005 I:0x00000006 T:0x00000004\n"));
    assert!(s.contains("TypeName: Type GZCLSID: 0x000000ab Version: 2\n"));
}

fn bmp(depth: u16, width: u32, height: u32, compression: u32, palette: &[u8], pixels: &[u8]) -> Cursor<Vec<u8>> {
    let offset = 54 + palette.len();
    let mut bytes = vec![0; 54];
    bytes[..2].copy_from_slice(b"BM");
    bytes[10..14].copy_from_slice(&(offset as u32).to_le_bytes());
    bytes[14..18].copy_from_slice(&40u32.to_le_bytes());
    bytes[18..22].copy_from_slice(&width.to_le_bytes());
    bytes[22..26].copy_from_slice(&height.to_le_bytes());
    bytes[26..28].copy_from_slice(&1u16.to_le_bytes());
    bytes[28..30].copy_from_slice(&depth.to_le_bytes());
    bytes[30..34].copy_from_slice(&compression.to_le_bytes());
    bytes[46..50].copy_from_slice(&((palette.len() / 4) as u32).to_le_bytes());
    bytes.extend(palette);
    bytes.extend(pixels);
    Cursor::new(bytes)
}
#[test]
fn bmp_file_palette_raw_and_rle8_escapes() {
    let (mut image, state) = image(32);
    assert_eq!(
        image.load_file_bmp(&mut bmp(24, 1, 2, 0, &[], &[255, 0, 0, 0, 0, 0, 255, 0])),
        Ok(true)
    );
    assert_eq!(state.lock().unwrap().pixels, [0xffff_0000, 0xff00_00ff]);
    assert_eq!(state.lock().unwrap().flags, 0x8040);
    let palette = [0, 0, 0, 0, 0, 0, 255, 0];
    assert_eq!(
        image.load_file_bmp(&mut bmp(8, 3, 2, 1, &palette, &[3, 1, 0, 0, 0, 3, 1, 0, 1, 0, 0, 1])),
        Ok(true)
    );
    assert_eq!(
        state.lock().unwrap().pixels,
        [
            0xffff_0000,
            0xff00_0000,
            0xffff_0000,
            0xffff_0000,
            0xffff_0000,
            0xffff_0000
        ]
    );
    assert_eq!(
        image.load_file_bmp(&mut bmp(8, 3, 1, 1, &palette, &[0, 2, 1, 0, 2, 1, 0, 1])),
        Ok(true)
    );
    assert_eq!(state.lock().unwrap().pixels, [0xff00_0000, 0xffff_0000, 0xffff_0000]);
    assert!(matches!(
        image.load_record(&mut bmp(24, 1, 1, 0, &[], &[0; 4])),
        Err(sc::Error::Unsupported("SCR5: record BMP offset handling"))
    ));
}
#[test]
fn bmp_four_bit_nibbles_and_dither_boundary() {
    let (mut image, state) = image(16);
    let palette = [0, 0, 0, 0, 0, 0, 255, 0];
    assert_eq!(
        image.load_file_bmp(&mut bmp(4, 3, 1, 0, &palette, &[0x10, 0x10, 0, 0])),
        Ok(true)
    );
    assert_eq!(state.lock().unwrap().pixels, [0xffff_0000, 0xff00_0000, 0xffff_0000]);
    image.target = Format::new(7, 16);
    assert!(matches!(
        image.load_file_tga(&mut tga(0, 24, 2, &[0; 6])),
        Err(sc::Error::Unsupported("SCR5: 4444 dithering"))
    ));
}
#[test]
fn explicit_record_bmp_policy_preserves_record_flags() {
    let (mut image, state) = image(32);
    let mut bytes = bmp(24, 1, 1, 0, &[], &[1, 2, 3, 0]);
    assert_eq!(
        image.load_record_bmp_with_policy(&mut bytes, RecordBmpOffset::Sequential),
        Ok(true)
    );
    let state = state.lock().unwrap();
    assert_eq!(state.flags, 0x8080);
    assert_eq!(state.pixels, [0xff03_0201]);
}
struct Agg(Arc<Mutex<Vec<String>>>);
impl Aggregator for Agg {
    fn set_outer(&mut self, attached: bool) {
        self.0.lock().unwrap().push(format!("outer:{attached}"));
    }
    fn query_interface(&mut self, iid: u32) -> bool {
        self.0.lock().unwrap().push(format!("qi:{iid}"));
        iid == 999
    }
    fn release(&mut self) {
        self.0.lock().unwrap().push("release-aggregator".into());
    }
}
struct AggregatedProvider {
    buffer: BufferHandle,
    trace: Arc<Mutex<Vec<String>>>,
}
impl BufferProvider for AggregatedProvider {
    fn create(&mut self, _: u32, _: u32, _: u32) -> Option<CreatedBuffer> {
        Some(CreatedBuffer {
            buffer: self.buffer.clone(),
            aggregator: Some(Arc::new(Mutex::new(Agg(self.trace.clone())))),
        })
    }
}
#[test]
fn aggregated_image_identity_and_destruction() {
    let (mut original, _) = image(32);
    let buffer = original.buffer().unwrap();
    let trace = Arc::new(Mutex::new(Vec::new()));
    let mut p = AggregatedProvider {
        buffer,
        trace: trace.clone(),
    };
    let mut r = PersistBufferResource::new(Format::new(8, 32), 5, &mut p);
    assert_eq!(
        r.query_interface(IMAGE_RESOURCE_IID),
        Some(ImageInterface::ImageResource)
    );
    assert_eq!(r.query_interface(999), Some(ImageInterface::Aggregator));
    assert_eq!(r.release(), 0);
    assert_eq!(
        *trace.lock().unwrap(),
        ["outer:true", "qi:999", "outer:false", "release-aggregator"]
    );
    original.set_buffer(None);
}
#[test]
fn ascii_all_scalar_arrays_strings_and_inline_object() {
    let (s, _) = graph_write(
        |w| {
            let values = [
                Scalar::Bool(true),
                Scalar::I8(-1),
                Scalar::U8(2),
                Scalar::I16(-3),
                Scalar::U16(4),
                Scalar::I32(-5),
                Scalar::U32(6),
                Scalar::F32(1.5),
                Scalar::F64(2.5),
            ];
            for value in values {
                w.scalar("x", value)?;
                w.array("a", value, &[value])?;
            }
            w.string("c", b"raw", false)?;
            w.string("s", b"text", true)?;
            w.object(
                "unused",
                Some(&Object(Arc::new(|w| w.scalar("inside", Scalar::Bool(false))))),
            )
        },
        false,
    );
    for line in [
        "bool x = true",
        "Sint8 x = -1",
        "Uint8 x = 2",
        "Sint16 x = -3",
        "Uint16 x = 4",
        "Sint32 x = -5",
        "Uint32 x = 6",
        "Float32 x = 1.5",
        "Float64 x = 2.5",
        "RZChar* c = raw",
        "RZString s = text",
    ] {
        assert!(s.contains(&format!("      {line}\n")), "{line}");
    }
    assert!(s.contains("      Object GZCLSID: 0x000000ab\n      {\n      bool inside = false\n      }\n\n"));
}
#[test]
fn token_read_failure_cleans_attachment_and_fields() {
    let mut s = text_record(b"[23,3,1]= 42, [24,3,1]= bad, ");
    s.raw_mut().seek(0);
    let s = Arc::new(Mutex::new(s));
    let mut t = TokenRecord::default();
    assert_eq!(t.init_read(s.clone(), None), Ok(false));
    assert!(!t.available(23));
    assert_eq!(t.cookie(), 0);
    assert!(t.key().is_none());
    assert_eq!(s.lock().unwrap().record.reference_count(), 1);
    assert!(t
        .create_field(Descriptor {
            id: 23,
            ty: 3,
            count: 0
        })
        .is_err());
    t.set(23, Values::U32(vec![1])).unwrap();
    assert!(matches!(
        t.create_field(Descriptor {
            id: 23,
            ty: 4,
            count: 1
        }),
        Err(sc::Error::Unsupported(_))
    ));
    assert!(!t.available(23));
}

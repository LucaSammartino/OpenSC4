// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::cell::RefCell;

#[test]
fn gzc39_descending_stable_order() {
    let mut f = framework::Framework::default();
    let mut h = Spy::default();
    for (id, priority) in [(1, -1), (2, 7), (3, 7), (4, 0)] {
        assert_eq!(f.add_ordered(&mut h, id, priority, id as Token), Ok(true));
    }
    assert_eq!(f.ordered().map(|e| e.0).collect::<Vec<_>>(), [2, 3, 4, 1]);
    assert_eq!(f.add_ordered(&mut h, 3, 7, 99), Ok(false));
    // Literal traversal stops at insertion; a duplicate below that point is unseen.
    assert_eq!(f.add_ordered(&mut h, 1, 8, 99), Ok(true));
}

#[test]
fn gzc40_44_buffer_identity_key_and_lifetime() {
    use framework::Face::*;
    let mut b = buffer::Buffer::default();
    let mut out = None;
    for (iid, expected) in [(1, Primary), (0x20732180, Primary), (0x86D72B57, Secondary)] {
        assert!(b.query_interface(iid, &mut out));
        assert_eq!(out, Some(expected));
    }
    assert!(!b.query_interface(99, &mut out));
    assert_eq!(out, Some(Secondary));
    assert_eq!([b.release(), b.release(), b.release()], [2, 1, 0]);
    assert!(b.destroyed);
    let mut b = buffer::Buffer::default();
    let mut key = 0x12345678;
    assert_eq!(b.colour_key(&mut key), Ok(false));
    assert_eq!(key, 0x12345678);
    b.attach_colour_key(0x89ABCDEF, true);
    assert_eq!(b.colour_key(&mut key), Ok(true));
    assert_eq!(key, 0x89ABCDEF);
    assert_eq!([b.add_ref(), b.add_ref(), b.release(), b.release()], [1, 2, 1, 0]);
    assert!(b.destroyed);
    let mut b = buffer::Buffer::default();
    assert_eq!(b.release(), u32::MAX);
    assert!(!b.destroyed);
}

#[test]
fn gzc45_46_buffer_unlock_mask_and_underflow() {
    let mut b = buffer::Buffer::default();
    b.init(&mut Alloc, 1, 1, 2, 8).unwrap();
    assert!(b.lock(0x8040));
    assert!(b.lock(0x0800));
    assert!(b.unlock(0x0800));
    assert_eq!((b.lock_flags, b.nesting), (0x8040, 1));
    assert!(b.unlock(0));
    assert_eq!((b.lock_flags, b.nesting, b.modifications), (0x8040, 0, 1));
    let mut b = buffer::Buffer::default();
    assert!(b.unlock(u32::MAX));
    assert_eq!((b.lock_flags, b.nesting), (0, u16::MAX));
    assert!(b.pixels().is_none());
}

#[test]
fn gzc47_49_support_final_release_is_not_shutdown() {
    for kind in [support::Kind::Misc, support::Kind::Tool] {
        let mut s = support::Support::new(kind);
        let mut out = Some(99);
        assert!(s.query_interface(0x452294AA, 42, &mut out));
        assert_eq!(out, Some(42));
        assert!(!s.query_interface(1, 42, &mut out));
        assert_eq!(out, Some(42));
        assert_eq!(s.add_ref(), 2);
        let mut h = Spy {
            services: true,
            ..Default::default()
        };
        s.init(&mut h, 42);
        h.events.clear();
        assert_eq!(s.release(&mut h, 42), 1);
        assert!(!s.destroyed());
        assert!(h.events.is_empty());
        assert_eq!(s.release(&mut h, 42), 0);
        assert!(s.destroyed());
        assert!(s.notifications().is_empty());
        if kind == support::Kind::Misc {
            assert_eq!(h.events, ["-20", "-21"]);
        } else {
            assert!(h.events.is_empty());
        }
        let mut fresh = support::Support::new(kind);
        assert_eq!(fresh.release(&mut h, 1), 0);
        assert!(fresh.destroyed());
    }
}

#[test]
fn gzc50_logger_init_thread_owns_delivery() {
    let log = Arc::new(logging::Log::default());
    let b = backend(true, false);
    assert!(log.add_backend("B", b.clone()));
    let (send, recv) = std::sync::mpsc::channel();
    let (done, go) = std::sync::mpsc::channel();
    let other = log.clone();
    let owner = std::thread::spawn(move || {
        other.attach_initialized_host();
        send.send(()).unwrap();
        go.recv().unwrap();
        other.on_tick();
    });
    recv.recv().unwrap();
    log.attach_initialized_host(); // Does not rebind to construction thread A.
    log.output("C", 1, "deferred");
    assert!(b.events.lock().unwrap().is_empty());
    done.send(()).unwrap();
    owner.join().unwrap();
    assert_eq!(*b.events.lock().unwrap(), ["filter", "deferred"]);
    log.shutdown(&mut LogLife(Vec::new()));
    log.attach_initialized_host(); // A later Init can bind A.
    assert!(log.add_backend("new", b.clone()));
    log.output("C", 1, "immediate");
    assert_eq!(b.events.lock().unwrap().last().unwrap(), "immediate");
}

#[test]
fn gzc51_failed_setters_preserve_valid_cache() {
    let mut f = filesystem::FileSystem::default();
    let mut h = Spy::default();
    f.data_directory(&mut h);
    f.plugin_directory(&mut h);
    h.events.clear();
    assert!(!f.set_data_directory(&mut h, "missing-data"));
    assert!(!f.set_plugin_directory(&mut h, "missing-plugin"));
    let before = h.events.clone();
    assert_eq!(f.data_directory(&mut h), "missing-data");
    assert_eq!(f.plugin_directory(&mut h), "missing-plugin");
    assert_eq!(h.events, before);
}

#[derive(Default)]
struct StreamHost {
    refs: Spy,
    next: Token,
    fail_allocate: bool,
    reject: bool,
    failure_write: Option<Token>,
}
impl References for StreamHost {
    fn add_ref(&mut self, t: Token) {
        self.refs.add_ref(t);
    }
    fn release(&mut self, t: Token) {
        self.refs.release(t);
    }
    fn query(&mut self, t: Token, iid: u32) -> Option<Token> {
        self.refs.query(t, iid)
    }
}
impl framework::Streams for StreamHost {
    fn create_null(&mut self, input: bool) -> Option<Token> {
        self.refs.events.push(format!("null:{input}"));
        if self.fail_allocate {
            return None;
        }
        self.next += 1;
        self.refs.add_ref(self.next);
        Some(self.next)
    }
    fn query_into(&mut self, t: Token, iid: u32, out: &mut Option<Token>) -> bool {
        if self.reject {
            self.refs.events.push(format!("reject:{t}:{iid:x}"));
            if let Some(t) = self.failure_write {
                *out = Some(t);
            }
            return false;
        }
        *out = self.refs.query(t, iid);
        true
    }
}

#[test]
fn gzc52_borrowed_framework_accessors() {
    let mut f = framework::Framework::default();
    assert_eq!(f.helper(1000), f.helper(1000));
    assert_eq!(f.com(1000), 1008);
    assert_eq!(f.helper(1000), 1232);
    f.set_application(Some(7));
    assert_eq!(f.application(), Some(7));
    f.set_application(None);
    assert_eq!(f.application(), None);
    assert_eq!(f.unnamed_flag, 0);
}

#[test]
fn gzc53_57_lazy_streams_replacement_and_failure_writes() {
    let mut f = framework::Framework::default();
    let mut h = StreamHost::default();
    for selector in 0..3 {
        let a = f.stream(&mut h, selector).unwrap();
        assert_eq!(f.stream(&mut h, selector), Some(a));
        assert_eq!(h.refs.counts[&a], 1);
    }
    assert!(f.set_stream(&mut h, 0, Some(50)));
    assert_eq!(h.refs.events.last().unwrap(), "+50");
    assert!(h.refs.events.contains(&"qi:50:c51b11cb".into()));
    let mut out = Some(99);
    assert!(f.query_stream(&mut h, 0, 7, Some(&mut out)));
    assert_eq!(out, Some(50));
    assert_eq!(h.refs.counts[&50], 2);
    h.reject = true;
    h.refs.events.clear();
    assert!(!f.set_stream(&mut h, 2, Some(51)));
    assert_eq!(h.refs.events, ["-3", "reject:51:51b11bd"]);
    assert_eq!(f.stream(&mut h, 2), Some(4));
    let before = h.refs.events.clone();
    assert!(!f.query_stream(&mut h, 3, 7, Some(&mut out)));
    assert!(!f.query_stream(&mut h, 0, 7, None));
    assert!(!f.set_stream(&mut h, 3, Some(51)));
    assert_eq!(h.refs.events, before);
    assert_eq!(out, Some(50));
    assert!(f.set_stream(&mut h, 1, None));
    assert_eq!(f.stream(&mut h, 1), Some(5));
    let mut f = framework::Framework::default();
    h.fail_allocate = true;
    assert_eq!(f.stream(&mut h, 1), None);
    h.fail_allocate = false;
    assert_eq!(f.stream(&mut h, 1), Some(6));
    h.failure_write = Some(88);
    assert!(!f.set_stream(&mut h, 0, Some(51)));
    assert_eq!(f.stream(&mut h, 0), Some(88));
    out = Some(99);
    assert!(!f.query_stream(&mut h, 0, 7, Some(&mut out)));
    assert_eq!(out, Some(88)); // Delegate failure writes are inherited too.
}

#[test]
fn gzc58_propagated_debug_is_separate_and_ignores_write_failure() {
    let mut f = framework::Framework::default();
    f.set_debug_level(7);
    let mut calls = Vec::new();
    f.propagate_debug_level(-2, |name, value| {
        calls.push((name.to_owned(), value.to_owned()));
        false
    });
    assert_eq!(f.debug_level(), 7);
    assert_eq!(calls, [("GZFRAMEWORK_DEBUG_LEVEL".into(), "-2".into())]);
}

#[test]
fn gzc59_62_com_running_and_enumeration() {
    let mut c = com::Com::default();
    let mut h = Spy::default();
    c.set_service_running(2);
    assert_eq!(c.service_running(), 2);
    c.set_service_running(0);
    assert_eq!(c.service_running(), 0);
    assert!(!c.initialized);
    for (name, id) in [("C", 30), ("A", 10), ("B", 20)] {
        h.library_ids.insert(name.into(), id);
        assert!(c.add_library(&mut h, 1, name));
    }
    h.events.clear();
    let mut count = 999;
    assert_eq!(c.library_array(&mut h, None, &mut count), 3);
    assert!(h.events.is_empty());
    let mut array = [99; 5];
    count = 2;
    assert_eq!(c.library_array(&mut h, Some(&mut array), &mut count), 2);
    assert_eq!(array, [10, 20, 99, 99, 99]);
    assert_eq!(h.events, ["+10", "+20"]);
    h.events.clear();
    count = 0;
    assert_eq!(c.library_array(&mut h, Some(&mut array), &mut count), 0);
    assert!(h.events.is_empty());
    count = 5;
    assert_eq!(c.library_array(&mut h, Some(&mut array), &mut count), 3);
    assert_eq!(array, [10, 20, 30, 99, 99]);
    assert_eq!(h.events, ["+10", "+20", "+30"]);
    c.enumerate_libraries(77, None);
    let mut visits = Vec::new();
    c.enumerate_libraries(
        77,
        Some(&mut |lib, context| {
            visits.push((lib, context));
            false
        }),
    );
    assert_eq!(visits, [(10, 77)]);
    assert_eq!(h.events, ["+10", "+20", "+30"]);
}

type Registration = (u32, Vec<u8>, Vec<u8>);
#[derive(Default)]
struct GeneratorSpy(RefCell<Vec<Registration>>);
impl commands::server::Generator for GeneratorSpy {
    fn registered(&self, id: u32, name: &[u8], info: &[u8]) {
        self.0.borrow_mut().push((id, name.to_vec(), info.to_vec()));
    }
    fn unregistered(&self, _: u32, _: &[u8], _: &[u8]) {
        panic!("no removal replay");
    }
    fn parse_text(&self, _: &[u8], _: u32, _: &mut u32, _: &mut Option<commands::SetRef>) -> bool {
        false
    }
    fn to_text(&self, _: u32, _: Option<&commands::SetRef>, _: &mut Vec<u8>) -> bool {
        false
    }
    fn output_text(&self, _: Option<&commands::SetRef>, _: &mut Vec<u8>) -> bool {
        false
    }
    fn shutdown(&self) {
        panic!("no public Shutdown on destruction");
    }
}

#[test]
fn gzc63_64_command_faces_and_destructor() {
    use commands::server::{Face::*, Server};
    let s = Server::new(Rc::new(Convert));
    let mut out = None;
    for (iid, face) in [
        (1, Primary),
        (0xCB90253A, Primary),
        (0xCB90253B, Primary),
        (0x287FB697, Service),
    ] {
        assert!(s.query_interface(Primary, iid, &mut out));
        assert_eq!(out, Some(face));
    }
    assert!(!s.query_interface(Primary, 999, &mut out));
    assert_eq!(out, Some(Service));
    assert!(s.query_interface(Service, 1, &mut out));
    assert_eq!(out, Some(Lifetime));
    assert_eq!(
        [s.release(), s.release(), s.release(), s.release(), s.release()],
        [4, 3, 2, 1, 0]
    );
    assert!(s.destroyed());
    let s = Server::new(Rc::new(Convert));
    let g = Rc::new(GeneratorSpy::default());
    assert!(s.register(1, Some(b"X"), None));
    assert!(s.register_generator(g.clone()));
    let dispatcher = Rc::new(TestDispatcher);
    s.set_dispatcher(Some(dispatcher.clone()));
    assert_eq!(Rc::strong_count(&dispatcher), 2);
    assert_eq!(s.release(), 0);
    assert!(s.destroyed());
    assert_eq!(s.command_count(), 0);
    assert_eq!(Rc::strong_count(&g), 1);
    assert_eq!(Rc::strong_count(&dispatcher), 1);
}
struct TestDispatcher;
impl commands::server::Dispatcher for TestDispatcher {
    fn execute(&self, _: u32, _: Option<&commands::SetRef>, _: Option<&commands::SetRef>) -> i32 {
        19
    }
}

struct Printf {
    sizes: Vec<usize>,
    statuses: std::collections::VecDeque<i32>,
}
impl Printf {
    fn new() -> Self {
        Self {
            sizes: Vec::new(),
            statuses: Default::default(),
        }
    }
}
impl formatting::Formatter for Printf {
    fn format(&mut self, format: &[u8], words: &[u32], buffer: &mut [u8]) -> i32 {
        self.sizes.push(buffer.len());
        let format = std::str::from_utf8(format).unwrap();
        let text = if format.contains("%u") {
            format.replace("%u", &words[0].to_string())
        } else if format.contains("%d") {
            format.replace("%d", &(words[0] as i32).to_string())
        } else {
            format.to_owned()
        };
        let n = text.len().min(buffer.len().saturating_sub(1));
        buffer[..n].copy_from_slice(&text.as_bytes()[..n]);
        buffer[n] = 0;
        self.statuses.pop_front().unwrap_or(text.len() as i32)
    }
    fn decode(&self, bytes: &[u8]) -> String {
        String::from_utf8(bytes.to_vec()).unwrap()
    }
}

#[test]
fn gzc65_66_formatted_command_slot19_bridge() {
    let mut f = Printf::new();
    assert_eq!(
        formatting::command(&mut f, b"%u", &[12], |text, n, word| {
            assert_eq!(&text[..n as usize], b"12");
            assert_eq!((n, word), (2, 12));
            37
        }),
        37
    );
    assert_eq!(f.sizes, [511]);
    assert_eq!(formatting::command(&mut f, b"", &[], |_, _, _| panic!("empty")), 2);
    f.statuses.push_back(-1);
    assert_eq!(
        formatting::command(&mut f, b"overflow", &[], |_, _, _| panic!("negative")),
        2
    );
    let s = commands::server::Server::new(Rc::new(Convert));
    assert_eq!(s.execute_formatted(&mut f, b"%u", &[12], |_| None), Ok(2));
}

#[test]
fn gzc67_68_generator_replay_identity_and_null() {
    let s = commands::server::Server::new(Rc::new(Convert));
    assert!(s.register_null_generator());
    assert_eq!(s.generator_count(), 1);
    assert!(s.unregister_null_generator());
    assert!(s.register(9, Some(b"nine"), Some(b"p9")));
    assert!(s.register(2, Some(b"two"), Some(b"p2")));
    let g = Rc::new(GeneratorSpy::default());
    let erased: Rc<dyn commands::server::Generator> = g.clone();
    assert!(s.register_generator(erased.clone()));
    assert!(s.register_generator(erased.clone()));
    assert_eq!(
        g.0.borrow().as_slice(),
        [
            (2, b"two".to_vec(), b"p2".to_vec()),
            (9, b"nine".to_vec(), b"p9".to_vec())
        ]
    );
    assert_eq!(Rc::strong_count(&g), 3); // g + erased + one retained reference.
    assert!(s.unregister_generator(&erased));
    assert!(s.unregister_generator(&erased));
    assert_eq!(Rc::strong_count(&g), 2);
    assert_eq!(g.0.borrow().len(), 2);
}

#[derive(Default)]
struct Sink(Mutex<Vec<String>>);
impl logging::Stream for Sink {
    fn write(&self, text: &str) {
        self.0.lock().unwrap().push(text.to_owned());
    }
}
struct StreamBack {
    stream: Option<Arc<dyn logging::Stream>>,
    events: Arc<Mutex<Vec<logging::Record>>>,
    dump_stream: Arc<Mutex<Option<usize>>>,
}
impl logging::Backend for StreamBack {
    fn accepts(&self, _: &str, _: i32) -> bool {
        true
    }
    fn output(&self, _: &logging::Log, record: &logging::Record) {
        self.events.lock().unwrap().push(record.clone());
        if let Some(stream) = &self.stream {
            stream.write(&record.text);
        }
    }
    fn set_level(&self, _: Option<&str>, _: i32) {}
    fn dump(&self, stream: &dyn logging::Stream) {
        *self.dump_stream.lock().unwrap() = Some(stream as *const dyn logging::Stream as *const () as usize);
        stream.write("backend levels\n");
    }
}
#[derive(Default)]
struct Factory {
    fail: bool,
    names: Vec<(String, u8)>,
    events: Arc<Mutex<Vec<logging::Record>>>,
    dump_stream: Arc<Mutex<Option<usize>>>,
}
impl logging::BackendFactory for Factory {
    fn create(
        &mut self,
        name: &str,
        stream: Option<Arc<dyn logging::Stream>>,
        timestamp: u8,
    ) -> Option<Arc<dyn logging::Backend>> {
        if self.fail {
            return None;
        }
        self.names.push((name.into(), timestamp));
        Some(Arc::new(StreamBack {
            stream,
            events: self.events.clone(),
            dump_stream: self.dump_stream.clone(),
        }))
    }
}

#[test]
fn gzc69_70_output_stream_allocation_and_duplicate_ownership() {
    let log = logging::Log::default();
    let original = backend(true, false);
    let mut output: Option<Arc<dyn logging::Backend>> = Some(original.clone());
    let mut factory = Factory {
        fail: true,
        ..Default::default()
    };
    assert!(!log.add_output_stream(&mut factory, "X", None, 7, Some(&mut output)));
    assert!(Arc::ptr_eq(
        output.as_ref().unwrap(),
        &(original.clone() as Arc<dyn logging::Backend>)
    ));
    assert!(log.backend("X").is_none());
    assert!(log.add_backend("X", original.clone()));
    factory.fail = false;
    let stream = Arc::new(Sink::default());
    assert!(log.add_output_stream(&mut factory, "X", Some(stream.clone()), 0, Some(&mut output)));
    assert_eq!(factory.names, [("X".into(), 0)]);
    assert_eq!(Arc::strong_count(&stream), 2);
    assert_eq!(Arc::strong_count(output.as_ref().unwrap()), 1);
    assert!(Arc::ptr_eq(
        &log.backend("X").unwrap(),
        &(original.clone() as Arc<dyn logging::Backend>)
    ));
    drop(output);
    assert_eq!(Arc::strong_count(&stream), 1);
    assert!(log.add_output_stream(&mut factory, "X", Some(stream.clone()), 255, None));
    assert_eq!(Arc::strong_count(&stream), 1); // Duplicate without out destroys new backend.
    assert!(log.add_output_stream(&mut factory, "Y", Some(stream.clone()), 2, None));
    assert_eq!(Arc::strong_count(&stream), 2); // Route owns the new backend.
    log.remove_all_backends();
    assert_eq!(Arc::strong_count(&stream), 1);
}

#[test]
fn gzc71_73_public_logging_guards_formatting_and_borrowed_sets() {
    let log = logging::Log::default();
    let mut factory = Factory::default();
    assert!(log.add_output_stream(&mut factory, "X", None, 0, None));
    let mut f = Printf::new();
    log.output_formatted(&mut f, Some("C"), Some(b"x=%d"), &[4]);
    assert!(f.sizes.is_empty());
    assert!(log.observed_categories(|s| s.unwrap().is_empty()));
    let backend_set = log.backend_names(|s| s.unwrap() as *const _ as usize);
    let category_set = log.observed_categories(|s| s.unwrap() as *const _ as usize);
    assert_ne!(backend_set, category_set);
    log.attach_initialized_host();
    log.output_formatted(&mut f, None, Some(b"x=%d"), &[4]);
    log.output_formatted(&mut f, Some("C"), None, &[4]);
    assert!(f.sizes.is_empty());
    log.output_formatted(&mut f, Some("C"), Some(b"x=%d"), &[4]);
    log.output_level_formatted(&mut f, Some("C"), 7, Some(b"y=%d"), &[5]);
    assert_eq!(
        *factory.events.lock().unwrap(),
        [
            logging::Record {
                category: "C".into(),
                level: 0,
                text: "x=4".into()
            },
            logging::Record {
                category: "C".into(),
                level: 7,
                text: "y=5".into()
            },
        ]
    );
    assert_eq!(
        log.observed_categories(|s| s.unwrap().iter().cloned().collect::<Vec<_>>()),
        ["C"]
    );
    assert_eq!(log.backend_names(|s| s.unwrap() as *const _ as usize), backend_set);
    assert_eq!(
        log.observed_categories(|s| s.unwrap() as *const _ as usize),
        category_set
    );
    log.shutdown(&mut LogLife(Vec::new()));
    assert!(log.backend_names(|s| s.is_none()));
    assert!(log.observed_categories(|s| s.is_none()));
}

#[test]
fn gzc74_dump_borrows_stream_and_does_not_drain() {
    let log = Arc::new(logging::Log::default());
    log.attach_initialized_host();
    let mut factory = Factory::default();
    log.add_output_stream(&mut factory, "X", None, 0, None);
    let other = log.clone();
    std::thread::spawn(move || other.output("C", 0, "pending"))
        .join()
        .unwrap();
    let stream = Sink::default();
    log.dump_output_levels(&stream, |level| {
        logging::string_from_filter_level(level).unwrap().into()
    });
    assert_eq!(
        *stream.0.lock().unwrap(),
        [
            "All Backends:\n",
            "-------------\n",
            "All unspecified groups showing none\n",
            "-------------\n",
            "backend levels\n"
        ]
    );
    assert_eq!(*factory.dump_stream.lock().unwrap(), Some(&stream as *const _ as usize));
    assert!(factory.events.lock().unwrap().is_empty());
    assert_eq!(log.default_level(), logging::MAX_LEVEL);
    log.on_tick();
    assert_eq!(factory.events.lock().unwrap().len(), 1);
}

#[test]
fn logging_formatter_retry_sizes_and_observed_filtered_category() {
    let log = logging::Log::default();
    log.attach_initialized_host();
    let b = backend(false, false);
    log.add_backend("decline", b);
    let mut f = Printf::new();
    f.statuses = [-1; 7].into();
    log.output_formatted(&mut f, Some("filtered"), Some(b"text"), &[]);
    assert_eq!(f.sizes, [1024, 2048, 4096, 8192, 16384, 32768, 65536]);
    assert!(log.observed_categories(|s| s.unwrap().contains("filtered")));
    f.sizes.clear();
    f.statuses = [3000, 4].into();
    log.output_formatted(&mut f, Some("filtered"), Some(b"text"), &[]);
    assert_eq!(f.sizes, [1024, 1025]);
}

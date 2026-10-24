//! web-network: service owner conformance vectors.
use fc4_core::rzurl::RzUrl;
use fc4_core::www::{http, service::*, Contract, Unsupported};

mod tests {
    use super::*;
    use std::collections::BTreeMap;
    struct Fake {
        effects: Vec<Effect>,
        http_effects: Vec<http::Effect>,
        admission_trace: Vec<&'static str>,
        callbacks: Vec<(Job, Notification)>,
        headers: BTreeMap<(u32, u32), Vec<u8>>,
        cookies: Vec<Vec<u8>>,
        insertions: Vec<(bool, (u32, u32), u32)>,
        size: u32,
        reads: usize,
        exists: bool,
        open: bool,
        read: bool,
        write: bool,
        cached: bool,
        cache_ram: bool,
        cache_file: bool,
        cancel_in_callback: Option<u32>,
        handlers: Vec<u32>,
        accept: bool,
        handler_result: bool,
        time: u32,
        ftp_submission: Option<u32>,
        ftp_calls: Vec<Vec<u8>>,
        request_cookie_reply: Option<Vec<u8>>,
        cookie_lookups: usize,
    }
    impl Default for Fake {
        fn default() -> Self {
            Self {
                effects: vec![],
                http_effects: vec![],
                admission_trace: vec![],
                callbacks: vec![],
                headers: BTreeMap::new(),
                cookies: vec![],
                insertions: vec![],
                size: 1,
                reads: 0,
                exists: true,
                open: true,
                read: true,
                write: true,
                cached: false,
                cache_ram: true,
                cache_file: true,
                cancel_in_callback: None,
                handlers: vec![],
                accept: false,
                handler_result: false,
                time: 1000,
                ftp_submission: None,
                ftp_calls: vec![],
                request_cookie_reply: None,
                cookie_lookups: 0,
            }
        }
    }
    impl http::Host for Fake {
        fn effect(&mut self, value: http::Effect) {
            if matches!(value, http::Effect::ConstructRequest(_)) {
                self.admission_trace.push("submit");
            }
            self.http_effects.push(value);
        }
        fn notify(&mut self, _: &mut http::Client, _: http::Job, _: bool) {}
    }
    impl Host for Fake {
        fn ftp_submit(&mut self, url: &[u8], a: u32, b: u32) -> Contract<u32> {
            assert_eq!((a, b), (0, 0));
            self.ftp_calls.push(url.to_vec());
            self.ftp_submission
                .ok_or(Unsupported::OwnedBody("Q7 FTP scheduler admission"))
        }
        fn effect(&mut self, value: Effect) {
            match value {
                Effect::Lock => self.admission_trace.push("www-lock"),
                Effect::Unlock => self.admission_trace.push("www-unlock"),
                Effect::EnterHttpAdmission => self.admission_trace.push("enter"),
                Effect::LeaveHttpAdmission => self.admission_trace.push("leave"),
                _ => {}
            }
            self.effects.push(value);
        }
        fn now(&mut self) -> u32 {
            self.time
        }
        fn file_exists(&mut self, _: &[u8]) -> bool {
            self.exists
        }
        fn path_name(&mut self, path: &[u8]) -> Vec<u8> {
            path.to_vec()
        }
        fn file_open(&mut self, _: &[u8], _: bool) -> Option<u32> {
            self.open.then_some(1)
        }
        fn file_size(&mut self, _: u32) -> u32 {
            self.size
        }
        fn file_read(&mut self, _: u32, _: u32) -> Option<Vec<u8>> {
            self.reads += 1;
            self.read.then(|| b"RAM".to_vec())
        }
        fn file_write(&mut self, _: u32, _: &[u8]) -> bool {
            self.write
        }
        fn file_close(&mut self, _: u32) {}
        fn cache_present(&mut self, _: &[u8]) -> bool {
            self.cached
        }
        fn cache_size(&mut self, _: &[u8]) -> u32 {
            self.size
        }
        fn cache_data(&mut self, _: &[u8]) -> Option<Vec<u8>> {
            self.reads += 1;
            self.cache_ram.then(|| b"cache-data".to_vec())
        }
        fn cache_file(&mut self, _: &[u8]) -> Option<Vec<u8>> {
            self.cache_file.then(|| b"cache-file".to_vec())
        }
        fn new_cache_file(&mut self) -> Vec<u8> {
            b"new-file".to_vec()
        }
        fn insert_cache(&mut self, _: &[u8], _: &[u8], file: bool, flags: (u32, u32), expiry: u32) -> bool {
            self.insertions.push((file, flags, expiry));
            true
        }
        fn cache_directory(&mut self) -> Vec<u8> {
            b"directory".to_vec()
        }
        fn set_cache_directory(&mut self, _: &[u8]) -> bool {
            false
        }
        fn header(&mut self, _: u32, family: u32, index: u32) -> Option<Vec<u8>> {
            self.headers.get(&(family, index)).cloned()
        }
        fn parse_time(&mut self, text: &[u8]) -> u32 {
            std::str::from_utf8(text).unwrap().parse().unwrap()
        }
        fn request_cookies(&mut self, _: &[u8]) -> Option<Vec<u8>> {
            self.cookie_lookups += 1;
            self.request_cookie_reply.clone()
        }
        fn response_cookie(&mut self, _: &[u8], text: &[u8]) {
            self.cookies.push(text.to_vec());
        }
        fn handler_init(&mut self, _: u32) -> bool {
            true
        }
        fn handler_shutdown(&mut self, id: u32) {
            self.handlers.push(id);
        }
        fn handler_accepts(&mut self, id: u32, _: &mut RzUrl) -> bool {
            self.handlers.push(id);
            self.accept
        }
        fn handler_load(&mut self, id: u32, _: &mut Job) -> bool {
            self.handlers.push(id);
            self.handler_result
        }
        fn notify(&mut self, service: &mut Www, job: &Job, notification: Notification) {
            assert_eq!(service.status(job.id), 0);
            self.callbacks.push((job.clone(), notification));
            if let Some(id) = self.cancel_in_callback.take() {
                assert!(service.cancel(self, id));
            }
        }
    }
    fn service() -> Www {
        Www::snapshot(http::Client::new(None))
    }
    fn resource(file: bool) -> http::Resource {
        http::Resource {
            refs: 1,
            strings: [b"http://host/x".to_vec(), b"payload".to_vec(), vec![]],
            file,
            redirects: 0,
        }
    }
    #[test]
    fn www21_null_target_skips_entire_suffix() {
        for success in [false, true] {
            let mut w = service();
            let mut h = Fake::default();
            w.active.push(Job {
                id: 1,
                foreign: 7,
                url: b"same".to_vec(),
                target: None,
                ..Job::default()
            });
            w.waiting.push(Job {
                id: 2,
                url: b"same".to_vec(),
                target: Some(9),
                ..Job::default()
            });
            let mut r = resource(true);
            let original = r.clone();
            assert_eq!(w.complete_http(&mut h, Some(7), success, Some(&mut r)), Ok(true));
            assert!(w.active.is_empty());
            assert_eq!(w.waiting.len(), 1);
            assert_eq!(r, original);
            assert!(h.callbacks.is_empty());
            assert!(h.insertions.is_empty());
            assert!(h.cookies.is_empty());
            assert_eq!(h.reads, 0);
        }
    }
    #[test]
    fn www22_cookie_record_construction() {
        for (enabled, reply, lookups) in [
            (0, Some(b"a=1; b=2".to_vec()), 0),
            (1, None, 1),
            (1, Some(b"a=1; b=2".to_vec()), 1),
            (1, Some(vec![]), 1),
        ] {
            let mut w = service();
            w.cookies = enabled;
            let mut h = Fake {
                request_cookie_reply: reply.clone(),
                ..Fake::default()
            };
            w.load_text(&mut h, b"http://host/x", 0, Some(9), 0, false).unwrap();
            assert_eq!(h.cookie_lookups, lookups);
            let copied: Vec<_> = h
                .http_effects
                .iter()
                .filter_map(|effect| {
                    if let http::Effect::CopyHeaders(headers) = effect {
                        Some(headers.clone())
                    } else {
                        None
                    }
                })
                .collect();
            let submitted = if enabled != 0 {
                reply
                    .clone()
                    .map(|value| vec![vec![(b"Cookie".to_vec(), value)]])
                    .unwrap_or_default()
            } else {
                vec![]
            };
            assert_eq!(copied, submitted); // True/empty still submitted exactly one record.
            let req = w.http.requests.values().next().unwrap();
            // The temporary one-record input is copied before admission; GET then
            // removes an empty first value (§4.14), even for a true cookie lookup.
            let expected = if enabled != 0 {
                reply
                    .filter(|v| !v.is_empty())
                    .map(|v| vec![(b"Cookie".to_vec(), v)])
                    .unwrap_or_default()
            } else {
                vec![]
            };
            assert_eq!(req.headers, expected);
            h.request_cookie_reply = Some(b"changed".to_vec());
            assert_eq!(req.headers, expected);
        }
    }
    #[test]
    fn identity_default_hold_order_and_dispatch_target_selection() {
        let mut w = service();
        let mut h = Fake::default();
        let mut out = 123;
        assert!(!w.query_interface(&mut h, 99, &mut out));
        assert_eq!(out, 123);
        assert!(h.effects.is_empty());
        assert!(w.query_interface(&mut h, 1, &mut out));
        assert_eq!(out, 4);
        w.set_default_target(&mut h, Some(9));
        w.set_default_target(&mut h, Some(9));
        w.set_default_target(&mut h, None);
        assert_eq!(
            &h.effects[1..],
            [
                Effect::AddTarget(9),
                Effect::AddTarget(9),
                Effect::ReleaseTarget(9),
                Effect::ReleaseTarget(9)
            ]
        );
        assert_eq!(w.load_text(&mut h, b"http://host/x", 0, None, 0, true), Ok(0));
        assert!(h.callbacks.is_empty());
    }
    #[test]
    fn constructed_components_and_ftp_admission_vectors() {
        let mut h = Fake::default();
        let mut w = Www::construct_with_host(&mut h, Some(80), 77);
        assert_eq!(w.http.target, Some(77));
        assert_eq!(w.http.lookup, Some(80));
        assert_eq!(h.effects.len(), 11);
        assert_eq!(h.effects[0], Effect::RegisterService(0x449b1939, 0));
        assert_eq!(
            &h.effects[9..],
            [
                Effect::Component("http", "connect-target"),
                Effect::Component("ftp", "connect-target")
            ]
        );
        assert_eq!(Www::construct().unwrap().next_job, 1);
        h.effects.clear();
        h.ftp_submission = Some(91);
        assert_eq!(w.load_text(&mut h, b"ftp://host/x", 3, Some(9), 0, true), Ok(2));
        assert_eq!(w.ftp_id(2), 91);
        assert_eq!(w.status(2), 2);
        assert_eq!(h.ftp_calls, [b"ftp://host/x".to_vec()]);
        assert!(w.cancel(&mut h, 2));
        assert!(h.http_effects.is_empty());
        assert!(!h.effects.contains(&Effect::ReleaseTarget(9))); // WWW-11
        h.effects.clear();
        h.ftp_submission = Some(0);
        assert_eq!(w.load_text(&mut h, b"ftp://host/x", 0, Some(9), 0, false), Ok(0));
        assert_eq!(h.effects, [Effect::AddTarget(9)]);
        assert!(h.callbacks.is_empty()); // WWW-16
        h.ftp_submission = Some(92);
        let id = w.load_text(&mut h, b"ftp://host/x", 3, Some(9), 0, false).unwrap();
        assert!(w.complete_ftp(&mut h, 92, true, b"received.bin"));
        assert_eq!(w.status(id), 0);
        assert_eq!(h.callbacks.last().unwrap().0.data, b"received.bin");
        assert!(h.callbacks.last().unwrap().0.file);
        assert!(h.insertions.is_empty() && h.cookies.is_empty());
    }
    #[test]
    fn handler_failure_owns_attempt_and_releases_after_callback() {
        let mut w = service();
        let mut h = Fake {
            accept: true,
            ..Fake::default()
        };
        w.add_handler(&mut h, 1);
        w.add_handler(&mut h, 2);
        assert_eq!(w.load_text(&mut h, b"http://host/x", 0, Some(9), 0, true), Ok(2));
        assert_eq!(h.handlers, [1, 1]);
        assert_eq!(h.callbacks[0].0.result, 0);
        assert_eq!(h.effects.last(), Some(&Effect::ReleaseTarget(9)));
        w.remove_handler(&mut h, 1);
        w.remove_all_handlers(&mut h);
        assert!(w.handlers.is_empty());
    }
    #[test]
    fn local_threshold_open_fallback_and_read_failure_nonzero_id() {
        for (size, reads, file) in [(499999, 1, false), (500000, 0, true)] {
            let mut w = service();
            let mut h = Fake {
                size,
                ..Fake::default()
            };
            assert_eq!(w.load_text(&mut h, b"file:///C|/x", 1, Some(9), 0, false), Ok(2));
            assert_eq!(h.reads, reads);
            assert_eq!(h.callbacks[0].0.file, file);
        }
        let mut w = service();
        let mut h = Fake {
            read: false,
            ..Fake::default()
        };
        assert_eq!(w.load_text(&mut h, b"file:///C|/x", 0, Some(9), 0, false), Ok(2));
        assert_eq!(h.callbacks[0].0.result, 0);
        let mut w = service();
        let mut h = Fake {
            open: false,
            ..Fake::default()
        };
        assert_eq!(w.load_text(&mut h, b"file:///C|/x", 0, Some(9), 0, false), Ok(2));
        assert!(h.callbacks[0].0.file);
        let mut w = service();
        let mut h = Fake {
            exists: false,
            ..Fake::default()
        };
        assert_eq!(w.load_text(&mut h, b"file:///C|/x", 0, Some(9), 0, false), Ok(0));
        assert_eq!(h.callbacks[0].0.progress, 0);
    }
    #[test]
    fn cache_threshold_and_failed_retrieval_continues_to_admission() {
        for (size, reads) in [(1999999, 1), (2000000, 0)] {
            let mut w = service();
            w.cache = 1;
            let mut h = Fake {
                size,
                cached: true,
                ..Fake::default()
            };
            assert_eq!(w.load_text(&mut h, b"http://host/x", 1, Some(9), 0, true), Ok(2));
            assert_eq!(h.reads, reads);
            assert_eq!(w.status(2), 0);
        }
        let mut w = service();
        w.cache = 1;
        let mut h = Fake {
            cached: true,
            cache_ram: false,
            cache_file: false,
            ..Fake::default()
        };
        assert_eq!(w.load_text(&mut h, b"http://host/x", 0, Some(9), 0, true), Ok(2));
        assert_eq!(w.status(2), 2);
        assert!(h.callbacks.is_empty());
        assert_eq!(w.http.queue.len(), 1);
    }
    #[test]
    fn http_admission_helper_spans_submission_and_www_insertion() {
        for counter in [0, u32::MAX] {
            let mut w = service();
            w.http.counter = counter;
            let mut h = Fake::default();
            let id = w.load_text(&mut h, b"http://host/x", 0, Some(9), 0, false).unwrap();
            if counter == 0 {
                assert_eq!(id, 2);
                assert_eq!(w.active.len(), 1);
                assert_eq!(
                    h.admission_trace,
                    [
                        "www-lock",
                        "www-unlock",
                        "enter",
                        "submit",
                        "www-lock",
                        "www-unlock",
                        "leave"
                    ]
                );
            } else {
                // WWW-16: HTTP's wrapped zero ID still queues a request, but
                // WWW inserts nothing and leaks its selected target hold.
                assert_eq!(id, 0);
                assert!(w.active.is_empty());
                assert_eq!(
                    h.admission_trace,
                    ["www-lock", "www-unlock", "enter", "submit", "leave"]
                );
                assert!(!h.effects.contains(&Effect::ReleaseTarget(9)));
            }
            assert_eq!(w.http.queue.len(), 1);
            assert!(h.callbacks.is_empty());
        }
    }
    #[test]
    fn duplicate_waiting_foreign_status_cancel_leaks_and_fanout_reentry() {
        let mut w = service();
        let mut h = Fake::default();
        for preference in [3, 0, 1] {
            w.load_text(
                &mut h,
                b"http://host/x",
                preference,
                Some(9),
                preference as u32 + 1,
                false,
            )
            .unwrap();
        }
        assert_eq!(w.http.queue.len(), 1);
        assert_eq!(w.status(3), 2);
        assert_eq!(w.http_id(3), 0);
        assert!(w.get_job(3).is_none());
        h.cancel_in_callback = Some(3);
        let mut r = resource(false);
        w.complete_http(&mut h, Some(1), true, Some(&mut r)).unwrap();
        assert_eq!(h.callbacks.iter().map(|c| c.0.id).collect::<Vec<_>>(), [2, 4]);
        assert!(h.callbacks[1].0.file);
        assert_eq!(h.callbacks[1].0.preference, 3);
        assert_eq!(h.effects.iter().filter(|e| **e == Effect::ReleaseTarget(9)).count(), 2);
        assert!(r.strings[1].is_empty());
        assert!(!r.file);
    }
    #[test]
    fn cancellation_late_resource_and_foreign_admission_zero_hold() {
        let mut w = service();
        let mut h = Fake::default();
        w.load_text(&mut h, b"http://host/x", 0, Some(9), 0, false).unwrap();
        assert!(w.cancel(&mut h, 2));
        assert!(!w.cancel(&mut h, 2));
        assert!(h.callbacks.is_empty());
        assert!(!h.effects.contains(&Effect::ReleaseTarget(9)));
        let mut r = resource(true);
        let before = r.clone();
        assert_eq!(w.complete_http(&mut h, Some(1), true, Some(&mut r)), Ok(true));
        assert_eq!(r, before);
        let mut w = service();
        w.http.counter = u32::MAX;
        let mut h = Fake::default();
        assert_eq!(w.load_text(&mut h, b"http://host/x", 0, Some(9), 0, false), Ok(0));
        assert!(!h.effects.contains(&Effect::ReleaseTarget(9)));
        assert_eq!(w.http.queue.len(), 1);
    }
    #[test]
    fn completion_conversion_failure_cookies_and_cache_before_target() {
        let mut w = service();
        w.cache = 1;
        let mut h = Fake {
            write: false,
            ..Fake::default()
        };
        for family in [0x35, 0x34] {
            for index in 0..33 {
                h.headers
                    .insert((family, index), format!("{family}:{index}").into_bytes());
            }
        }
        w.load_text(&mut h, b"http://host/x", 3, Some(9), 0, false).unwrap();
        let mut r = resource(false);
        w.complete_http(&mut h, Some(1), true, Some(&mut r)).unwrap();
        assert_eq!(h.callbacks[0].0.result, 1);
        assert!(!h.callbacks[0].0.file);
        assert_eq!(h.cookies.len(), 64);
        assert_eq!(h.cookies[31], b"53:31");
        assert_eq!(h.cookies[32], b"52:0");
        assert_eq!(h.insertions, [(false, (1, 1), 260200)]);
    }
    #[test]
    fn policy_distinguishing_headers_and_wrapping_dates() {
        for (family, text, insert) in [
            (4, b"xno-cachey".as_slice(), false),
            (4, b"NO-CACHE", true),
            (1, b" PRIVATE ", false),
            (1, b"private=field, no-store=1, max-age=60", true),
            (1, b"must-revalidate, max-age", true),
            (1, b"a,b,c,d,e,f,no-store", true),
            (1, b"a,b,c,d,e,no-store,f", false),
            (1, b"\"a,no-store\", public", true),
        ] {
            let mut h = Fake::default();
            h.headers.insert((family, 0), text.to_vec());
            assert_eq!(cache_policy(&mut h, true, Some((b"url", b"data", false)), 1), insert);
            if insert {
                assert_eq!(h.insertions[0].2, 260200);
            }
        }
        let mut h = Fake::default();
        h.headers.insert((3, 0), b"4294901760".to_vec());
        assert!(cache_policy(&mut h, true, Some((b"url", b"file", true)), 1));
        assert_eq!(h.insertions[0], (true, (0, 1), 0x2f480));
        h.headers.insert((0x2c, 0), b"7".to_vec());
        h.headers.insert((0x2d, 0), b"900".to_vec());
        assert!(cache_policy(&mut h, true, Some((b"url", b"data", false)), 1));
        assert_eq!(h.insertions[1].2, 7);
        h.headers.insert((1, 0), b"max-age\0=60".to_vec());
        h.headers.insert((3, 0), b"1000".to_vec());
        assert!(cache_policy(&mut h, true, Some((b"url", b"data", false)), 1));
        assert_eq!(h.insertions[2].2, 1060);
        assert!(!cache_policy(&mut h, false, Some((b"url", b"data", false)), 1));
        assert!(!cache_policy(&mut h, true, None, 1));
    }
    #[test]
    fn splitter_empty_slots_quote_and_directory_side_effects() {
        assert_eq!(
            cache_tokens(b",a,,b, ,"),
            vec![b"".to_vec(), b"a".to_vec(), b"".to_vec(), b"b".to_vec(), b"".to_vec()]
        );
        assert_eq!(cache_tokens(b"\"a,b\",c"), vec![b"a,b".to_vec(), b"c".to_vec()]);
        assert!(cache_tokens(b"\"\"").is_empty());
        let mut w = service();
        let mut h = Fake::default();
        assert!(!w.set_directory(&mut h, b"out"));
        assert_eq!(w.http.directory, b"out\\");
        assert_eq!(w.http.cache_setting, b"_cache");
        assert_eq!(
            h.effects,
            [
                Effect::CookieFilename(b"out\\CookieData.ini".to_vec()),
                Effect::HttpDirectory(b"out\\".to_vec()),
                Effect::HttpCacheSetting(b"_cache".to_vec()),
                Effect::FtpDirectory(b"out\\".to_vec())
            ]
        );
        h.cached = true;
        assert!(w.visited(&mut h, b"url"));
        assert_eq!(w.cache, 0);
    }
    #[test]
    fn progress_borrowed_no_waiter_update_and_cancel_all() {
        let mut w = service();
        let mut h = Fake::default();
        w.load_text(&mut h, b"http://host/x", 0, Some(9), 0, false).unwrap();
        w.load_text(&mut h, b"http://host/x", 1, Some(9), 0, false).unwrap();
        // Progress callbacks observe live membership (unlike terminal callbacks).
        let before = w.active[0].progress;
        assert_eq!(w.progress(&mut h, true, None, Some([1, 2, 3])), Ok(true));
        assert!(h.callbacks.is_empty());
        assert!(w.progress(&mut h, false, None, Some([1, 2, 3])).is_err());
        assert_eq!(w.active[0].progress, before);
        assert_eq!(w.cancel_all(&mut h), 2);
        assert!(w.active.is_empty() && w.waiting.is_empty());
        assert_eq!(w.do_message(&mut h, Incoming::Unknown), Ok(true));
        w.post_init(&mut h);
        w.shutdown(&mut h);
        assert!(!w.initialized);
    }
}

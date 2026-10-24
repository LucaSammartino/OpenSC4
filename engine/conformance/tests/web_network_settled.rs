//! Constructed fixtures from the q17/q18 published contracts, not live traces.
use fc4_core::www::http::{parse_records, Request};

fn pairs(input: &[(&[u8], &[u8])]) -> Vec<(Vec<u8>, Vec<u8>)> {
    input.iter().map(|(a, b)| (a.to_vec(), b.to_vec())).collect()
}
#[test]
fn norm01_through_norm08() {
    let mut request = Request {
        method: b"GET".to_vec(),
        raw_header: b"X: a\r\nX: b".to_vec(),
        ..Request::default()
    };
    request.normalize();
    assert_eq!(request.headers, pairs(&[(b"X", b"a"), (b"X", b"b")])); // NORM01
    assert!(request.raw_header.is_empty());
    request.headers = pairs(&[(b"A", b"v")]);
    request.raw_header = b"X: a".to_vec();
    request.normalize();
    assert_eq!(request.headers, pairs(&[(b"A", b"v")]));
    assert_eq!(request.raw_header, b"X: a"); // NORM02
    for (method, expected) in [
        (b"GET".as_slice(), pairs(&[(b"Y", b"z")])),
        (b"OPTIONS", pairs(&[(b"X", b""), (b"Y", b"z")])),
        (b"WAT", pairs(&[(b"X", b""), (b"Y", b"z")])),
    ] {
        request = Request {
            method: method.to_vec(),
            raw_header: b"X:\r\nY: z".to_vec(),
            ..Request::default()
        };
        request.normalize();
        assert_eq!(request.headers, expected); // NORM03/04
    }
    assert!(parse_records(b"X:").is_empty());
    assert_eq!(parse_records(b"X:\r\n"), pairs(&[(b"X", b"")])); // NORM05
    assert_eq!(parse_records(b"X : a \t"), pairs(&[(b"X ", b"a \t")])); // NORM06
    assert_eq!(
        parse_records(b"X:a\r\n b\r\nY:c"),
        pairs(&[(b"X", b"ab"), (b"Y", b"c")])
    );
    assert_eq!(
        parse_records(b"X:a\r\n\tb\r\nY:c"),
        pairs(&[(b"X", b"a"), (b"Y", b"c")])
    ); // NORM07
    request = Request {
        method: b"GET".to_vec(),
        headers: pairs(&[(b"A", b""), (b"B", b""), (b"C", b"v")]),
        ..Request::default()
    };
    request.normalize();
    assert_eq!(request.headers, pairs(&[(b"B", b""), (b"C", b"v")])); // NORM08
}
#[test]
fn http26_through_http34_dates() {
    use fc4_core::www::{http_date::*, notification::*};
    for (text, kind) in [
        (b"Sun, 06 Nov 1994 08:49:37 GMT".as_slice(), 1),
        (b"Sunday, 06-Nov-94 08:49:37 GMT", 2),
        (b"Sunday, 06-Nov-1994 08:49:37 GMT", 3),
        (b"Sun Nov  6 08:49:37 1994", 4),
    ] {
        let (actual, c) = parse_date(text).unwrap().unwrap();
        assert_eq!(actual, kind);
        assert_eq!(
            (c.weekday, c.day, c.month, c.year, c.yearday, c.hour, c.minute, c.second),
            (
                7,
                6,
                11,
                1994,
                311,
                8,
                if kind == 4 { 4 } else { 49 },
                if kind == 4 { 0 } else { 37 }
            )
        );
    }
    for (year, expected) in [(b"0049", 2049), (b"0050", 1950)] {
        let text = [b"Sun, 06 Nov ".as_slice(), year, b" 08:49:37 GMT"].concat();
        assert_eq!(parse_date(&text).unwrap().unwrap().1.year, expected); // HTTP30
    }
    assert_eq!(
        parse_date(b"Sun, 01 Jan 2000 08:49:37 GMT").unwrap().unwrap().1.yearday,
        3
    ); // HTTP31
    let c = parse_date(b"Bad, 06 Xxx 1994 08:49:37 PST").unwrap().unwrap().1;
    assert_eq!((c.weekday, c.month, c.yearday), (0, 0, 0x00636544 + 7)); // HTTP32
    struct Clock(u32);
    impl Runtime for Clock {
        fn mktime(&mut self, c: RuntimeCalendar) -> u32 {
            assert_eq!(c.is_dst, 1);
            self.0
        }
        fn adjustment(&mut self) -> u32 {
            28800
        }
    }
    assert_eq!(
        seconds(&mut Clock(10000), b"Sun, 06 Nov 1994 08:49:37 GMT"),
        Ok(0xffffb690)
    ); // HTTP33
    assert_eq!(
        seconds(&mut Clock(u32::MAX), b"Sun, 06 Nov 1994 08:49:37 GMT"),
        Ok(0xffff8f7f)
    ); // HTTP34
}
#[test]
fn http10_through_http14_and_http22_through_http25() {
    use fc4_core::www::{http::*, http_wire::*};
    let mut request = Request {
        method: b"GET".to_vec(),
        url: b"http://h/a".to_vec(),
        ..Request::default()
    };
    assert_eq!(
        request_bytes(&mut request, None).unwrap(),
        b"GET /a HTTP/1.1\r\nHost: h\r\n\r\n"
    ); // HTTP10
    request.method = b"OPTIONS".to_vec();
    request.raw_header.clear();
    assert_eq!(
        request_bytes(&mut request, None).unwrap(),
        b"OPTIONS * HTTP/1.1\r\nHost: h\r\n\r\n"
    ); // HTTP11
    request.headers = pairs(&[(b"Host", b""), (b"X", b"a"), (b"X", b"b")]);
    request.method = b"GET".to_vec();
    request.raw_header.clear();
    assert_eq!(
        request_bytes(&mut request, None).unwrap(),
        b"GET /a HTTP/1.1\r\nHost: \r\nX: a\r\nX: b\r\n\r\n"
    ); // HTTP12
    request.raw_header = b"custom".to_vec();
    request.body = b"xyz".to_vec();
    assert_eq!(request_bytes(&mut request, None).unwrap(), b"customxyz"); // HTTP13
    for (value, expected) in [
        (b"Chunked".as_slice(), false),
        (b"chunked, gzip", false),
        (b"chunked", true),
    ] {
        assert_eq!(
            framing(2, 200, &pairs(&[(b"Transfer-Encoding", value)])).chunked,
            expected
        ); // HTTP14
    }
    assert_eq!(framing(2, 200, &vec![]).length, 0x7fffffff);
    assert_eq!(framing(3, 200, &vec![]).length, 0);
    assert!(!expired(u32::MAX, 2, 1));
    assert!(expired(u32::MAX, 2, 2)); // HTTP22
    struct Recorder;
    impl Host for Recorder {
        fn effect(&mut self, _: Effect) {}
        fn notify(&mut self, _: &mut Client, _: Job, _: bool) {}
    }
    let mut client = Client::new(None);
    let mut job = Job([1, 1, 0, 1, 9, 7, 0, 0]);
    client.requests.insert(
        7,
        Request {
            method: b"GET".to_vec(),
            url: b"http://h/a".to_vec(),
            redirects: 4,
            raw_header: b"old".to_vec(),
            ..Request::default()
        },
    );
    let headers = pairs(&[(b"Location", b"b")]);
    assert_eq!(finalize(&mut client, &mut Recorder, &mut job, 302, &headers), Ok(true));
    assert_eq!(client.requests[&7].url, b"http://h/b");
    assert_eq!(client.requests[&7].redirects, 5);
    assert!(client.requests[&7].raw_header.is_empty());
    assert_eq!(job.0[3], 2);
    assert_eq!(job.0[4], 0);
    assert_eq!(finalize(&mut client, &mut Recorder, &mut job, 302, &headers), Ok(false)); // HTTP23
    client.requests.get_mut(&7).unwrap().method = b"POST".to_vec();
    client.requests.get_mut(&7).unwrap().redirects = 0;
    assert_eq!(finalize(&mut client, &mut Recorder, &mut job, 302, &headers), Ok(false));
    assert_eq!(job.0[1], 8); // HTTP24
    for (method, result) in [(b"GET".as_slice(), 9), (b"PUT", 9), (b"OPTIONS", 8)] {
        client.requests.get_mut(&7).unwrap().method = method.to_vec();
        assert_eq!(finalize(&mut client, &mut Recorder, &mut job, 401, &headers), Ok(false));
        assert_eq!(job.0[1], result); // HTTP25
    }
}

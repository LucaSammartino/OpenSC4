//! Constructed transport/support fixtures; no live-server equivalence claim.
use fc4_core::www::{
    http::{self, Client, Job, Request},
    http_transport::*,
    http_wire::{ReadHost, ReadStep},
    support,
};
use std::collections::VecDeque;

struct Io {
    reads: VecDeque<ReadStep>,
    sends: VecDeque<Option<usize>>,
    sent: Vec<u8>,
    open: bool,
    now: u32,
    cancel: bool,
    resolve: Option<[u8; 4]>,
    connect: bool,
    resolutions: usize,
    connections: usize,
    closes: usize,
    effects: Vec<http::Effect>,
}
impl Default for Io {
    fn default() -> Self {
        Self {
            reads: VecDeque::new(),
            sends: VecDeque::new(),
            sent: Vec::new(),
            open: true,
            now: 100,
            cancel: false,
            resolve: Some([1, 2, 3, 4]),
            connect: true,
            resolutions: 0,
            connections: 0,
            closes: 0,
            effects: Vec::new(),
        }
    }
}
impl ReadHost for Io {
    fn now(&mut self) -> u32 {
        self.now
    }
    fn cancel(&mut self) -> bool {
        self.cancel
    }
    fn clear_cancel(&mut self) {
        self.cancel = false;
    }
    fn connected(&mut self) -> bool {
        self.open
    }
    fn receive(&mut self, capacity: usize, timeout: u32) -> ReadStep {
        assert_eq!((capacity, timeout), (1024, 1000));
        self.reads.pop_front().unwrap_or(ReadStep::Failed)
    }
    fn temp_path(&mut self, _: &[u8]) -> Option<Vec<u8>> {
        None
    }
    fn open_temp(&mut self, _: &[u8], _: (u32, u32, u32)) -> bool {
        false
    }
    fn write_temp(&mut self, _: &[u8]) -> bool {
        false
    }
    fn trim_temp(&mut self, _: u32) {}
    fn close_temp(&mut self) {}
    fn remove_temp(&mut self, _: &[u8]) {}
    fn progress(&mut self, _: u32, _: u32) {}
}
impl http::Host for Io {
    fn effect(&mut self, effect: http::Effect) {
        self.effects.push(effect);
    }
    fn notify(&mut self, _: &mut Client, _: Job, _: bool) {
        panic!("transaction must not complete a queued job");
    }
}
impl Host for Io {
    fn resolve(&mut self, _: &[u8]) -> Option<[u8; 4]> {
        self.resolutions += 1;
        self.resolve
    }
    fn connect(&mut self, _: [u8; 4], _: u16) -> bool {
        self.connections += 1;
        self.open = self.connect;
        self.connect
    }
    fn close(&mut self) {
        self.closes += 1;
        self.open = false;
    }
    fn send(&mut self, bytes: &[u8]) -> Option<usize> {
        let count = self.sends.pop_front().unwrap_or(Some(bytes.len()));
        if let Some(n) = count {
            self.sent.extend_from_slice(&bytes[..n]);
        }
        count
    }
}
fn fixture(url: &[u8]) -> (Client, Job, Peer) {
    let mut client = Client::new(None);
    client.requests.insert(
        7,
        Request {
            method: b"GET".to_vec(),
            url: url.to_vec(),
            ..Request::default()
        },
    );
    (client, Job([1, 1, 0, 1, 0, 7, 0, 0]), Peer::default())
}
#[test]
fn header_fragmentation_informational_status_zero_and_interrupts() {
    let mut io = Io::default();
    io.reads = [
        ReadStep::Pending,
        ReadStep::Data(Vec::new()),
        ReadStep::Data(b"HTTP/1.1 200 OK\r\nX: a\r\nX: b\r\n\r".to_vec()),
        ReadStep::Data(b"\nabc".to_vec()),
    ]
    .into();
    let mut result = 1;
    let response = read_header(&mut io, 100, 15, false, &mut result).unwrap();
    assert_eq!((response.status, response.major, response.minor), (200, 1, 1));
    assert_eq!(response.initial, b"abc");
    assert_eq!(response.headers.len(), 2);
    for skip in [false, true] {
        io.reads = [ReadStep::Data(
            b"HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK\r\n\r\nabc".to_vec(),
        )]
        .into();
        assert_eq!(
            read_header(&mut io, 100, 15, skip, &mut result).unwrap().status,
            if skip { 200 } else { 100 }
        );
    }
    io.reads = [ReadStep::Data(b"HTTP/1.1 0 Bad\r\n\r\n".to_vec())].into();
    assert!(read_header(&mut io, 100, 15, false, &mut result).is_none());
    io.cancel = true;
    assert!(read_header(&mut io, 100, 15, false, &mut result).is_none());
    assert_eq!(result, 7);
    assert!(!io.cancel);
    io.now = 116;
    assert!(read_header(&mut io, 100, 15, false, &mut result).is_none());
    assert_eq!(result, 5);
}
#[test]
fn worker_short_sends_numeric_lookup_reuse_and_framing() {
    let (mut client, mut job, mut peer) = fixture(b"http://1.2.3.4/a");
    let mut io = Io::default();
    io.sends = [Some(0), Some(3)].into();
    io.reads = [ReadStep::Data(
        b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabcXY".to_vec(),
    )]
    .into();
    let Outcome::Complete {
        response,
        body,
        overflow,
    } = execute(&mut client, &mut io, &mut job, &mut peer, 100, None).unwrap()
    else {
        panic!("unexpected retry")
    };
    assert_eq!(job.status(), 0);
    assert_eq!(response.unwrap().status, 200);
    assert_eq!(body.bytes, b"abcXY");
    assert_eq!(overflow, b"XY");
    assert_eq!((io.resolutions, io.connections), (0, 1));
    assert_eq!(io.sent, b"GET /a HTTP/1.1\r\nHost: 1.2.3.4\r\n\r\n");
    io.reads = [ReadStep::Data(
        b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\na".to_vec(),
    )]
    .into();
    execute(&mut client, &mut io, &mut job, &mut peer, 100, None).unwrap();
    assert_eq!(io.connections, 1);
}
#[test]
fn worker_failure_retry_redirect_and_error_codes() {
    for (version, retry) in [(b"1.0".as_slice(), false), (b"1.1", true)] {
        let (mut client, mut job, mut peer) = fixture(b"http://h/a");
        let mut io = Io::default();
        io.reads = [
            ReadStep::Data([b"HTTP/".as_slice(), version, b" 200 OK\r\nContent-Length: 2\r\n\r\n"].concat()),
            ReadStep::Failed,
        ]
        .into();
        let outcome = execute(&mut client, &mut io, &mut job, &mut peer, 100, None).unwrap();
        assert_eq!(matches!(outcome, Outcome::Requeued), retry);
        assert_eq!(client.queue.len(), usize::from(retry));
        assert_eq!(job.status(), if retry { 1 } else { 6 });
        assert!(!peer.initialized);
    }
    let (mut client, mut job, mut peer) = fixture(b"http://h/a");
    let mut io = Io::default();
    io.reads = [ReadStep::Data(
        b"HTTP/1.1 302 Found\r\nLocation: b\r\nContent-Length: 1\r\n\r\na".to_vec(),
    )]
    .into();
    assert_eq!(
        execute(&mut client, &mut io, &mut job, &mut peer, 100, None).unwrap(),
        Outcome::Requeued
    );
    assert_eq!(client.requests[&7].url, b"http://h/b");
    for expected in [2, 3, 4, 5, 7] {
        let (mut client, mut job, mut peer) = fixture(b"http://h/a");
        let mut io = Io::default();
        match expected {
            2 => client.requests.get_mut(&7).unwrap().method = b"?".to_vec(),
            3 => io.resolve = None,
            4 => io.connect = false,
            5 => io.now = 116,
            7 => io.cancel = true,
            _ => unreachable!(),
        }
        execute(&mut client, &mut io, &mut job, &mut peer, 100, None).unwrap();
        assert_eq!(job.status(), expected);
        assert!(client.queue.is_empty());
    }
}

#[derive(Default)]
struct Support {
    effects: Vec<support::Effect>,
    records: Vec<[u32; 3]>,
    removed: Vec<u32>,
    reads: VecDeque<(i32, Vec<u8>)>,
    closed: usize,
    logs: Vec<Vec<u8>>,
}
impl support::Host for Support {
    fn effect(&mut self, effect: support::Effect) {
        self.effects.push(effect);
    }
    fn progress(&mut self, recipient: u32, record: &[u32; 3]) {
        assert_eq!(recipient, 9);
        self.records.push(*record);
    }
    fn remove_lookup(&mut self, id: u32) -> bool {
        self.removed.push(id);
        id != 2
    }
}
impl support::DrainHost for Support {
    fn open(&mut self) -> bool {
        self.closed == 0
    }
    fn blocking(&mut self, value: bool) {
        assert!(value);
    }
    fn connected(&mut self) -> bool {
        true
    }
    fn readable(&mut self, timeout: u32) -> bool {
        assert_eq!(timeout, 0);
        true
    }
    fn receive(&mut self, capacity: usize) -> Option<(i32, Vec<u8>)> {
        assert_eq!(capacity, 1024);
        self.reads.pop_front()
    }
    fn log(&mut self, bytes: &[u8], count: usize) {
        self.logs.push(bytes[..count].to_vec());
    }
    fn close(&mut self) {
        self.closed += 1;
    }
}
#[test]
fn sup02_sup03_sup06_sup07_sup09() {
    let mut host = Support::default();
    support::forward_progress(&mut host, true, true, Some(9), [900, 1000]);
    support::forward_progress(&mut host, true, true, Some(9), [900, 0]);
    support::forward_progress(&mut host, false, true, Some(9), [900, 0]);
    assert_eq!(host.records, [[900, 1000, 1], [900, 0, 50]]);
    assert_eq!(host.effects.len(), 6);
    host.reads = [(7, b"1234567".to_vec()), (0, Vec::new()), (4, b"late".to_vec())].into();
    let mut counter = 0;
    support::drain_close(&mut host, &mut counter, true);
    assert_eq!((counter, host.closed, host.reads.len()), (7, 1, 1));
    assert_eq!(host.logs, [b"1234567".to_vec()]);
    let mut destination = vec![1];
    for (result, length) in [(1023, 1023), (1024, 0), (-1, 0)] {
        assert_eq!(support::formatted(&mut destination, &[b'x'; 1024], result), result);
        assert_eq!(destination.len(), length);
    }
    assert_eq!(
        (0..4).map(|i| support::address_byte(0x04030201, i)).collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
    assert_eq!(
        (0..4).map(|i| support::port_byte(0x3412, i)).collect::<Vec<_>>(),
        [0x12, 0x34, 0, 0]
    );
    let ids = [3, 1, 2].into_iter().collect();
    assert_eq!(support::Name::remove_ids(&mut host, &ids), 2);
    assert_eq!(host.removed, [1, 2, 3]);
    assert_eq!(ids.len(), 3);
    let mut name = support::Name::default();
    assert_eq!(name.settings, [10, 1800]);
    assert_eq!(name.release(&mut host), 0);
    assert_eq!(host.effects.last(), Some(&support::Effect::FreeName));
}

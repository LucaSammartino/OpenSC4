//! Constructed transport/support fixtures; no live-server equivalence claim.
use fc4_core::www::{
    http::{self, Client, Job, Request, Resource},
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
    endpoints: Vec<([u8; 4], u16)>,
    names: Vec<Vec<u8>>,
    observed: Vec<(Job, Option<Request>, Option<Resource>)>,
    retain_resources: bool,
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
            endpoints: Vec::new(),
            names: Vec::new(),
            observed: Vec::new(),
            retain_resources: false,
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
    fn notify(&mut self, client: &mut Client, job: Job, success: bool) {
        assert_eq!(success, job.status() == 0);
        assert!(client.workers.values().any(|w| w.current == Some(job)));
        self.observed.push((
            job,
            client.requests.get(&job.0[5]).cloned(),
            client.resources.get(&job.0[6]).cloned(),
        ));
        if self.retain_resources {
            assert_eq!(client.retain_request(self, job.0[5]).unwrap(), 2);
            if job.0[6] != 0 {
                assert_eq!(client.retain_resource(self, job.0[6]).unwrap(), 2);
            }
        }
    }
}
impl Host for Io {
    fn resolve(&mut self, server: &[u8]) -> Option<[u8; 4]> {
        self.resolutions += 1;
        self.names.push(server.to_vec());
        self.resolve
    }
    fn connect(&mut self, address: [u8; 4], port: u16) -> bool {
        self.connections += 1;
        self.endpoints.push((address, port));
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

// The named WWW19/WWW20 fixtures are the 31 Q19/Q20 proposed vectors.
fn status_resource(name: &[u8]) -> Resource {
    Resource {
        headers: vec![(name.to_vec(), vec![])],
        ..Resource::default()
    }
}
fn status_case(name: &[u8], status: i32, version: (i32, i32), reuse: bool) {
    let mut resource = status_resource(name);
    assert_eq!(resource.status(), status);
    assert_eq!(resource.version(), version);
    assert_eq!(
        fc4_core::www::http_wire::reusable(version.0 as u32, version.1 as u32),
        reuse
    );
    let mut io = Io {
        reads: [ReadStep::Data([name, b"\r\n\r\n"].concat())].into(),
        ..Io::default()
    };
    let response = read_header(&mut io, 100, 15, false, &mut 1);
    assert_eq!(
        response.map(|r| r.status),
        if status == 0 { None } else { Some(status) }
    );
}
#[test]
fn www20_s1_standard_status_version() {
    status_case(b"HTTP/1.1 200 OK", 200, (1, 1), true);
}
#[test]
fn www20_s2_non_http_status_prefix() {
    status_case(b"garbage 200suffix", 200, (0, 0), false);
}
#[test]
fn www20_s3_unvalidated_version_separators() {
    status_case(b"HTTPx1y1 200", 200, (1, 1), true);
}
#[test]
fn www20_s4_lowercase_version() {
    status_case(b"http/1.1 200", 200, (0, 0), false);
}
#[test]
fn www20_s5_multidigit_version() {
    status_case(b"HTTP/12.34 200", 200, (1, 0), false);
}
#[test]
fn www20_s6_signed_and_truncated_status() {
    for (token, status) in [(b"+200".as_slice(), 200), (b"-200", -200), (b"2x00", 2), (b"x200", 0)] {
        status_case(&[b"HTTP/1.1 ", token].concat(), status, (1, 1), true);
    }
}
#[test]
fn www20_s7_missing_second_token_or_records() {
    status_case(b"HTTP/1.1", 0, (1, 1), true);
    let mut resource = Resource::default();
    assert_eq!(resource.status(), 0);
    assert_eq!(resource.version(), (0, 0));
}
#[test]
fn www20_s8_crt_whitespace_and_ignored_tokens() {
    status_case(b"HTTP/1.1\t200 ignored 500", 200, (1, 1), true);
    for separator in [b' ', b'\t', 11, 12] {
        status_case(
            &[b"HTTP/1.1".as_slice(), &[separator], b"200"].concat(),
            200,
            (1, 1),
            true,
        );
    }
    for separator in *b"\r\n" {
        assert_eq!(
            status_resource(&[b"HTTP/1.1".as_slice(), &[separator], b"200"].concat()).status(),
            200
        );
    }
    let resource = status_resource(b"HTTP/1.1\xff200");
    assert_eq!(resource.status(), 0);
    assert_eq!(resource.status_in_locale(|b| b == 255), 200);
}
#[test]
fn www20_s9_status_name_383_bytes() {
    let mut name = b"HTTP/1.1 200 ".to_vec();
    name.resize(383, b' ');
    status_case(&name, 200, (1, 1), true);
}
#[test]
fn www20_s10_status_name_384_bytes() {
    let mut name = b"HTTP/1.1 200 ".to_vec();
    name.resize(384, b' ');
    status_case(&name, 0, (1, 1), true);
}
#[test]
fn www20_s11_status_32_bit_wrap() {
    status_case(b"HTTP/1.1 4294967496", 200, (1, 1), true);
    status_case(b"HTTP/1.1 4294967296", 0, (1, 1), true);
}
#[test]
fn www20_s12_worker_does_not_skip_informational() {
    let (mut client, mut job, mut peer) = fixture(b"http://h/a");
    let mut io = Io {
        reads: [
            ReadStep::Data(b"HTTP/1.1 100 Continue\r\n\r\n".to_vec()),
            ReadStep::Data(b"HTTP/1.1 200 OK\r\n\r\n".to_vec()),
        ]
        .into(),
        ..Io::default()
    };
    let Outcome::Complete { response, .. } = execute(&mut client, &mut io, &mut job, &mut peer, 100, None).unwrap()
    else {
        panic!("100 must reach finalization")
    };
    assert_eq!(response.unwrap().status, 100);
    assert_eq!(job.status(), 8);
    assert_eq!(io.reads.len(), 1);
}
#[test]
fn www20_s13_cached_version_survives_header_change() {
    let mut resource = status_resource(b"HTTP/1.1 200");
    resource.major = 3;
    resource.minor = 7;
    resource.headers[0].0 = b"http/1.1 200".to_vec();
    assert_eq!(resource.version(), (3, 7));
}
fn proxy(server: &[u8], port: u16) -> Proxy<'_> {
    Proxy {
        server,
        port,
        header14: b"Proxy-Connection",
    }
}
fn one_byte(io: &mut Io) {
    io.reads = [ReadStep::Data(
        b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\na".to_vec(),
    )]
    .into();
}
#[test]
fn www20_p1_proxy_formats_origin_and_connects_origin() {
    let (mut client, mut job, mut peer) = fixture(b"http://origin.test:8080/a");
    let mut io = Io::default();
    one_byte(&mut io);
    execute_with_proxy(
        &mut client,
        &mut io,
        &mut job,
        &mut peer,
        100,
        Some(proxy(b"proxy.test", 3128)),
    )
    .unwrap();
    assert_eq!(io.names, [b"origin.test".to_vec()]);
    assert_eq!(io.endpoints, [([1, 2, 3, 4], 8080)]);
    assert_eq!(io.sent, b"GET http://origin.test:8080/a HTTP/1.1\r\n\r\n");
}
#[test]
fn www20_p2_numeric_origin_bypasses_lookup_in_proxy_mode() {
    let (mut client, mut job, mut peer) = fixture(b"http://5.6.7.8:81/a");
    let mut io = Io {
        reads: [ReadStep::Data(
            b"HTTPx1y1 200suffix\r\nContent-Length: 1\r\n\r\na".to_vec(),
        )]
        .into(),
        ..Io::default()
    };
    execute_with_proxy(
        &mut client,
        &mut io,
        &mut job,
        &mut peer,
        100,
        Some(proxy(b"proxy.test", 3128)),
    )
    .unwrap();
    assert!(io.names.is_empty());
    assert_eq!(io.endpoints, [([5, 6, 7, 8], 81)]);
    assert_eq!(job.status(), 0);
    assert!(peer.reusable);
}
#[test]
fn www20_p3_zero_and_80_connect_same_port_with_different_reuse_keys() {
    let (mut client, mut job, mut peer) = fixture(b"http://origin.test:0/a");
    let mut io = Io::default();
    one_byte(&mut io);
    execute(&mut client, &mut io, &mut job, &mut peer, 100, None).unwrap();
    assert_eq!(peer.port, 0);
    client.requests.get_mut(&7).unwrap().url = b"http://origin.test:80/a".to_vec();
    one_byte(&mut io);
    execute(&mut client, &mut io, &mut job, &mut peer, 100, None).unwrap();
    assert_eq!(io.endpoints, [([1, 2, 3, 4], 80), ([1, 2, 3, 4], 80)]);
    assert_eq!((peer.port, io.closes), (80, 2));
}
#[test]
fn www20_p4_proxy_endpoint_change_preserves_origin_reuse() {
    let (mut client, mut job, mut peer) = fixture(b"http://origin.test/a");
    let mut io = Io::default();
    one_byte(&mut io);
    execute_with_proxy(
        &mut client,
        &mut io,
        &mut job,
        &mut peer,
        100,
        Some(proxy(b"proxy.test", 3128)),
    )
    .unwrap();
    one_byte(&mut io);
    execute_with_proxy(
        &mut client,
        &mut io,
        &mut job,
        &mut peer,
        100,
        Some(proxy(b"other.test", 9000)),
    )
    .unwrap();
    assert_eq!((io.connections, io.resolutions, io.closes), (1, 1, 1));
}
fn admitted() -> (Client, Io, u32, u32, Peer) {
    let mut client = Client::new(None);
    client.target = Some(99);
    let mut io = Io::default();
    client
        .submit_headers(&mut io, 2, b"http://origin.test/a", &[], 0x1234)
        .unwrap();
    let worker = client.active.unwrap();
    let request = client.queue[0].0[5];
    client.dequeue(&mut io, worker, 100).unwrap().unwrap();
    (client, io, worker, request, Peer::default())
}
fn releases(io: &Io) -> Vec<http::Effect> {
    io.effects
        .iter()
        .filter(|e| matches!(e, http::Effect::ReleaseRequest(_) | http::Effect::ReleaseResource(_)))
        .cloned()
        .collect()
}
#[test]
fn www20_r1_dequeue_preserves_request_and_custom_without_retain() {
    let (mut client, mut io, worker, request, _) = admitted();
    let current = client.workers.get_mut(&worker).unwrap().current.take().unwrap();
    client.queue.push_back(current);
    io.effects.clear();
    let job = client.dequeue(&mut io, worker, 1000).unwrap().unwrap();
    assert_eq!(job.0, [1, 1, 1000, 1, 0, request, 0, 0x1234]);
    assert_eq!(client.requests[&request].refs, 1);
    assert_eq!(
        io.effects,
        [
            http::Effect::Lock,
            http::Effect::FreeNode(current),
            http::Effect::Unlock
        ]
    );
}
#[test]
fn www20_r2_lookup_failure_completes_with_null_response() {
    let (mut client, mut io, worker, request, mut peer) = admitted();
    io.resolve = None;
    io.effects.clear();
    run_current(&mut client, &mut io, worker, &mut peer, None).unwrap();
    assert_eq!(io.observed[0].0.status(), 3);
    assert!(io.observed[0].1.is_some() && io.observed[0].2.is_none());
    assert_eq!(releases(&io), [http::Effect::ReleaseRequest(request)]);
    assert!(client.requests.is_empty() && client.resources.is_empty());
    let notify = io
        .effects
        .iter()
        .position(|e| matches!(e, http::Effect::Notify(..)))
        .unwrap();
    let release = io
        .effects
        .iter()
        .position(|e| matches!(e, http::Effect::ReleaseRequest(_)))
        .unwrap();
    assert!(notify < release);
}
#[test]
fn www20_r3_response_is_distinct_and_held_before_finalization() {
    let (mut client, mut io, worker, request, mut peer) = admitted();
    one_byte(&mut io);
    let mut job = client.workers[&worker].current.unwrap();
    execute(&mut client, &mut io, &mut job, &mut peer, 100, None).unwrap();
    assert_eq!(job.0[5], request);
    assert_ne!(job.0[6], request);
    let response = &client.resources[&job.0[6]];
    assert_eq!(response.refs, 1);
    assert_eq!(response.body, b"a");
    assert!(response.strings.iter().all(Vec::is_empty));
    assert!(io.observed.is_empty());
}
#[test]
fn www20_r4_metadata_and_response_data_preserved_through_callback() {
    let (mut client, mut io, worker, request, mut peer) = admitted();
    one_byte(&mut io);
    let req = client.requests.get_mut(&request).unwrap();
    req.original_url = b"http://old/a".to_vec();
    req.redirects = 4;
    io.effects.clear();
    run_current(&mut client, &mut io, worker, &mut peer, None).unwrap();
    let (job, req, res) = &io.observed[0];
    let req = req.as_ref().unwrap();
    let res = res.as_ref().unwrap();
    assert_eq!(
        res.strings,
        [req.method.clone(), req.url.clone(), req.original_url.clone()]
    );
    assert_eq!((req.refs, res.refs, res.redirects, job.0[4]), (1, 1, 4, 100));
    assert_eq!((res.major, res.minor), (1, 1));
    assert_eq!(res.body, b"a");
    assert_eq!(res.headers[1], (b"Content-Length".to_vec(), b"1".to_vec()));
    assert_eq!(
        releases(&io),
        [
            http::Effect::ReleaseRequest(request),
            http::Effect::ReleaseResource(job.0[6])
        ]
    );
    assert!(client.requests.is_empty() && client.resources.is_empty());
    assert!(client.workers[&worker].current.is_none());
}
#[test]
fn callback_retained_resources_survive_worker_completion() {
    let (mut client, mut io, worker, request, mut peer) = admitted();
    one_byte(&mut io);
    io.retain_resources = true;
    io.effects.clear();
    run_current(&mut client, &mut io, worker, &mut peer, None).unwrap();

    let (job, observed_request, observed_response) = &io.observed[0];
    let response = job.0[6];
    assert_eq!(observed_request.as_ref().unwrap().refs, 1);
    assert_eq!(observed_response.as_ref().unwrap().refs, 1);
    assert_eq!(client.requests[&request].refs, 1);
    assert_eq!(client.resources[&response].refs, 1);
    assert_eq!(client.resources[&response].body, b"a");
    assert!(client.workers[&worker].current.is_none());
    let notified = io
        .effects
        .iter()
        .position(|e| matches!(e, http::Effect::Notify(..)))
        .unwrap();
    assert_eq!(
        &io.effects[notified + 1..notified + 3],
        [http::Effect::AddRequest(request), http::Effect::AddResponse(response)]
    );
    assert_eq!(
        releases(&io),
        [
            http::Effect::ReleaseRequest(request),
            http::Effect::ReleaseResource(response)
        ]
    );
    assert!(!io
        .effects
        .iter()
        .any(|effect| matches!(effect, http::Effect::DeleteRequest(_) | http::Effect::DeleteResource(_))));

    // HTTP lifetime §2.1: callers own only references acquired during notification.
    assert_eq!(client.release_request(&mut io, request).unwrap(), 0);
    client.release_resource(&mut io, response).unwrap();
    assert!(client.requests.is_empty() && client.resources.is_empty());
    assert_eq!(
        &io.effects[io.effects.len() - 4..],
        [
            http::Effect::ReleaseRequest(request),
            http::Effect::DeleteRequest(request),
            http::Effect::ReleaseResource(response),
            http::Effect::DeleteResource(response)
        ]
    );
}
#[test]
fn callback_can_retain_request_when_response_is_null() {
    let (mut client, mut io, worker, request, mut peer) = admitted();
    io.resolve = None;
    io.retain_resources = true;
    io.effects.clear();
    run_current(&mut client, &mut io, worker, &mut peer, None).unwrap();

    let (job, observed_request, observed_response) = &io.observed[0];
    assert_eq!(job.status(), 3);
    assert_eq!(observed_request.as_ref().unwrap().refs, 1);
    assert!(observed_response.is_none());
    assert_eq!(client.requests[&request].refs, 1);
    assert!(client.resources.is_empty());
    assert!(client.workers[&worker].current.is_none());
    let notified = io
        .effects
        .iter()
        .position(|e| matches!(e, http::Effect::Notify(..)))
        .unwrap();
    assert_eq!(
        &io.effects[notified + 1..notified + 5],
        [
            http::Effect::AddRequest(request),
            http::Effect::ReleaseMessage,
            http::Effect::ReleaseClient,
            http::Effect::ReleaseRequest(request)
        ]
    );
    assert_eq!(client.release_request(&mut io, request).unwrap(), 0);
    assert!(client.requests.is_empty());
}
#[test]
fn www20_r5_zero_status_leaks_unpublished_local_response() {
    let (mut client, mut io, worker, request, mut peer) = admitted();
    io.reads = [ReadStep::Data(b"HTTP/1.1 0 bad\r\n\r\n".to_vec())].into();
    io.effects.clear();
    run_current(&mut client, &mut io, worker, &mut peer, None).unwrap();
    assert_eq!(io.observed[0].0.status(), 6);
    assert!(io.observed[0].2.is_none());
    assert_eq!(client.resources.len(), 1);
    assert_eq!(client.resources.values().next().unwrap().refs, 1);
    assert_eq!(releases(&io), [http::Effect::ReleaseRequest(request)]);
    assert!(!io.effects.iter().any(|e| matches!(e, http::Effect::PublishResponse(_))));
}
#[test]
fn www20_r6_body_failure_requeues_shallow_record_and_leaks_local_response() {
    let (mut client, mut io, worker, request, mut peer) = admitted();
    io.reads = [
        ReadStep::Data(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n".to_vec()),
        ReadStep::Failed,
    ]
    .into();
    io.effects.clear();
    assert_eq!(
        run_current(&mut client, &mut io, worker, &mut peer, None).unwrap(),
        Outcome::Requeued
    );
    assert_eq!(client.queue[0].0, [1, 1, 100, 2, 0, request, 0, 0x1234]);
    assert_eq!(client.resources.values().next().unwrap().refs, 1);
    assert!(io.observed.is_empty() && releases(&io).is_empty());
    assert!(!io
        .effects
        .iter()
        .any(|e| matches!(e, http::Effect::AddRequest(_) | http::Effect::PublishResponse(_))));
}
fn redirect_attempt(client: &mut Client, io: &mut Io, worker: u32, peer: &mut Peer) -> u32 {
    io.reads = [ReadStep::Data(
        b"HTTP/1.1 302 Found\r\nLocation: http://next.test/b\r\nContent-Length: 1\r\n\r\nr".to_vec(),
    )]
    .into();
    assert_eq!(run_current(client, io, worker, peer, None).unwrap(), Outcome::Requeued);
    let response = client.queue[0].0[6];
    assert_ne!(response, 0);
    client.dequeue(io, worker, 101).unwrap().unwrap();
    response
}
#[test]
fn www20_r7_redirect_response_survives_next_lookup_failure() {
    let (mut client, mut io, worker, request, mut peer) = admitted();
    let response = redirect_attempt(&mut client, &mut io, worker, &mut peer);
    io.resolve = None;
    io.effects.clear();
    run_current(&mut client, &mut io, worker, &mut peer, None).unwrap();
    let (job, _, res) = &io.observed[0];
    assert_eq!((job.status(), job.0[6]), (3, response));
    let res = res.as_ref().unwrap();
    assert_eq!(res.strings[1], b"http://next.test/b");
    assert_eq!(res.strings[2], b"http://origin.test/a");
    assert_eq!((res.redirects, res.body.as_slice()), (1, b"r".as_slice()));
    assert_eq!(
        releases(&io),
        [
            http::Effect::ReleaseRequest(request),
            http::Effect::ReleaseResource(response)
        ]
    );
}
#[test]
fn callback_can_retain_redirect_response_after_lookup_failure() {
    let (mut client, mut io, worker, request, mut peer) = admitted();
    let response = redirect_attempt(&mut client, &mut io, worker, &mut peer);
    io.resolve = None;
    io.retain_resources = true;
    io.effects.clear();
    run_current(&mut client, &mut io, worker, &mut peer, None).unwrap();

    // HTTP lifetime §2.1: an earlier published response stays valid on early failure.
    assert_eq!(io.observed.len(), 1);
    let (job, observed_request, observed_response) = &io.observed[0];
    assert_eq!((job.status(), job.0[6], job.0[4]), (3, response, 100));
    assert_eq!(observed_request.as_ref().unwrap().refs, 1);
    assert_eq!(observed_response.as_ref().unwrap().refs, 1);
    assert_eq!(client.requests[&request].refs, 1);
    assert_eq!(client.resources[&response].refs, 1);
    assert!(client.workers[&worker].current.is_none());
    assert_eq!(
        releases(&io),
        [
            http::Effect::ReleaseRequest(request),
            http::Effect::ReleaseResource(response)
        ]
    );

    // Copied response metadata and body outlive the caller's last request hold.
    assert_eq!(client.release_request(&mut io, request).unwrap(), 0);
    assert!(client.requests.is_empty());
    let retained = &client.resources[&response];
    assert_eq!(
        retained.strings,
        [
            b"GET".to_vec(),
            b"http://next.test/b".to_vec(),
            b"http://origin.test/a".to_vec()
        ]
    );
    assert_eq!((retained.redirects, retained.body.as_slice()), (1, b"r".as_slice()));
    client.release_resource(&mut io, response).unwrap();
    assert!(client.resources.is_empty());
    assert_eq!(
        &io.effects[io.effects.len() - 4..],
        [
            http::Effect::ReleaseRequest(request),
            http::Effect::DeleteRequest(request),
            http::Effect::ReleaseResource(response),
            http::Effect::DeleteResource(response)
        ]
    );
}
#[test]
fn www20_r8_new_response_overwrites_old_without_release() {
    let (mut client, mut io, worker, request, mut peer) = admitted();
    let old = redirect_attempt(&mut client, &mut io, worker, &mut peer);
    one_byte(&mut io);
    io.effects.clear();
    run_current(&mut client, &mut io, worker, &mut peer, None).unwrap();
    let current = io.observed[0].0 .0[6];
    assert_ne!(current, old);
    assert_eq!(client.resources[&old].refs, 1);
    assert!(!client.resources.contains_key(&current));
    assert_eq!(
        releases(&io),
        [
            http::Effect::ReleaseRequest(request),
            http::Effect::ReleaseResource(current)
        ]
    );
}
#[test]
fn www20_r9_stop_bypasses_callback_and_both_releases() {
    let (mut client, mut io, worker, request, mut peer) = admitted();
    one_byte(&mut io);
    let mut job = client.workers[&worker].current.unwrap();
    execute(&mut client, &mut io, &mut job, &mut peer, 100, None).unwrap();
    let record = client.workers.get_mut(&worker).unwrap();
    record.current = Some(job);
    record.stop = true;
    io.effects.clear();
    client.final_boundary(&mut io, worker).unwrap();
    assert!(io.observed.is_empty() && releases(&io).is_empty());
    assert_eq!(client.requests[&request].refs, 1);
    assert_eq!(client.resources.values().next().unwrap().refs, 1);
    assert!(client.workers[&worker].current.is_none());
    assert!(!client.workers[&worker].cancel);
    assert!(io
        .effects
        .iter()
        .any(|e| matches!(e, http::Effect::FinalBoundary(job, true) if job.status() == 7)));
}
#[test]
fn header_fragmentation_informational_status_zero_and_interrupts() {
    let mut io = Io {
        reads: [
            ReadStep::Pending,
            ReadStep::Data(Vec::new()),
            ReadStep::Data(b"HTTP/1.1 200 OK\r\nX: a\r\nX: b\r\n\r".to_vec()),
            ReadStep::Data(b"\nabc".to_vec()),
        ]
        .into(),
        ..Io::default()
    };
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
    let mut io = Io {
        sends: [Some(0), Some(3)].into(),
        reads: [ReadStep::Data(
            b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabcXY".to_vec(),
        )]
        .into(),
        ..Io::default()
    };
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
        let mut io = Io {
            reads: [
                ReadStep::Data([b"HTTP/".as_slice(), version, b" 200 OK\r\nContent-Length: 2\r\n\r\n"].concat()),
                ReadStep::Failed,
            ]
            .into(),
            ..Io::default()
        };
        let outcome = execute(&mut client, &mut io, &mut job, &mut peer, 100, None).unwrap();
        assert_eq!(matches!(outcome, Outcome::Requeued), retry);
        assert_eq!(client.queue.len(), usize::from(retry));
        assert_eq!(job.status(), if retry { 1 } else { 6 });
        assert!(!peer.initialized);
    }
    let (mut client, mut job, mut peer) = fixture(b"http://h/a");
    let mut io = Io {
        reads: [ReadStep::Data(
            b"HTTP/1.1 302 Found\r\nLocation: b\r\nContent-Length: 1\r\n\r\na".to_vec(),
        )]
        .into(),
        ..Io::default()
    };
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

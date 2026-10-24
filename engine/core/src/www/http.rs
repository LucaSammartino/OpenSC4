//! Reached HTTP admission/cancel bodies and bounded FOREIGN lifetime evidence.
//! IO remains injected; request normalization is byte preserving.
use super::{directory, Contract, Unsupported};
use std::collections::{BTreeMap, VecDeque};

const METHODS: [&[u8]; 10] = [
    b"OPTIONS", b"GET", b"HEAD", b"POST", b"PUT", b"DELETE", b"TRACE", b"PATCH", b"LINK", b"UNLINK",
];
/// 00817d00.
pub fn method_name(id: u32) -> Option<&'static [u8]> {
    id.checked_sub(1).and_then(|i| METHODS.get(i as usize).copied())
}
/// 00817bd0, C strncmp semantics, including zero length and embedded NUL.
pub fn method_id(input: &[u8], length: u32) -> u32 {
    for (i, method) in METHODS.iter().enumerate() {
        let mut equal = true;
        for n in 0..(length as usize).min(method.len().max(input.len()) + 1) {
            let a = input.get(n).copied().unwrap_or(0);
            let b = method.get(n).copied().unwrap_or(0);
            if a != b {
                equal = false;
                break;
            }
            if a == 0 {
                break;
            }
        }
        if equal {
            return i as u32 + 1;
        }
    }
    0
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Request {
    pub refs: u32,
    pub method: Vec<u8>,
    pub url: Vec<u8>,
    pub body: Vec<u8>,
    pub representation: u8,
    pub records: Vec<[u32; 6]>,
    pub headers: Vec<(Vec<u8>, Vec<u8>)>,
    pub raw_header: Vec<u8>,
    pub original_url: Vec<u8>,
    pub redirects: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resource {
    pub refs: u32,
    /// Method, current URL, original URL; never the response body (Q20).
    pub strings: [Vec<u8>; 3],
    pub redirects: u32,
    pub file: bool,
    pub raw_header: Vec<u8>,
    pub headers: Vec<(Vec<u8>, Vec<u8>)>,
    pub body: Vec<u8>,
    pub major: i32,
    pub minor: i32,
}
impl Default for Resource {
    fn default() -> Self {
        Self {
            refs: 0,
            strings: Default::default(),
            redirects: 0,
            file: false,
            raw_header: Vec::new(),
            headers: Vec::new(),
            body: Vec::new(),
            major: -1,
            minor: -1,
        }
    }
}
impl Resource {
    /// HTTP wire §1.1, CRT C-locale scanner, C-string bytes and wrapping atoi.
    pub fn status(&self) -> i32 {
        self.status_in_locale(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n' | 11 | 12))
    }
    /// Runtime CRT classification is a foreign input, not Unicode whitespace.
    pub fn status_in_locale(&self, whitespace: impl Fn(u8) -> bool) -> i32 {
        let Some((name, _)) = self.headers.first() else {
            return 0;
        };
        if name.len() >= 384 {
            return 0;
        }
        let text = name.split(|b| *b == 0).next().unwrap_or_default();
        let mut tokens = text.split(|b| whitespace(*b)).filter(|t| !t.is_empty());
        tokens.next();
        let Some(token) = tokens.next() else { return 0 };
        // The first byte is left untouched; truncate from the second byte onward.
        let end = token
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, b)| !b.is_ascii_digit())
            .map_or(token.len(), |(i, _)| i);
        super::cookies::decimal(&token[..end])
    }
    pub fn version(&mut self) -> (i32, i32) {
        if self.major == -1 {
            self.major = 0;
            self.minor = 0;
            if let Some((name, _)) = self.headers.first() {
                if name.len() >= 8 && name.starts_with(b"HTTP") {
                    if name[5].is_ascii_digit() {
                        self.major = i32::from(name[5] - b'0');
                    }
                    if name[7].is_ascii_digit() {
                        self.minor = i32::from(name[7] - b'0');
                    }
                }
            }
        }
        (self.major, self.minor)
    }
}
impl Request {
    pub fn normalize(&mut self) {
        if !self.raw_header.is_empty() && self.headers.is_empty() && self.records.is_empty() {
            self.headers = parse_records(&self.raw_header);
            self.raw_header.clear();
        }
        let method = method_id(&self.method, u32::MAX);
        if method != 0 && method != 1 {
            if self.headers.first().is_some_and(|p| p.1.is_empty()) {
                self.headers.remove(0);
            } else if self.records.first().is_some_and(|r| r[3] == r[4]) {
                self.records.remove(0);
            }
        }
    }
}
/// Shared literal byte parser (§4.14), including space-only folded lines.
pub fn parse_records(text: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut out = Vec::new();
    let (mut name, mut value) = (Vec::new(), Vec::new());
    let mut in_value = false;
    let mut i = 0;
    while i < text.len() {
        let b = text[i];
        if matches!(b, b'\r' | b'\n') {
            while i < text.len() && matches!(text[i], b'\r' | b'\n') {
                i += 1;
            }
            if text.get(i) == Some(&b' ') {
                in_value = true;
                while i < text.len() && matches!(text[i], b' ' | b'\t') {
                    i += 1;
                }
            } else {
                if !name.is_empty() {
                    out.push((std::mem::take(&mut name), std::mem::take(&mut value)));
                }
                value.clear();
                in_value = false;
            }
            continue;
        }
        if (b == b':' && value.is_empty()) || (!in_value && name.is_empty() && matches!(b, b' ' | b'\t')) {
            in_value = true;
            i += 1;
            while i < text.len() && matches!(text[i], b' ' | b'\t') {
                i += 1;
            }
            continue;
        }
        if in_value {
            value.push(b);
        } else {
            name.push(b);
        }
        i += 1;
    }
    if !name.is_empty() && !value.is_empty() {
        out.push((name, value));
    }
    out
}
/// Exact eight-word copied record of 00818100. Words 5/6 are identities, not Rust ownership.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Job(pub [u32; 8]);
impl Job {
    pub fn id(self) -> u32 {
        self.0[0]
    }
    pub fn status(self) -> u32 {
        self.0[1]
    }
}
#[derive(Clone, Debug, Default)]
pub struct Worker {
    pub running: bool,
    pub stop: bool,
    pub cancel: bool,
    pub current: Option<Job>,
    pub socket: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    Lock,
    Unlock,
    ConstructRequest(u32),
    ConstructResponse(u32),
    AddResponse(u32),
    PublishResponse(u32),
    FinalBoundary(Job, bool),
    CopyHeaders(Vec<(Vec<u8>, Vec<u8>)>),
    AddRequest(u32),
    ReleaseRequest(u32),
    DeleteRequest(u32),
    AllocateNode(Job),
    FreeNode(Job),
    AddClient,
    StartWorker(u32),
    ReleaseClient,
    CreateMessage,
    AddMessage,
    ReleaseMessage,
    Notify(Job, bool),
    ReleaseResource(u32),
    DeleteResource(u32),
    CloseSocket(u32),
    DestroySocket(u32),
    FreeWorker(u32),
    AddTarget(u32),
    ReleaseTarget(u32),
    RestoreVtable,
    LogIsOpen(u32),
    CloseLog(u32),
    DestroyLog(u32, u32),
    DeleteLookup(u32, u32),
    DestroyMember(&'static str),
    FreeClient,
}
pub trait Host {
    fn effect(&mut self, effect: Effect);
    /// Resources still held during this callback; arbitrary supported reentry is possible.
    fn notify(&mut self, client: &mut Client, job: Job, success: bool);
}

/// 00818210 logical state. Native table/address offsets are not portable pointers.
#[derive(Clone, Debug)]
pub struct Client {
    pub references: u32,
    pub receive_log: Option<(u32, bool)>,
    pub send_log: Option<(u32, bool)>,
    pub initialized: bool,
    pub target: Option<u32>,
    pub counter: u32,
    pub suffix: Vec<u8>,
    pub timeout: u32,
    pub strings: [Vec<u8>; 6],
    /// Logical settings from §4.9; no unqualified native string-offset mapping.
    pub directory: Vec<u8>,
    pub cache_setting: Vec<u8>,
    pub lookup: Option<u32>,
    pub address: super::ftp::Address,
    pub queue: VecDeque<Job>,
    pub active: Option<u32>,
    pub workers: BTreeMap<u32, Worker>,
    pub requests: BTreeMap<u32, Request>,
    pub resources: BTreeMap<u32, Resource>,
    next_identity: u32,
}
impl Client {
    pub fn init(&mut self) -> bool {
        self.initialized = true;
        true
    }
    pub fn new(allocated_lookup: Option<u32>) -> Self {
        Self {
            references: 0,
            receive_log: None,
            send_log: None,
            initialized: false,
            target: None,
            counter: 0,
            suffix: b".tmp".to_vec(),
            timeout: 15,
            strings: Default::default(),
            directory: Vec::new(),
            cache_setting: Vec::new(),
            lookup: allocated_lookup,
            address: Default::default(),
            queue: VecDeque::new(),
            active: None,
            workers: BTreeMap::new(),
            requests: BTreeMap::new(),
            resources: BTreeMap::new(),
            next_identity: 1,
        }
    }
    pub fn query_interface(&mut self, host: &mut impl Host, identity: u32, iid: u32, output: &mut u32) -> bool {
        if iid != 1 {
            return false;
        }
        *output = identity;
        self.references = self.references.wrapping_add(1);
        host.effect(Effect::AddClient);
        true
    }
    /// HTTP setter deliberately retains/releases even an identical target.
    pub fn set_target(&mut self, host: &mut impl Host, target: Option<u32>) {
        if let Some(target) = target {
            host.effect(Effect::AddTarget(target));
        }
        if let Some(old) = self.target {
            host.effect(Effect::ReleaseTarget(old));
        }
        self.target = target;
    }
    /// Open-state booleans are foreign IsOpen replies for the two supplied handles.
    pub fn destroy(&mut self, host: &mut impl Host, flags: u32) {
        host.effect(Effect::RestoreVtable);
        self.shutdown(host);
        if let Some(target) = self.target.take() {
            host.effect(Effect::ReleaseTarget(target));
        }
        for log in [self.receive_log.take(), self.send_log.take()].into_iter().flatten() {
            host.effect(Effect::LogIsOpen(log.0));
            if log.1 {
                host.effect(Effect::CloseLog(log.0));
            }
            host.effect(Effect::DestroyLog(log.0, 1));
        }
        if let Some(lookup) = self.lookup.take() {
            host.effect(Effect::DeleteLookup(lookup, 1));
        }
        for member in [
            "proxy +c8",
            "queue +b0",
            "critical-section +a4",
            "thread +9c",
            "string +80",
            "string +6c",
            "string +54",
            "string +40",
            "string +28",
            "string +14",
        ] {
            host.effect(Effect::DestroyMember(member));
        }
        self.queue.clear();
        self.strings = Default::default();
        if flags & 1 != 0 {
            host.effect(Effect::FreeClient);
        }
    }
    fn identity(&mut self) -> u32 {
        loop {
            let id = self.next_identity;
            self.next_identity = id.wrapping_add(1);
            if id != 0
                && !self.requests.contains_key(&id)
                && !self.resources.contains_key(&id)
                && !self.workers.contains_key(&id)
            {
                return id;
            }
        }
    }
    /// 00819650: copy and normalize only a nonempty directory.
    pub fn set_directory(&mut self, path: &[u8]) {
        self.directory = directory(path);
    }
    /// 008196b0: length-bearing assignment, without path normalization.
    pub fn set_cache_setting(&mut self, value: &[u8]) {
        self.cache_setting = value.to_vec();
    }
    /// 00817790: zero and one both invoke deleting destructor; no zero guard.
    pub fn release_request(&mut self, host: &mut impl Host, id: u32) -> Contract<u32> {
        let request = self
            .requests
            .get_mut(&id)
            .ok_or(Unsupported::NativeInputDomain("request identity"))?;
        host.effect(Effect::ReleaseRequest(id));
        if request.refs > 1 {
            request.refs -= 1;
            Ok(request.refs)
        } else {
            host.effect(Effect::DeleteRequest(id));
            self.requests.remove(&id);
            Ok(0)
        }
    }
    /// 0081cda0: wrappers 8/9/10 reject; PUT alone copies a mandatory body.
    #[allow(clippy::too_many_arguments)] // Preserve the published submission argument channels.
    pub fn submit(
        &mut self,
        host: &mut impl Host,
        method: u32,
        url: &[u8],
        body: Option<&[u8]>,
        representation: u8,
        records: &[[u32; 6]],
        custom: u32,
    ) -> Contract<u32> {
        self.submit_inner(host, method, url, body, representation, records, &[], custom)
    }
    pub fn submit_headers(
        &mut self,
        host: &mut impl Host,
        method: u32,
        url: &[u8],
        headers: &[(Vec<u8>, Vec<u8>)],
        custom: u32,
    ) -> Contract<u32> {
        self.submit_inner(host, method, url, None, 0, &[], headers, custom)
    }
    #[allow(clippy::too_many_arguments)]
    fn submit_inner(
        &mut self,
        host: &mut impl Host,
        method: u32,
        url: &[u8],
        body: Option<&[u8]>,
        representation: u8,
        records: &[[u32; 6]],
        headers: &[(Vec<u8>, Vec<u8>)],
        custom: u32,
    ) -> Contract<u32> {
        if matches!(method, 8..=10) {
            return Ok(0);
        }
        let name = method_name(method).ok_or(Unsupported::NativeInputDomain("method converter returned null"))?;
        let id = self.identity();
        host.effect(Effect::ConstructRequest(id));
        let mut request = Request {
            refs: 1,
            method: name.to_vec(),
            url: url.to_vec(),
            ..Request::default()
        };
        host.effect(Effect::AddRequest(id));
        if method == 5 {
            let Some(body) = body else {
                self.requests.insert(id, request);
                self.release_request(host, id)?;
                return Ok(0);
            };
            request.body = body.to_vec();
            request.representation = representation;
        }
        request.records.extend_from_slice(records);
        request.headers.extend_from_slice(headers);
        if !headers.is_empty() {
            host.effect(Effect::CopyHeaders(request.headers.clone()));
        }
        self.requests.insert(id, request);
        let result = self.admit(host, Some(id), custom);
        self.release_request(host, id)?;
        result
    }
    /// 0081ca90: raw-header parsing and first-empty-value erasure precede queueing.
    pub fn admit(&mut self, host: &mut impl Host, request: Option<u32>, custom: u32) -> Contract<u32> {
        let Some(id) = request else {
            return Ok(0);
        };
        let req = self
            .requests
            .get_mut(&id)
            .ok_or(Unsupported::NativeInputDomain("request identity"))?;
        if req.method.is_empty() || req.url.is_empty() {
            return Ok(0);
        }
        req.normalize();
        req.refs = req.refs.wrapping_add(1);
        host.effect(Effect::AddRequest(id));
        host.effect(Effect::Lock);
        self.counter = self.counter.wrapping_add(1);
        let job = Job([self.counter, 1, 0, 1, 0, id, 0, custom]);
        self.append(host, job);
        host.effect(Effect::Unlock);
        self.activate(host);
        Ok(job.id())
    }
    /// 00818100, no field-specific ownership operation.
    pub fn append(&mut self, host: &mut impl Host, job: Job) {
        host.effect(Effect::AllocateNode(job));
        self.queue.push_back(job);
    }
    /// Lifetime §2.1: unlink only the node, publish a shallow copy, timestamp it.
    pub fn dequeue(&mut self, host: &mut impl Host, worker: u32, now: u32) -> Contract<Option<Job>> {
        let record = self
            .workers
            .get_mut(&worker)
            .ok_or(Unsupported::NativeInputDomain("worker identity"))?;
        if record.current.is_some() {
            return Err(Unsupported::NativeInputDomain("dequeue with current job"));
        }
        host.effect(Effect::Lock);
        let job = self.queue.pop_front().map(|mut job| {
            host.effect(Effect::FreeNode(job));
            job.0[2] = now;
            record.current = Some(job);
            job
        });
        host.effect(Effect::Unlock);
        Ok(job)
    }
    pub fn create_response(&mut self, host: &mut impl Host) -> u32 {
        let id = self.identity();
        host.effect(Effect::ConstructResponse(id));
        self.resources.insert(
            id,
            Resource {
                refs: 1,
                ..Resource::default()
            },
        );
        host.effect(Effect::AddResponse(id));
        id
    }
    /// Reached foreign activation dependency, not another WWW inventory row.
    pub fn activate(&mut self, host: &mut impl Host) {
        host.effect(Effect::Lock);
        let id = if let Some(id) = self.active {
            id
        } else {
            let id = self.identity();
            self.workers.insert(id, Worker::default());
            self.active = Some(id);
            id
        };
        let worker = self.workers.get_mut(&id).expect("active worker snapshot");
        worker.stop = false;
        if !worker.running {
            worker.running = true;
            host.effect(Effect::AddClient);
            host.effect(Effect::StartWorker(id));
        }
        host.effect(Effect::Unlock);
    }
    /// 0081cc10: queued removal wins; it leaks the queue's request hold.
    pub fn cancel(&mut self, host: &mut impl Host, id: u32) -> bool {
        host.effect(Effect::Lock);
        if let Some(position) = self.queue.iter().position(|j| j.id() == id) {
            let job = self.queue.remove(position).expect("located queue job");
            host.effect(Effect::FreeNode(job));
            host.effect(Effect::Unlock);
            return true;
        }
        let active = self.active.and_then(|w| self.workers.get(&w).map(|r| (w, r.current)));
        if let Some((worker, Some(job))) = active {
            if job.id() == id {
                let record = self.workers.get_mut(&worker).expect("located worker");
                record.stop = true;
                record.cancel = true;
                self.active = None;
                if !self.queue.is_empty() {
                    self.activate(host);
                }
                host.effect(Effect::Unlock);
                return true;
            }
        }
        host.effect(Effect::Unlock);
        false
    }
    /// Release a completion or caller-owned response hold (HTTP lifetime §2.1).
    pub fn release_resource(&mut self, host: &mut impl Host, id: u32) -> Contract<()> {
        if id == 0 {
            return Ok(());
        }
        let resource = self
            .resources
            .get_mut(&id)
            .ok_or(Unsupported::NativeInputDomain("resource identity"))?;
        host.effect(Effect::ReleaseResource(id));
        if resource.refs > 1 {
            resource.refs -= 1;
        } else {
            host.effect(Effect::DeleteResource(id));
            self.resources.remove(&id);
        }
        Ok(())
    }
    /// Lifetime §2.1: metadata copy, callback, then request and response releases.
    pub fn complete(&mut self, host: &mut impl Host, mut job: Job) -> Contract<()> {
        let internal = job.0[5];
        let external = job.0[6];
        if internal != 0 && external != 0 {
            let source = self
                .requests
                .get(&internal)
                .ok_or(Unsupported::NativeInputDomain("completion request"))?
                .clone();
            let destination = self
                .resources
                .get_mut(&external)
                .ok_or(Unsupported::NativeInputDomain("external resource"))?;
            destination.strings = [source.method, source.url, source.original_url];
            destination.redirects = source.redirects;
        }
        job.0[4] = 100;
        if self.target.is_some() {
            host.effect(Effect::AddClient);
            host.effect(Effect::CreateMessage);
            host.effect(Effect::AddMessage);
            host.effect(Effect::Notify(job, job.status() == 0));
            host.notify(self, job, job.status() == 0);
            host.effect(Effect::ReleaseMessage);
            host.effect(Effect::ReleaseClient);
        }
        if internal != 0 {
            self.release_request(host, internal)?;
        }
        self.release_resource(host, external)?;
        Ok(())
    }
    /// FOREIGN worker final boundary only. Stop skips the completion AND its releases.
    pub fn final_boundary(&mut self, host: &mut impl Host, worker: u32) -> Contract<()> {
        let record = self
            .workers
            .get_mut(&worker)
            .ok_or(Unsupported::NativeInputDomain("worker identity"))?;
        let Some(mut job) = record.current else {
            return Err(Unsupported::NativeInputDomain("current job"));
        };
        if record.cancel || record.stop {
            record.cancel = false;
            job.0[1] = 7;
        }
        let stop = record.stop;
        if !stop {
            job.0[4] = 100;
        }
        record.current = Some(job);
        host.effect(Effect::FinalBoundary(job, stop));
        if !stop {
            self.complete(host, job)?;
        }
        self.workers.get_mut(&worker).expect("worker still alive").current = None;
        Ok(())
    }
    /// FOREIGN stop/drain/shutdown boundaries. No request release or join.
    pub fn stop(&mut self, host: &mut impl Host) {
        host.effect(Effect::Lock);
        if let Some(id) = self.active.take() {
            if let Some(worker) = self.workers.get_mut(&id) {
                worker.cancel = true;
                worker.stop = true;
            }
            if !self.queue.is_empty() {
                self.activate(host);
            }
        }
        host.effect(Effect::Unlock);
    }
    pub fn drain(&mut self, host: &mut impl Host) {
        host.effect(Effect::Lock);
        self.stop(host);
        for job in self.queue.drain(..) {
            host.effect(Effect::FreeNode(job));
        }
        host.effect(Effect::Unlock);
    }
    pub fn shutdown(&mut self, host: &mut impl Host) -> bool {
        if !self.initialized {
            return true;
        }
        host.effect(Effect::Lock);
        if let Some(socket) = self
            .active
            .and_then(|id| self.workers.get(&id))
            .filter(|r| r.running)
            .map(|r| r.socket)
        {
            self.drain(host);
            host.effect(Effect::CloseSocket(socket));
        }
        self.initialized = false;
        host.effect(Effect::Unlock);
        true
    }
    pub fn worker_exit(&mut self, host: &mut impl Host, id: u32) -> Contract<()> {
        host.effect(Effect::Lock);
        self.active = None;
        let record = self
            .workers
            .remove(&id)
            .ok_or(Unsupported::NativeInputDomain("worker identity"))?;
        // Socket IO stays foreign; the qualified lifetime sequence is owned here.
        host.effect(Effect::CloseSocket(record.socket));
        host.effect(Effect::DestroySocket(record.socket));
        host.effect(Effect::FreeWorker(id));
        host.effect(Effect::Unlock);
        host.effect(Effect::ReleaseClient);
        Ok(())
    }
}

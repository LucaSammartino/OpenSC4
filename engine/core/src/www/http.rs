//! Reached HTTP admission/cancel bodies and bounded FOREIGN lifetime evidence.
//! No HTTP worker transport, resource parser, normalization or destructor algorithm.
use super::{Contract, Unsupported};
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
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Resource {
    pub refs: u32,
    pub strings: [Vec<u8>; 3],
    pub redirects: u32,
    pub file: bool,
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
    AddRequest(u32),
    ReleaseRequest(u32),
    DeleteRequest(u32),
    AllocateNode(Job),
    FreeNode(Job),
    AddClient,
    StartWorker(u32),
    ReleaseClient,
    Notify(Job, bool),
    ReleaseResource(u32),
    DeleteResource(u32),
    CloseSocket(u32),
    DestroySocket(u32),
    FreeWorker(u32),
}
pub trait Host {
    fn effect(&mut self, effect: Effect);
    /// Resources still held during this callback; arbitrary supported reentry is possible.
    fn notify(&mut self, client: &mut Client, job: Job, success: bool);
}

/// 00818210 logical state. Native table/address offsets are not portable pointers.
#[derive(Clone, Debug)]
pub struct Client {
    pub initialized: bool,
    pub target: Option<u32>,
    pub counter: u32,
    pub suffix: Vec<u8>,
    pub timeout: u32,
    pub strings: [Vec<u8>; 6],
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
    pub fn new(allocated_lookup: Option<u32>) -> Self {
        Self {
            initialized: false,
            target: None,
            counter: 0,
            suffix: b".tmp".to_vec(),
            timeout: 15,
            strings: Default::default(),
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
    fn identity(&mut self) -> u32 {
        let id = self.next_identity;
        self.next_identity = id.wrapping_add(1);
        id
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
        self.requests.insert(id, request);
        let result = self.admit(host, Some(id), custom);
        self.release_request(host, id)?;
        result
    }
    /// 0081ca90. Normalization helper calls are explicit owned O barriers.
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
        let method = method_id(&req.method, u32::MAX);
        if !req.body.is_empty() && req.records.is_empty() {
            return Err(Unsupported::OwnedBody("0081acf0 request normalization"));
        }
        if method != 0 && method != 1 && req.records.first().is_some_and(|r| r[3] == r[4]) {
            return Err(Unsupported::OwnedBody("0081ac20 first record normalization"));
        }
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
    fn release_resource(&mut self, host: &mut impl Host, id: u32) -> Contract<()> {
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
    /// FOREIGN 008196e0: copy content, callback, then internal/external releases.
    pub fn complete(&mut self, host: &mut impl Host, mut job: Job) -> Contract<()> {
        let internal = job.0[4];
        let external = job.0[6];
        if internal != 0 && external != 0 {
            let source = self
                .resources
                .get(&internal)
                .ok_or(Unsupported::NativeInputDomain("internal resource"))?
                .clone();
            let destination = self
                .resources
                .get_mut(&external)
                .ok_or(Unsupported::NativeInputDomain("external resource"))?;
            destination.strings = source.strings;
            destination.redirects = source.redirects;
        }
        job.0[2] = 100;
        if self.target.is_some() {
            host.effect(Effect::Notify(job, job.status() == 0));
            host.notify(self, job, job.status() == 0);
        }
        self.release_resource(host, internal)?;
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
        record.current = Some(job);
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
            .get(&id)
            .ok_or(Unsupported::NativeInputDomain("worker identity"))?;
        // The actual close adapter 00817af0 is O: retain the boundary, don't fake it.
        host.effect(Effect::Unlock);
        let _ = record;
        Err(Unsupported::OwnedBody("00817af0 worker socket close"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Fake {
        effects: Vec<Effect>,
        observed: Vec<(Job, Vec<u32>)>,
    }
    impl Host for Fake {
        fn effect(&mut self, effect: Effect) {
            self.effects.push(effect);
        }
        fn notify(&mut self, client: &mut Client, job: Job, _: bool) {
            self.observed
                .push((job, client.resources.values().map(|r| r.refs).collect()));
        }
    }
    #[test]
    fn method_prefix_and_wrapper_boundaries() {
        assert_eq!(method_id(b"GETaway", 1), 2);
        assert_eq!(method_id(b"GETaway", u32::MAX), 0);
        assert_eq!(method_id(b"anything", 0), 1);
        for id in 1..=10 {
            assert_eq!(method_id(method_name(id).unwrap(), u32::MAX), id);
        }
        let mut client = Client::new(None);
        let mut host = Fake::default();
        assert_eq!(client.submit(&mut host, 8, b"url", None, 0, &[], 0), Ok(0));
        assert!(host.effects.is_empty());
        assert_eq!(client.submit(&mut host, 5, b"url", None, 0, &[], 0), Ok(0));
        assert!(client.requests.is_empty());
        assert_eq!(client.admit(&mut host, None, 0), Ok(0));
        assert!(client.submit(&mut host, 5, b"url", Some(b"data"), 0, &[], 0).is_err());
    }
    #[test]
    fn admission_wrap_raw_cancel_leaks_and_first_match_wins() {
        let mut client = Client::new(None);
        let mut host = Fake::default();
        client.counter = u32::MAX;
        assert_eq!(client.submit(&mut host, 2, b"url", None, 0, &[], 8), Ok(0));
        let job = client.queue[0];
        assert_eq!(job.0[7], 8);
        assert_eq!(client.requests[&job.0[5]].refs, 1);
        client.append(&mut host, job);
        let worker = client.active.unwrap();
        client.workers.get_mut(&worker).unwrap().current = Some(job);
        assert!(client.cancel(&mut host, 0));
        assert_eq!(client.queue.len(), 1);
        assert!(!client.workers[&worker].stop);
        assert_eq!(client.requests[&job.0[5]].refs, 1);
        assert!(client.cancel(&mut host, 0));
        assert!(client.cancel(&mut host, 0));
        assert!(client.workers[&worker].stop);
        assert!(client.active.is_none());
        assert!(!client.cancel(&mut host, 99));
    }
    fn resources(client: &mut Client) -> Job {
        client.resources.insert(
            50,
            Resource {
                refs: 2,
                strings: [b"url".to_vec(), b"data".to_vec(), vec![]],
                redirects: 3,
                file: false,
            },
        );
        client.resources.insert(
            51,
            Resource {
                refs: 2,
                ..Resource::default()
            },
        );
        Job([9, 0, 0, 1, 50, 0, 51, 0])
    }
    #[test]
    fn cancellation_callback_before_release_and_stop_leaks() {
        for stop in [false, true] {
            let mut client = Client::new(None);
            client.target = Some(1);
            let mut host = Fake::default();
            let job = resources(&mut client);
            client.workers.insert(
                1,
                Worker {
                    running: true,
                    cancel: true,
                    stop,
                    current: Some(job),
                    socket: 0,
                },
            );
            client.final_boundary(&mut host, 1).unwrap();
            assert!(!client.workers[&1].cancel);
            assert!(client.workers[&1].current.is_none());
            if stop {
                assert!(host.observed.is_empty());
                assert_eq!(client.resources[&50].refs, 2);
            } else {
                assert_eq!(host.observed[0].1, [2, 2]);
                assert_eq!(host.observed[0].0.status(), 7);
                assert_eq!(client.resources[&50].refs, 1);
                assert_eq!(client.resources[&51].strings[1], b"data");
                assert_eq!(
                    host.effects
                        .iter()
                        .filter(|e| matches!(e, Effect::ReleaseResource(_)))
                        .cloned()
                        .collect::<Vec<_>>(),
                    [Effect::ReleaseResource(50), Effect::ReleaseResource(51)]
                );
            }
        }
    }
    #[test]
    fn shutdown_replacement_then_raw_drain_no_join() {
        let mut client = Client::new(None);
        let mut host = Fake::default();
        client.submit(&mut host, 2, b"url", None, 0, &[], 0).unwrap();
        let old = client.active.unwrap();
        assert!(client.shutdown(&mut host));
        assert_eq!(client.queue.len(), 1);
        client.initialized = true;
        client.workers.get_mut(&old).unwrap().socket = 88;
        assert!(client.shutdown(&mut host));
        let replacement = client.active.unwrap();
        assert_ne!(replacement, old);
        assert!(client.queue.is_empty());
        assert!(client.workers[&old].stop);
        assert!(host.effects.contains(&Effect::CloseSocket(88)));
        assert!(client.worker_exit(&mut host, old).is_err());
        assert!(client.active.is_none());
        assert!(client.workers.contains_key(&replacement));
    }
    #[test]
    fn zero_and_one_request_ref_delete() {
        let mut client = Client::new(None);
        let mut host = Fake::default();
        client.requests.insert(
            1,
            Request {
                refs: 2,
                ..Request::default()
            },
        );
        assert_eq!(client.release_request(&mut host, 1), Ok(1));
        assert_eq!(client.release_request(&mut host, 1), Ok(0));
        client.requests.insert(2, Request::default());
        assert_eq!(client.release_request(&mut host, 2), Ok(0));
        assert_eq!(client.timeout, 15);
        assert_eq!(client.suffix, b".tmp");
        assert_eq!(client.lookup, None);
    }
}

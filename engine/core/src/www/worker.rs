//! Published serial outer FTP worker and reference lifecycle (00816ee0).
//! Admission/control/container O bodies are not emulated. Queue snapshots are inputs.
use super::{ftp, leaf, Contract, Unsupported};
use crate::rzurl::{url_type, RzUrl};
use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    pub id: u32,
    pub status: u32,
    pub url: Vec<u8>,
    pub destination: Vec<u8>,
    pub time: u32,
    pub progress: u32,
    pub custom: u32,
}
/// 00812840 initializes strings/auxiliary words only, not leading ID/status.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EmptyValue {
    pub id: Option<u32>,
    pub status: Option<u32>,
    pub strings: [Vec<u8>; 2],
    pub auxiliary: [u32; 2],
}
#[derive(Clone, Debug, Default)]
pub struct Record {
    pub active: bool,
    pub stop: bool,
    pub cancel: bool,
    pub suspend: i32,
    pub current: Option<Job>,
}
#[derive(Clone, Debug)]
pub struct Scheduler {
    pub session: Option<ftp::Session>,
    pub initialized: bool,
    pub references: u32,
    pub counter: u32,
    pub lookup: Option<u32>,
    pub queue: VecDeque<Job>,
    pub record: Option<Record>,
    pub default_directory: Vec<u8>,
    pub timeout: u64,
    pub target: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    Lock,
    Unlock,
    Sleep(u32),
    AddScheduler,
    StartWorker,
    ReleaseScheduler,
    DestroyQueueNode,
    DestroyQueueStrings,
    PublishCurrent,
    ClearCurrent,
    FreeRecord,
    CreateMessage,
    AddMessage,
    ReleaseMessage,
    DestroyLocalStrings,
    DestroyUrl,
    AllocateQueueNode,
    DestroyCriticalSection,
    DestroySynchronization,
    DestroyStoredStrings,
    FreeScheduler,
    EraseFirstString,
    UnlinkQueueNode,
}
pub trait Host: ftp::Io {
    fn worker_effect(&mut self, effect: Effect);
    fn complete_job(&mut self, scheduler: &mut Scheduler, job: &Job, success: bool);
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Suspended,
    Dequeued,
    Exited,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new(None)
    }
}
impl Scheduler {
    /// 00813d70 is scheduler construction, not session initialization.
    pub fn new(lookup: Option<u32>) -> Self {
        Self {
            session: None,
            initialized: false,
            references: 0,
            counter: 0,
            lookup,
            queue: VecDeque::new(),
            record: None,
            default_directory: Vec::new(),
            timeout: 15,
            target: None,
        }
    }
    pub fn admit(&mut self, host: &mut impl Host, url: &[u8], destination: Option<&[u8]>, custom: u32) -> u32 {
        if url.is_empty() {
            return 0;
        }
        let destination = destination
            .filter(|d| !d.is_empty())
            .map_or_else(|| self.default_directory.clone(), super::directory);
        host.worker_effect(Effect::Lock);
        self.counter = self.counter.wrapping_add(1);
        let id = self.counter;
        self.queue.push_back(Job {
            id,
            status: 1,
            url: url.to_vec(),
            destination,
            time: 0,
            progress: 0,
            custom,
        });
        host.worker_effect(Effect::AllocateQueueNode);
        host.worker_effect(Effect::Unlock);
        self.start(host);
        host.worker_effect(Effect::DestroyLocalStrings);
        id
    }
    /// 00817580: no rollback if thread startup fails; caller owns the acquired hold.
    pub fn start(&mut self, host: &mut impl Host) {
        host.worker_effect(Effect::Lock);
        let record = self.record.get_or_insert_with(Record::default);
        record.stop = false;
        if !record.active {
            record.active = true;
            host.worker_effect(Effect::AddScheduler);
            host.worker_effect(Effect::StartWorker);
        }
        host.worker_effect(Effect::Unlock);
    }
    /// 00816ee0 loop prefix. Suspend uses literal 1000000 and rechecks stop next step.
    pub fn dequeue(&mut self, host: &mut impl Host) -> Contract<Step> {
        let record = self
            .record
            .as_mut()
            .ok_or(Unsupported::NativeInputDomain("FTP worker record"))?;
        if record.stop {
            self.exit(host);
            return Ok(Step::Exited);
        }
        if record.suspend > 0 {
            host.worker_effect(Effect::Sleep(1000000));
            return Ok(Step::Suspended);
        }
        host.worker_effect(Effect::Lock);
        if let Some(mut job) = self.queue.pop_front() {
            // 00813cb0 copies all words/strings; no target reference operation.
            host.worker_effect(Effect::DestroyQueueStrings);
            host.worker_effect(Effect::DestroyQueueNode);
            job.time = host.now();
            record.current = Some(job);
            host.worker_effect(Effect::PublishCurrent);
            host.worker_effect(Effect::Unlock);
            Ok(Step::Dequeued)
        } else {
            record.active = false;
            host.worker_effect(Effect::Unlock);
            self.exit(host);
            Ok(Step::Exited)
        }
    }
    fn exit(&mut self, host: &mut impl Host) {
        host.worker_effect(Effect::Lock);
        self.record = None;
        host.worker_effect(Effect::FreeRecord);
        host.worker_effect(Effect::Unlock);
        host.worker_effect(Effect::DestroyLocalStrings);
    }
    /// Fully owned queue/type decision; fresh FTP sessions reach the O init boundary.
    /// No supplied native-result callback can manufacture a completed RETR here.
    pub fn process_current(&mut self, host: &mut impl Host) -> Contract<()> {
        let job = self
            .record
            .as_ref()
            .and_then(|r| r.current.as_ref())
            .ok_or(Unsupported::NativeInputDomain("FTP current job"))?;
        let mut url = RzUrl::from_text(&job.url);
        if url.url_type() != url_type::FTP {
            return self.final_boundary(host);
        }
        let mut session = ftp::Session::default();
        let Some(host_address) = ftp::numeric_host(url.server()).or_else(|| ftp::lookup(host, url.server())) else {
            return self.final_boundary(host);
        };
        session.initialize()?;
        session.address = ftp::Address {
            host: host_address,
            port_word: (url.port() as u16).swap_bytes(),
        };
        self.transfer_current(host, &mut session, &mut url)
    }
    /// Configuration and transfer after the fresh-session initialized byte is set.
    pub fn transfer_current(
        &mut self,
        host: &mut impl Host,
        session: &mut ftp::Session,
        url: &mut RzUrl,
    ) -> Contract<()> {
        // Target identity 1 represents this scheduler. Add new before releasing old.
        host.effect(ftp::Effect::AddTarget(1));
        if let Some(old) = session.target.replace(1) {
            host.effect(ftp::Effect::ReleaseTarget(old));
        }
        session.timeout = (self.timeout as u32).wrapping_mul(1000);
        session.async_preference = false;
        session.requested_type = 203;
        session.pending_host.clear();
        if session.address.port_word == 0 {
            session.address.port_word = session.default_port;
        }
        let mut retrieved = false;
        if !self.flags()? && session.login(host)? && !self.flags()? {
            let job = self
                .record
                .as_ref()
                .and_then(|r| r.current.as_ref())
                .expect("current job");
            let remote = url.absolute_path().to_vec();
            let mut destination = if job.destination.is_empty() {
                self.default_directory.clone()
            } else {
                job.destination.clone()
            };
            destination.extend(leaf(&remote));
            retrieved = session.transfer(host, false, &remote, Some(&destination), None)?;
        }
        if session.state != 0 {
            session.disconnect(host, false)?;
        }
        if retrieved {
            self.record
                .as_mut()
                .and_then(|r| r.current.as_mut())
                .expect("current job")
                .status = 0;
        }
        session.destroy(host, 1)?;
        self.final_boundary(host)
    }
    fn flags(&self) -> Contract<bool> {
        let record = self
            .record
            .as_ref()
            .ok_or(Unsupported::NativeInputDomain("FTP worker record"))?;
        Ok(record.cancel || record.stop)
    }
    /// 00812190/00816ee0 final boundary: cancel fails and notifies; stop suppresses.
    pub fn final_boundary(&mut self, host: &mut impl Host) -> Contract<()> {
        let record = self
            .record
            .as_mut()
            .ok_or(Unsupported::NativeInputDomain("FTP worker record"))?;
        let mut job = record
            .current
            .clone()
            .ok_or(Unsupported::NativeInputDomain("FTP current job"))?;
        if record.cancel || record.stop {
            record.cancel = false;
            job.status = 7;
        }
        let stop = record.stop;
        if !stop {
            job.progress = 100;
            record.current = Some(job.clone());
            if self.target.is_some() {
                host.worker_effect(Effect::CreateMessage);
                host.worker_effect(Effect::AddMessage);
                host.complete_job(self, &job, job.status == 0);
                host.worker_effect(Effect::ReleaseMessage);
            }
        }
        if let Some(record) = self.record.as_mut() {
            record.current = None;
        }
        host.worker_effect(Effect::ClearCurrent);
        host.worker_effect(Effect::DestroyUrl);
        Ok(())
    }
    /// 00817560 wrapper's hold release happens after the worker exits, not before.
    pub fn wrapper_return(host: &mut impl Host) -> u32 {
        host.worker_effect(Effect::ReleaseScheduler);
        0
    }
    pub fn adjust_worker_reference(&mut self, host: &mut impl Host, add: bool) -> u32 {
        host.worker_effect(Effect::Lock);
        let value = self.record.as_mut().map_or(0, |record| {
            record.suspend = if add {
                record.suspend.wrapping_add(1)
            } else {
                record.suspend.wrapping_sub(1)
            };
            record.suspend as u32
        });
        host.worker_effect(Effect::Unlock);
        value
    }
    /// 00812a50 intentionally frees only the first string, not the full value.
    pub fn erase_range(&mut self, host: &mut impl Host, first: usize, last: usize) -> usize {
        for _ in first..last {
            self.queue.remove(first);
            host.worker_effect(Effect::UnlinkQueueNode);
            host.worker_effect(Effect::EraseFirstString);
            host.worker_effect(Effect::DestroyQueueNode);
        }
        first
    }
    /// Q18 section 4.15: no worker join, callback or queued-target release.
    pub fn destroy(&mut self, host: &mut impl Host, deleting_flags: u32) -> Contract<()> {
        if self.initialized {
            self.initialized = false;
            if let Some(target) = self.target.take() {
                host.effect(ftp::Effect::ReleaseTarget(target));
            }
        }
        if let Some(mut session) = self.session.take() {
            session.destroy(host, 1)?;
        }
        host.worker_effect(Effect::DestroyCriticalSection);
        host.worker_effect(Effect::DestroySynchronization);
        for _ in self.queue.drain(..) {
            host.worker_effect(Effect::DestroyQueueStrings);
            host.worker_effect(Effect::DestroyQueueNode);
        }
        host.worker_effect(Effect::DestroyStoredStrings);
        if deleting_flags & 1 != 0 {
            host.worker_effect(Effect::FreeScheduler);
        }
        Ok(())
    }
}

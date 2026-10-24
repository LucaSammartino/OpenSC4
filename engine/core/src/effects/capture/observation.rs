// SPDX-License-Identifier: GPL-3.0-or-later
//! EF21 observation bookkeeping. Inputs are facts supplied by native call
//! interceptors, not inferred from snapshots or calendar notifications.
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Init,
    Bootstrap,
    Tick,
    Begin,
    Parse,
    End,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Enter(Operation),
    Leave(Operation),
    Request {
        key: [u32; 3],
        iid: u32,
        success: bool,
        resource: Option<u32>,
    },
    Read {
        resource: u32,
        success: bool,
    },
    Release(u32),
    PublishBefore(u32),
    PublishAfter(u32),
    Register {
        path: Vec<u8>,
        flags: u32,
    },
    ParseResult {
        success: bool,
        diagnostic: Vec<u8>,
    },
    Message(u32),
    Modified(bool),
}

/// Trace validity is distinct from availability of pre-attachment history.
/// A host must additionally exclude every unobserved writer and must not
/// pump/reenter while copying. This model cannot establish those host facts.
#[derive(Debug)]
pub struct Observation {
    pub trace: Vec<(u32, Event)>,
    pub generation: u64,
    owner: u32,
    stack: Vec<Operation>,
    resources: BTreeMap<u32, u32>,
    publications: Vec<u32>,
    initialized: bool,
    boundary_seen: bool,
    history: bool,
    invalid: bool,
}

impl Observation {
    pub fn attach(owner: u32, already_initialized: bool) -> Self {
        Self {
            trace: Vec::new(),
            generation: 0,
            owner,
            stack: Vec::new(),
            resources: BTreeMap::new(),
            publications: Vec::new(),
            initialized: already_initialized,
            boundary_seen: false,
            history: !already_initialized,
            invalid: false,
        }
    }

    pub fn record(&mut self, thread: u32, event: Event) {
        self.trace.push((thread, event.clone()));
        if thread != self.owner {
            self.invalid = true;
        }
        match event {
            Event::Enter(op) => {
                self.boundary_seen = false;
                self.stack.push(op);
            }
            Event::Leave(op) => {
                if self.stack.pop() != Some(op) {
                    self.invalid = true;
                }
                if op == Operation::Init {
                    self.initialized = true;
                }
                if self.stack.is_empty() && matches!(op, Operation::Init | Operation::Tick) {
                    self.boundary_seen = true;
                    self.generation += 1;
                }
            }
            Event::Request { success, resource, .. } => {
                // Failure never makes an output-cell residue into a resource.
                if success {
                    if let Some(r) = resource.filter(|r| *r != 0) {
                        *self.resources.entry(r).or_default() += 1;
                    } else {
                        self.invalid = true;
                    }
                }
            }
            Event::Read { resource, .. } => {
                // A natural Read can occur inside a service request before
                // its return exposes the resource. Its this pointer is live
                // only for that call; this event does not create a lease.
                if resource == 0 {
                    self.invalid = true;
                }
            }
            Event::Release(r) => {
                if self.publications.contains(&r) {
                    self.invalid = true;
                }
                match self.resources.get_mut(&r) {
                    Some(count) if *count > 1 => *count -= 1,
                    Some(_) => {
                        self.resources.remove(&r);
                    }
                    None => self.invalid = true,
                }
            }
            Event::PublishBefore(r) => {
                if !self.resources.contains_key(&r) {
                    self.invalid = true;
                }
                self.publications.push(r);
            }
            Event::PublishAfter(r) if self.publications.pop() != Some(r) => {
                self.invalid = true;
            }
            _ => {}
        }
    }

    pub fn borrowed(&self, resource: u32) -> bool {
        !self.invalid && self.resources.contains_key(&resource)
    }

    /// A retained-only sample can follow late attachment at a new verified
    /// boundary. It must still report unavailable bootstrap/resource history.
    pub fn retained_available(&self, thread: u32, writers_excluded: bool) -> bool {
        thread == self.owner
            && writers_excluded
            && self.initialized
            && self.boundary_seen
            && self.stack.is_empty()
            && self.publications.is_empty()
            && !self.invalid
    }

    pub fn history_available(&self) -> bool {
        self.history && !self.invalid
    }
}

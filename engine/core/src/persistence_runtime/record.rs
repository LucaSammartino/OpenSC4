// SPDX-License-Identifier: GPL-3.0-or-later
use super::{Key, RECORD_IID, UNKNOWN_IID};
use std::sync::Arc;

/// Narrow segment reference/close primitives. Identity is supplied by the
/// caller, not guessed from a native record address or a resource key.
pub trait Segment: Send + Sync {
    fn add_ref(&self);
    fn release(&self);
    fn close_record(&self, identity: u64) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordInterface {
    Record,
    Serial,
    Input,
    Output,
}

pub struct DbRecord {
    references: u16,
    flags: u16,
    key: Key,
    identity: u64,
    segment: Option<Arc<dyn Segment>>,
}

impl DbRecord {
    /// The spec does not supply the initial count: it is an explicit input
    /// rather than an inferred native constructor default (PR9).
    pub fn new(
        key: Key,
        references: u16,
        identity: u64,
        segment: Option<Arc<dyn Segment>>,
    ) -> Self {
        if let Some(segment) = &segment {
            segment.add_ref();
        }
        Self {
            references,
            flags: 0,
            key,
            identity,
            segment,
        }
    }
    pub fn reference_count(&self) -> u16 {
        self.references
    }
    pub fn add_ref(&mut self) -> u16 {
        self.references = self.references.wrapping_add(1);
        self.references
    }
    pub fn release(&mut self) -> u16 {
        if self.references > 1 {
            self.references -= 1;
            self.references
        } else {
            // Native destruction returns zero without exposing count zero.
            self.release_segment();
            0
        }
    }
    pub fn query_interface(&mut self, iid: u32) -> Option<RecordInterface> {
        if iid == RECORD_IID || iid == UNKNOWN_IID {
            self.add_ref();
            Some(RecordInterface::Record)
        } else {
            None
        }
    }
    pub fn key(&self) -> Key {
        self.key
    }
    pub fn set_key(&mut self, key: Key) {
        self.key = key;
    }
    pub fn access_flags(&self) -> u16 {
        self.flags
    }
    pub fn set_access_flags(&mut self, flags: u16) {
        self.flags = flags;
    }
    pub fn close(&mut self) -> bool {
        if let Some(segment) = self.segment.take() {
            let result = segment.close_record(self.identity);
            segment.release();
            result
        } else {
            false
        }
    }
    fn release_segment(&mut self) {
        if let Some(segment) = self.segment.take() {
            segment.release();
        }
    }
}
impl Drop for DbRecord {
    fn drop(&mut self) {
        self.release_segment();
    }
}

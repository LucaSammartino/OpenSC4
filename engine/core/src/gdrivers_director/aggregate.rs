// SPDX-License-Identifier: GPL-3.0-or-later
//! Inventory §5.6: shallow value sequences. Unknown native bytes stay unknown.
use super::{Allocation, Fault, Host, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ARecord {
    pub key: Option<u32>,
    pub entries: [Option<u32>; 9],
    pub values: [Option<u32>; 9],
    pub flag: Option<u8>,
    pub working: [Option<u32>; 2],
    pub opaque: [Option<u8>; 119],
}
impl ARecord {
    /// 0x008DF260: the ninth value is intentionally not initialized.
    pub fn partial_default() -> Self {
        let mut values = [Some(0); 9];
        values[8] = None;
        Self {
            key: Some(0),
            entries: [Some(0); 9],
            values,
            flag: Some(0),
            working: [None; 2],
            opaque: [None; 119],
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BRecord {
    pub owned: Option<u32>,
    pub opaque: [Option<u8>; 44],
}

pub trait ValueRecord: Clone {
    const BYTES: u32;
}
impl ValueRecord for ARecord {
    const BYTES: u32 = 204;
}
impl ValueRecord for BRecord {
    const BYTES: u32 = 48;
}

#[derive(Debug)]
pub struct Sequence<T> {
    pub records: Vec<T>,
    pub capacity: u32,
    pub storage: u32,
}
impl<T> Default for Sequence<T> {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            capacity: 0,
            storage: 0,
        }
    }
}
impl<T: ValueRecord> Sequence<T> {
    /// 0x008DEC30/0x008DED50; omit_suffix is the final native byte, not a bool word.
    pub fn reallocate(
        &mut self,
        position: usize,
        count: u32,
        value: &T,
        omit_suffix: u8,
        host: &mut impl Host,
    ) -> Result<()> {
        let old_size = self.records.len() as u32;
        let capacity = old_size.wrapping_add(old_size.max(count));
        let storage = host.alloc(capacity.wrapping_mul(T::BYTES), Allocation::Heap);
        if storage == 0 {
            return Err(Fault::NullWrite);
        }
        let snapshot = value.clone();
        let mut records = self.records[..position].to_vec();
        records.extend(std::iter::repeat(snapshot).take(count as usize));
        if omit_suffix == 0 {
            records.extend_from_slice(&self.records[position..]);
        }
        host.free(self.storage, Allocation::Heap);
        self.records = records;
        self.capacity = capacity;
        self.storage = storage;
        Ok(())
    }
    /// 0x008DEF60/0x008DF0B0. Vec's insertion has the same observable shallow
    /// value ordering; explicitly snapshot before any shift for self-aliasing callers.
    pub fn insert(&mut self, position: usize, count: u32, value: &T, host: &mut impl Host) -> Result<()> {
        if count == 0 {
            return Ok(());
        }
        let snapshot = value.clone();
        if self.capacity.wrapping_sub(self.records.len() as u32) < count {
            return self.reallocate(position, count, &snapshot, 0, host);
        }
        self.records
            .splice(position..position, std::iter::repeat(snapshot).take(count as usize));
        Ok(())
    }
    pub fn insert_from(&mut self, position: usize, count: u32, source: usize, host: &mut impl Host) -> Result<()> {
        if count == 0 {
            return Ok(());
        }
        let value = self.records[source].clone();
        self.insert(position, count, &value, host)
    }
}
impl Sequence<ARecord> {
    /// 0x008DF1F0: shrinking forgets values, not their pointees or backing capacity.
    pub fn resize(&mut self, count: u32, fill: &ARecord, host: &mut impl Host) -> Result<()> {
        let old = self.records.len() as u32;
        if count < old {
            self.records.truncate(count as usize);
            return Ok(());
        }
        self.insert(old as usize, count - old, fill, host)
    }
    pub fn resize_default(&mut self, count: u32, host: &mut impl Host) -> Result<()> {
        self.resize(count, &ARecord::partial_default(), host)
    }
}

#[derive(Debug)]
pub struct Aggregate {
    pub a: Sequence<ARecord>,
    pub b: Sequence<BRecord>,
    pub counters: [u32; 2],
}
impl Aggregate {
    /// 0x008DF2E0: no checked allocation recovery before the first record is touched.
    pub fn new(host: &mut impl Host) -> Result<Self> {
        let mut this = Self {
            a: Sequence::default(),
            b: Sequence::default(),
            counters: [0; 2],
        };
        this.a.resize_default(1, host)?;
        this.a.records[0].flag = Some(1);
        this.a.records[0].working = [Some(0); 2];
        Ok(this)
    }
    /// 0x008DEE80: free order is B pointees, A's nine pointees, B storage, A storage.
    /// Backing identities/capacities deliberately survive. Repeated destruction is invalid.
    pub fn destroy(&mut self, host: &mut impl Host) {
        for record in &self.b.records {
            if let Some(pointer) = record.owned {
                host.free(pointer, Allocation::Heap);
            }
        }
        for record in &mut self.a.records {
            for pointer in &mut record.entries {
                if let Some(value) = *pointer {
                    host.free(value, Allocation::Heap);
                }
                *pointer = Some(0);
            }
        }
        if self.b.storage != 0 {
            host.free(self.b.storage, Allocation::Heap);
        }
        if self.a.storage != 0 {
            host.free(self.a.storage, Allocation::Heap);
        }
    }
}

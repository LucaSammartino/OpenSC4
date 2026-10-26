// SPDX-License-Identifier: GPL-3.0-or-later
//! Semantic container helpers from model-maker §12.5. Foreign counted handles
//! need explicit cleanup with a host; Rust owns the backing allocations.
use super::materials::assign_counted;
use super::{Handle, ObjectHost};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Entry {
    pub reference: Option<Handle>,
    pub bytes: [u8; 3],
}

fn copy(entry: &Entry, host: &mut impl ObjectHost) -> Entry {
    if let Some(object) = entry.reference {
        host.retain(object);
    }
    Entry {
        reference: entry.reference,
        bytes: entry.bytes,
    }
}

pub fn copy_construct(source: &[Entry], host: &mut impl ObjectHost) -> Vec<Entry> {
    source.iter().map(|entry| copy(entry, host)).collect()
}

pub fn construct_copies(source: &Entry, count: usize, host: &mut impl ObjectHost) -> Vec<Entry> {
    (0..count).map(|_| copy(source, host)).collect()
}

pub fn assign_range(destination: &mut [Entry], source: &[Entry], host: &mut impl ObjectHost) {
    assert!(destination.len() >= source.len());
    for (destination, source) in destination.iter_mut().zip(source) {
        assign_counted(&mut destination.reference, source.reference, host);
        destination.bytes = source.bytes;
    }
}

pub fn clear(entries: &mut Vec<Entry>, host: &mut impl ObjectHost) {
    for entry in entries.drain(..) {
        if let Some(reference) = entry.reference {
            host.release(reference);
        }
    }
}

/// Returns the native capacity, independent of the Rust allocator's reservation.
/// This is the grow path, with the specified prefix/copies/suffix construction.
pub fn grow_insert(
    entries: &mut Vec<Entry>,
    at: usize,
    count: usize,
    source: &Entry,
    host: &mut impl ObjectHost,
) -> usize {
    assert!(at <= entries.len());
    let capacity = entries.len() + entries.len().max(count);
    let mut grown = Vec::with_capacity(capacity);
    grown.extend(copy_construct(&entries[..at], host));
    grown.extend(construct_copies(source, count, host));
    grown.extend(copy_construct(&entries[at..], host));
    clear(entries, host);
    *entries = grown;
    capacity
}

pub fn zero_pairs(count: usize) -> Vec<[u32; 2]> {
    vec![[0; 2]; count]
}
pub fn seed_triples(count: usize, external_seed: u32) -> Vec<[u32; 3]> {
    vec![[external_seed; 3]; count]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Host {
        events: Vec<(bool, Handle)>,
    }
    impl ObjectHost for Host {
        fn retain(&mut self, h: Handle) {
            self.events.push((true, h));
        }
        fn release(&mut self, h: Handle) {
            self.events.push((false, h));
        }
    }
    fn entry(h: Handle) -> Entry {
        Entry {
            reference: Some(h),
            bytes: [h as u8, 2, 3],
        }
    }
    #[test]
    fn counted_vectors_grow_prefix_copies_suffix_then_release_old_buffer() {
        let mut host = Host::default();
        let mut values = copy_construct(&[entry(1), entry(2)], &mut host);
        host.events.clear();
        assert_eq!(grow_insert(&mut values, 1, 3, &entry(9), &mut host), 5);
        assert_eq!(
            values.iter().map(|v| v.reference).collect::<Vec<_>>(),
            [Some(1), Some(9), Some(9), Some(9), Some(2)]
        );
        assert_eq!(
            host.events,
            [
                (true, 1),
                (true, 9),
                (true, 9),
                (true, 9),
                (true, 2),
                (false, 1),
                (false, 2)
            ]
        );
        host.events.clear();
        assign_range(&mut values[..2], &[entry(1), entry(8)], &mut host);
        assert_eq!(host.events, [(true, 8), (false, 9)]);
        assert_eq!(values[1].bytes, [8, 2, 3]);
        assert_eq!(grow_insert(&mut values, 5, 1, &Entry::default(), &mut host), 10);
        clear(&mut values, &mut host);
        assert!(values.is_empty());
        assert_eq!(zero_pairs(2), [[0, 0]; 2]);
        assert_eq!(seed_triples(2, 0x3F80_0000), [[0x3F80_0000; 3]; 2]);
    }
}

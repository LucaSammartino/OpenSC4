// SPDX-License-Identifier: GPL-3.0-or-later
//! Owner-local containers, persistence-manager §§4.1, 5, 8.1, 11.
use std::collections::VecDeque;

pub(super) const PRIMES: [u32; 26] = [
    0xc1, 0x185, 0x301, 0x607, 0xc07, 0x1807, 0x3001, 0x6011, 0xc005, 0x1800d, 0x30005, 0x60019, 0xc0001, 0x180005,
    0x30000b, 0x60000d, 0xc00005, 0x1800013, 0x3000005, 0x6000017, 0xc000013, 0x18000005, 0x30000059, 0x60000005,
    0xc0000001, 0xfffffffb,
];

pub(super) trait HashKey: Copy + Eq {
    fn hash(self) -> u32;
}

impl HashKey for u32 {
    fn hash(self) -> u32 {
        self
    }
}

#[derive(Debug)]
pub(super) struct Table<K, V> {
    buckets: Vec<Vec<(K, V)>>,
    len: usize,
}

impl<K: HashKey, V> Table<K, V> {
    pub fn new() -> Self {
        Self {
            buckets: (0..193).map(|_| Vec::new()).collect(),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }
    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }
    fn bucket(&self, key: K) -> usize {
        (key.hash() as u64 % self.buckets.len() as u64) as usize
    }

    pub fn get(&self, key: K) -> Option<&V> {
        self.buckets[self.bucket(key)]
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v)
    }

    fn grow(&mut self) {
        if self.len < self.buckets.len() {
            return;
        }
        let count = PRIMES
            .iter()
            .copied()
            .find(|p| u64::from(*p) > self.buckets.len() as u64)
            .unwrap_or(PRIMES[25]) as usize;
        if count == self.buckets.len() {
            return;
        }
        let old = std::mem::replace(&mut self.buckets, (0..count).map(|_| Vec::new()).collect());
        // Ascending old buckets, head-to-tail, each moved to the NEW head.
        for chain in old {
            for (k, v) in chain {
                let index = self.bucket(k);
                self.buckets[index].insert(0, (k, v));
            }
        }
    }

    pub fn insert_duplicate(&mut self, key: K, value: V) {
        self.grow();
        let index = self.bucket(key);
        let chain = &mut self.buckets[index];
        let position = chain.iter().position(|(k, _)| *k == key).map_or(0, |i| i + 1);
        chain.insert(position, (key, value));
        self.len += 1;
    }

    pub fn insert_unique(&mut self, key: K, value: V) -> bool {
        if self.get(key).is_some() {
            return false;
        }
        self.grow();
        let index = self.bucket(key);
        self.buckets[index].insert(0, (key, value));
        self.len += 1;
        true
    }

    pub fn remove(&mut self, key: K) -> Option<V> {
        let index = self.bucket(key);
        let chain = &mut self.buckets[index];
        let at = chain.iter().position(|(k, _)| *k == key)?;
        self.len -= 1;
        Some(chain.remove(at).1)
    }

    pub fn entries(&self) -> impl Iterator<Item = &(K, V)> {
        self.buckets.iter().flatten()
    }

    pub fn retain(&mut self, mut keep: impl FnMut(K, &V) -> bool) {
        for chain in &mut self.buckets {
            chain.retain(|(k, v)| keep(*k, v));
        }
        self.len = self.buckets.iter().map(Vec::len).sum();
    }

    pub fn clear(&mut self) {
        for chain in &mut self.buckets {
            chain.clear();
        }
        self.len = 0;
    }

    pub fn destroy(&mut self) {
        self.buckets = Vec::new();
        self.len = 0;
    }
}

/// The segment owner's 32-pointer allocation blocks. Rust's VecDeque supplies
/// the block-pointer map's remap/reallocation; element moves are owner-local.
#[derive(Debug)]
pub(super) struct PointerDeque<T: Copy> {
    blocks: VecDeque<Box<[Option<T>; 32]>>,
    first: usize,
    len: usize,
}

impl<T: Copy> PointerDeque<T> {
    pub fn new() -> Self {
        Self {
            blocks: VecDeque::new(),
            first: 0,
            len: 0,
        }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn get(&self, index: usize) -> Option<T> {
        if index >= self.len {
            return None;
        }
        let position = self.first + index;
        self.blocks[position / 32][position % 32]
    }
    fn set(&mut self, index: usize, value: Option<T>) {
        let position = self.first + index;
        self.blocks[position / 32][position % 32] = value;
    }
    pub fn iter(&self) -> impl Iterator<Item = T> + '_ {
        (0..self.len).map(|i| self.get(i).unwrap())
    }
    pub fn push_back(&mut self, value: T) {
        if self.first + self.len == self.blocks.len() * 32 {
            self.blocks.push_back(Box::new([None; 32]));
        }
        self.set(self.len, Some(value));
        self.len += 1;
    }
    pub fn push_front(&mut self, value: T) {
        if self.first == 0 {
            self.blocks.push_front(Box::new([None; 32]));
            self.first = 32;
        }
        self.first -= 1;
        self.len += 1;
        self.set(0, Some(value));
    }
    pub fn erase(&mut self, index: usize) -> T {
        let result = self.get(index).expect("valid deque erase index");
        if index < self.len / 2 {
            for i in (1..=index).rev() {
                self.set(i, self.get(i - 1));
            }
            self.set(0, None);
            self.first += 1;
            self.len -= 1;
            if self.first == 32 {
                self.blocks.pop_front();
                self.first = 0;
            }
        } else {
            for i in index..self.len - 1 {
                self.set(i, self.get(i + 1));
            }
            self.set(self.len - 1, None);
            self.len -= 1;
            if self.first + self.len <= (self.blocks.len() - 1) * 32 {
                self.blocks.pop_back();
            }
        }
        if self.len == 0 {
            self.clear();
        }
        result
    }
    pub fn clear(&mut self) {
        self.blocks.clear();
        self.first = 0;
        self.len = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deque_crosses_blocks_and_erases_both_sides_without_reordering() {
        let mut deque = PointerDeque::new();
        for value in 0..100 {
            deque.push_back(value);
        }
        for value in (100..150).rev() {
            deque.push_front(value);
        }
        let mut expected: Vec<_> = (100..150).chain(0..100).collect();
        for index in [31, 32, 128, 0, 60, 140, 1, 32] {
            assert_eq!(deque.erase(index), expected.remove(index));
            assert_eq!(deque.iter().collect::<Vec<_>>(), expected);
        }
        while !expected.is_empty() {
            let index = if expected.len() % 2 == 0 { 0 } else { expected.len() - 1 };
            assert_eq!(deque.erase(index), expected.remove(index));
        }
        assert!(deque.blocks.is_empty());
        deque.push_front(9);
        deque.push_back(10);
        assert_eq!(deque.iter().collect::<Vec<_>>(), [9, 10]);
    }

    #[test]
    fn duplicate_chain_reverses_at_growth_and_bucket_iteration_is_unsigned() {
        let mut table = Table::new();
        table.insert_duplicate(7, 'A');
        table.insert_duplicate(7, 'B');
        table.insert_duplicate(7, 'C');
        assert_eq!(
            table
                .entries()
                .filter(|(k, _)| *k == 7)
                .map(|(_, v)| *v)
                .collect::<Vec<_>>(),
            ['A', 'C', 'B']
        );
        for k in 1000..1190 {
            table.insert_duplicate(k, 'X');
        }
        assert_eq!(table.bucket_count(), 193);
        table.insert_duplicate(u32::MAX, 'Y');
        assert_eq!(table.bucket_count(), 389);
        assert_eq!(
            table
                .entries()
                .filter(|(k, _)| *k == 7)
                .map(|(_, v)| *v)
                .collect::<Vec<_>>(),
            ['B', 'C', 'A']
        );
        assert_eq!(table.get(u32::MAX), Some(&'Y'));
        assert_eq!(PRIMES[25], 0xffff_fffb);
        assert!(PRIMES.windows(2).all(|w| w[0] < w[1]));
        table.destroy();
        assert_eq!(table.buckets.capacity(), 0);
    }
}

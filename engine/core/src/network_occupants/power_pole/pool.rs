//! Portable allocation policy for the native 0x124-byte pole pool (§4.1.1).
//! Tokens describe pool-relative offsets; they are never host addresses.

use std::collections::VecDeque;

const BLOCKS: u32 = 112;
const FULL: u32 = 0x4000_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Block {
    pub pool: usize,
    pub offset: u32,
}

struct Pool {
    free: VecDeque<u32>,
    count: u32,
}

#[derive(Default)]
pub struct PolePool {
    pools: Vec<Option<Pool>>,
    current: Option<usize>,
}

impl PolePool {
    pub fn allocate(&mut self) -> Block {
        if let Some(current) = self.current {
            let pool = self.pools[current].as_mut().expect("current pool");
            if pool.free.is_empty() {
                pool.count |= FULL;
                self.current = None;
            }
        }
        let index = *self.current.get_or_insert_with(|| {
            // SPEC-GAP: §4.1.1 does not order multiple available pools; use the first.
            if let Some(index) = self
                .pools
                .iter()
                .position(|p| p.as_ref().is_some_and(|p| p.count & FULL == 0))
            {
                index
            } else {
                self.pools.push(Some(Pool {
                    free: (0..BLOCKS).map(|i| 0x10 + i * 0x124).collect(),
                    count: 0,
                }));
                self.pools.len() - 1
            }
        });
        let pool = self.pools[index].as_mut().expect("available pool");
        let offset = pool.free.pop_front().expect("nonexhausted pool");
        pool.count += 1;
        Block { pool: index, offset }
    }

    /// Call after object destruction, only with a live block from this owner.
    pub fn destroy(&mut self, block: Block, deleting: bool) {
        if !deleting {
            return;
        }
        let pool = self.pools[block.pool].as_mut().expect("live block pool");
        pool.free.push_front(block.offset);
        pool.count -= 1;
        if pool.count == FULL + 84 {
            pool.count &= !FULL;
        }
        if pool.count == 0 && self.pools.iter().flatten().filter(|p| p.count & FULL == 0).count() > 1 {
            self.pools[block.pool] = None;
            if self.current == Some(block.pool) {
                self.current = None;
            }
        }
    }

    pub fn count(&self, pool: usize) -> Option<u32> {
        self.pools.get(pool)?.as_ref().map(|p| p.count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_pool_policy_conformance_62() {
        let mut pool = PolePool::default();
        let blocks: Vec<_> = (0..112).map(|_| pool.allocate()).collect();
        assert_eq!(blocks[0], Block { pool: 0, offset: 0x10 });
        assert_eq!(blocks[1].offset, 0x134);
        assert_eq!(blocks[111].offset, 0x7EAC);
        assert_eq!(pool.count(0), Some(112));
        let next = pool.allocate();
        assert_eq!(next.pool, 1);
        assert_eq!(pool.count(0), Some(FULL + 112));
        for &block in &blocks[..27] {
            pool.destroy(block, true);
        }
        assert_eq!(pool.count(0), Some(FULL + 85));
        pool.destroy(blocks[27], true);
        assert_eq!(pool.count(0), Some(84));
        pool.destroy(blocks[28], false);
        assert_eq!(pool.count(0), Some(84));
        for &block in &blocks[28..] {
            pool.destroy(block, true);
        }
        assert_eq!(pool.count(0), None);
        pool.destroy(next, true);
        assert_eq!(pool.count(1), Some(0));
        assert_eq!(pool.allocate(), next);
    }
}

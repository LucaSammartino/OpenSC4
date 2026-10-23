// SPDX-License-Identifier: GPL-3.0-or-later
//! particles.md §5. Logical record slots, 128 per refill; native addresses
//! are not reproduced. Payload/list metadata allocation remains portable.
use std::sync::{Arc, Mutex, OnceLock, Weak};
#[derive(Debug, Default)]
pub struct Pool {
    free: Vec<usize>,
    blocks: usize,
}
impl Pool {
    pub fn blocks(&self) -> usize {
        self.blocks
    }
    pub fn free_count(&self) -> usize {
        self.free.len()
    }
    /// The heap-success reply is a foreign primitive. No particle constructor
    /// or random draw is performed on allocation failure.
    pub fn allocate(pool: &Arc<Mutex<Self>>, mut heap: impl FnMut(usize) -> bool) -> Option<Lease> {
        let mut state = pool.lock().expect("particle pool lock");
        if state.free.is_empty() {
            if !heap(8704 + 8) {
                return None;
            }
            state.free.try_reserve(128).ok()?;
            let first = state.blocks * 128;
            state.free.extend((first..first + 128).rev());
            state.blocks += 1;
        }
        let id = state.free.pop().expect("refilled pool");
        Some(Lease(Arc::new(ReturnSlot {
            pool: Arc::downgrade(pool),
            id,
        })))
    }
}
#[derive(Debug)]
struct ReturnSlot {
    pool: Weak<Mutex<Pool>>,
    id: usize,
}
impl Drop for ReturnSlot {
    fn drop(&mut self) {
        if let Some(pool) = self.pool.upgrade() {
            pool.lock().expect("particle pool lock").free.push(self.id);
        }
    }
}
#[derive(Clone, Debug)]
pub struct Lease(Arc<ReturnSlot>);
impl Lease {
    pub fn id(&self) -> usize {
        self.0.id
    }
}
impl PartialEq for Lease {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
pub fn global_effect_pool() -> Arc<Mutex<Pool>> {
    static POOL: OnceLock<Arc<Mutex<Pool>>> = OnceLock::new();
    Arc::clone(POOL.get_or_init(|| Arc::new(Mutex::new(Pool::default()))))
}

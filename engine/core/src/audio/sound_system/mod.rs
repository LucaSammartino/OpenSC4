// SPDX-License-Identifier: GPL-3.0-or-later
//! HIT, freshness and framework sound owners from `sound-system.md`.
//! Device IO and serializers are foreign. Unsettled algorithms never succeed
//! as no-ops: they return the spec's numbered [`Unsupported`] question.

pub mod directors;
pub mod framework;
pub mod fresh;
pub mod hit;
pub mod playlist;
pub mod track;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Explicitly unsettled in the source spec, not a backend error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Unsupported(pub &'static str);

/// Original unchecked dereferences are explicit failure boundaries in safe Rust,
/// never ordinary Boolean failure or a successful no-op.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeFault {
    MissingSoundSystem,
    MissingPropertyCache,
    NullConstructionIdentity,
    NullSound,
}

pub type Properties = BTreeMap<u32, i32>;

/// Foreign sound interface. References are acquired by cloning an Arc and
/// released by dropping that clone; adapters balance native AddRef/Release.
pub trait Sound: Send + Sync {
    fn load(&self) -> bool;
    fn unload(&self) -> bool;
    fn is_playing(&self) -> bool;
    fn pause(&self);
    fn unpause(&self);
    fn play(&self, payload: u32, looping: bool, final_flag: bool) -> bool;
    fn set_volume(&self, volume: i32) -> bool;
}

pub type SoundRef = Arc<dyn Sound>;

/// A successful QI owns one additional reference. `offset` is metadata for
/// adapters, never pointer arithmetic on the portable Rust owner.
pub struct InterfaceView<T> {
    pub owner: Arc<Mutex<T>>,
    pub offset: u32,
}

/// Scheduler is foreign; period classification and actual clocks are SS8.
pub trait Scheduler {
    fn register(&mut self, period: u32);
    fn unregister(&mut self);
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod closure_tests;

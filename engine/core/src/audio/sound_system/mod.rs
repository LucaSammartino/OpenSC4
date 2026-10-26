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

mod fresh_playback;
mod hit_config;
mod hit_resource;
mod track_vm;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Explicitly unsettled in the source spec, not a backend error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Unsupported(pub &'static str);

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
    // Optional foreign sound views. Absence must not advertise backend success.
    fn stop(&self) -> bool {
        false
    }
    fn kill(&self) -> bool {
        false
    }
    fn is_loaded(&self) -> bool {
        false
    }
    fn is_fading(&self) -> bool {
        false
    }
    fn frequency(&self) -> i32 {
        0
    }
    fn set_frequency(&self, _value: i32) -> bool {
        false
    }
    fn set_pan(&self, _value: i32) -> bool {
        false
    }
    fn spatial(&self, _head: bool, _min: f32, _max: f32, _position: Option<[f32; 3]>) -> bool {
        false
    }
    fn set_velocity(&self, _value: [f32; 3]) -> bool {
        false
    }
    fn set_min_distance(&self, _value: f32) -> bool {
        false
    }
    fn set_max_distance(&self, _value: f32) -> bool {
        false
    }
    fn set_position(&self, _value: [f32; 3]) -> bool {
        false
    }
    fn set_integer(&self, _id: u32, _value: i32, _suppress: bool) -> bool {
        false
    }
    fn fade(&self, _duration: u32, _target: i32) -> bool {
        false
    }
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
mod closure_tests;
#[cfg(test)]
mod tests;

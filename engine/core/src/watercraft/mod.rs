//! Portable watercraft owner, from `docs/specs/watercraft.md`.
//!
//! No original watercraft implementation is used as a provider.

mod collision;
mod commands;
mod crash;
pub use crash::{CrashEffect, CrashEffectSet, CrashEffects, CrashInstance, GroundingTerrain};
mod following;
mod inherited;
mod interface;
pub use interface::{
    AutomatonInterfaces, InterfacePointer, Release, DELETING_INTERFACE, LIFECYCLE_INTERFACE, QUERY_INTERFACE_OFFSETS,
};
mod lifecycle;
pub use lifecycle::{InitializationServices, LifecycleServices, ShutdownServices, ShutdownStack, SHUTDOWN_SOUND_EVENT};
mod math;
mod models;
mod movement;
pub use models::{acquire_model, ModelHandle, ModelServices, ModelStorage};
mod operations;
mod owner;
mod physics;
mod presentation;
pub use presentation::{PresentationServices, PresentationSnapshot};
mod record;
mod routes;
mod runtime;
pub use runtime::{AnimationServices, AnimationTimes, DriveServices, MotionServices, RopeEffect, SpatialServices};
mod states;
mod subsidiary;
mod targets;
pub use subsidiary::{AnimatedModel, AnimationService, Lighting, Subsidiary};
pub use targets::AiTargets;

pub use collision::{CollisionQueries, Footprint, ModelGeometry};
pub use commands::{CommandChannel, DriveCommand, PlayerEffects, SpeedTween};
pub use movement::Gear;
pub use operations::{candidate_directions, Message, MessageDisposition};
pub use owner::*;
pub use physics::{Publication, Transform};
pub use record::{WatercraftRecord, CLASS_ID, RECORD_VERSION};
pub use routes::{direction_step, quantize_heading, sign, RouteError, RouteReader};
pub use states::CrashCase;

#[cfg(test)]
mod closure_tests;
#[cfg(test)]
mod collision_tests;
#[cfg(test)]
mod edge_tests;
#[cfg(test)]
mod effect_tests;
#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
mod model_tests;
#[cfg(test)]
mod physics_tests;
#[cfg(test)]
mod round2_tests;
#[cfg(test)]
mod runtime_tests;
#[cfg(test)]
mod tests;

/// The forty watercraft drive parameters (watercraft §2).
#[derive(Clone, Debug, PartialEq)]
pub struct DriveParameters(pub [f32; 40]);

impl Default for DriveParameters {
    #[allow(clippy::approx_constant)] // Exact decimal constants in watercraft §2.
    fn default() -> Self {
        Self([
            0.1, 0.001, 1.0, 0.05, 0.2, 5.0, -0.1, 0.3, -2.0, 2.0, 0.001, 50.0, 5.0, 0.001, 0.001, 0.1, 0.5, 1.0472,
            0.3142, 0.1309, 0.2618, 8000.0, 0.7854, 0.95, 2.1817, 1.0472, 10.0, 0.5, 0.25, 0.75, -2000.0, 3000.0,
            150.0, 0.0, 2.0, 100.0, 300.0, 90.0, 1.0, 1.0,
        ])
    }
}

impl DriveParameters {
    /// `None` means no list in the entire group ancestry. Zero entries select
    /// defaults; dependent defaults use the effective referenced entries (§2).
    pub fn load(list: Option<&[f32; 40]>) -> Self {
        let mut result = Self::default();
        if let Some(list) = list {
            for (index, value) in list.iter().copied().enumerate() {
                if value != 0.0 {
                    result.0[index] = value;
                }
            }
            if list[8] == 0.0 {
                result.0[8] = -result.0[9];
            }
            if list[17] == 0.0 {
                result.0[17] = result.0[18] / result.0[7];
            }
            if list[19] == 0.0 {
                result.0[19] = result.0[20] / result.0[9];
            }
        }
        result
    }
}

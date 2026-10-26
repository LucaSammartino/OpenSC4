// SPDX-License-Identifier: GPL-3.0-or-later
//! Input-only import for the default-off S3 experiment (lighting §§1.2,4.4,4.6).
use super::*;

impl LightingManager {
    /// Import cached inputs, never an expected native output.
    pub fn inplace_cached_inputs(night: bool, global: Color, shadow: Vector, axes: [Color; 6]) -> Self {
        let mut manager = Self::new(LightingStatics::new(Parameters::initial()), RegisterPrecision::Bits24);
        manager.night = night;
        manager.global_light = global;
        manager.shadow_direction = shadow;
        manager.axes = axes;
        manager
    }
}

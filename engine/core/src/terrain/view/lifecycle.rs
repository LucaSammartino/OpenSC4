// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

use super::{Terrain, TerrainViewHeights, TerrainViewIndices};

/// The display-grid byte belongs to the process, not a saved view instance.
#[derive(Clone, Debug, Default)]
pub struct TerrainViewShared {
    pub display_grid: u8,
}

/// Portable part of §15.5.14's lifecycle boundary. A host still has to route
/// the native direct entries to these methods and publish the initialized
/// collaborators/identities. This type is not a factory or a renderer.
#[derive(Clone, Debug)]
pub struct TerrainViewLifecycle {
    restored: bool,
    heights: Option<TerrainViewHeights>,
    indices: Option<TerrainViewIndices>,
    altitude_bounds: [f32; 2],
}

impl Default for TerrainViewLifecycle {
    fn default() -> Self {
        // SPEC-GAP: the constructor's ±FLT_MAX band is represented as an empty max/min interval until the first enabled Update.
        Self {
            restored: false,
            heights: None,
            indices: None,
            altitude_bounds: [-f32::MAX, f32::MAX],
        }
    }
}

impl TerrainViewLifecycle {
    /// Read precedes Init. Set the restored byte before any stream validation;
    /// invalid input must not change the process-wide byte or size city caches.
    pub fn read(&mut self, payload: &[u8], shared: &mut TerrainViewShared) -> bool {
        self.restored = true;
        if payload.len() < 3 || payload[..2] != 1u16.to_le_bytes() {
            return false;
        }
        shared.display_grid = payload[2];
        true
    }

    pub fn write(&self, shared: &TerrainViewShared) -> [u8; 3] {
        [1, 0, shared.display_grid]
    }

    /// A second Init is refused. The host must initialize references, messages,
    /// draw callbacks and published fields before it marks the terrain ready.
    pub fn init(&mut self, source: &Terrain, preference_grid: u8, shared: &mut TerrainViewShared) -> bool {
        if self.indices.is_some() {
            return false;
        }
        if !self.restored {
            shared.display_grid = preference_grid;
        }
        self.heights = Some(TerrainViewHeights::new(source));
        self.indices = Some(TerrainViewIndices::new(source));
        true
    }

    /// Host entry order is (z,x), unlike the portable index helper's (x,z).
    pub fn set_triangle_indices_for_cell(&mut self, z: u32, x: u32, flags: u16) -> bool {
        self.indices
            .as_mut()
            .is_some_and(|indices| indices.rewrite_cell(x, z, flags))
    }

    pub fn indices(&self) -> Option<&TerrainViewIndices> {
        self.indices.as_ref()
    }

    pub fn heights(&self) -> Option<&TerrainViewHeights> {
        self.heights.as_ref()
    }

    /// Max first, then min; disabled Updates leave both values stale.
    pub fn update_altitude_bounds(&mut self, source: &Terrain, enabled: bool) {
        if enabled && self.indices.is_some() {
            self.altitude_bounds = [source.max_height(), source.min_height()];
        }
    }

    pub fn altitude_bounds(&self) -> [f32; 2] {
        self.altitude_bounds
    }

    /// Teardown is explicit, even while other clients still hold the view.
    /// Repeated Shutdown/final destruction does not perform it twice.
    pub fn shutdown(&mut self) -> bool {
        let initialized = self.indices.take().is_some();
        self.heights = None;
        initialized
    }
}

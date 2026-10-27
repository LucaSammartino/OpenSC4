// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

/// Plumbing's context prefix and suffix (§15.5.15.7). Coverage-dependent draws
/// belong between them; they cannot be inferred from an empty control case.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlumbingCall {
    EnableBlend(bool),
    SetBlendFunc(u32, u32),
    EnableDepthMask(bool),
    EnableTexture(bool, u32),
    SetDepthOffset(f32),
}

pub fn plumbing_calls(saved_offset: f32) -> ([PlumbingCall; 5], [PlumbingCall; 3]) {
    use PlumbingCall::*;
    (
        [
            EnableBlend(true),
            SetBlendFunc(4, 5),
            EnableDepthMask(false),
            EnableTexture(false, 0),
            SetDepthOffset(-2.0),
        ],
        [SetDepthOffset(saved_offset), EnableBlend(false), EnableDepthMask(true)],
    )
}

/// Extension calls on a previously acquired interface. Snapshot mapped vertex
/// bytes before consuming Flush's Unlock; neither Release nor a mapping
/// failure substitutes a plain-path draw (§15.5.15.7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtensionFlushCall {
    Unlock {
        handle: u32,
    },
    Draw {
        handle: u32,
        first: u32,
        count: u32,
        indices: Vec<u16>,
    },
}

#[derive(Clone, Debug, Default)]
pub struct ExtensionBuffer {
    mapped: bool,
    pending_vertices: u32,
    indices: Vec<u16>,
}

impl ExtensionBuffer {
    /// The host's CreateBuffer result. Only a non-null mapping resets counts.
    pub fn mapped(&mut self, success: bool) {
        self.mapped = success;
        if success {
            self.pending_vertices = 0;
            self.indices.clear();
        }
    }

    /// Already remapped data from the producer, not an invented batching rule.
    pub fn append(&mut self, vertices: u32, indices: &[u16]) -> bool {
        if !self.mapped {
            return false;
        }
        self.pending_vertices = self.pending_vertices.wrapping_add(vertices);
        self.indices.extend_from_slice(indices);
        true
    }

    pub fn pending_counts(&self) -> (u32, usize) {
        (self.pending_vertices, self.indices.len())
    }

    pub fn flush(&mut self, handle: u32) -> Vec<ExtensionFlushCall> {
        if !self.mapped {
            return Vec::new();
        }
        self.mapped = false;
        let mut calls = vec![ExtensionFlushCall::Unlock { handle }];
        if !self.indices.is_empty() {
            calls.push(ExtensionFlushCall::Draw {
                handle,
                first: 0,
                count: 3 * (self.indices.len() as u32 / 3),
                indices: std::mem::take(&mut self.indices),
            });
        }
        self.pending_vertices = 0;
        calls
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit coloured-cell planning only (§4.3). No native cell inclusion,
//! layer/depth transform or manager algorithm is inferred (QV10).
use super::Unsupported;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColoredCell {
    pub origin: [f32; 2],
    pub extent: [f32; 2],
    pub altitude: f32,
    pub argb: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OverlayEffect {
    Register {
        renderer_slot: u8,
        layer: u8,
        priority: u32,
    },
    Mark(ColoredCell),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OverlayState {
    registered: bool,
}

impl OverlayState {
    pub fn registered(&self) -> bool {
        self.registered
    }

    /// Emit one Mark per supplied coloured cell, verbatim. No alpha threshold,
    /// damaged-road selection, geometry construction or unregistration is claimed.
    /// Registration-once is explicit in §4.3; it requires no highlight equality.
    pub fn plan(
        &mut self,
        overlay_on: bool,
        cells: &[ColoredCell],
    ) -> Result<Vec<OverlayEffect>, Unsupported> {
        if !overlay_on {
            return Ok(Vec::new());
        }
        if cells.iter().any(|cell| {
            !cell.altitude.is_finite()
                || cell.origin.iter().any(|v| !v.is_finite())
                || cell.extent.iter().any(|v| !v.is_finite() || *v <= 0.0)
        }) {
            return Err(Unsupported::GridGeometry);
        }
        let mut effects = Vec::new();
        if !self.registered {
            effects.push(OverlayEffect::Register {
                renderer_slot: 32,
                layer: 5,
                priority: 1000,
            });
            self.registered = true;
        }
        effects.extend(cells.iter().copied().map(OverlayEffect::Mark));
        Ok(effects)
    }
}

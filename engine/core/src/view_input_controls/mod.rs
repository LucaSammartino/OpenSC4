// SPDX-License-Identifier: GPL-3.0-or-later
//! Published view-input-controls.md tool algorithms. No command dispatch or
//! full cSC4ViewInputControl base implementation; see README.md for inventory.
//! Hosts expose primitive input, resource, GUI and world operations only.

mod demolish;
mod label;
mod level;
mod props;
mod scroll;
mod sign;

pub use crate::view_utilities::{Pixel, Vec3};
pub use demolish::SignLabelDemolish;
pub use label::*;
pub use level::{LevelTerrain, LevelView, TerrainAccess};
pub use props::*;
pub use scroll::{CityScroll, ScrollHost, ScrollMotion, ScrollTunables};
pub use sign::*;

pub const PICK_UP: u32 = 0x2C16_DA45;
pub const PUT_DOWN: u32 = 0x2C16_DA29;
pub const DEMOLISH: u32 = 0x8A5E_C047;
pub const SOUND_EVENT: u32 = 0xEA45_1903;

/// Only supporting base-control operations required by these tools. A host's
/// cursor lookup returns false for an unknown resource; it must not substitute
/// a tool algorithm or call a native tool method. No COM ownership is claimed.
pub trait ControlHost {
    fn set_cursor(&mut self, resource: u32) -> bool;
    fn am_capturing(&self) -> bool;
    fn capture(&mut self) -> bool;
    fn release_capture(&mut self) -> bool;
    fn remove_top_control(&mut self, stack: bool);
    fn post_sound(&mut self, event: u32, sound: u32);
}

pub(crate) fn sound(host: &mut impl ControlHost, id: u32) {
    if id != 0 {
        host.post_sound(SOUND_EVENT, id);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    /// VIC1: command makers/global-terrain cases other than published tool 7.
    CommandDispatch,
    /// VIC2: no invented clipboard/IME service protocol or subscription adapter.
    ServiceProtocol,
    /// VIC3: inconsistent or out-of-contract primitive data has no native rule.
    PrimitiveData,
    /// VIC4: incomplete/invalid code-page sequences have no published rule.
    CharacterEncoding,
    /// VIC5: GUI/resource allocation failure has no published recovery contract.
    ResourceUnavailable,
}

#[cfg(test)]
mod tests;

// SPDX-License-Identifier: GPL-3.0-or-later
//! MySim input-control owners, from mysim-tools.md. No C ABI/plugin wiring.
//! Window state uses the shared base tree; existing bitmap controls and callout
//! widgets perform drawing. All interface lookups return borrowed identities;
//! adapters balance temporary QI references. Stored handles are explicitly held.

mod building;
pub mod change_house;
pub mod change_job;
pub mod dispatch;
mod hover;
pub mod move_in;

pub use crate::ui_windows::base::{Host as WindowHost, Rect, Window, Windows};
pub use crate::widget_controls::Handle;
pub use building::{BuildingKind, BuildingServices, BuildingTool, SimView};
pub use hover::{eq_text, hq_text, job_skin, wealth_fill, HoverServices};

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    /// Open MST1: names/semantics of building slots 9 and 11.
    BuildingSlotsMst1,
    /// Open MST2: names/semantics of lot slots 33, 35, 41 and 63.
    LotSlotsMst2,
    /// Open MST3: pylon slots 7 and 20.
    PylonSlotsMst3,
    MissingLot,
    StaleMeterFill,
    ScreenCoordinate,
    Precondition(&'static str),
}
pub type Result<T> = std::result::Result<T, Unsupported>;
pub type Key = [u32; 3];

pub trait InputServices: WindowHost {
    fn add_ref(&mut self, object: Handle);
    fn release(&mut self, object: Handle);
    fn base_init(&mut self) -> bool;
    fn base_shutdown(&mut self);
    fn base_destroy(&mut self);
    fn set_cursor(&mut self, cursor: u32);
    fn end_input(&mut self);
    fn is_current_control(&mut self) -> bool;
    fn view_window(&mut self) -> Option<Window>;
    fn main_window(&mut self) -> Option<Window>;
    fn focus_view(&mut self);
}

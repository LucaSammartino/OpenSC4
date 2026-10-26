// SPDX-License-Identifier: GPL-3.0-or-later
//! cSC4MySimWinProc, from mysim-windows.md §1. No native UI or C ABI wiring.
//! The caller supplies existing widget bindings; window visibility/reference
//! operations reuse ui_windows::base. Foreign services return borrowed handles
//! unless explicitly documented as created. Call shutdown before discarding.

use crate::ui_windows::base::{Host as WindowHost, Window, Windows};
use crate::widget_controls::Handle;

#[cfg(test)]
mod tests;

pub type Sim = u64;
pub const PANELS: [u32; 7] = [
    0xca1f_1d9c,
    0xea1f_1e4d,
    0xaa1f_1ec5,
    0xea1f_1e5e,
    0x6a61_e29f,
    0xabba_a2d3,
    0xea1f_1e4e,
];
pub const MESSAGES: [u32; 6] = [
    0xaa25_827c,
    0xca09_fa6c,
    0x8a09_fa64,
    0x8a09_fa65,
    0xac08_746e,
    0xac08_746f,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    /// Open MW1: semantic interpretation of the raw set-aside operations.
    MessageVisibility,
    /// Open MW2: statistics-caption layout and transport-message effects.
    StatisticsLayout,
    TransportChoice,
    /// Open MW3: summary omits transport-button and HTML selection rules.
    TransportButtonImage,
    MessageHtmlLayout,
    Precondition(&'static str),
}
pub type Result<T> = std::result::Result<T, Unsupported>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Portrait {
    Resource { group: u32, instance: u32 },
    File(String),
}

/// Bindings to existing controls, not new button/text-control implementations.
/// The spec does not publish their IDs; the UI script adapter supplies them.
#[derive(Clone, Debug)]
pub struct Bindings {
    pub root: Window,
    pub buttons: [Handle; 5],
    pub body: [Handle; 2],
    pub mode_name: Handle,
    pub mode_portrait: Handle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotImage {
    Add,
    Sim(Portrait),
}

/// Individual simulator, widget and service calls. Widget layout, text editing,
/// HTML parsing and drawing stay with the existing controls, not this owner.
pub trait Host: WindowHost {
    fn add_ref_sim(&mut self, sim: Sim);
    fn release_sim(&mut self, sim: Sim);
    fn subscribe(&mut self, message: u32, subscribe: bool);
    fn unregister_service(&mut self, service: u32);
    fn bind_button(&mut self, button: Handle, slot: usize);
    fn current_sims(&mut self) -> Vec<Sim>;
    fn sim_slot(&mut self, sim: Sim) -> usize;
    fn portrait(&mut self, sim: Sim) -> Portrait;
    fn name(&mut self, sim: Sim) -> String;
    fn button_image(&mut self, button: Handle, image: SlotImage);
    fn caption(&mut self, control: Handle, text: &str);
    fn image(&mut self, control: Handle, portrait: &Portrait);
    /// Created reference. run_pick_dialog may reenter the window's messages.
    fn create_pick_dialog(&mut self, slot: usize) -> Option<Handle>;
    fn run_pick_dialog(&mut self, dialog: Handle);
    fn close_pick_dialog(&mut self, dialog: Handle);
    fn release_pick_dialog(&mut self, dialog: Handle);
    fn drop_sim(&mut self, sim: Sim);
}

#[derive(Debug, Default)]
pub struct MySimWindow {
    bindings: Option<Bindings>,
    sims: [Option<Sim>; 5],
    selected: Option<usize>,
    shown: bool,
    pick_dialog: Option<Handle>,
}

impl MySimWindow {
    pub fn slots(&self) -> &[Option<Sim>; 5] {
        &self.sims
    }
    pub fn selected_slot(&self) -> Option<usize> {
        self.selected
    }
    pub fn selected_sim(&self) -> Option<Sim> {
        self.selected.and_then(|slot| self.sims[slot])
    }
    pub fn shown(&self) -> bool {
        self.shown
    }
    pub fn init(&mut self, bindings: Bindings, windows: &mut Windows, host: &mut impl Host) -> bool {
        if self.bindings.is_some() {
            return true;
        }
        windows.add_ref(bindings.root);
        for (slot, button) in bindings.buttons.iter().enumerate() {
            host.bind_button(*button, slot);
        }
        self.bindings = Some(bindings);
        self.hide(windows, host);
        for message in MESSAGES {
            host.subscribe(message, true);
        }
        true
    }
    pub fn shutdown(&mut self, windows: &mut Windows, host: &mut impl Host) {
        let Some(bindings) = self.bindings.take() else {
            return;
        };
        for message in MESSAGES {
            host.subscribe(message, false);
        }
        host.unregister_service(0xebe0_e860);
        if let Some(dialog) = self.pick_dialog.take() {
            host.close_pick_dialog(dialog);
            host.release_pick_dialog(dialog);
        }
        for sim in &mut self.sims {
            if let Some(sim) = sim.take() {
                host.release_sim(sim);
            }
        }
        windows.release(host, bindings.root);
        self.selected = None;
        self.shown = false;
    }
    fn panels(&self, windows: &mut Windows, host: &mut impl Host, visible: Option<u32>) {
        let Some(bindings) = &self.bindings else {
            return;
        };
        for id in PANELS {
            if let Some(panel) = windows.child_from_id_recursive(bindings.root, id) {
                if visible == Some(id) {
                    windows.show(host, panel);
                } else {
                    windows.hide(host, panel);
                }
            }
        }
    }
    pub fn hide(&mut self, windows: &mut Windows, host: &mut impl Host) {
        self.panels(windows, host, None);
        self.shown = false;
    }
    pub fn show(&mut self, windows: &mut Windows, host: &mut impl Host) -> Result<()> {
        self.set_main_window(windows, host)?;
        self.shown = true;
        Ok(())
    }
    pub fn set_main_window(&mut self, windows: &mut Windows, host: &mut impl Host) -> Result<()> {
        self.get_current_sims(host)?;
        // SPEC-GAP: §1.2 lists the unlabelled first panel as the main panel.
        self.panels(windows, host, Some(PANELS[0]));
        Ok(())
    }
    pub fn set_dialog_state(&mut self, windows: &mut Windows, host: &mut impl Host) {
        self.panels(windows, host, Some(0xea1f_1e5e));
    }
    pub fn set_mode_window(&mut self, windows: &mut Windows, host: &mut impl Host) -> Result<()> {
        let sim = self
            .selected_sim()
            .ok_or(Unsupported::Precondition("selected Sim"))?;
        let bindings = self
            .bindings
            .as_ref()
            .ok_or(Unsupported::Precondition("Init"))?;
        let name = host.name(sim);
        let portrait = host.portrait(sim);
        host.caption(bindings.mode_name, &name);
        host.image(bindings.mode_portrait, &portrait);
        self.panels(windows, host, Some(0xea1f_1e4e));
        Ok(())
    }
    pub fn set_stat_window(&self) -> Result<()> {
        // SPEC-GAP: MW2 leaves the caption layout open; do not assign fields.
        Err(Unsupported::StatisticsLayout)
    }
    pub fn get_current_sims(&mut self, host: &mut impl Host) -> Result<()> {
        let bindings = self
            .bindings
            .as_ref()
            .ok_or(Unsupported::Precondition("Init"))?;
        let mut next = [None; 5];
        for sim in host.current_sims() {
            let slot = host.sim_slot(sim);
            if slot >= 5 || next[slot].is_some() {
                // SPEC-GAP: malformed slot lists are rejected without changing retained state.
                return Err(Unsupported::Precondition("unique slots 0..5"));
            }
            next[slot] = Some(sim);
        }
        // Acquire all replacements before releasing any old references.
        for sim in next.iter().flatten() {
            host.add_ref_sim(*sim);
        }
        for sim in self.sims.iter().flatten() {
            host.release_sim(*sim);
        }
        self.sims = next;
        for (slot, sim) in self.sims.iter().enumerate() {
            let image = match sim {
                Some(sim) => SlotImage::Sim(host.portrait(*sim)),
                None => SlotImage::Add,
            };
            host.button_image(bindings.buttons[slot], image);
        }
        Ok(())
    }
    pub fn set_current_sim(
        &mut self,
        slot: usize,
        windows: &mut Windows,
        host: &mut impl Host,
    ) -> Result<()> {
        if slot >= 5 || self.bindings.is_none() {
            return Err(Unsupported::Precondition("initialized slot 0..5"));
        }
        self.selected = Some(slot);
        if self.sims[slot].is_none() {
            self.add_sim(slot, windows, host);
            return Ok(());
        }
        self.set_transport_button_image()
    }
    pub fn set_transport_button_image(&self) -> Result<()> {
        // SPEC-GAP: MW3; the spec names this method but gives no image mapping.
        Err(Unsupported::TransportButtonImage)
    }
    pub fn add_sim(&mut self, slot: usize, windows: &mut Windows, host: &mut impl Host) {
        let Some(bindings) = &self.bindings else {
            return;
        };
        for id in [0xea1f_1e4d, 0xaa1f_1ec5] {
            if let Some(panel) = windows.child_from_id_recursive(bindings.root, id) {
                windows.set_flag(host, panel, 2, false);
            }
        }
        if let Some(dialog) = host.create_pick_dialog(slot) {
            self.pick_dialog = Some(dialog);
            host.run_pick_dialog(dialog);
            if self.pick_dialog.take().is_some() {
                host.release_pick_dialog(dialog);
            }
        }
    }
    pub fn boot_sim(&mut self, windows: &mut Windows, host: &mut impl Host) -> Result<()> {
        if let Some(sim) = self.selected_sim() {
            host.drop_sim(sim);
        }
        self.selected = None;
        self.set_main_window(windows, host)
    }
}

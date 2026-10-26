// SPDX-License-Identifier: GPL-3.0-or-later
//! mysim-windows §1.6: resource enumeration and modal lifecycle, not MW2 effects.
use super::*;

pub const ICON_PROPERTIES: [u32; 5] = [0xebfc_5e26, 0x2c0c_922e, 0xebfc_5fba, 0x899a_fbad, 0xebfc_5e5e];
pub const WALK_BUTTONS: [u32; 4] = [0x2c20_1cb0, 0x2c20_1cb1, 0x2c20_1cb2, 0x3456_0000];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vehicle {
    pub exemplar: Handle,
    pub icon_properties: [Option<Vec<u32>>; 5],
    pub name: String,
}

pub trait TransportHost {
    fn vehicle_group(&mut self, sim: Sim) -> u32;
    fn automata_exemplars(&mut self, group: u32) -> Vec<Handle>;
    fn property_words(&mut self, exemplar: Handle, id: u32) -> Option<Vec<u32>>;
    fn property_name(&mut self, exemplar: Handle, id: u32) -> Option<String>;
    /// Return previous pause state, for restoration after the modal window.
    fn set_paused(&mut self, paused: bool) -> bool;
    /// Created reference, or failure.
    fn create_transport_dialog(&mut self, script: u32) -> Option<Handle>;
    fn run_transport_dialog(&mut self, dialog: Handle);
    fn release_transport_dialog(&mut self, dialog: Handle);
    fn post(&mut self, message: u32);
    /// Each returns the created tool reference to the caller, who must release it.
    fn create_change_house_tool(&mut self, sim: Sim) -> Option<Handle>;
    fn create_change_job_tool(&mut self, sim: Sim) -> Option<Handle>;
}

pub fn vehicle_icons(sim: Sim, host: &mut impl TransportHost) -> Vec<Vehicle> {
    let group = host.vehicle_group(sim);
    host.automata_exemplars(group)
        .into_iter()
        .map(|exemplar| Vehicle {
            exemplar,
            icon_properties: ICON_PROPERTIES.map(|id| host.property_words(exemplar, id)),
            name: host
                .property_name(exemplar, 0x8a41_6a99)
                .unwrap_or_else(|| "(Automata needs a name!)".into()),
        })
        .collect()
}

pub fn select_car(host: &mut impl TransportHost) {
    host.post(0xac08_746e);
}
pub fn select_pedestrian(host: &mut impl TransportHost) {
    host.post(0xac08_746f);
}

impl MySimWindow {
    pub fn show_choose_transport_dialog(&self, host: &mut impl TransportHost) -> Result<()> {
        if self.selected_sim().is_none() {
            return Err(Unsupported::Precondition("selected Sim"));
        }
        let was_paused = host.set_paused(true);
        let dialog = host.create_transport_dialog(0xcbf3_2603);
        if let Some(dialog) = dialog {
            host.run_transport_dialog(dialog);
            host.release_transport_dialog(dialog);
        }
        // SPEC-GAP: restore the previous pause state after success or failure.
        host.set_paused(was_paused);
        // SPEC-GAP: MW3 omits property-to-widget layout; MW2 omits choice effects.
        Err(Unsupported::TransportChoice)
    }
    pub fn create_change_house_tool(&self, host: &mut impl TransportHost) -> Result<Option<Handle>> {
        let sim = self
            .selected_sim()
            .ok_or(Unsupported::Precondition("selected Sim"))?;
        Ok(host.create_change_house_tool(sim))
    }
    pub fn create_change_job_tool(&self, host: &mut impl TransportHost) -> Result<Option<Handle>> {
        let sim = self
            .selected_sim()
            .ok_or(Unsupported::Precondition("selected Sim"))?;
        Ok(host.create_change_job_tool(sim))
    }
}

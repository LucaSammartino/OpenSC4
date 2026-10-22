// SPDX-License-Identifier: GPL-3.0-or-later
//! Init/shutdown, external resource reads and failure text (§§1,3,7).

use super::*;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Globals {
    pub static_initialized: bool,
    pub power_initialized: bool,
    pub pole_exemplars: BTreeMap<u32, u32>,
}

pub trait Resources {
    fn register_reload(&mut self, argument: u32);
    fn reload_rules(&mut self);
    fn base_init(&mut self) -> bool;
    fn base_shutdown(&mut self) -> bool;
    fn exemplar(&mut self, tgi: (u32, u32, u32), iid: u32) -> Option<Reference>;
    fn uint_property(&mut self, resource: Reference, id: u32) -> Option<u32>;
    fn float_property(&mut self, resource: Reference, id: u32) -> Option<f32>;
    fn release_resource(&mut self, resource: Reference);
    fn file(&mut self, tgi: (u32, u32, u32), argument: u32) -> Option<Reference>;
    fn open_ini(&mut self, file: Reference) -> Option<Reference>;
    fn enumerate_ini(&mut self, ini: Reference, section: &str) -> Vec<(u32, u32)>;
    fn destroy_ini(&mut self, ini: Reference);
}

pub fn static_init(globals: &mut Globals, resources: &mut impl Resources) -> bool {
    if !globals.static_initialized {
        globals.static_initialized = true;
        resources.register_reload(0);
        resources.reload_rules();
    }
    true
}

pub fn read_power_pole_infos(globals: &mut Globals, resources: &mut impl Resources) {
    if let Some(file) = resources.file((0, 0x8A59_71C5, 0x8A59_93B9), 0) {
        if let Some(ini) = resources.open_ini(file) {
            for (key, value) in resources.enumerate_ini(ini, "PowerPoles") {
                // No invented bounds check or exemplar validation on the callback.
                globals.pole_exemplars.insert(key, value);
            }
            resources.destroy_ini(ini);
        }
        resources.release_resource(file);
    }
}

pub fn init(tool: &mut PowerLineTool, globals: &mut Globals, resources: &mut impl Resources) -> bool {
    if tool.initialized { return true; }
    if !globals.power_initialized {
        globals.power_initialized = true;
        static_init(globals, resources);
        read_power_pole_infos(globals, resources);
    }
    tool.initialized = true;
    let _ = resources.base_init();
    tool.params.v_3a4 = 20;
    tool.params.v_3a8 = 7.0;
    if let Some(exemplar) = resources.exemplar(PARAM_EXEMPLAR, 0xA521_60F5) {
        if let Some(v) = resources.uint_property(exemplar, PROP_3A4) { tool.params.v_3a4 = v; }
        if let Some(v) = resources.float_property(exemplar, PROP_3A8) { tool.params.v_3a8 = v; }
        if let Some(v) = resources.uint_property(exemplar, PROP_COST_PER_POLE) { tool.params.cost_per_pole = v as i32; }
        if let Some(v) = resources.float_property(exemplar, PROP_COST_PER_WIRE) { tool.params.cost_per_wire = v; }
        resources.release_resource(exemplar);
    }
    // Structural clears deliberately do not run saved-endpoint releases.
    tool.poles.clear();
    tool.wire_vectors.clear();
    true
}

pub fn shutdown(tool: &mut PowerLineTool, resources: &mut impl Resources) -> bool {
    if !tool.initialized { return true; }
    tool.initialized = false;
    resources.base_shutdown()
}

pub trait FailureText {
    fn base_reason(&mut self, reason: u32, output: &mut String);
    fn zone_at(&mut self, cell: (i32, i32)) -> i32;
    fn load_string(&mut self, group: u32, instance: u32, output: &mut String, argument: u32);
}

pub fn failure_reason(tool: &PowerLineTool, host: &mut impl FailureText, reason: u32, output: &mut String) {
    if reason != 0 {
        host.base_reason(reason, output);
    } else if tool.anchors.last().is_some_and(|&cell| host.zone_at(cell) > 0) {
        host.load_string(0x2A59_2FD1, 0x8AAD_D2B5, output, 0);
    }
}

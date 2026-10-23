// SPDX-License-Identifier: GPL-3.0-or-later
//! cLUAExtensionsCollector and cLUAExtensionsCollectorUI (lua-host.md §8).
//! Callback bodies and the VM remain foreign; tokens identify callbacks, not addresses.
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Function(&'static str);

impl Function {
    /// Stable symbolic identity for the foreign callback binding.
    pub fn name(self) -> &'static str {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Extension {
    pub name: &'static str,
    pub function: Function,
    pub is_global: bool,
}

impl Extension {
    const fn new(name: &'static str, is_global: bool) -> Self {
        Self {
            name,
            function: Function(name),
            is_global,
        }
    }
}

/// cLUAExtensionsCollector: source order from advisors.md §8.2, referenced by
/// lua-host.md §8.1. Construction appends to the same vector as the UI collector.
pub struct SimulationCollector;

impl SimulationCollector {
    pub fn append_to(entries: &mut Vec<Extension>) {
        for name in [
            "advice_info",
            "trend_value",
            "trend_slope",
            "trend_count",
            "camera_jump",
            "camera_zoom",
            "ordinance_is_on",
            "ordinance_set_off",
            "ordinance_set_on",
            "ordinance_is_available",
            "ordinance_name",
            "ordinance_description",
            "ordinance_enact_fee",
            "ordinance_repeal_fee",
            "ordinance_monthly_cost",
            "difficulty_level",
            "event_feedback",
            "expire_advice",
            "retire_advice",
            "advice_subject",
            "trigger_event",
            "building_is_group",
            "event_building_distance",
            "mysim_distance_to_closest_building",
            "mysim_distance_to_closest_landmark_or_reward",
            "mysim_local_school_grade",
            "mysim_local_hospital_grade",
            "mysim_dispatch_tool",
            "event_disaster_distance",
            "random_chance",
            "nd_cancel",
            "reward_instance_count",
            "unlock_reward_building",
            "get_reward_building_state",
            "is_tutorial",
        ] {
            entries.push(Extension::new(name, false));
        }
        for name in ["_combine_guids", "_dofile_from_resource", "print"] {
            entries.push(Extension::new(name, true));
        }
    }
}

/// cLUAExtensionsCollectorUI: all nine callbacks target `game`.
pub struct UiCollector;

impl UiCollector {
    pub fn append_to(entries: &mut Vec<Extension>) {
        for name in [
            "tool_plop_network",
            "tool_plop_zone",
            "tool_plop_building",
            "tool_plop_flora",
            "tool_button",
            "window_budget",
            "window_map",
            "window_query",
            "window_graph",
        ] {
            entries.push(Extension::new(name, false));
        }
    }
}

/// One process-wide registry, built once in CRT construction order (§8.1).
/// Immutable after construction; it can be read by any host thread.
pub fn extensions() -> &'static [Extension] {
    static ENTRIES: OnceLock<Vec<Extension>> = OnceLock::new();
    ENTRIES.get_or_init(|| {
        let mut entries = Vec::with_capacity(47);
        SimulationCollector::append_to(&mut entries);
        UiCollector::append_to(&mut entries);
        entries
    })
}

/// Primitive foreign Lua operations. `set_table` is deliberately unprotected:
/// the adapter owns Lua's panic/error path (including the missing `game` case).
/// Errors escape immediately without synthesizing success or restoring the stack.
/// `register` binds the token's foreign callback as a real global named `name`.
pub trait RegistrationVm {
    type Error;
    fn top(&mut self) -> Result<i32, Self::Error>;
    fn get_globals(&mut self) -> Result<(), Self::Error>;
    fn get_global(&mut self, name: &str) -> Result<(), Self::Error>;
    fn register(&mut self, function: Function, name: &str) -> Result<(), Self::Error>;
    fn push_string(&mut self, value: &str) -> Result<(), Self::Error>;
    fn set_table(&mut self, index: i32) -> Result<(), Self::Error>;
    fn pop(&mut self, count: i32) -> Result<(), Self::Error>;
    fn set_top(&mut self, top: i32) -> Result<(), Self::Error>;
}

/// lua-host.md §8.2: ordered registration, visible `_tf_`, balanced success stack.
/// A normal return is always true; Lua errors remain foreign control flow.
pub fn register_api<V: RegistrationVm>(vm: &mut V, global: bool) -> Result<bool, V::Error> {
    let saved_top = vm.top()?;
    for entry in extensions().iter().filter(|e| e.is_global == global) {
        let before = vm.top()?;
        if global {
            vm.get_globals()?;
        } else {
            vm.get_global("game")?;
        }
        if vm.top()? != before {
            vm.register(entry.function, "_tf_")?;
            vm.push_string(entry.name)?;
            vm.get_global("_tf_")?;
            vm.set_table(-3)?;
            vm.pop(1)?;
        }
    }
    vm.set_top(saved_top)?;
    Ok(true)
}

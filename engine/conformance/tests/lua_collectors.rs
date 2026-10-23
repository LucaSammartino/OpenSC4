// SPDX-License-Identifier: GPL-3.0-or-later
//! lua-host.md §8.1–8.2 and §16 T9. The VM and all callback bodies are foreign.
use fc4_core::lua_script_server::collectors::{
    extensions, register_api, Function, RegistrationVm, SimulationCollector, UiCollector,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    Sentinel,
    Globals,
    Game,
    Nil,
    Text(String),
    Function(Function),
}

#[derive(Default)]
struct Vm {
    stack: Vec<Value>,
    globals: BTreeMap<String, Value>,
    game: Option<BTreeMap<String, Value>>,
    operations: Vec<String>,
    suppress_globals_push: bool,
}

impl RegistrationVm for Vm {
    type Error = &'static str;

    fn top(&mut self) -> Result<i32, Self::Error> {
        Ok(self.stack.len() as i32)
    }
    fn get_globals(&mut self) -> Result<(), Self::Error> {
        self.operations.push("globals".into());
        if !self.suppress_globals_push {
            self.stack.push(Value::Globals);
        }
        Ok(())
    }
    fn get_global(&mut self, name: &str) -> Result<(), Self::Error> {
        self.operations.push(format!("global:{name}"));
        let value = if name == "game" {
            if self.game.is_some() {
                Value::Game
            } else {
                Value::Nil
            }
        } else {
            self.globals.get(name).cloned().unwrap_or(Value::Nil)
        };
        self.stack.push(value);
        Ok(())
    }
    fn register(&mut self, function: Function, name: &str) -> Result<(), Self::Error> {
        self.operations.push(format!("register:{name}:{}", function.name()));
        self.globals.insert(name.into(), Value::Function(function));
        Ok(())
    }
    fn push_string(&mut self, value: &str) -> Result<(), Self::Error> {
        self.operations.push(format!("string:{value}"));
        self.stack.push(Value::Text(value.into()));
        Ok(())
    }
    fn set_table(&mut self, index: i32) -> Result<(), Self::Error> {
        self.operations.push(format!("set_table:{index}"));
        assert_eq!(index, -3);
        let target = self.stack[self.stack.len() - 3].clone();
        // Like Lua, nil is still pushed by GetGlobal and errors at SetTable.
        if target == Value::Nil {
            return Err("unprotected nil table");
        }
        let function = self.stack.pop().unwrap();
        let Value::Text(name) = self.stack.pop().unwrap() else {
            panic!("expected key string");
        };
        match target {
            Value::Globals => self.globals.insert(name, function),
            Value::Game => self.game.as_mut().unwrap().insert(name, function),
            _ => panic!("unexpected target"),
        };
        Ok(())
    }
    fn pop(&mut self, count: i32) -> Result<(), Self::Error> {
        self.operations.push(format!("pop:{count}"));
        assert_eq!(count, 1);
        self.stack.pop().unwrap();
        Ok(())
    }
    fn set_top(&mut self, top: i32) -> Result<(), Self::Error> {
        self.operations.push(format!("set_top:{top}"));
        self.stack.truncate(top as usize);
        Ok(())
    }
}

// Independent specification order (advisors.md §8.2, lua-host.md §8.1).
const NAMES: &str = "advice_info trend_value trend_slope trend_count camera_jump camera_zoom \
ordinance_is_on ordinance_set_off ordinance_set_on ordinance_is_available ordinance_name \
ordinance_description ordinance_enact_fee ordinance_repeal_fee ordinance_monthly_cost \
difficulty_level event_feedback expire_advice retire_advice advice_subject trigger_event \
building_is_group event_building_distance mysim_distance_to_closest_building \
mysim_distance_to_closest_landmark_or_reward mysim_local_school_grade mysim_local_hospital_grade \
mysim_dispatch_tool event_disaster_distance random_chance nd_cancel reward_instance_count \
unlock_reward_building get_reward_building_state is_tutorial _combine_guids _dofile_from_resource \
print tool_plop_network tool_plop_zone tool_plop_building tool_plop_flora tool_button window_budget \
window_map window_query window_graph";

#[test]
fn collector_order_flags_identity_and_shared_process_registry() {
    let expected: Vec<_> = NAMES.split_whitespace().collect();
    let entries = extensions();
    assert_eq!(entries.len(), 47);
    assert_eq!(entries.iter().map(|e| e.name).collect::<Vec<_>>(), expected);
    assert_eq!(expected.iter().collect::<BTreeSet<_>>().len(), 47);
    for (i, entry) in entries.iter().enumerate() {
        assert_eq!(entry.is_global, (35..38).contains(&i));
        assert_eq!(entry.function.name(), expected[i]);
    }
    assert!(std::ptr::eq(entries, extensions()));
    assert!(std::thread::spawn(|| std::ptr::eq(extensions(), extensions()))
        .join()
        .unwrap());
}

#[test]
fn both_collectors_append_without_clearing_existing_entries() {
    let mut entries = vec![extensions()[46]];
    SimulationCollector::append_to(&mut entries);
    assert_eq!(entries.len(), 39);
    assert_eq!(&entries[1..], &extensions()[..38]);
    UiCollector::append_to(&mut entries);
    assert_eq!(entries[0].name, "window_graph");
    assert_eq!(&entries[1..], extensions());
}

#[test]
fn t9_register_api_globals_and_all_44_game_names() {
    let mut vm = Vm {
        stack: vec![Value::Sentinel, Value::Sentinel],
        game: Some(BTreeMap::new()),
        ..Vm::default()
    };
    assert_eq!(register_api(&mut vm, true), Ok(true));
    assert_eq!(vm.stack, [Value::Sentinel, Value::Sentinel]);
    assert!(vm.game.as_ref().unwrap().is_empty());
    assert_eq!(vm.globals.len(), 4); // three callbacks plus the visible scratch global
    assert_eq!(vm.globals["_tf_"], vm.globals["print"]);
    assert_eq!(
        vm.operations[0..6],
        [
            "globals",
            "register:_tf_:_combine_guids",
            "string:_combine_guids",
            "global:_tf_",
            "set_table:-3",
            "pop:1",
        ]
    );
    vm.operations.clear();
    assert_eq!(register_api(&mut vm, false), Ok(true));
    assert_eq!(vm.stack, [Value::Sentinel, Value::Sentinel]);
    let game = vm.game.as_ref().unwrap();
    assert_eq!(game.len(), 44);
    assert_eq!(vm.globals["_tf_"], game["window_graph"]);
    let expected: Vec<_> = NAMES.split_whitespace().collect();
    for (i, name) in expected.iter().enumerate() {
        if !(35..38).contains(&i) {
            let Value::Function(function) = game[*name] else {
                panic!("not a function: {name}");
            };
            assert_eq!(function.name(), *name);
        }
    }
    let registered: Vec<_> = vm
        .operations
        .iter()
        .filter_map(|op| op.strip_prefix("register:_tf_:"))
        .collect();
    assert_eq!(
        registered,
        expected[..35]
            .iter()
            .chain(&expected[38..])
            .copied()
            .collect::<Vec<_>>()
    );
    // Reload reassigns the callbacks, preserving unrelated table entries.
    vm.game.as_mut().unwrap().insert("custom".into(), Value::Sentinel);
    assert_eq!(register_api(&mut vm, false), Ok(true));
    assert_eq!(vm.game.as_ref().unwrap().len(), 45);
    assert_eq!(vm.game.as_ref().unwrap()["custom"], Value::Sentinel);
    assert_eq!(vm.globals["_tf_"], vm.game.as_ref().unwrap()["window_graph"]);
}

#[test]
fn missing_game_reaches_unprotected_set_table_and_does_not_fake_success() {
    let mut vm = Vm::default();
    assert_eq!(register_api(&mut vm, false), Err("unprotected nil table"));
    assert_eq!(
        vm.operations,
        [
            "global:game",
            "register:_tf_:advice_info",
            "string:advice_info",
            "global:_tf_",
            "set_table:-3",
        ]
    );
    assert_eq!(vm.stack.len(), 3);
    assert_eq!(vm.stack[0], Value::Nil);
    assert_eq!(vm.globals["_tf_"], Value::Function(extensions()[0].function));
}

#[test]
fn unchanged_target_top_skips_assignment_and_still_restores_top() {
    let mut vm = Vm {
        stack: vec![Value::Sentinel],
        suppress_globals_push: true,
        ..Vm::default()
    };
    assert_eq!(register_api(&mut vm, true), Ok(true));
    assert_eq!(vm.operations, ["globals", "globals", "globals", "set_top:1"]);
    assert!(vm.globals.is_empty());
    assert_eq!(vm.stack, [Value::Sentinel]);
}

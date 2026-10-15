// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The `game.*` functions and the globals (§8.3).
//!
//! The Lua host calls [`AdvisorSystem::call_lua`] with the function's name
//! and its arguments; the result is the list of returned values (empty: no
//! value). "ctx" is the context advice (§6.7).

use std::rc::Rc;

use crate::x87::r24;

use super::subject::AdviceSubject;
use super::system::AdvisorSystem;
use super::{event, flags, AdvisorKind, MsgObj, OutMessage};

/// A Lua value crossing the API.
#[derive(Clone, Debug, Default)]
pub enum LuaValue {
    #[default]
    Nil,
    Bool(bool),
    Number(f64),
    /// A string (tag-1 userdata on the way out).
    Str(Vec<u8>),
    /// A subject userdata (tag 0); `None` = a null userdata.
    Subject(Option<Rc<AdviceSubject>>),
}

impl PartialEq for LuaValue {
    fn eq(&self, o: &LuaValue) -> bool {
        match (self, o) {
            (LuaValue::Nil, LuaValue::Nil) => true,
            (LuaValue::Bool(a), LuaValue::Bool(b)) => a == b,
            (LuaValue::Number(a), LuaValue::Number(b)) => a.to_bits() == b.to_bits() || a == b,
            (LuaValue::Str(a), LuaValue::Str(b)) => a == b,
            (LuaValue::Subject(a), LuaValue::Subject(b)) => match (a, b) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                _ => false,
            },
            _ => false,
        }
    }
}

impl LuaValue {
    /// A number argument (Lua converts numeric strings [choice: no]).
    pub fn num(&self) -> Option<f64> {
        match self {
            LuaValue::Number(n) => Some(*n),
            _ => None,
        }
    }
    /// An integer argument, truncated toward zero.
    pub fn int(&self) -> Option<i64> {
        self.num().map(|n| if n.is_nan() { 0 } else { n.trunc() as i64 })
    }
    /// A subject argument.
    pub fn subject(&self) -> Option<Rc<AdviceSubject>> {
        match self {
            LuaValue::Subject(s) => s.clone(),
            _ => None,
        }
    }
}

/// The names of the UI functions (no simulation effect).
pub const UI_FUNCTIONS: [&str; 9] = [
    "camera_jump",
    "camera_zoom",
    "tool_plop_building",
    "tool_plop_network",
    "tool_button",
    "window_open",
    "window_close",
    "window_query",
    "advice_info",
];

/// The `trend_slope` scale (`63.6618f`).
pub const TREND_SLOPE_SCALE: f32 = 63.6618;

/// `trend_slope`: `f32(atan(m) × 63.6618f)`.
pub fn trend_slope_of(m: f32) -> f64 {
    r24(f64::from(m).atan() * f64::from(TREND_SLOPE_SCALE))
}

/// `_combine_guids(a, b)`: `a XOR b` as a signed int32; −1 on bad
/// arguments.
pub fn combine_guids(a: Option<i64>, b: Option<i64>) -> i32 {
    match (a, b) {
        (Some(a), Some(b)) => ((a as u32) ^ (b as u32)) as i32,
        _ => -1,
    }
}

fn one(v: LuaValue) -> Vec<LuaValue> {
    vec![v]
}

fn num(v: f64) -> Vec<LuaValue> {
    one(LuaValue::Number(v))
}

fn yes_nil(b: bool) -> Vec<LuaValue> {
    if b {
        num(1.0)
    } else {
        one(LuaValue::Nil)
    }
}

impl AdvisorSystem {
    /// Calls the `game.*` function or global `name` (§8.3). `None`: not a
    /// function of this module.
    pub fn call_lua(self: &Rc<Self>, name: &str, args: &[LuaValue]) -> Option<Vec<LuaValue>> {
        let h = self.shared().host.clone();
        let arg = |i: usize| args.get(i).cloned().unwrap_or_default();
        let ctx = self.context_advice();
        let now = h.sim_date().day_number;
        let months_back = |m: i64| now.wrapping_sub((m as i32).wrapping_mul(30));
        let r = match name {
            // ---- history ----
            "trend_value" => {
                let (Some(t), Some(m)) = (arg(0).int(), arg(1).int()) else {
                    return Some(vec![]);
                };
                num(h.history_value(t as u32, months_back(m)).unwrap_or(0.0))
            }
            "trend_slope" => {
                let (Some(t), Some(m)) = (arg(0).int(), arg(1).int()) else {
                    return Some(vec![]);
                };
                num(h
                    .history_trend(t as u32, months_back(m), now)
                    .map_or(0.0, trend_slope_of))
            }
            "trend_count" => {
                let (Some(t), Some(m)) = (arg(0).int(), arg(1).int()) else {
                    return Some(vec![]);
                };
                num(h
                    .history_count(t as u32, months_back(m).wrapping_add(1), now)
                    .unwrap_or(0.0))
            }
            // ---- ordinances ----
            "ordinance_is_on" | "ordinance_is_available" => {
                let Some(id) = arg(0).int() else { return Some(vec![]) };
                yes_nil(h.ordinance_flag(id as u32, name == "ordinance_is_on") == Some(true))
            }
            "ordinance_set_on" | "ordinance_set_off" => {
                let Some(id) = arg(0).int() else { return Some(vec![]) };
                h.set_ordinance_on(id as u32, name == "ordinance_set_on");
                vec![]
            }
            "ordinance_enact_fee" | "ordinance_repeal_fee" | "ordinance_monthly_cost" => {
                let Some(id) = arg(0).int() else { return Some(vec![]) };
                let which = match name {
                    "ordinance_enact_fee" => 0,
                    "ordinance_repeal_fee" => 1,
                    _ => 2,
                };
                num(h.ordinance_amount(id as u32, which).unwrap_or(0) as f64)
            }
            "ordinance_name" | "ordinance_description" => {
                let Some(id) = arg(0).int() else { return Some(vec![]) };
                let which = u32::from(name == "ordinance_description");
                one(h.ordinance_text(id as u32, which).map_or(LuaValue::Nil, LuaValue::Str))
            }
            // ---- the context advice ----
            "event_feedback" => {
                let Some(a) = ctx else { return Some(vec![]) };
                let n = arg(0).int().unwrap_or(0) as u32;
                let obj = a.event_subject().and_then(|s| s.object());
                h.post(&OutMessage::new(
                    a.event_id(),
                    n,
                    obj.map_or(MsgObj::None, MsgObj::Host),
                ));
                vec![]
            }
            "expire_advice" => {
                if let Some(a) = ctx {
                    a.set_flags(flags::EXPIRE_REQUEST, true);
                }
                vec![]
            }
            "retire_advice" => {
                if let Some(a) = ctx {
                    if let Some(adv) = self.get_advisor(a.advisor_id()) {
                        adv.retire_advice(Some(&a));
                    }
                }
                vec![]
            }
            "advice_subject" => {
                // [choice] No context or no subject: nil.
                match ctx.and_then(|a| a.event_subject()) {
                    Some(s) => one(LuaValue::Subject(Some(s.clone_subject()))),
                    None => one(LuaValue::Nil),
                }
            }
            "trigger_event" => {
                let Some(g) = arg(0).int() else { return Some(vec![]) };
                self.push_event(g as u32, None);
                vec![]
            }
            "difficulty_level" => num(f64::from(h.difficulty().unwrap_or(-1))),
            "random_chance" => {
                let Some(p) = arg(0).num() else { return Some(vec![]) };
                let r = self.with_random(|g| g.range_f64(0.0, 100.0));
                yes_nil(r < p)
            }
            "is_tutorial" => {
                let t = h.tutorial_guid();
                match arg(0).int() {
                    Some(g) => yes_nil(t == Some(g as u32)),
                    // [question] Without an argument: any tutorial.
                    None => yes_nil(t.is_some_and(|t| t != 0)),
                }
            }
            // ---- subjects ----
            "building_is_group" => {
                let (Some(s), Some(g)) = (arg(0).subject(), arg(1).int()) else {
                    return Some(vec![]);
                };
                let Some(o) = s.object() else {
                    return Some(one(LuaValue::Nil));
                };
                yes_nil(h.is_occupant_group(o, g as u32))
            }
            "event_building_distance" => {
                let Some(a) = ctx else { return Some(vec![]) };
                match a.event_subject() {
                    Some(s) if a.event_id() == event::MYSIM_BUILDING_ADD => num(f64::from(s.data1())),
                    _ => one(LuaValue::Nil),
                }
            }
            "event_disaster_distance" => {
                let Some(a) = ctx else { return Some(vec![]) };
                match a.event_subject() {
                    Some(s) => num(f64::from(s.data1())),
                    None => one(LuaValue::Nil),
                }
            }
            "nd_cancel" => {
                if let Some(o) = arg(0).subject().and_then(|s| s.object()) {
                    if h.deal_type(o).is_some() {
                        h.cancel_deal(o);
                    }
                }
                vec![]
            }
            // ---- MySim (mysims.md §7.2) ----
            "mysim_distance_to_closest_building"
            | "mysim_distance_to_closest_landmark_or_reward"
            | "mysim_local_school_grade"
            | "mysim_local_hospital_grade"
            | "mysim_dispatch_tool" => {
                let ext = ctx
                    .and_then(|a| self.get_advisor(a.advisor_id()))
                    .and_then(|adv| adv.ext());
                let Some(ext) = ext else {
                    return Some(one(LuaValue::Nil));
                };
                let g = arg(0).int().unwrap_or(0) as u32;
                let v = match name {
                    "mysim_distance_to_closest_building" => ext.distance_to_closest(g).map(f64::from),
                    "mysim_distance_to_closest_landmark_or_reward" => ext.distance_to_closest_special(g).map(f64::from),
                    "mysim_local_school_grade" => ext.local_school_grade(g).map(f64::from),
                    "mysim_local_hospital_grade" => ext.local_hospital_grade(g).map(f64::from),
                    _ => ext.create_dispatch_tool().map(|b| if b { 1.0 } else { 0.0 }),
                };
                one(v.map_or(LuaValue::Nil, LuaValue::Number))
            }
            // ---- rewards ----
            "reward_instance_count" => {
                let Some(id) = arg(0).int() else { return Some(vec![]) };
                match h.conditional_status(id as u32) {
                    Some(st) => num(f64::from(st.building_count)),
                    None => {
                        let n = self
                            .advisors()
                            .into_iter()
                            .find(|(_, a)| a.kind() == AdvisorKind::CityPlanning)
                            .and_then(|(_, a)| a.state().landmarks.get(&(id as i32)).copied())
                            .unwrap_or(0);
                        num(f64::from(n))
                    }
                }
            }
            "get_reward_building_state" => {
                let Some(id) = arg(0).int() else { return Some(vec![]) };
                match h.conditional_status(id as u32) {
                    None => num(1.0),
                    Some(st) if st.status & super::status_bits::AVAILABLE != 0 => num(2.0),
                    // Its argument [quirk].
                    Some(_) => one(arg(0)),
                }
            }
            "unlock_reward_building" => {
                let Some(id) = arg(0).int() else { return Some(vec![]) };
                let mut st = h.conditional_status(id as u32).unwrap_or_default();
                st.status = super::status_bits::AVAILABLE;
                h.update_conditional_status(id as u32, &st);
                vec![]
            }
            // ---- globals ----
            "_combine_guids" => num(f64::from(combine_guids(arg(0).int(), arg(1).int()))),
            "print" => vec![],
            n if UI_FUNCTIONS.contains(&n) => vec![],
            _ => return None,
        };
        Some(r)
    }
}

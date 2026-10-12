// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! A demand node (`cISC4Demand`, class `0x8990C12D`, §3, §12.2).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::props::round_half_even_f32;
use crate::stream::{InStream, OutStream, SaveObject};

use super::{DemandPoster, CLSID_DEMAND_NODE};

/// Record version written (§12.2).
pub const NODE_RECORD_VERSION: u16 = 2;

/// `+8.507059e37` (bit pattern `0x7E7FFFFF`, §3.1).
const DEFAULT_MAX: u32 = 0x7E7F_FFFF;
/// `−8.507059e37` (bit pattern `0xFE7FFFFF`).
const DEFAULT_MIN: u32 = 0xFE7F_FFFF;

/// The node state (§3.1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeData {
    pub loaded_from_save: bool,
    pub initialized: bool,
    pub id: u32,
    pub supply: f32,
    pub demand: f32,
    pub new_supply: f32,
    pub new_demand: f32,
    pub debug_lock_value: f32,
    pub debug_lock_flag: f32,
    pub active_max: f32,
    pub active_min: f32,
    pub active_demand: f32,
    pub flag: u8,
    pub economy_modifier: f32,
    pub tax_modifier: f32,
    /// `SC4Percentage` (1.0 = 100 %).
    pub demand_cap: f32,
    /// 0 none, 1 = quantity in supply, 2 = in demand. Not saved.
    pub region_use: u32,
}

impl Default for NodeData {
    fn default() -> Self {
        NodeData {
            loaded_from_save: false,
            initialized: false,
            id: 0,
            supply: 0.0,
            demand: 0.0,
            new_supply: 0.0,
            new_demand: 0.0,
            debug_lock_value: 0.0,
            debug_lock_flag: 0.0,
            active_max: f32::from_bits(DEFAULT_MAX),
            active_min: f32::from_bits(DEFAULT_MIN),
            active_demand: 0.0,
            flag: 1,
            economy_modifier: 0.0,
            tax_modifier: 0.0,
            demand_cap: 0.0,
            region_use: 0,
        }
    }
}

impl NodeData {
    /// Resets every §3.1 value to its initial value, keeping the two
    /// lifecycle flags.
    fn reset_values(&mut self) {
        *self = NodeData {
            loaded_from_save: self.loaded_from_save,
            initialized: self.initialized,
            ..NodeData::default()
        };
    }

    /// `N` (§3.4): the normalised active demand in −1000…+1000, in `f32`.
    fn normalised(&self) -> f32 {
        if self.active_demand >= 0.0 {
            self.active_demand / self.active_max * 1000.0
        } else {
            -(self.active_demand / self.active_min) * 1000.0
        }
    }
}

/// `clamp(v, lo, hi)` as §3.3 defines it: `hi` if `v > hi`, else `lo` if
/// `v < lo`, else `v`.
fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    if v > hi {
        hi
    } else if v < lo {
        lo
    } else {
        v
    }
}

/// `min` / `max` with the comparison order of the spec (`a` unless `b` is
/// strictly smaller / larger).
fn fmin(a: f32, b: f32) -> f32 {
    if b < a {
        b
    } else {
        a
    }
}

fn fmax(a: f32, b: f32) -> f32 {
    if b > a {
        b
    } else {
        a
    }
}

/// A float to `i32` the way a C cast truncates; out-of-range values and NaN
/// saturate (Rust `as`), which never panics.
fn to_i32(x: f32) -> i32 {
    x as i32
}

/// A demand node.
pub struct DemandNode {
    poster: Rc<dyn DemandPoster>,
    token: Cell<u64>,
    d: RefCell<NodeData>,
}

impl DemandNode {
    /// A fresh node (as its class factory creates it).
    pub fn new(poster: Rc<dyn DemandPoster>, token: u64) -> Rc<DemandNode> {
        Rc::new(DemandNode {
            poster,
            token: Cell::new(token),
            d: RefCell::new(NodeData::default()),
        })
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    /// A copy of the node state.
    pub fn snapshot(&self) -> NodeData {
        *self.d.borrow()
    }

    fn post(&self, key: u32, data2: i32) {
        self.poster.active_demand_changed(self.token(), key, data2);
    }

    // ---- lifecycle (§3.2) ----

    /// `Init()`: on the first call, resets every value unless the node was
    /// loaded from a save. Acquiring the city and the message server is the
    /// shim's part.
    pub fn init(&self) -> bool {
        let mut d = self.d.borrow_mut();
        if !d.initialized {
            d.initialized = true;
            if !d.loaded_from_save {
                d.reset_values();
            }
        }
        true
    }

    /// `Shutdown()`.
    pub fn shutdown(&self) -> bool {
        self.d.borrow_mut().initialized = false;
        true
    }

    /// `SimulationBegin()` (§3.2): recalculates unless loaded from a save,
    /// then posts again with `data2 = trunc(N)`.
    pub fn simulation_begin(&self) -> bool {
        if !self.d.borrow().loaded_from_save {
            self.calculate_active_demand(false);
        }
        let (key, n) = {
            let d = self.d.borrow();
            (d.id, d.normalised())
        };
        self.post(key, to_i32(n.trunc()));
        true
    }

    // ---- accessors (§3.2) ----

    pub fn id(&self) -> u32 {
        self.d.borrow().id
    }
    pub fn set_id(&self, id: u32) -> bool {
        self.d.borrow_mut().id = id;
        true
    }
    pub fn supply(&self) -> f32 {
        self.d.borrow().supply
    }
    pub fn demand(&self) -> f32 {
        self.d.borrow().demand
    }
    pub fn new_supply(&self) -> f32 {
        self.d.borrow().new_supply
    }
    pub fn new_demand(&self) -> f32 {
        self.d.borrow().new_demand
    }
    pub fn active_demand(&self) -> f32 {
        self.d.borrow().active_demand
    }
    pub fn active_max(&self) -> f32 {
        self.d.borrow().active_max
    }
    pub fn active_min(&self) -> f32 {
        self.d.borrow().active_min
    }
    pub fn economy_modifier(&self) -> f32 {
        self.d.borrow().economy_modifier
    }
    pub fn tax_modifier(&self) -> f32 {
        self.d.borrow().tax_modifier
    }
    /// `GetDemandCap()`: the shim returns a pointer to a float it keeps in
    /// step with this value.
    pub fn demand_cap(&self) -> f32 {
        self.d.borrow().demand_cap
    }
    pub fn region_use(&self) -> u32 {
        self.d.borrow().region_use
    }

    /// `AddToSupplyValue(x)`: pending only.
    pub fn add_to_supply(&self, x: f32) -> bool {
        self.d.borrow_mut().new_supply += x;
        true
    }
    /// `AddToDemandValue(x)`: pending only.
    pub fn add_to_demand(&self, x: f32) -> bool {
        self.d.borrow_mut().new_demand += x;
        true
    }
    /// `SetSupplyValue(x)`: the committed total, no propagation.
    pub fn set_supply(&self, x: f32) -> bool {
        self.d.borrow_mut().supply = x;
        true
    }
    /// `SetDemandValue(x)`.
    pub fn set_demand(&self, x: f32) -> bool {
        self.d.borrow_mut().demand = x;
        true
    }
    /// `SetActiveDemandMax(x)`: never below the minimum.
    pub fn set_active_max(&self, x: f32) -> bool {
        let mut d = self.d.borrow_mut();
        d.active_max = x;
        if d.active_max < d.active_min {
            d.active_max = d.active_min;
        }
        true
    }
    /// `SetActiveDemandMin(x)`: never above the maximum.
    pub fn set_active_min(&self, x: f32) -> bool {
        let mut d = self.d.borrow_mut();
        d.active_min = x;
        if d.active_min > d.active_max {
            d.active_min = d.active_max;
        }
        true
    }
    /// `SetEconomyModifier(x)`: no recalculation.
    pub fn set_economy_modifier(&self, x: f32) -> bool {
        self.d.borrow_mut().economy_modifier = x;
        true
    }
    /// `SetTaxModifier(x)`: no recalculation.
    pub fn set_tax_modifier(&self, x: f32) -> bool {
        self.d.borrow_mut().tax_modifier = x;
        true
    }
    /// `SetDemandCap(cap)`: stored unclamped, no recalculation.
    pub fn set_demand_cap(&self, x: f32) -> bool {
        self.d.borrow_mut().demand_cap = x;
        true
    }
    pub fn set_region_use(&self, v: u32) -> bool {
        self.d.borrow_mut().region_use = v;
        true
    }

    /// `EndOfCycle()`: folds the pending values into the totals,
    /// recalculates (which posts) and returns whether anything was pending.
    /// [compat] The shim returns this as a bool in the integer register.
    pub fn end_of_cycle(&self) -> bool {
        let changed = {
            let mut d = self.d.borrow_mut();
            let changed = d.new_supply != 0.0 || d.new_demand != 0.0;
            d.supply += d.new_supply;
            d.demand += d.new_demand;
            d.new_supply = 0.0;
            d.new_demand = 0.0;
            changed
        };
        self.calculate_active_demand(false);
        changed
    }

    /// `DebugIsValueLocked()`: always false. (`DebugLockValue` and
    /// `DebugUnlockValue` do nothing.)
    pub fn debug_is_value_locked(&self) -> bool {
        false
    }

    /// `CalculateActiveDemand(smooth)` (§3.3), then posts ActiveDemandChanged
    /// with `data2 = roundHalfEven(N)` (§3.4).
    pub fn calculate_active_demand(&self, smooth: bool) {
        let (key, n) = {
            let mut d = self.d.borrow_mut();
            let (min, max) = (d.active_min, d.active_max);
            let base = clamp(d.demand - d.supply, min, max);
            let m = d.economy_modifier + d.tax_modifier;
            let bonus = fmin(max * m, d.demand * m);
            let mut target = clamp(base + bonus, min, max);
            target = fmax(min, fmin(target, max - max * d.demand_cap));
            if smooth {
                // [choice] kept for completeness; no caller uses it (§3.3).
                let step = (max - min) * 0.07;
                let delta = target - d.active_demand;
                let a = d.active_demand;
                if delta.abs() >= step {
                    d.active_demand = if delta > 0.0 {
                        if target > 0.0 && a < 0.0 && a + step < 0.0 {
                            0.0
                        } else {
                            a + step
                        }
                    } else if target < 0.0 && a > 0.0 && a - step > 0.0 {
                        0.0
                    } else {
                        a - step
                    };
                } else {
                    d.active_demand = target;
                }
            } else {
                d.active_demand = target;
            }
            (d.id, d.normalised())
        };
        self.post(key, to_i32(round_half_even_f32(n)));
    }
}

impl SaveObject for DemandNode {
    fn class_id(&self) -> u32 {
        CLSID_DEMAND_NODE
    }

    /// §12.2.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        let d = *self.d.borrow();
        out.put_u16(NODE_RECORD_VERSION);
        out.put_u32(d.id);
        out.put_f32(d.supply);
        out.put_f32(d.demand);
        out.put_f32(d.new_supply);
        out.put_f32(d.new_demand);
        out.put_f32(d.debug_lock_value);
        out.put_f32(d.debug_lock_flag);
        out.put_f32(d.active_min);
        out.put_f32(d.active_max);
        out.put_f32(d.active_demand);
        out.put_u8(d.flag);
        out.put_f32(d.economy_modifier);
        out.put_f32(d.tax_modifier);
        out.put_f32(d.demand_cap);
        out.error() == 0
    }

    /// §12.2: sets loadedFromSave first; fails for version ≥ 3; version 1
    /// stores the cap as a percent byte.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        self.d.borrow_mut().loaded_from_save = true;
        let Some(version) = inp.get_u16() else { return false };
        if version >= 3 {
            return false;
        }
        let parsed = (|| -> Option<NodeData> {
            let cur = *self.d.borrow();
            let mut d = NodeData {
                loaded_from_save: true,
                initialized: cur.initialized,
                region_use: cur.region_use,
                ..NodeData::default()
            };
            d.id = inp.get_u32()?;
            d.supply = inp.get_f32()?;
            d.demand = inp.get_f32()?;
            d.new_supply = inp.get_f32()?;
            d.new_demand = inp.get_f32()?;
            d.debug_lock_value = inp.get_f32()?;
            d.debug_lock_flag = inp.get_f32()?;
            d.active_min = inp.get_f32()?;
            d.active_max = inp.get_f32()?;
            d.active_demand = inp.get_f32()?;
            d.flag = inp.get_u8()?;
            d.economy_modifier = inp.get_f32()?;
            d.tax_modifier = inp.get_f32()?;
            d.demand_cap = if version >= 2 {
                inp.get_f32()?
            } else {
                f32::from(inp.get_u8()?) * 0.01
            };
            Some(d)
        })();
        match parsed {
            Some(d) => {
                *self.d.borrow_mut() = d;
                inp.error() == 0
            }
            None => false,
        }
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::{ByteReader, ByteWriter};

    #[derive(Default)]
    struct Log(RefCell<Vec<(u32, i32)>>);
    impl DemandPoster for Log {
        fn active_demand_changed(&self, _node: u64, key: u32, data2: i32) {
            self.0.borrow_mut().push((key, data2));
        }
    }

    fn node() -> (Rc<Log>, Rc<DemandNode>) {
        let log = Rc::new(Log::default());
        let n = DemandNode::new(log.clone(), 7);
        n.init();
        n.set_id(0x21010);
        (log, n)
    }

    #[test]
    fn pending_values_fold_at_end_of_cycle() {
        let (log, n) = node();
        n.add_to_supply(5.0);
        n.add_to_demand(7.0);
        assert_eq!(
            (n.supply(), n.demand(), n.new_supply(), n.new_demand()),
            (0.0, 0.0, 5.0, 7.0)
        );
        assert!(log.0.borrow().is_empty());
        assert!(n.end_of_cycle());
        assert_eq!(
            (n.supply(), n.demand(), n.new_supply(), n.new_demand()),
            (5.0, 7.0, 0.0, 0.0)
        );
        assert_eq!(*log.0.borrow(), [(0x21010, 0)]);
        assert!(!n.end_of_cycle());
        assert_eq!(log.0.borrow().len(), 2);
    }

    #[test]
    fn active_demand_formula() {
        let (log, n) = node();
        n.set_active_min(-6000.0);
        n.set_active_max(6000.0);
        n.set_demand(1000.0);
        n.set_supply(400.0);
        n.calculate_active_demand(false);
        assert_eq!(n.active_demand(), 600.0);
        n.set_economy_modifier(0.03);
        n.calculate_active_demand(false);
        assert_eq!(n.active_demand(), 630.0);
        n.set_demand_cap(0.95);
        n.calculate_active_demand(false);
        assert_eq!(n.active_demand(), 300.0);
        let got: Vec<i32> = log.0.borrow().iter().map(|e| e.1).collect();
        assert_eq!(got, [100, 105, 50]);
        // Negative values normalise by the minimum.
        n.set_economy_modifier(0.0);
        n.set_demand_cap(0.0);
        n.set_demand(0.0);
        n.set_supply(1500.0);
        n.calculate_active_demand(false);
        assert_eq!(log.0.borrow().last().unwrap().1, -250);
        // Ties round to even; SimulationBegin truncates.
        n.set_supply(0.0);
        n.set_demand(3.0);
        n.calculate_active_demand(false);
        assert_eq!(log.0.borrow().last().unwrap().1, 0);
        n.set_demand(9.0);
        n.calculate_active_demand(false);
        assert_eq!(log.0.borrow().last().unwrap().1, 2);
        log.0.borrow_mut().clear();
        n.simulation_begin();
        assert_eq!(*log.0.borrow(), [(0x21010, 2), (0x21010, 1)]);
    }

    #[test]
    fn no_min_max_reports_zero() {
        let (log, n) = node();
        n.set_demand(1000.0);
        n.calculate_active_demand(false);
        assert_eq!(n.active_demand(), 1000.0);
        assert_eq!(log.0.borrow()[0].1, 0);
    }

    #[test]
    fn min_max_ordering() {
        let (_, n) = node();
        n.set_active_min(-10.0);
        n.set_active_max(10.0);
        n.set_active_min(20.0);
        assert_eq!(n.active_min(), 10.0);
        n.set_active_max(-50.0);
        assert_eq!(n.active_max(), 10.0);
    }

    #[test]
    fn slew_limiter() {
        let (_, n) = node();
        n.set_active_min(-100.0);
        n.set_active_max(100.0);
        n.set_demand(100.0);
        n.calculate_active_demand(true);
        assert_eq!(n.active_demand(), 14.0); // step 200 · 0.07
    }

    #[test]
    fn record_round_trip_and_versions() {
        let (_, n) = node();
        n.set_active_min(-6000.0);
        n.set_active_max(6000.0);
        n.set_supply(1.5);
        n.add_to_demand(2.5);
        n.set_demand_cap(0.25);
        let mut w = ByteWriter::new();
        assert!(n.write(&mut w));
        let bytes = w.into_bytes();
        assert_eq!(bytes.len(), 2 + 4 + 4 * 9 + 1 + 4 * 3);
        let (_, m) = node();
        assert!(m.read(&mut ByteReader::new(&bytes)));
        let mut w2 = ByteWriter::new();
        m.write(&mut w2);
        assert_eq!(w2.bytes(), bytes.as_slice());
        assert!(m.snapshot().loaded_from_save);
        // Version 1: byte percent cap.
        let mut v1 = bytes.clone();
        v1[0] = 1;
        v1.truncate(v1.len() - 4);
        v1.push(50);
        let (_, k) = node();
        assert!(k.read(&mut ByteReader::new(&v1)));
        assert_eq!(k.demand_cap(), 0.5);
        // Version 3 is rejected, but loadedFromSave is set.
        let mut v3 = bytes.clone();
        v3[0] = 3;
        let (_, r) = node();
        assert!(!r.read(&mut ByteReader::new(&v3)));
        assert!(r.snapshot().loaded_from_save);
    }

    #[test]
    fn init_resets_unless_loaded() {
        let log = Rc::new(Log::default());
        let n = DemandNode::new(log, 0);
        n.set_supply(5.0);
        n.set_id(3);
        n.init();
        assert_eq!((n.supply(), n.id()), (0.0, 0));
    }
}

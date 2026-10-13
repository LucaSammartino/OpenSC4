// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cISC4DemandSimulator` (§4–§13).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use crate::property::PropertyHolder;
use crate::props;
use crate::stream::{InStream, OutStream, RefValue, SaveObject};

use super::tuning::{DemandExemplar, DemandTuning, TUNING_KEY};
use super::{
    building_prop, connection, history, msg, DemandHost, DemandNode, BUSINESS_DEV_TYPES, CENSUS_KEYS,
    CLSID_DEMAND_SIMULATOR, DEMAND_EXEMPLAR_GROUP, DEMAND_EXEMPLAR_TYPE, INDEX_REGIONAL, RESIDENTIAL_DEV_TYPES,
    SIMULATOR_TYPE, TOTAL_NODES,
};

/// Record version written (§12.1).
pub const SIM_RECORD_VERSION: u16 = 2;

/// [choice] Propagation gives up after this many passes (§4.4).
const MAX_PASSES: u32 = 1000;

/// Census states (§6.1).
const CENSUS_START: u8 = 0;
const CENSUS_SCANNING: u8 = 1;
const CENSUS_HISTORY: u8 = 2;
const CENSUS_DONE: u8 = 3;

/// Link types (§4.3); the discriminants are the exemplar property IDs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkKind {
    Drives = 0x47BB_3F30,
    Satisfies = 0x47BB_3F31,
    Contributes = 0x47BB_3F32,
    Caps = 0x47BB_3F41,
    Sums = 0x47BB_3F50,
}

/// One link `(sourceKey, targetKey, type, factor)` (§4.2 step 5).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Link {
    pub source: u32,
    pub target: u32,
    pub kind: LinkKind,
    /// `percent · 0.01` for Drives/Satisfies/Contributes; 1.0 otherwise.
    pub factor: f32,
}

/// One agent effect `(nodeKey, satisfies, creates)` (§7.1).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Effect {
    key: u32,
    satisfies: f32,
    creates: f32,
}

#[derive(Default)]
struct SimData {
    initialized: bool,
    loaded_from_save: bool,
    nodes: BTreeMap<u32, Rc<DemandNode>>,
    /// Links by source key, in creation order.
    links: BTreeMap<u32, Vec<Link>>,
    census_state: u8,
    cells: (i32, i32),
    cursor: (i32, i32),
    current: BTreeMap<u32, f32>,
    previous: BTreeMap<u32, f32>,
    co_economy_bonus: f32,
    i_economy_bonus: f32,
    commercial_relief: f32,
    industrial_relief: f32,
    tuning: DemandTuning,
    effects: BTreeMap<u32, Vec<Effect>>,
    prev_cities: Vec<u64>,
    export_cities: Vec<u64>,
    warned: BTreeSet<(u32, u32)>,
}

/// A copy of the observable simulator state (tests, diagnostics, the C ABI).
#[derive(Clone, Debug, PartialEq)]
pub struct DemandSnapshot {
    pub initialized: bool,
    pub loaded_from_save: bool,
    pub census_state: u8,
    pub cursor: (i32, i32),
    pub co_economy_bonus: f32,
    pub i_economy_bonus: f32,
    pub commercial_relief: f32,
    pub industrial_relief: f32,
    pub current_census: BTreeMap<u32, f32>,
    pub previous_census: BTreeMap<u32, f32>,
    pub node_keys: Vec<u32>,
    pub link_count: usize,
    pub effect_building_types: Vec<u32>,
    pub export_cities: Vec<u64>,
}

/// The demand simulator.
pub struct DemandSim {
    host: Rc<dyn DemandHost>,
    token: Cell<u64>,
    d: RefCell<SimData>,
}

/// `clamp(v, 0, 1)` as §3.3 defines it (NaN passes through).
fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

// ---- shared with the demand shadow (super::shadow) ------------------------------------------------

/// Every demand exemplar (§4.1) in the resource manager's enumeration order
/// (§4.2): type `0x6534284A`, ExemplarType 3.
pub(super) fn demand_exemplars(host: &dyn DemandHost) -> Vec<DemandExemplar> {
    let mut out = Vec::new();
    for key in host.exemplar_keys() {
        if key.type_id != props::EXEMPLAR_TYPE {
            continue;
        }
        let Some(ex) = host.exemplar(key) else { continue };
        if !props::has_exemplar_type(&ex, DEMAND_EXEMPLAR_TYPE) {
            continue;
        }
        out.push(DemandExemplar::from_holder(key.instance, &ex));
    }
    out
}

/// §4.1 link factor `r24(float(percent) · 0.01f)`: the integer is exact
/// (`FILD`), the constant is the float32 `0.01f`, the product is rounded to
/// float32 once. The f64 product is exact for `percent < 2^29` (the shipped
/// data uses 0…1500), so the one cast to f32 is that single rounding.
pub(super) fn link_factor(percent: u32) -> f32 {
    (f64::from(percent) * f64::from(0.01f32)) as f32
}

/// §4.2 step 5: the links of one exemplar, in creation order (every
/// Satisfies pair, every Drives pair, every Contributes pair, every Caps key,
/// every Sums key).
pub(super) fn exemplar_links(e: &DemandExemplar) -> Vec<Link> {
    let mut out = Vec::new();
    let pct = |kind, pairs: &[(u32, u32)], out: &mut Vec<Link>| {
        for &(target, percent) in pairs {
            out.push(Link {
                source: e.key,
                target,
                kind,
                factor: link_factor(percent),
            });
        }
    };
    pct(LinkKind::Satisfies, &e.satisfies, &mut out);
    pct(LinkKind::Drives, &e.drives, &mut out);
    pct(LinkKind::Contributes, &e.contributes, &mut out);
    for (kind, list) in [(LinkKind::Caps, &e.caps), (LinkKind::Sums, &e.sums)] {
        for &target in list {
            out.push(Link {
                source: e.key,
                target,
                kind,
                factor: 1.0,
            });
        }
    }
    out
}

/// What one `DoDemandIndexLinks` did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PropagationStats {
    /// Passes run (the last one stable unless `settled` is false).
    pub passes: u32,
    /// False when [`MAX_PASSES`] ran out.
    pub settled: bool,
    /// Transfers and sums corrections performed (each made a pass unstable).
    pub transfers: u32,
    /// `EndOfCycle` calls (one per node per pass).
    pub end_of_cycles: u32,
    /// Link visits whose target is not in the node map (ignored, [choice]).
    pub missing_targets: u32,
}

/// `DoDemandIndexLinks()` (§4.4) over a node map (ascending key order) and
/// its links: the propagation of [`DemandSim`] and of the demand shadow.
/// `missing(source, target)` is called for every visit of a link whose target
/// has no node.
pub(super) fn propagate(
    nodes: &BTreeMap<u32, Rc<DemandNode>>,
    links: &BTreeMap<u32, Vec<Link>>,
    missing: &mut dyn FnMut(u32, u32),
) -> PropagationStats {
    let mut st = PropagationStats::default();
    for _ in 0..MAX_PASSES {
        st.passes += 1;
        let mut stable = true;
        for (key, node) in nodes {
            let mut sum_s = 0.0f32;
            let mut sum_d = 0.0f32;
            let mut has_sums = false;
            for l in links.get(key).map(Vec::as_slice).unwrap_or(&[]) {
                let Some(t) = nodes.get(&l.target) else {
                    st.missing_targets += 1;
                    missing(l.source, l.target);
                    continue;
                };
                match l.kind {
                    LinkKind::Drives | LinkKind::Satisfies => {
                        let v = node.new_supply() * l.factor;
                        if v.abs() > 0.01 {
                            if l.kind == LinkKind::Drives {
                                t.add_to_demand(v);
                            } else {
                                t.add_to_supply(v);
                            }
                            stable = false;
                            st.transfers += 1;
                        }
                    }
                    LinkKind::Contributes => {
                        let v = node.new_demand() * l.factor;
                        if v.abs() > 0.01 {
                            t.add_to_demand(v);
                            stable = false;
                            st.transfers += 1;
                        }
                    }
                    LinkKind::Caps => {
                        let s = node.supply().max(0.01);
                        let dd = node.demand().max(0.0);
                        t.set_demand_cap(if dd < s { dd / s } else { 1.0 });
                    }
                    LinkKind::Sums => {
                        sum_s += t.supply();
                        sum_d += t.demand();
                        has_sums = true;
                    }
                }
            }
            node.end_of_cycle();
            st.end_of_cycles += 1;
            if has_sums {
                let ds = sum_s - node.supply();
                if ds.abs() > 0.01 {
                    node.add_to_supply(ds);
                    stable = false;
                    st.transfers += 1;
                }
                let dd = sum_d - node.demand();
                if dd.abs() > 0.01 {
                    node.add_to_demand(dd);
                    stable = false;
                    st.transfers += 1;
                }
            }
        }
        if stable {
            st.settled = true;
            return st;
        }
    }
    st
}

/// `GetNeutralTaxRate()` (§5) from the supplies of `0x1001`, `0x3001` and
/// `0x4001` (0 for a missing node), unclamped; 0 without the curve.
pub(super) fn neutral_tax_rate_of(tuning: &DemandTuning, r: f32, c: f32, i: f32) -> f32 {
    let pop = r + c + i;
    tuning.neutral_tax_curve.as_ref().map_or(0.0, |cv| cv.eval(pop))
}

/// The §8.1 terms of one of the 12 total nodes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ModifierTerms {
    /// The highest tax rate in the penalty window, or `neutral · 10`.
    pub max_rate: f32,
    /// Whether `max_rate` came from the history (false: [choice] no data).
    pub from_history: bool,
    /// `r24(r24(max_rate · 0.1f) · recip)`, `recip = r24(1.0f / neutral)`.
    pub ratio: f32,
    /// `curveTax(ratio)` (0 without the curve).
    pub tax_curve: f32,
    /// `GetCumulativeFactor(0x2A650000 + id, 0)`.
    pub ordinance_factor: f32,
    /// `Nation.GetEconomyModifier(id)`.
    pub nation: f32,
    /// `r24(r24(factor − 1.0f) + taxMod)`.
    pub tax_modifier: f32,
    /// `nation` plus the CO / I bonus.
    pub economy_modifier: f32,
}

/// The per-update inputs of the §8.1 tax ratio: the clamped neutral rate and
/// its float32 reciprocal, computed once and shared by all 12 types.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TaxBasis {
    /// `neutral_raw`, or `0.1f` if it is below `0.1f` (a NaN is kept).
    pub neutral: f32,
    /// `r24(1.0f / neutral)`.
    pub recip: f32,
}

impl TaxBasis {
    /// §8.1 steps 1-2 from `GetNeutralTaxRate()`.
    pub fn new(neutral_raw: f32) -> TaxBasis {
        // Not `f32::max`: that would turn a NaN into 0.1, the original keeps it.
        let neutral = if neutral_raw < 0.1f32 { 0.1f32 } else { neutral_raw };
        TaxBasis {
            neutral,
            recip: 1.0f32 / neutral,
        }
    }
}

/// §8.1 for total node `index` ([`TOTAL_NODES`] order), with the clamped
/// neutral rate and its reciprocal in `basis`. Reads the history, the
/// ordinance factor and the nation through `host` (in that order); writes
/// nothing. Every operation is float32 (one rounding each, *[compat]*).
pub(super) fn total_node_modifiers(
    index: usize,
    basis: TaxBasis,
    tuning: &DemandTuning,
    co_bonus: f32,
    i_bonus: f32,
    today: u32,
    host: &dyn DemandHost,
) -> ModifierTerms {
    let id = TOTAL_NODES[index].1;
    let series = history::TAX_RATE_BASE + index as u32;
    let history = host.history_max(series, today.saturating_sub(tuning.tax_penalty_days), today);
    let max_rate = history.unwrap_or(basis.neutral * 10.0f32);
    // A multiplication by the rounded reciprocal, not a division (§8.1 step 3).
    let ratio = (max_rate * 0.1f32) * basis.recip;
    let tax_curve = tuning.tax_modifier_curve.as_ref().map_or(0.0, |c| c.eval(ratio));
    let factor = host.ordinance_factor(0x2A65_0000 + id, 0);
    let nation = host.nation_economy_modifier(id);
    let mut econ = nation;
    if id == 0x3320 || id == 0x3330 {
        econ += co_bonus;
    }
    if id >= 0x4100 {
        econ += i_bonus;
    }
    ModifierTerms {
        max_rate,
        from_history: history.is_some(),
        ratio,
        tax_curve,
        ordinance_factor: factor,
        nation,
        tax_modifier: (factor - 1.0f32) + tax_curve,
        economy_modifier: econ,
    }
}

impl DemandSim {
    /// A fresh demand simulator (as its class factory creates it).
    pub fn new(host: Rc<dyn DemandHost>, token: u64) -> Rc<DemandSim> {
        Rc::new(DemandSim {
            host,
            token: Cell::new(token),
            d: RefCell::new(SimData {
                census_state: CENSUS_DONE,
                ..Default::default()
            }),
        })
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn host(&self) -> &Rc<dyn DemandHost> {
        &self.host
    }

    /// `GetSimulatorType()` (§5).
    pub fn simulator_type(&self) -> u32 {
        SIMULATOR_TYPE
    }

    pub fn is_initialized(&self) -> bool {
        self.d.borrow().initialized
    }

    /// The node with `key`, if any (no reference added).
    pub fn node(&self, key: u32) -> Option<Rc<DemandNode>> {
        self.d.borrow().nodes.get(&key).cloned()
    }

    /// Node keys, ascending.
    pub fn node_keys(&self) -> Vec<u32> {
        self.d.borrow().nodes.keys().copied().collect()
    }

    /// Every link, by source key then creation order.
    pub fn links(&self) -> Vec<Link> {
        self.d.borrow().links.values().flatten().copied().collect()
    }

    pub fn tuning(&self) -> DemandTuning {
        self.d.borrow().tuning.clone()
    }

    pub fn snapshot(&self) -> DemandSnapshot {
        let d = self.d.borrow();
        DemandSnapshot {
            initialized: d.initialized,
            loaded_from_save: d.loaded_from_save,
            census_state: d.census_state,
            cursor: d.cursor,
            co_economy_bonus: d.co_economy_bonus,
            i_economy_bonus: d.i_economy_bonus,
            commercial_relief: d.commercial_relief,
            industrial_relief: d.industrial_relief,
            current_census: d.current.clone(),
            previous_census: d.previous.clone(),
            node_keys: d.nodes.keys().copied().collect(),
            link_count: d.links.values().map(Vec::len).sum(),
            effect_building_types: d.effects.keys().copied().collect(),
            export_cities: d.export_cities.clone(),
        }
    }

    /// Sets the two saved economy bonuses (§8.1, §15 Q5: nothing in the
    /// original writes them; tests and plugins may).
    pub fn set_economy_bonuses(&self, co: f32, i: f32) {
        let mut d = self.d.borrow_mut();
        d.co_economy_bonus = co;
        d.i_economy_bonus = i;
    }

    fn warn_once(&self, source: u32, target: u32) {
        let first = self.d.borrow_mut().warned.insert((source, target));
        if first {
            self.host.warn(&format!(
                "demand: link 0x{source:05X} -> 0x{target:05X} has no target node; ignored"
            ));
        }
    }

    // ---- lifecycle (§13) --------------------------------------------------------------------------------

    /// `Init()` (the core part: steps 1–3, 5–7 and 10; acquiring services,
    /// subscribing (step 8, [`msg::SUBSCRIPTIONS`]) and registering the
    /// agents (step 9, [`super::AGENT_LEVELS`]) are the shim's).
    pub fn init(&self) -> bool {
        if self.d.borrow().initialized {
            return true;
        }
        self.build_graph();
        let cells = self.host.cell_counts();
        {
            let mut d = self.d.borrow_mut();
            d.cells = cells;
            d.cursor = (0, 0);
            if d.loaded_from_save {
                d.census_state = CENSUS_START;
            } else {
                d.census_state = CENSUS_DONE;
                d.co_economy_bonus = 0.0;
                d.i_economy_bonus = 0.0;
                d.commercial_relief = 0.0;
                d.industrial_relief = 0.0;
                d.previous.clear();
                for k in CENSUS_KEYS {
                    d.previous.insert(k, 0.0);
                }
            }
        }
        self.reload_tuning();
        self.reset_regional_demands(true);
        for s in history::DAILY {
            self.host.set_history_queue_length(s, 1);
        }
        self.d.borrow_mut().initialized = true;
        self.do_demand_index_links();
        true
    }

    /// `Shutdown()` (core part): shuts down and releases every node and
    /// clears the census, the links ([choice]), the curves and the effects.
    pub fn shutdown(&self) -> bool {
        if !self.d.borrow().initialized {
            return true;
        }
        let nodes = {
            let mut d = self.d.borrow_mut();
            d.initialized = false;
            d.links.clear();
            d.current.clear();
            d.previous.clear();
            d.effects.clear();
            d.tuning.neutral_tax_curve = None;
            d.tuning.tax_modifier_curve = None;
            d.prev_cities.clear();
            d.export_cities.clear();
            std::mem::take(&mut d.nodes)
        };
        for (_, n) in nodes {
            n.shutdown();
            self.host.release(n.token());
        }
        true
    }

    /// §4.2: builds the node map and the links from the demand exemplars.
    fn build_graph(&self) {
        let loaded = self.d.borrow().loaded_from_save;
        for e in demand_exemplars(&*self.host) {
            let existing = self.node(e.key);
            let (node, is_new) = match existing {
                Some(n) => {
                    n.init();
                    (n, false)
                }
                None => {
                    let Some(n) = self.host.create_node() else { continue };
                    n.init();
                    n.set_id(e.key);
                    self.d.borrow_mut().nodes.insert(e.key, n.clone());
                    (n, true)
                }
            };
            if is_new || !loaded {
                if let Some(v) = e.initial_supply {
                    node.set_supply(v as f32);
                }
                if let Some(v) = e.initial_demand {
                    node.set_demand(v as f32);
                }
            }
            if let Some(v) = e.min {
                node.set_active_min(v as f32);
            }
            if let Some(v) = e.max {
                node.set_active_max(v as f32);
            }
            node.set_region_use(e.region_use);
            self.d
                .borrow_mut()
                .links
                .entry(e.key)
                .or_default()
                .extend(exemplar_links(&e));
        }
    }

    /// Re-reads the tuning (§11.1) and the agent effects (§7.1): `Init` step
    /// 5 and the tuning-reload message.
    pub fn reload_tuning(&self) {
        let ex = self.host.exemplar(TUNING_KEY);
        let tuning = DemandTuning::from_exemplar(ex.as_deref());
        self.d.borrow_mut().tuning = tuning;
        self.read_agent_effects();
    }

    /// Replaces the tuning (hosts that read the exemplar themselves, tests).
    pub fn set_tuning(&self, tuning: DemandTuning) {
        self.d.borrow_mut().tuning = tuning;
    }

    /// §7.1: the effect list of every building type.
    pub fn read_agent_effects(&self) {
        for bt in self.host.all_building_types() {
            let Some(ex) = self.host.building_exemplar(bt) else {
                continue;
            };
            let list = self.effects_of(&ex);
            if !list.is_empty() {
                self.d.borrow_mut().effects.insert(bt, list);
            }
        }
    }

    fn effects_of(&self, ex: &PropertyHolder) -> Vec<Effect> {
        let d = self.d.borrow();
        let mut list = Vec::new();
        let created = props::u32_pairs(ex, building_prop::CREATED);
        let created_amounts = props::f32s(ex, building_prop::CREATED_AMOUNTS).unwrap_or_default();
        for (i, (key, amount)) in created.into_iter().enumerate() {
            if d.nodes.contains_key(&key) {
                let creates = created_amounts.get(i).copied().unwrap_or(amount as f32);
                list.push(Effect {
                    key,
                    satisfies: 0.0,
                    creates,
                });
            }
        }
        let satisfied = props::u32_pairs(ex, building_prop::SATISFIED);
        let satisfied_amounts = props::f32s(ex, building_prop::SATISFIED_AMOUNTS).unwrap_or_default();
        for (i, (key, amount)) in satisfied.into_iter().enumerate() {
            if d.nodes.contains_key(&key) {
                let satisfies = satisfied_amounts.get(i).copied().unwrap_or(amount as f32);
                list.push(Effect {
                    key,
                    satisfies,
                    creates: 0.0,
                });
            }
        }
        list
    }

    // ---- interface methods (§5) ------------------------------------------------------------------------

    /// `GetDemand(demandID, demandIndex)`: initializes first if needed.
    pub fn get_demand(&self, demand_id: u32, demand_index: u32) -> Option<Rc<DemandNode>> {
        if !self.is_initialized() {
            self.init();
        }
        self.node(demand_id.wrapping_add(demand_index))
    }

    /// `UpdateOccupantEffects(buildingType, oldPct, newPct)`.
    pub fn update_occupant_effects(&self, building_type: u32, old_pct: f32, new_pct: f32) {
        if old_pct != new_pct {
            self.add_agent_effect(building_type, false, old_pct);
            self.add_agent_effect(building_type, true, new_pct);
        }
    }

    /// `CalculateJobsPerUnitOfDemand(out, demandID, demandIndex)`: from the
    /// current exemplar data.
    pub fn jobs_per_unit(&self, demand_id: u32, demand_index: u32) -> [f32; 3] {
        let mut out = [0.0f32; 3];
        let key = props::exemplar_key(DEMAND_EXEMPLAR_GROUP, demand_id.wrapping_add(demand_index));
        if let Some(ex) = self.host.exemplar(key) {
            for (target, percent) in props::u32_pairs(&ex, LinkKind::Drives as u32) {
                for (w, slot) in out.iter_mut().enumerate() {
                    if target == demand_index.wrapping_add(0x2010 + 0x10 * w as u32) {
                        *slot = percent as f32 / 100.0;
                    }
                }
            }
        }
        out
    }

    /// `GetJobsBySensus(type)`. [choice] a missing node gives 0.
    pub fn jobs_by_census(&self, census_type: u32) -> u32 {
        if matches!(census_type, 0x1011 | 0x1021 | 0x1031) {
            return 0;
        }
        let Some(node) = self.node(census_type) else { return 0 };
        let j = self.jobs_per_unit(census_type, 0);
        let x = (j[0] + j[1] + j[2]) * node.supply();
        if x <= 0.0 || x.is_nan() {
            0
        } else {
            x as u32
        }
    }

    /// `GetNeutralTaxRate()`, unclamped. [choice] a missing node contributes
    /// 0 and an absent curve yields 0.
    pub fn neutral_tax_rate(&self) -> f32 {
        let supply = |k| self.node(k).map_or(0.0, |n| n.supply());
        neutral_tax_rate_of(&self.d.borrow().tuning, supply(0x1001), supply(0x3001), supply(0x4001))
    }

    /// `GetLocalPopulationSummary(map)`.
    pub fn local_population_summary(&self) -> BTreeMap<u32, i32> {
        let nodes: Vec<(u32, Rc<DemandNode>)> = self.d.borrow().nodes.iter().map(|(k, n)| (*k, n.clone())).collect();
        let mut map = BTreeMap::new();
        for (k, n) in nodes {
            match n.region_use() {
                1 => {
                    map.insert(k, n.supply() as i32);
                }
                2 => {
                    map.insert(k, n.demand() as i32);
                }
                _ => {}
            }
        }
        map
    }

    // ---- messages (§10.2) --------------------------------------------------------------------------------

    /// `DoMessage` for the agent levels and the notifications that carry no
    /// lot (StateChanged goes through [`Self::on_state_changed`]). Always
    /// returns true ("finished").
    pub fn handle_message(&self, msg_type: u32, _data1: u32, _data2: u32, data3: u32) -> bool {
        if !self.is_initialized() {
            return true;
        }
        match msg_type {
            msg::SIM_BEGIN => self.simulator_begin(),
            msg::SIM_NEW_DAY => {
                self.do_idle(5);
                self.daily();
            }
            msg::SIM_NEW_MONTH => self.monthly(),
            msg::SIM_IDLE => self.do_idle(data3),
            msg::PRE_SAVE => {
                self.do_demand_index_links();
                self.do_demand_index_links();
            }
            msg::CONNECTION_CREATED | msg::CONNECTION_DELETED => self.reset_regional_demands(false),
            msg::INGRED_CHANGES_MADE => self.reload_tuning(),
            _ => {}
        }
        true
    }

    /// SimulatorBegin: `SimulationBegin()` on every node in key order.
    pub fn simulator_begin(&self) {
        for n in self.nodes_in_order() {
            n.simulation_begin();
        }
    }

    /// NewDay after the census slice: propagation, then the 12 daily series.
    pub fn daily(&self) {
        self.do_demand_index_links();
        for (i, (key, _)) in TOTAL_NODES.iter().enumerate() {
            if let Some(n) = self.node(*key) {
                self.host.record_history(history::DAILY[i], n.active_demand() as i32);
            }
        }
    }

    /// NewMonth: cap relief, export, then a finished census restarts.
    pub fn monthly(&self) {
        self.evaluate_connection_utilization();
        self.export_demand();
        let mut d = self.d.borrow_mut();
        if d.census_state == CENSUS_DONE {
            d.census_state = CENSUS_START;
        }
    }

    fn nodes_in_order(&self) -> Vec<Rc<DemandNode>> {
        self.d.borrow().nodes.values().cloned().collect()
    }

    // ---- propagation (§4.4) ------------------------------------------------------------------------------

    /// `DoDemandIndexLinks()` ([`propagate`]).
    pub fn do_demand_index_links(&self) {
        let (nodes, links) = {
            let d = self.d.borrow();
            (d.nodes.clone(), d.links.clone())
        };
        let stats = propagate(&nodes, &links, &mut |s, t| self.warn_once(s, t));
        if !stats.settled {
            self.host
                .warn(&format!("demand: propagation did not settle after {MAX_PASSES} passes"));
        }
    }

    // ---- census (§6) -------------------------------------------------------------------------------------

    /// `DoIdle(ms)` (§6.2).
    pub fn do_idle(&self, ms: u32) {
        let start = self.host.now_ms();
        loop {
            let state = self.d.borrow().census_state;
            match state {
                CENSUS_START => {
                    self.update_modifiers();
                    let mut d = self.d.borrow_mut();
                    for k in CENSUS_KEYS {
                        d.current.insert(k, 0.0);
                    }
                    d.cursor = (0, 0);
                    d.census_state = CENSUS_SCANNING;
                }
                CENSUS_SCANNING => {
                    let (cx, cz) = self.d.borrow().cells;
                    loop {
                        let (x, z) = self.d.borrow().cursor;
                        if cx <= 0 || cz <= 0 || z >= cz {
                            self.apply_census();
                            self.d.borrow_mut().census_state = CENSUS_HISTORY;
                            return;
                        }
                        self.perform_census(x, z);
                        {
                            let mut d = self.d.borrow_mut();
                            d.cursor = if x + 1 >= cx { (0, z + 1) } else { (x + 1, z) };
                        }
                        if self.d.borrow().cursor.1 >= cz {
                            continue; // finished: apply on the next loop turn
                        }
                        if self.host.now_ms().wrapping_sub(start) >= ms {
                            return;
                        }
                    }
                }
                CENSUS_HISTORY => {
                    self.record_census_history();
                    self.d.borrow_mut().census_state = CENSUS_DONE;
                    return;
                }
                _ => return,
            }
        }
    }

    /// `PerformCensus(x, z)` (§6.3).
    pub fn perform_census(&self, x: i32, z: i32) {
        let h = &self.host;
        let Some(lot) = h.census_lot(x, z) else { return };
        if lot.building_type == 0 {
            return;
        }
        let purpose = h.building_profile(lot.building_type).map_or(0, |p| p.purpose);
        if !matches!(purpose, 1 | 2 | 3 | 5 | 6 | 7 | 8) {
            return; // purpose 0, 4 (Tourism) and > 8: ignored
        }
        let wealth = lot.wealth;
        let dev = h.developer_type(purpose, wealth);
        let occ = clamp01(f32::from(h.desirability(dev, x, z)) / 100.0);
        let cap = f32::from(h.lot_capacity(x, z, dev));
        let mut adds: Vec<(u32, f32)> = Vec::with_capacity(2);
        if purpose == 1 {
            let e = (h.cell_eq(x, z) as i64) as u8;
            let level = ((f32::from(e) / 50.1 + 1.0) as i32).clamp(1, 4) as u32;
            let wf = h.cell_workforce_percent(x, z);
            let w = wealth.wrapping_mul(0x10);
            adds.push((0x2100u32.wrapping_add(w).wrapping_add(level), wf * occ * cap));
            adds.push((0x1001u32.wrapping_add(w), occ * cap));
        } else {
            adds.push((dev.wrapping_add(1), occ * cap));
        }
        let mut d = self.d.borrow_mut();
        for (k, v) in adds {
            *d.current.entry(k).or_insert(0.0) += v;
        }
    }

    /// Applies the census (§6.4). [choice] The current map's keys are
    /// visited (it always holds the 24 census keys); a missing previous value
    /// counts as 0.
    pub fn apply_census(&self) {
        let current: Vec<(u32, f32)> = self.d.borrow().current.iter().map(|(k, v)| (*k, *v)).collect();
        for (k, cur) in current {
            let prev = self.d.borrow().previous.get(&k).copied().unwrap_or(0.0);
            let delta = cur - prev;
            if delta == 0.0 {
                continue;
            }
            let Some(node) = self.node(k) else { continue };
            match node.region_use() {
                1 => {
                    node.add_to_supply(delta);
                }
                2 => {
                    node.add_to_demand(delta);
                }
                _ => {}
            }
            self.d.borrow_mut().previous.insert(k, cur);
        }
    }

    /// Census state 2: the history of §10.4.
    pub fn record_census_history(&self) {
        let h = &self.host;
        let supply = |k| self.node(k).map(|n| n.supply());
        if let Some(s) = supply(0x1001) {
            h.record_history(history::RESIDENTIAL_TOTAL, s as i32);
        }
        for (i, k) in [0x1011u32, 0x1021, 0x1031].into_iter().enumerate() {
            if let Some(s) = supply(k) {
                h.record_history(history::RESIDENTS[i], s as i32);
            }
        }
        let mut c_jobs = 0u32;
        for (i, k) in [0x3111u32, 0x3121, 0x3131, 0x3321, 0x3331].into_iter().enumerate() {
            let j = self.jobs_by_census(k);
            c_jobs = c_jobs.wrapping_add(j);
            h.record_history(history::C_JOBS[i], j as i32);
        }
        h.record_history(history::C_JOBS_TOTAL, c_jobs as i32);
        let mut i_jobs = 0u32;
        for (i, k) in [0x4101u32, 0x4201, 0x4301, 0x4401].into_iter().enumerate() {
            let j = self.jobs_by_census(k);
            i_jobs = i_jobs.wrapping_add(j);
            h.record_history(history::I_JOBS[i], j as i32);
        }
        h.record_history(history::I_JOBS_TOTAL, i_jobs as i32);
        if let Some(jobs) = self.node(0x2000).map(|n| n.demand()) {
            h.record_history(history::JOBS_DEMAND, jobs as i32);
            let unfilled = jobs - c_jobs as f32 - i_jobs as f32;
            h.record_history(history::UNFILLED_JOBS, unfilled.max(0.0) as i32);
        }
        let salary = self.d.borrow().tuning.salary;
        let mut total: i64 = 0;
        let mut weighted: i64 = 0;
        let keys = [
            0x2111u32, 0x2112, 0x2113, 0x2114, 0x2121, 0x2122, 0x2123, 0x2124, 0x2131, 0x2132, 0x2133, 0x2134,
        ];
        for (i, k) in keys.into_iter().enumerate() {
            let n = supply(k).map_or(0, |s| s as i64);
            total = total.wrapping_add(n);
            weighted = weighted.wrapping_add(n.wrapping_mul(i64::from(salary[i])));
        }
        let avg = if total == 0 { 0 } else { weighted.wrapping_div(total) };
        h.record_history(history::AVERAGE_SALARY, avg as i32);
    }

    // ---- occupancy (§7) ----------------------------------------------------------------------------------

    /// `kMessageStateChanged` (§7): `lot` is the message's IGZUnknown,
    /// `data1`/`data2` the old/new HabitationState, `data3` the building type
    /// (0 = unknown), `data4` the occupant wealth.
    pub fn on_state_changed(&self, lot: u64, old_state: u32, new_state: u32, building_type: u32, wealth: u32) {
        if !self.is_initialized() {
            return;
        }
        let occupied = |s| s == 2 || s == 3;
        let (old_occ, new_occ) = (occupied(old_state), occupied(new_state));
        if old_occ == new_occ {
            return;
        }
        let bt = if building_type == 0 {
            self.host.lot_building_type(lot)
        } else {
            building_type
        };
        if bt == 0 || self.host.lot_excluded(lot) {
            return;
        }
        self.add_agent_effect(bt, new_occ, 1.0);
        self.update_capacity(bt, wealth, new_occ);
    }

    /// `AddAgentEffect(bt, add, pct)` (§7.1).
    pub fn add_agent_effect(&self, building_type: u32, add: bool, pct: f32) {
        let effects = self.d.borrow().effects.get(&building_type).cloned().unwrap_or_default();
        let sign = if add { 1.0f32 } else { -1.0 };
        for e in effects {
            let Some(n) = self.node(e.key) else { continue };
            if e.satisfies != 0.0 {
                n.add_to_supply(sign * e.satisfies * pct);
            }
            if e.creates != 0.0 {
                n.add_to_demand(sign * e.creates * pct);
            }
        }
    }

    /// `UpdateCapacity(bt, wealth, occupied)` (§7.2).
    pub fn update_capacity(&self, building_type: u32, wealth: u32, occupied: bool) {
        let Some(profile) = self.host.building_profile(building_type) else {
            return;
        };
        if profile.purpose == 0 {
            return;
        }
        let dev = self.host.developer_type(profile.purpose, wealth);
        let mut c: u32 = 0;
        for map in &profile.capacities {
            for &(t, cap) in map {
                if t == dev {
                    c = c.wrapping_add(u32::from(cap));
                }
            }
        }
        if let Some(n) = self.node(dev) {
            let c = c as f32;
            n.add_to_supply(if occupied { c } else { -c });
        }
    }

    // ---- monthly modifiers and cap relief (§8) ----------------------------------------------------------

    /// The tax and economy update of census state 0 (§8.1).
    pub fn update_modifiers(&self) {
        let basis = TaxBasis::new(self.neutral_tax_rate());
        let (tuning, co_bonus, i_bonus) = {
            let d = self.d.borrow();
            (d.tuning.clone(), d.co_economy_bonus, d.i_economy_bonus)
        };
        let today = self.host.today();
        for (i, (key, _)) in TOTAL_NODES.iter().enumerate() {
            let Some(node) = self.node(*key) else { continue };
            let m = total_node_modifiers(i, basis, &tuning, co_bonus, i_bonus, today, &*self.host);
            node.set_tax_modifier(m.tax_modifier);
            node.set_economy_modifier(m.economy_modifier);
        }
    }

    /// `EvaluateConnectionUtilization()` (§8.3).
    pub fn evaluate_connection_utilization(&self) {
        let h = &self.host;
        let t = self.d.borrow().tuning.clone();
        let (old_c, old_i) = {
            let d = self.d.borrow();
            (d.commercial_relief, d.industrial_relief)
        };
        let conns = h.connections();
        let mut industrial = 0.0f32;
        for c in &conns {
            if matches!(
                c.conn_type,
                connection::RAIL
                    | connection::ROAD
                    | connection::HIGHWAY
                    | connection::AVENUE
                    | connection::DIRT_ROAD
                    | connection::GROUND_HIGHWAY
            ) {
                industrial += h.traffic_arrived(c.token);
            }
        }
        industrial *= t.relief_per_trip;
        let cities =
            |ty: u32| -> BTreeSet<u64> { conns.iter().filter(|c| c.conn_type == ty).map(|c| c.city).collect() };
        let count = |ty: u32| conns.iter().filter(|c| c.conn_type == ty).count();
        let n_of = |cities: usize, conns: usize| ((cities + conns).max(1)) as f32;
        let a: BTreeSet<u64> = cities(connection::HIGHWAY)
            .union(&cities(connection::GROUND_HIGHWAY))
            .copied()
            .collect();
        let n_a = n_of(a.len(), count(connection::HIGHWAY));
        let b: BTreeSet<u64> = cities(connection::AVENUE).difference(&a).copied().collect();
        let n_b = n_of(b.len(), count(connection::AVENUE));
        let n_c = n_of(cities(connection::ROAD).len(), count(connection::ROAD));
        let n_d = n_of(cities(connection::DIRT_ROAD).len(), count(connection::DIRT_ROAD));
        let n_e = n_of(cities(connection::RAIL).len(), count(connection::RAIL));
        let f: BTreeSet<u64> = cities(connection::LIGHT_RAIL)
            .union(&cities(connection::MONORAIL))
            .copied()
            .collect::<BTreeSet<u64>>()
            .union(&cities(connection::SUBWAY))
            .copied()
            .collect();
        let n_f = n_of(f.len(), count(connection::LIGHT_RAIL) + count(connection::SUBWAY));
        let r = &t.relief_by_network;
        let mut commercial = 0.0f32;
        commercial += r[2] * n_a.log2()
            + r[6] * n_b.log2()
            + r[0] * n_c.log2()
            + r[11] * n_d.log2()
            + r[1] * n_e.log2()
            + r[7] * n_f.log2();
        let air = h.airport_trips();
        let sea = h.seaport_trips();
        commercial += air * t.relief_air_c + sea * t.relief_sea_c;
        industrial += air * t.relief_air_i + sea * t.relief_sea_i;
        {
            let mut d = self.d.borrow_mut();
            d.commercial_relief = commercial;
            d.industrial_relief = industrial;
        }
        for k in [0x4800u32, 0x4900] {
            if let Some(n) = self.node(k) {
                n.add_to_supply(industrial - old_i);
            }
        }
        if let Some(n) = self.node(0x3800) {
            n.add_to_supply(commercial - old_c);
        }
    }

    // ---- region (§9) -------------------------------------------------------------------------------------

    /// `ResetRegionalDemands(initial)` (§9.1).
    pub fn reset_regional_demands(&self, initial: bool) {
        let h = &self.host;
        let new_list = h.all_connected_cities();
        let export = h.connected_cities();
        let me = h.own_regional_city();
        let prev = self.d.borrow().prev_cities.clone();
        let sum = |list: &[u64], key: u32| -> f32 {
            let mut s = 0.0f32;
            for &c in list {
                if c != me {
                    s += h.city_extrapolated_population(c, key) as f32;
                }
            }
            s
        };
        let nodes: Vec<(u32, Rc<DemandNode>)> = self.d.borrow().nodes.iter().map(|(k, n)| (*k, n.clone())).collect();
        for (key, n) in nodes {
            let ru = n.region_use();
            if ru != 1 && ru != 2 {
                continue;
            }
            let Some(r) = self.node(key.wrapping_add(INDEX_REGIONAL)) else {
                continue;
            };
            let new_sum = sum(&new_list, key);
            if ru == 1 {
                let delta = if initial {
                    new_sum - (r.supply() + r.new_supply())
                } else {
                    new_sum - sum(&prev, key)
                };
                r.add_to_supply(delta);
            } else {
                let delta = if initial {
                    new_sum - (r.demand() + r.new_demand())
                } else {
                    new_sum - sum(&prev, key)
                };
                r.add_to_demand(delta);
            }
        }
        let mut d = self.d.borrow_mut();
        d.prev_cities = new_list;
        d.export_cities = export;
    }

    /// `ExportDemand()` (§9.2). [choice] The sort is stable.
    pub fn export_demand(&self) {
        let h = &self.host;
        let today = h.today();
        let export = self.d.borrow().export_cities.clone();
        let mut totals = [0i32; 3];
        for g in h.growth_developers() {
            let dev = g.sub_type;
            let last = h.day_of_last_successful_build(dev);
            if !(last < today && today - last > 65) {
                continue;
            }
            let Some((supply, demand)) = g.source else { continue };
            let mut unmet = demand - supply;
            if unmet.is_nan() || unmet < 10.0 {
                continue;
            }
            let mut order: Vec<(f32, u64)> = export.iter().map(|&c| (3.0 * h.city_tax_rate(c, dev), c)).collect();
            order.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            for (_, c) in order {
                let mut amt = h.city_allowable_extrapolation(c, dev).min(1000);
                if amt as f32 > unmet {
                    amt = unmet as i32;
                }
                if amt >= 10 {
                    h.city_extrapolate_growth(c, dev, amt as f32);
                    if let Some(n) = self.node(dev.wrapping_add(INDEX_REGIONAL)) {
                        n.add_to_supply(amt as f32);
                    }
                    unmet -= amt as f32;
                    if (1..=3).contains(&g.kind) {
                        let slot = &mut totals[(g.kind - 1) as usize];
                        *slot = slot.wrapping_add(amt);
                    }
                    self.extrapolate_residential_census(c, dev, amt);
                    self.extrapolate_business_census(c, dev, amt);
                }
                if unmet < 10.0 {
                    break;
                }
            }
        }
        h.record_history(history::EXPORT_R, totals[0]);
        h.record_history(history::EXPORT_C, totals[1]);
        h.record_history(history::EXPORT_I, totals[2]);
    }

    fn ratio(&self, city: u64, dev: u32) -> f32 {
        let p = self.host.city_population(city, dev);
        if p >= 1 {
            self.host.city_population(city, dev.wrapping_add(1)) as f32 / p as f32
        } else {
            0.8
        }
    }

    fn extrapolate_residential_census(&self, city: u64, dev: u32, amt: i32) {
        if !RESIDENTIAL_DEV_TYPES.contains(&dev) {
            return;
        }
        let h = &self.host;
        let workers = self.ratio(city, dev) * h.city_workforce_percentage(city) * amt as f32;
        let mut e = [0i32; 4];
        for (k, slot) in e.iter_mut().enumerate() {
            *slot = h.city_population(city, dev + 0x1100 + k as u32 + 1);
        }
        let mut total: i32 = e.iter().fold(0i32, |a, &b| a.wrapping_add(b));
        if total == 0 {
            e[0] = 1;
            total = 1;
        }
        let share = |k: usize| workers * e[k] as f32 / total as f32;
        for k in 0..4 {
            if let Some(n) = self.node(dev + 0x11100 + k as u32 + 1) {
                n.add_to_supply(share(k));
            }
        }
        for k in 0..4 {
            h.city_extrapolate_growth(city, dev + 0x1100 + k as u32 + 1, share(k));
        }
    }

    fn extrapolate_business_census(&self, city: u64, dev: u32, amt: i32) {
        if !BUSINESS_DEV_TYPES.contains(&dev) {
            return;
        }
        let h = &self.host;
        let n = self.ratio(city, dev) * amt as f32;
        if let Some(node) = self.node(dev + 0x10001) {
            node.add_to_supply(n);
        }
        h.city_extrapolate_growth(city, dev + 1, n);
        let j = self.jobs_per_unit(dev + 1, 0);
        for (w, jw) in j.iter().enumerate() {
            let v = n * jw;
            if let Some(node) = self.node(0x12010 + 0x10 * w as u32) {
                node.add_to_demand(v);
            }
            h.city_extrapolate_growth(city, 0x2010 + 0x10 * w as u32, v);
        }
    }
}

impl Drop for DemandSim {
    fn drop(&mut self) {
        let d = self.d.get_mut();
        for (_, n) in std::mem::take(&mut d.nodes) {
            self.host.release(n.token());
        }
    }
}

// ---- persistence (§12.1) ------------------------------------------------------------------------------

impl SaveObject for DemandSim {
    fn class_id(&self) -> u32 {
        CLSID_DEMAND_SIMULATOR
    }

    fn write(&self, out: &mut dyn OutStream) -> bool {
        if out.error() != 0 {
            return false;
        }
        let d = self.d.borrow();
        out.put_u16(SIM_RECORD_VERSION);
        out.put_f32(d.co_economy_bonus);
        out.put_f32(d.i_economy_bonus);
        out.put_f32(d.commercial_relief);
        out.put_f32(d.industrial_relief);
        out.put_u32(d.nodes.len() as u32);
        for (k, n) in &d.nodes {
            out.put_u32(*k);
            out.put_ref(&RefValue::Object(n.clone() as Rc<dyn SaveObject>));
        }
        out.put_u32(d.previous.len() as u32);
        for (k, v) in &d.previous {
            out.put_u32(*k);
            out.put_f32(*v);
        }
        out.error() == 0
    }

    fn read(&self, inp: &mut dyn InStream) -> bool {
        if inp.error() != 0 {
            return false;
        }
        let Some(version) = inp.get_u16() else { return false };
        if version >= 3 {
            return false;
        }
        self.d.borrow_mut().loaded_from_save = true;
        let mut nodes: Vec<(u32, Rc<DemandNode>)> = Vec::new();
        let parsed = (|| -> Option<([f32; 4], BTreeMap<u32, f32>)> {
            if version < 2 {
                for _ in 0..4 {
                    inp.get_u32()?;
                }
            }
            let vals = [inp.get_f32()?, inp.get_f32()?, inp.get_f32()?, inp.get_f32()?];
            for _ in 0..inp.get_u32()? {
                let key = inp.get_u32()?;
                match inp.get_ref()? {
                    r @ RefValue::Object(_) => {
                        if let Some(n) = r.object::<DemandNode>() {
                            nodes.push((key, n));
                        }
                    }
                    RefValue::Host { token, .. } => self.host.release(token),
                    RefValue::Null => {}
                }
            }
            let mut census = BTreeMap::new();
            for _ in 0..inp.get_u32()? {
                let k = inp.get_u32()?;
                census.insert(k, inp.get_f32()?);
            }
            Some((vals, census))
        })();
        let Some((vals, census)) = parsed else { return false };
        if inp.error() != 0 {
            return false;
        }
        let old = {
            let mut d = self.d.borrow_mut();
            d.co_economy_bonus = vals[0];
            d.i_economy_bonus = vals[1];
            d.commercial_relief = vals[2];
            d.industrial_relief = vals[3];
            d.previous = census;
            std::mem::take(&mut d.nodes)
        };
        for (_, n) in old {
            self.host.release(n.token());
        }
        for (key, n) in nodes {
            self.host.add_ref(n.token()); // the map holds a reference
            if let Some(prev) = self.d.borrow_mut().nodes.insert(key, n) {
                self.host.release(prev.token());
            }
        }
        true
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
    use crate::demand::{exemplar_prop, BuildingProfile, CensusLot, Connection, DemandPoster, GrowthDeveloper};
    use crate::props::{exemplar_key, PROP_EXEMPLAR_TYPE};
    use crate::stream::{ByteReader, ByteWriter, RefResolver, ResKey, SaveRegistry};
    use crate::variant::{mem, Elem, Variant};
    use std::rc::Weak;

    type Cell5 = (CensusLot, i16, u16, f32, f32);

    /// A minimal city: demand exemplars, a census grid, connections.
    #[derive(Default)]
    struct Fake {
        me: RefCell<Weak<Fake>>,
        exemplars: RefCell<Vec<(ResKey, Rc<PropertyHolder>)>>,
        posts: RefCell<Vec<(u32, i32)>>,
        history: RefCell<Vec<(u32, i32)>>,
        warnings: Cell<u32>,
        cells: Cell<(i32, i32)>,
        census: RefCell<BTreeMap<(i32, i32), Cell5>>,
        connections: RefCell<Vec<(Connection, f32)>>,
        next: Cell<u64>,
    }

    impl Fake {
        fn new() -> Rc<Fake> {
            let f = Rc::new(Fake::default());
            *f.me.borrow_mut() = Rc::downgrade(&f);
            f
        }
        fn make_node(&self) -> Rc<DemandNode> {
            self.next.set(self.next.get() + 1);
            DemandNode::new(
                self.me.borrow().upgrade().unwrap() as Rc<dyn DemandPoster>,
                self.next.get(),
            )
        }
        fn add_exemplar(&self, instance: u32, props: &[(u32, Variant)]) {
            let mut h = PropertyHolder::new();
            h.add(PROP_EXEMPLAR_TYPE, Variant::Scalar(Elem::U32(3)));
            for (id, v) in props {
                h.add(*id, v.clone());
            }
            self.exemplars
                .borrow_mut()
                .push((exemplar_key(DEMAND_EXEMPLAR_GROUP, instance), Rc::new(h)));
        }
    }

    fn pairs(p: &[(u32, u32)]) -> Variant {
        Variant::array(
            mem::UINT32,
            p.iter().flat_map(|&(a, b)| [Elem::U32(a), Elem::U32(b)]).collect(),
        )
    }

    fn list(p: &[u32]) -> Variant {
        Variant::array(mem::UINT32, p.iter().map(|&a| Elem::U32(a)).collect())
    }

    impl DemandPoster for Fake {
        fn active_demand_changed(&self, _node: u64, key: u32, data2: i32) {
            self.posts.borrow_mut().push((key, data2));
        }
    }

    impl DemandHost for Fake {
        fn exemplar_keys(&self) -> Vec<ResKey> {
            self.exemplars.borrow().iter().map(|e| e.0).collect()
        }
        fn exemplar(&self, key: ResKey) -> Option<Rc<PropertyHolder>> {
            self.exemplars
                .borrow()
                .iter()
                .rev()
                .find(|e| e.0 == key)
                .map(|e| e.1.clone())
        }
        fn warn(&self, _text: &str) {
            self.warnings.set(self.warnings.get() + 1);
        }
        fn create_node(&self) -> Option<Rc<DemandNode>> {
            Some(self.make_node())
        }
        fn add_ref(&self, _token: u64) {}
        fn release(&self, _token: u64) {}
        fn cell_counts(&self) -> (i32, i32) {
            self.cells.get()
        }
        fn today(&self) -> u32 {
            2_451_545
        }
        fn now_ms(&self) -> u32 {
            0
        }
        fn census_lot(&self, x: i32, z: i32) -> Option<CensusLot> {
            self.census.borrow().get(&(x, z)).map(|c| c.0)
        }
        fn building_profile(&self, building_type: u32) -> Option<BuildingProfile> {
            (building_type == 1).then(|| BuildingProfile {
                purpose: 1,
                capacities: vec![vec![(0x1010, 30)]],
            })
        }
        fn developer_type(&self, purpose: u32, wealth: u32) -> u32 {
            if purpose == 1 {
                0x1000u32.wrapping_add(wealth.wrapping_mul(0x10))
            } else {
                0
            }
        }
        fn desirability(&self, _dev_type: u32, x: i32, z: i32) -> i16 {
            self.census.borrow().get(&(x, z)).map_or(0, |c| c.1)
        }
        fn lot_capacity(&self, x: i32, z: i32, _dev_type: u32) -> u16 {
            self.census.borrow().get(&(x, z)).map_or(0, |c| c.2)
        }
        fn cell_eq(&self, x: i32, z: i32) -> f32 {
            self.census.borrow().get(&(x, z)).map_or(0.0, |c| c.3)
        }
        fn cell_workforce_percent(&self, x: i32, z: i32) -> f32 {
            self.census.borrow().get(&(x, z)).map_or(0.0, |c| c.4)
        }
        fn lot_building_type(&self, _lot: u64) -> u32 {
            1
        }
        fn lot_excluded(&self, lot: u64) -> bool {
            lot == 99
        }
        fn all_building_types(&self) -> Vec<u32> {
            vec![]
        }
        fn building_exemplar(&self, _building_type: u32) -> Option<Rc<PropertyHolder>> {
            None
        }
        fn history_max(&self, _series: u32, _from: u32, _to: u32) -> Option<f32> {
            None
        }
        fn ordinance_factor(&self, _prop: u32, _index: u32) -> f32 {
            1.0
        }
        fn nation_economy_modifier(&self, _demand_id: u32) -> f32 {
            0.0
        }
        fn connections(&self) -> Vec<Connection> {
            self.connections.borrow().iter().map(|c| c.0).collect()
        }
        fn traffic_arrived(&self, connection: u64) -> f32 {
            self.connections
                .borrow()
                .iter()
                .find(|c| c.0.token == connection)
                .map_or(0.0, |c| c.1)
        }
        fn airport_trips(&self) -> f32 {
            0.0
        }
        fn seaport_trips(&self) -> f32 {
            0.0
        }
        fn all_connected_cities(&self) -> Vec<u64> {
            vec![]
        }
        fn connected_cities(&self) -> Vec<u64> {
            vec![]
        }
        fn own_regional_city(&self) -> u64 {
            0
        }
        fn city_population(&self, _c: u64, _t: u32) -> i32 {
            0
        }
        fn city_extrapolated_population(&self, _c: u64, _t: u32) -> i32 {
            0
        }
        fn city_allowable_extrapolation(&self, _c: u64, _t: u32) -> i32 {
            0
        }
        fn city_tax_rate(&self, _c: u64, _t: u32) -> f32 {
            0.0
        }
        fn city_workforce_percentage(&self, _c: u64) -> f32 {
            0.0
        }
        fn city_extrapolate_growth(&self, _c: u64, _t: u32, _a: f32) {}
        fn growth_developers(&self) -> Vec<GrowthDeveloper> {
            vec![]
        }
        fn day_of_last_successful_build(&self, _dev: u32) -> u32 {
            0
        }
        fn record_history(&self, series: u32, value: i32) {
            self.history.borrow_mut().push((series, value));
        }
        fn set_history_queue_length(&self, _series: u32, _length: u32) {}
    }

    fn sim(f: &Rc<Fake>) -> Rc<DemandSim> {
        let s = DemandSim::new(f.clone() as Rc<dyn DemandHost>, 1);
        assert!(s.init());
        s
    }

    #[test]
    fn links_transfer_drop_and_sum() {
        let f = Fake::new();
        f.add_exemplar(
            0x100,
            &[
                (LinkKind::Drives as u32, pairs(&[(0x200, 1)])),
                (LinkKind::Satisfies as u32, pairs(&[(0x300, 2)])),
                (LinkKind::Caps as u32, list(&[0x500])),
            ],
        );
        f.add_exemplar(0x200, &[]);
        f.add_exemplar(0x300, &[]);
        f.add_exemplar(0x400, &[(LinkKind::Sums as u32, list(&[0x300, 0x999]))]);
        f.add_exemplar(0x500, &[]);
        let s = sim(&f);
        assert_eq!(f.warnings.get(), 1); // 0x999 has no node
        assert_eq!(f.posts.borrow().len(), 5); // one pass, one post per node
        s.node(0x100).unwrap().add_to_supply(1.0);
        s.node(0x100).unwrap().add_to_demand(4.0);
        s.do_demand_index_links();
        assert_eq!(s.node(0x200).unwrap().demand(), 0.0); // 0.01 is dropped
        assert!((s.node(0x300).unwrap().supply() - 0.02).abs() < 1e-6);
        assert!((s.node(0x400).unwrap().supply() - 0.02).abs() < 1e-6);
        assert_eq!(s.node(0x500).unwrap().demand_cap(), 1.0); // demand 4 >= supply 1
        assert_eq!(f.warnings.get(), 1);
    }

    #[test]
    fn census_applies_deltas_by_region_use() {
        let f = Fake::new();
        f.cells.set((2, 1));
        let lot = CensusLot {
            building_type: 1,
            wealth: 1,
        };
        f.census.borrow_mut().insert((0, 0), (lot, 100, 10, 120.0, 0.5));
        f.census.borrow_mut().insert((1, 0), (lot, 50, 10, 0.0, 1.0));
        f.add_exemplar(0x1011, &[(exemplar_prop::REGION_USE, Variant::Scalar(Elem::U8(1)))]);
        f.add_exemplar(0x2113, &[(exemplar_prop::REGION_USE, Variant::Scalar(Elem::U8(2)))]);
        let s = sim(&f);
        assert_eq!(s.snapshot().census_state, CENSUS_DONE);
        s.handle_message(msg::SIM_NEW_MONTH, 0, 0, 0);
        s.do_idle(1000);
        let snap = s.snapshot();
        assert_eq!(snap.census_state, CENSUS_HISTORY);
        assert_eq!(snap.current_census[&0x1011], 15.0);
        assert_eq!(snap.current_census[&0x2113], 5.0); // EQ 120 -> level 3
        assert_eq!(snap.current_census[&0x2111], 5.0);
        assert_eq!(snap.previous_census[&0x2111], 0.0); // no node
        assert_eq!(s.node(0x1011).unwrap().new_supply(), 15.0);
        assert_eq!(s.node(0x2113).unwrap().new_demand(), 5.0);
        s.do_idle(0);
        assert_eq!(s.snapshot().census_state, CENSUS_DONE);
        assert!(f.history.borrow().contains(&(history::RESIDENTS[0], 0)));
    }

    #[test]
    fn occupancy_capacity_and_exclusion() {
        let f = Fake::new();
        f.add_exemplar(0x1010, &[]);
        let s = sim(&f);
        s.on_state_changed(5, 1, 3, 0, 1);
        assert_eq!(s.node(0x1010).unwrap().new_supply(), 30.0);
        s.on_state_changed(5, 3, 2, 0, 1);
        assert_eq!(s.node(0x1010).unwrap().new_supply(), 30.0);
        s.on_state_changed(99, 1, 3, 0, 1);
        assert_eq!(s.node(0x1010).unwrap().new_supply(), 30.0);
        s.on_state_changed(5, 3, 4, 0, 1);
        assert_eq!(s.node(0x1010).unwrap().new_supply(), 0.0);
    }

    #[test]
    fn relief_is_a_delta() {
        let f = Fake::new();
        f.add_exemplar(0x3800, &[]);
        f.add_exemplar(
            0x4900,
            &[(exemplar_prop::INITIAL_SUPPLY, Variant::Scalar(Elem::I32(30000)))],
        );
        let s = sim(&f);
        let mut t = s.tuning();
        t.relief_by_network[2] = 100_000.0;
        s.set_tuning(t);
        f.connections.borrow_mut().push((
            Connection {
                token: 1,
                conn_type: connection::HIGHWAY,
                city: 9,
            },
            2.0,
        ));
        s.evaluate_connection_utilization();
        s.do_demand_index_links();
        assert_eq!(s.node(0x3800).unwrap().supply(), 100_000.0);
        assert_eq!(s.node(0x4900).unwrap().supply(), 30_040.0);
        f.connections.borrow_mut().clear();
        s.evaluate_connection_utilization();
        s.do_demand_index_links();
        assert_eq!(s.node(0x3800).unwrap().supply(), 0.0);
        assert_eq!(s.node(0x4900).unwrap().supply(), 30_000.0);
    }

    struct Resolver(Vec<Rc<DemandNode>>);
    impl RefResolver for Resolver {
        fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
            assert_eq!(class_id, crate::demand::CLSID_DEMAND_NODE);
            self.0
                .get((object_id as usize).wrapping_sub(1))
                .map(|n| RefValue::Object(n.clone() as Rc<dyn SaveObject>))
        }
    }

    #[test]
    fn save_round_trip() {
        let f = Fake::new();
        f.add_exemplar(
            0x1010,
            &[(exemplar_prop::INITIAL_DEMAND, Variant::Scalar(Elem::I32(20)))],
        );
        f.add_exemplar(0x2010, &[]);
        let s = sim(&f);
        s.set_economy_bonuses(0.5, 0.25);
        s.node(0x1010).unwrap().add_to_demand(5.0);
        s.do_demand_index_links();
        let write = |s: &DemandSim| {
            let mut w = ByteWriter::with_registry(Rc::new(RefCell::new(SaveRegistry::new())));
            assert!(s.write(&mut w));
            w.into_bytes()
        };
        let bytes = write(&s);
        assert_eq!(&bytes[..2], &[2, 0]);
        // The nodes as the store would load them (object IDs 1, 2 in key order).
        let nodes: Vec<Rc<DemandNode>> = s
            .node_keys()
            .iter()
            .map(|&k| {
                let n = f.make_node();
                let mut w = ByteWriter::new();
                s.node(k).unwrap().write(&mut w);
                assert!(n.read(&mut ByteReader::new(&w.into_bytes())));
                n
            })
            .collect();
        let loaded = DemandSim::new(f.clone() as Rc<dyn DemandHost>, 2);
        let mut res = Resolver(nodes);
        assert!(loaded.read(&mut ByteReader::with_resolver(&bytes, &mut res)));
        assert_eq!(write(&loaded), bytes);
        assert!(loaded.init());
        assert_eq!(loaded.snapshot().census_state, CENSUS_START);
        assert_eq!(loaded.node(0x1010).unwrap().demand(), 25.0);
        // Version 3 is rejected without marking the simulator loaded.
        let mut v3 = bytes.clone();
        v3[0] = 3;
        let other = DemandSim::new(f.clone() as Rc<dyn DemandHost>, 3);
        assert!(!other.read(&mut ByteReader::new(&v3)));
        assert!(!other.snapshot().loaded_from_save);
    }

    #[test]
    fn hostile_inputs_do_not_panic() {
        let f = Fake::new();
        f.cells.set((-5, i32::MAX));
        f.add_exemplar(u32::MAX, &[(LinkKind::Drives as u32, pairs(&[(0, u32::MAX)]))]);
        f.add_exemplar(0, &[(LinkKind::Contributes as u32, pairs(&[(u32::MAX, 1500)]))]);
        let s = sim(&f);
        s.node(0).unwrap().add_to_supply(f32::NAN);
        s.node(u32::MAX).unwrap().add_to_supply(f32::INFINITY);
        s.do_demand_index_links();
        s.handle_message(msg::SIM_NEW_MONTH, 0, 0, 0);
        s.do_idle(u32::MAX);
        s.do_idle(0);
        s.update_capacity(1, u32::MAX, true);
        s.jobs_by_census(u32::MAX);
        s.export_demand();
        for junk in [
            &[][..],
            &[2, 0, 1][..],
            &[1, 0, 0xFF, 0xFF][..],
            &[
                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF,
            ][..],
        ] {
            let t = DemandSim::new(f.clone() as Rc<dyn DemandHost>, 9);
            let _ = t.read(&mut ByteReader::new(junk));
        }
    }
}

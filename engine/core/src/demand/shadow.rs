// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The demand shadow (diagnostics; no spec behaviour of its own): FreeCity4's
//! demand propagation (§4.4) and monthly modifiers (§8.1) run over node
//! states copied from another demand simulator (the original game's), so the
//! two can be compared on identical inputs in the same process.
//!
//! Everything here is the code [`DemandSim`](super::DemandSim) runs: the
//! links come from the same exemplar decoding ([`super::simulator`]'s
//! `demand_exemplars` / `exemplar_links`), propagation is the same
//! `propagate` over [`DemandNode`]s seeded with the copied states, and the
//! modifiers are the same `total_node_modifiers`. Nothing is posted (the
//! nodes have a poster that drops ActiveDemandChanged), and the host is only
//! asked for exemplars (building the graph) and for the §8.1 inputs
//! (`today`, `history_max`, `ordinance_factor`, `nation_economy_modifier`).

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use super::simulator::{
    demand_exemplars, exemplar_links, neutral_tax_rate_of, propagate, total_node_modifiers, ModifierTerms,
    PropagationStats,
};
use super::tuning::{DemandTuning, TUNING_KEY};
use super::{DemandHost, DemandNode, DemandPoster, Link, LinkKind, NodeData};

/// Drops ActiveDemandChanged: the shadow's nodes post nothing.
struct NoPoster;

impl DemandPoster for NoPoster {
    fn active_demand_changed(&self, _node: u64, _key: u32, _data2: i32) {}
}

/// The §8.1 result for the 12 total nodes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShadowModifiers {
    /// `GetNeutralTaxRate()` from the given census supplies (unclamped).
    pub neutral_raw: f32,
    /// `max(0.1, neutral_raw)`.
    pub neutral: f32,
    /// The `today` the host gave.
    pub today: u32,
    pub nodes: [ModifierTerms; 12],
}

/// The demand graph (§4.2) and tuning (§11.1) as FreeCity4 builds them.
pub struct ShadowGraph {
    links: BTreeMap<u32, Vec<Link>>,
    /// Exemplar node keys and link targets, ascending.
    keys: Vec<u32>,
    /// Keys that receive transfers (Drives / Satisfies / Contributes targets)
    /// or are corrected by their own Sums links.
    fed: BTreeSet<u32>,
    exemplars: u32,
    tuning: DemandTuning,
    tuning_found: bool,
}

impl ShadowGraph {
    /// Reads the demand exemplars and the tuning exemplar through `host`
    /// (`exemplar_keys` and `exemplar` only).
    pub fn from_host(host: &dyn DemandHost) -> ShadowGraph {
        let mut links: BTreeMap<u32, Vec<Link>> = BTreeMap::new();
        let mut keys = BTreeSet::new();
        let mut exemplars = 0u32;
        for e in demand_exemplars(host) {
            exemplars += 1;
            keys.insert(e.key);
            links.entry(e.key).or_default().extend(exemplar_links(&e));
        }
        let mut fed = BTreeSet::new();
        for l in links.values().flatten() {
            keys.insert(l.target);
            match l.kind {
                LinkKind::Drives | LinkKind::Satisfies | LinkKind::Contributes => {
                    fed.insert(l.target);
                }
                LinkKind::Sums => {
                    fed.insert(l.source);
                }
                LinkKind::Caps => {}
            }
        }
        let tuning_holder = host.exemplar(TUNING_KEY);
        ShadowGraph {
            links,
            keys: keys.into_iter().collect(),
            fed,
            exemplars,
            tuning: DemandTuning::from_exemplar(tuning_holder.as_deref()),
            tuning_found: tuning_holder.is_some(),
        }
    }

    /// Demand exemplars read.
    pub fn exemplar_count(&self) -> u32 {
        self.exemplars
    }

    /// Exemplar node keys and link targets, ascending.
    pub fn keys(&self) -> &[u32] {
        &self.keys
    }

    pub fn link_count(&self) -> usize {
        self.links.values().map(Vec::len).sum()
    }

    pub fn tuning(&self) -> &DemandTuning {
        &self.tuning
    }

    /// Whether the tuning exemplar was found (else: the code defaults).
    pub fn tuning_found(&self) -> bool {
        self.tuning_found
    }

    /// Whether propagation can change `key`'s totals other than by folding
    /// its own pending values: it is the target of a transfer link or
    /// corrects itself through Sums links.
    pub fn is_fed(&self, key: u32) -> bool {
        self.fed.contains(&key)
    }

    /// `DoDemandIndexLinks()` (§4.4) over `nodes` (each `id` is its key; keys
    /// must be distinct), which hold the result afterwards. Links whose
    /// target is not among `nodes` are ignored and counted.
    pub fn propagate(&self, nodes: &mut [NodeData]) -> PropagationStats {
        let poster: Rc<dyn DemandPoster> = Rc::new(NoPoster);
        let map: BTreeMap<u32, Rc<DemandNode>> = nodes
            .iter()
            .map(|d| (d.id, DemandNode::with_data(poster.clone(), 0, *d)))
            .collect();
        let stats = propagate(&map, &self.links, &mut |_, _| {});
        for d in nodes.iter_mut() {
            if let Some(n) = map.get(&d.id) {
                *d = n.snapshot();
            }
        }
        stats
    }

    /// `GetNeutralTaxRate()` (§5) from the supplies of `0x1001`, `0x3001` and
    /// `0x4001` (pass 0 for a missing node).
    pub fn neutral_tax_rate(&self, r: f32, c: f32, i: f32) -> f32 {
        neutral_tax_rate_of(&self.tuning, r, c, i)
    }

    /// §8.1 for the 12 total nodes, from the census supplies (as for
    /// [`Self::neutral_tax_rate`]) and the saved CO / I economy bonuses.
    pub fn modifiers(
        &self,
        host: &dyn DemandHost,
        r: f32,
        c: f32,
        i: f32,
        co_bonus: f32,
        i_bonus: f32,
    ) -> ShadowModifiers {
        let neutral_raw = self.neutral_tax_rate(r, c, i);
        let neutral = neutral_raw.max(0.1);
        let today = host.today();
        let mut out = ShadowModifiers {
            neutral_raw,
            neutral,
            today,
            nodes: [ModifierTerms::default(); 12],
        };
        for (idx, slot) in out.nodes.iter_mut().enumerate() {
            *slot = total_node_modifiers(idx, neutral, &self.tuning, co_bonus, i_bonus, today, host);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demand::{
        exemplar_prop, BuildingProfile, CensusLot, Connection, DemandSim, GrowthDeveloper, DEMAND_EXEMPLAR_GROUP,
    };
    use crate::property::PropertyHolder;
    use crate::props::{exemplar_key, PROP_EXEMPLAR_TYPE};
    use crate::stream::ResKey;
    use crate::variant::{mem, Elem, Variant};
    use std::cell::{Cell, RefCell};

    /// Exemplars, and §8.1 inputs; every other host call is counted.
    #[derive(Default)]
    struct Fake {
        exemplars: RefCell<Vec<(ResKey, Rc<PropertyHolder>)>>,
        history: Cell<Option<f32>>,
        other_calls: Cell<u32>,
    }

    impl Fake {
        fn add(&self, instance: u32, props: &[(u32, Variant)]) {
            let mut h = PropertyHolder::new();
            h.add(PROP_EXEMPLAR_TYPE, Variant::Scalar(Elem::U32(3)));
            for (id, v) in props {
                h.add(*id, v.clone());
            }
            self.exemplars
                .borrow_mut()
                .push((exemplar_key(DEMAND_EXEMPLAR_GROUP, instance), Rc::new(h)));
        }
        fn other(&self) {
            self.other_calls.set(self.other_calls.get() + 1);
        }
    }

    impl DemandPoster for Fake {
        fn active_demand_changed(&self, _node: u64, _key: u32, _data2: i32) {}
    }

    impl DemandHost for Fake {
        fn exemplar_keys(&self) -> Vec<ResKey> {
            self.exemplars.borrow().iter().map(|e| e.0).collect()
        }
        fn exemplar(&self, key: ResKey) -> Option<Rc<PropertyHolder>> {
            self.exemplars.borrow().iter().find(|e| e.0 == key).map(|e| e.1.clone())
        }
        fn create_node(&self) -> Option<Rc<DemandNode>> {
            Some(DemandNode::new(Rc::new(NoPoster), 0))
        }
        fn add_ref(&self, _token: u64) {}
        fn release(&self, _token: u64) {}
        fn cell_counts(&self) -> (i32, i32) {
            (0, 0)
        }
        fn today(&self) -> u32 {
            2_451_600
        }
        fn now_ms(&self) -> u32 {
            0
        }
        fn census_lot(&self, _x: i32, _z: i32) -> Option<CensusLot> {
            self.other();
            None
        }
        fn building_profile(&self, _bt: u32) -> Option<BuildingProfile> {
            self.other();
            None
        }
        fn developer_type(&self, _p: u32, _w: u32) -> u32 {
            self.other();
            0
        }
        fn desirability(&self, _d: u32, _x: i32, _z: i32) -> i16 {
            self.other();
            0
        }
        fn lot_capacity(&self, _x: i32, _z: i32, _d: u32) -> u16 {
            self.other();
            0
        }
        fn cell_eq(&self, _x: i32, _z: i32) -> f32 {
            self.other();
            0.0
        }
        fn cell_workforce_percent(&self, _x: i32, _z: i32) -> f32 {
            self.other();
            0.0
        }
        fn lot_building_type(&self, _lot: u64) -> u32 {
            self.other();
            0
        }
        fn lot_excluded(&self, _lot: u64) -> bool {
            self.other();
            false
        }
        fn all_building_types(&self) -> Vec<u32> {
            vec![]
        }
        fn building_exemplar(&self, _bt: u32) -> Option<Rc<PropertyHolder>> {
            None
        }
        fn history_max(&self, _s: u32, _from: u32, _to: u32) -> Option<f32> {
            self.history.get()
        }
        fn ordinance_factor(&self, prop: u32, _index: u32) -> f32 {
            if prop == 0x2A65_1010 {
                1.25
            } else {
                1.0
            }
        }
        fn nation_economy_modifier(&self, _id: u32) -> f32 {
            0.06
        }
        fn connections(&self) -> Vec<Connection> {
            self.other();
            vec![]
        }
        fn traffic_arrived(&self, _c: u64) -> f32 {
            self.other();
            0.0
        }
        fn airport_trips(&self) -> f32 {
            self.other();
            0.0
        }
        fn seaport_trips(&self) -> f32 {
            self.other();
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
            self.other();
            0
        }
        fn city_extrapolated_population(&self, _c: u64, _t: u32) -> i32 {
            0
        }
        fn city_allowable_extrapolation(&self, _c: u64, _t: u32) -> i32 {
            self.other();
            0
        }
        fn city_tax_rate(&self, _c: u64, _t: u32) -> f32 {
            self.other();
            0.0
        }
        fn city_workforce_percentage(&self, _c: u64) -> f32 {
            self.other();
            0.0
        }
        fn city_extrapolate_growth(&self, _c: u64, _t: u32, _a: f32) {
            self.other();
        }
        fn growth_developers(&self) -> Vec<GrowthDeveloper> {
            vec![]
        }
        fn day_of_last_successful_build(&self, _d: u32) -> u32 {
            0
        }
        fn record_history(&self, _s: u32, _v: i32) {
            self.other();
        }
        fn set_history_queue_length(&self, _s: u32, _l: u32) {}
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

    fn i32v(v: i32) -> Variant {
        Variant::Scalar(Elem::I32(v))
    }

    /// A small graph with every link type: census 0x1011 drives a cap
    /// resource and a total node, the resource caps it, a sum node sums two
    /// nodes, a contributes chain feeds a lower key.
    fn graph(f: &Fake) {
        f.add(
            0x1011,
            &[
                (LinkKind::Drives as u32, pairs(&[(0x1810, 100), (0x21010, 200)])),
                (exemplar_prop::REGION_USE, Variant::Scalar(Elem::U8(1))),
            ],
        );
        f.add(
            0x1810,
            &[
                (exemplar_prop::INITIAL_SUPPLY, i32v(50_000)),
                (LinkKind::Caps as u32, list(&[0x21010])),
            ],
        );
        f.add(
            0x21010,
            &[
                (exemplar_prop::MIN, i32v(-6000)),
                (exemplar_prop::MAX, i32v(6000)),
                (exemplar_prop::INITIAL_DEMAND, i32v(20)),
                (LinkKind::Contributes as u32, pairs(&[(0x1001, 37)])),
            ],
        );
        f.add(0x1001, &[(LinkKind::Sums as u32, list(&[0x1011, 0x1810]))]);
        f.add(0x3001, &[(LinkKind::Satisfies as u32, pairs(&[(0x1001, 150)]))]);
    }

    #[test]
    fn shadow_propagation_is_the_simulators_bit_for_bit() {
        let f = Rc::new(Fake::default());
        graph(&f);
        let sim = DemandSim::new(f.clone() as Rc<dyn DemandHost>, 1);
        assert!(sim.init());
        let g = ShadowGraph::from_host(&*f);
        assert_eq!(g.exemplar_count(), 5);
        assert_eq!(g.keys(), &[0x1001, 0x1011, 0x1810, 0x3001, 0x21010]);
        assert_eq!(g.link_count(), sim.links().len());
        assert!(g.is_fed(0x21010) && g.is_fed(0x1001) && g.is_fed(0x1810));
        assert!(!g.is_fed(0x1011) && !g.is_fed(0x3001));
        assert!(!g.tuning_found());

        // Several days of injected deltas and modifiers, with values that do
        // not add exactly in f32.
        for (day, inject) in [(0, 10.3f32), (1, 0.7), (2, -3.33), (3, 1234.567)] {
            let n = sim.node(0x1011).unwrap();
            n.add_to_supply(inject);
            sim.node(0x3001).unwrap().add_to_supply(inject * 0.37);
            let t = sim.node(0x21010).unwrap();
            t.set_economy_modifier(0.03 * day as f32);
            t.set_tax_modifier(-0.011 * day as f32);
            let mut states: Vec<NodeData> = sim
                .node_keys()
                .iter()
                .map(|k| sim.node(*k).unwrap().snapshot())
                .collect();
            let stats = g.propagate(&mut states);
            assert!(stats.settled && stats.passes >= 2 && stats.missing_targets == 0);
            assert_eq!(stats.end_of_cycles, stats.passes * 5);
            sim.do_demand_index_links();
            for s in &states {
                let real = sim.node(s.id).unwrap().snapshot();
                for (a, b) in [
                    (s.supply, real.supply),
                    (s.demand, real.demand),
                    (s.new_supply, real.new_supply),
                    (s.new_demand, real.new_demand),
                    (s.active_demand, real.active_demand),
                    (s.demand_cap, real.demand_cap),
                ] {
                    assert_eq!(a.to_bits(), b.to_bits(), "node 0x{:X} day {day}", s.id);
                }
            }
        }
        assert_eq!(f.other_calls.get(), 0);
    }

    #[test]
    fn missing_targets_are_counted_not_fatal() {
        let f = Fake::default();
        graph(&f);
        let g = ShadowGraph::from_host(&f);
        let mut only = vec![NodeData {
            id: 0x1011,
            new_supply: 5.0,
            ..NodeData::default()
        }];
        let st = g.propagate(&mut only);
        assert!(st.settled);
        assert_eq!(st.missing_targets, 2); // 0x1810 and 0x21010, first pass only
        assert_eq!(only[0].supply, 5.0);
    }

    #[test]
    fn modifiers_are_the_simulators() {
        let f = Rc::new(Fake::default());
        graph(&f);
        f.history.set(Some(90.0));
        let sim = DemandSim::new(f.clone() as Rc<dyn DemandHost>, 1);
        assert!(sim.init());
        let g = ShadowGraph::from_host(&*f);
        let s = |k| sim.node(k).map_or(0.0, |n| n.supply());
        let m = g.modifiers(&*f, s(0x1001), s(0x3001), s(0x4001), 0.0, 0.0);
        assert_eq!(m.neutral_raw, sim.neutral_tax_rate());
        assert_eq!(m.neutral, 0.1); // no curve: 0, clamped
        sim.update_modifiers();
        let t = sim.node(0x21010).unwrap();
        assert_eq!(m.nodes[0].tax_modifier.to_bits(), t.tax_modifier().to_bits());
        assert_eq!(m.nodes[0].economy_modifier.to_bits(), t.economy_modifier().to_bits());
        assert_eq!(m.nodes[0].tax_modifier, 0.25);
        assert!(m.nodes[0].from_history);
        assert_eq!(m.nodes[0].max_rate, 90.0);
        assert_eq!(m.nodes[3].tax_modifier, 0.0);
        assert_eq!(m.nodes[3].economy_modifier, 0.06);
        assert_eq!(m.today, 2_451_600);
        f.history.set(None);
        let m = g.modifiers(&*f, 0.0, 0.0, 0.0, 0.5, 0.25);
        assert!(!m.nodes[0].from_history);
        assert_eq!(m.nodes[0].max_rate, 1.0); // neutral · 10
        assert_eq!(m.nodes[6].economy_modifier, 0.06f32 + 0.5); // CO$$: + co bonus
        assert_eq!(m.nodes[8].economy_modifier, 0.06f32 + 0.25); // I-R: + i bonus
        assert_eq!(f.other_calls.get(), 0);
    }

    /// Runs 42 and 43 (demand shadow, Spandau): §8.1's tax term with the
    /// shipped curve (0.2, 0.3), (1, 0), (5, −3.5), ordinance factor 1 and
    /// the history max 70 (I-R: 90), at the neutral rates the original's
    /// GetNeutralTaxRate returned (bit-identical to ours). 10 of 11 checks
    /// agree; on 2001-10-01 of run 43 the original is 4 ulp above us. One
    /// operation order reproduces all 132 values: `(maxRate · 0.1) ·
    /// (1 / neutral)` (the reciprocal rounded to f32 first), not the spec's
    /// literal `maxRate · 0.1 / neutral` (demand §8.1; open question, not
    /// changed here). This test pins both until the question is answered.
    #[test]
    fn tax_ratio_operation_order_runs_42_43() {
        let f = Fake::default();
        let tuning = DemandTuning {
            tax_modifier_curve: crate::curve::ResponseCurve::from_flat(&[0.2, 0.3, 1.0, 0.0, 5.0, -3.5]),
            ..DemandTuning::default()
        };
        // Cs$ (index 3): the fake's ordinance factor is 1.0 for it.
        let ours = |neutral: f32, max: f32| {
            f.history.set(Some(max));
            total_node_modifiers(3, neutral, &tuning, 0.0, 0.0, 2_452_184, &f).tax_modifier
        };
        let reciprocal = |neutral: f32, max: f32| {
            let r = (max * 0.1) * (1.0 / neutral);
            1.0f32 - 1.0 + tuning.tax_modifier_curve.as_ref().unwrap().eval(r)
        };
        let b = f32::from_bits;
        // Run 43, 2001-10-01: neutral 8.99755573; orig 0.0832541287, ours 0.0832540989.
        let n = b(0x410F_F5FD);
        assert_eq!(ours(n, 70.0).to_bits(), 0x3DAA_8120);
        assert_eq!(reciprocal(n, 70.0).to_bits(), 0x3DAA_8124);
        // I-R (max 90): -0.000237718225 under both.
        assert_eq!(ours(n, 90.0).to_bits(), 0xB979_4400);
        assert_eq!(reciprocal(n, 90.0).to_bits(), 0xB979_4400);
        // The other checks (neutral bits, the original's tax bits): equal under both.
        let agree: [(u32, u32); 10] = [
            (0x410F_F67A, 0x3DAA_8328), // 2001-09-09 (runs 42 and 43)
            (0x410F_F55F, 0x3DAA_7E92), // run 43 2001-11-01
            (0x410F_F4F9, 0x3DAA_7CEC), // run 43 2001-12-01
            (0x410F_F4CB, 0x3DAA_7C2C), // run 43 2002-01-01
            (0x410F_F4A4, 0x3DAA_7B88), // run 43 2002-02-01
            (0x410F_F5FF, 0x3DAA_8128), // run 42 2001-10-01
            (0x410F_F56A, 0x3DAA_7EC0), // run 42 2001-11-01
            (0x410F_F4DA, 0x3DAA_7C6A), // run 42 2001-12-01
            (0x410F_F49F, 0x3DAA_7B74), // run 42 2002-01-01
            (0x410F_F477, 0x3DAA_7AD0), // run 42 2002-02-01
        ];
        for (nb, tb) in agree {
            assert_eq!(ours(b(nb), 70.0).to_bits(), tb, "neutral {:#x}", nb);
            assert_eq!(reciprocal(b(nb), 70.0).to_bits(), tb, "neutral {:#x}", nb);
        }
    }
}

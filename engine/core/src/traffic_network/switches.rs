// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Transit switches (§2.4, §4.7): stations and other lots that change the
//! travel type. Activation (§4.7.1), the rules (§4.7.2), the switch phase of
//! `DoConnectionsChanged` (§4.7.4) and deactivation (§4.7.5). The entry cost
//! (§4.7.3) is on [`NetworkState`](super::NetworkState).

use crate::variant::{Elem, Variant};

use super::model::{Cx, NetworkModel};
use super::{prop, side, travel, CellRect, NetworkHost, DX, DZ, NET_OF_TRAVEL, ROT};

/// A transit-switch rule (§2.4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rule {
    /// 0 = leaving the switch, 1 = entering the switch (§4.7.4).
    pub kind: u8,
    /// One bool per city side.
    pub side: [bool; 4],
    /// The travel type a traveller has.
    pub from: u8,
    /// The travel type it becomes.
    pub to: u8,
}

/// What `ReadTransitSwitchRules` produces (§4.7.2), with the switch flags
/// derived from the rules (§2.4).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuleSet {
    pub rules: Vec<Rule>,
    /// OR of `1 << to` over the kind-0 rules.
    pub to_mask: u16,
    /// From the "to" types of **all** rules, whatever their kind (TN19):
    /// 2 → 1, 3 → 2, 6 → 4, 7 → 8, 8 → 0x10, plus 0x40 when a rule links
    /// subway and elevated train.
    pub flags: u8,
}

/// What the network part of `DoLotActivated` did (§4.7.1). The simulator's
/// step 7 (traffic-simulator.md §4.1.3: the strategy bucket, the one
/// `RandomUint32Uniform(100)` draw, the budget items and the destination
/// bits) follows only for [`LotActivation::Done`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LotActivation {
    /// The lot has no building: DoLotActivated returns at once.
    NoBuilding,
    /// §4.7.1 step 1: the byte size of `0xE90E25A1` is not a multiple of 4,
    /// so DoLotActivated **returned** (TN1). No switch, no ferry activation,
    /// no rebuild, and none of step 7, not even the draw.
    Returned,
    /// Steps 2–6 ran as far as they apply; step 7 follows.
    Done {
        /// Whether a transit switch was created (always, once the property
        /// is present and step 1 passed, even with no rules).
        switch_created: bool,
    },
}

impl LotActivation {
    /// Whether a transit switch was created.
    pub fn switch_created(self) -> bool {
        matches!(self, LotActivation::Done { switch_created: true })
    }

    /// Whether the simulator's step 7 follows (§4.7.1 step 7).
    pub fn continues(self) -> bool {
        matches!(self, LotActivation::Done { .. })
    }
}

/// A transit switch (§2.4).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TransitSwitch {
    /// The occupant that carries the rules (one reference held); 0 = none.
    pub building: u64,
    /// Traffic through the switch (pathfinder and simulator).
    pub load: f32,
    /// Building property `0xE90E25A2`.
    pub time_factor: f32,
    /// Building property `0xE90E25A3`.
    pub capacity: f32,
    /// See [`RuleSet::flags`].
    pub flags: u8,
    /// The subway access point (§4.7.1 step 3).
    pub has_access_point: bool,
    pub access_x: u8,
    pub access_z: u8,
    /// Two f32 counters, saved, owned by traffic-simulator.md.
    pub counter_a: f32,
    pub counter_b: f32,
    /// The rules.
    pub rules: Vec<Rule>,
    /// The kind-0 "to" mask.
    pub to_mask: u16,
    /// The toll, building property `0x0BFC0A4C` read as f32 (absent: 0.0),
    /// kept as its bits (§2.4; traffic-simulator.md §4.5, TS15).
    pub simulator_value: u32,
    /// "Counts as intersection", building property `0x4BFC47B0` (§4.6).
    pub counts_as_intersection: bool,
}

impl TransitSwitch {
    /// The mode mask (§4.7.1 step 4): OR of `1 << from` over the kind-0 rules
    /// with `from < 8`.
    pub fn mode_mask(&self) -> u8 {
        self.rules
            .iter()
            .filter(|r| r.kind == 0 && r.from < 8)
            .fold(0u8, |m, r| m | (1 << r.from))
    }
}

/// The switch flag of a rule's "to" type (§2.4).
fn flag_for_to(to: u8) -> u8 {
    match to {
        2 => 0x01,
        3 => 0x02,
        6 => 0x04,
        7 => 0x08,
        8 => 0x10,
        _ => 0,
    }
}

/// `ReadTransitSwitchRules` on the rules of property `0xE90E25A1` (§4.7.2),
/// each given as the u32 of its four bytes `b0…b3` (b0 the least
/// significant), with the lot's facing `f`: `kind = (b0 = 0x82) ? 1 : 0`;
/// `side[ROT[f][i]] = bit (7 − i) of b1`; `from = b2`, `to = b3`. Kind-0
/// rules OR `1 << to` into the "to" mask; the flags come from every rule,
/// whatever its kind (TN19). A facing above 3 uses its low two bits
/// [choice].
pub fn read_rules(facing: u32, entries: &[u32]) -> RuleSet {
    let rot = ROT[(facing & 3) as usize];
    let mut set = RuleSet::default();
    for &entry in entries {
        let [b0, b1, b2, b3] = entry.to_le_bytes();
        let mut r = Rule {
            kind: u8::from(b0 == 0x82),
            side: [false; 4],
            from: b2,
            to: b3,
        };
        for (i, &s) in rot.iter().enumerate() {
            r.side[s] = b1 & (0x80 >> i) != 0;
        }
        if r.kind == 0 && r.to < 16 {
            set.to_mask |= 1 << r.to;
        }
        set.flags |= flag_for_to(r.to);
        if (r.from == 6 && r.to == 7) || (r.from == 7 && r.to == 6) {
            set.flags |= 0x40;
        }
        set.rules.push(r);
    }
    set
}

/// The raw bytes `0x005FFB40` copies out of property `0xE90E25A1` (§4.7.2):
/// the variant's `GetCount()` bytes of its data (`RefUint8()`), whatever
/// the element type. So the "byte size" of §4.7.1 step 1 is the element
/// count: a u8 array (every shipped switch) gives all its bytes; a u32 array
/// of N entries gives only its first N bytes, which are its first N/4
/// entries whole *[quirk]*; a scalar has count 0 and gives none. An empty
/// variant and an object reference give none too (TN22, from the exe): the
/// count is a field of its own, which the constructors and `Destroy` zero
/// and the object-reference setter leaves alone. The property still counts
/// as present, so DoLotActivated builds a switch with no rules (mode mask 0,
/// flags 0, no access point); only an absent property skips the switch.
pub fn rule_bytes(v: &Variant) -> Vec<u8> {
    fn push(bytes: &mut Vec<u8>, e: &Elem) {
        match *e {
            Elem::Bool(b) => bytes.push(u8::from(b)),
            Elem::U8(v) | Elem::Char(v) | Elem::RzChar(v) => bytes.push(v),
            Elem::I8(v) => bytes.extend(v.to_le_bytes()),
            Elem::U16(v) | Elem::UnicodeChar(v) => bytes.extend(v.to_le_bytes()),
            Elem::I16(v) => bytes.extend(v.to_le_bytes()),
            Elem::U32(v) => bytes.extend(v.to_le_bytes()),
            Elem::I32(v) => bytes.extend(v.to_le_bytes()),
            Elem::F32(v) => bytes.extend(v.to_bits().to_le_bytes()),
            Elem::U64(v) | Elem::VoidPtr(v) => bytes.extend(v.to_le_bytes()),
            Elem::I64(v) => bytes.extend(v.to_le_bytes()),
            Elem::F64(v) => bytes.extend(v.to_bits().to_le_bytes()),
        }
    }
    let Variant::Array { items, .. } = v else {
        return Vec::new();
    };
    let mut bytes = Vec::new();
    items.iter().for_each(|e| push(&mut bytes, e));
    bytes.truncate(items.len());
    bytes
}

/// The `n = ⌊count / 4⌋` rules of the raw bytes (§4.7.2), each as the u32 of
/// its four bytes (b0 the least significant) for [`read_rules`]. Trailing
/// bytes are ignored here; DoLotActivated refuses them first (§4.7.1 step
/// 1).
pub fn rule_entries(bytes: &[u8]) -> Vec<u32> {
    bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn f32_prop(h: &dyn NetworkHost, occ: u64, id: u32) -> f32 {
    h.occupant_property(occ, id)
        .and_then(|v| v.items().first().map(|e| e.as_f64() as f32))
        .unwrap_or(0.0)
}

fn u32_prop(h: &dyn NetworkHost, occ: u64, id: u32) -> u32 {
    h.occupant_property(occ, id)
        .and_then(|v| v.items().first().map(|e| e.as_i64() as u32))
        .unwrap_or(0)
}

impl Cx<'_> {
    /// `DoLotActivated`, the network part (§4.7.1 and §4.9.1). The
    /// strategy-bucket registration, the one random draw, the budget items
    /// and the destination bits that follow are the simulator's (§4.7.1 step
    /// 7, §12); they follow only for [`LotActivation::Done`].
    ///
    /// Steps 1–5 apply when the building has property `0xE90E25A1`. A byte
    /// size (the element count, §4.7.2) that is not a multiple of 4 returns
    /// from DoLotActivated: no switch, no ferry part, no rebuild, no step 7
    /// (TN1).
    pub(crate) fn lot_activated(&mut self, lot: u64) -> LotActivation {
        let Some(building) = self.h.lot_building(lot) else {
            return LotActivation::NoBuilding;
        };
        let rect = self.h.lot_bounding_rect(lot);
        let mut created = false;
        if let Some(p) = self.h.occupant_property(building, prop::SWITCH_RULES) {
            // Step 1.
            if rule_bytes(&p).len() % 4 != 0 {
                return LotActivation::Returned;
            }
            self.create_switch(building, rect);
            created = true;
        }
        // Step 6.
        if self.h.occupant_property(building, prop::FERRY_TYPE).is_some() {
            self.ferry_activated(lot, building, rect);
            self.do_connections_changed(rect);
        } else if created {
            self.do_connections_changed(rect);
        }
        LotActivation::Done {
            switch_created: created,
        }
    }

    /// §4.7.1 steps 2–5: builds the switch, writes the grids over the lot
    /// rectangle and inserts it under the next switch ID.
    fn create_switch(&mut self, building: u64, lot_rect: CellRect) {
        let h = self.h;
        // Step 2: the building reference, the four properties, the rules and
        // the flags (a fresh switch: with no lot it keeps none).
        let rs = self.read_switch_rules(building).unwrap_or_default();
        h.add_ref(building);
        let mut sw = TransitSwitch {
            building,
            time_factor: f32_prop(h, building, prop::SWITCH_TIME_FACTOR),
            capacity: f32_prop(h, building, prop::SWITCH_CAPACITY),
            simulator_value: f32_prop(h, building, prop::SWITCH_SIMULATOR).to_bits(),
            counts_as_intersection: u32_prop(h, building, prop::COUNTS_AS_INTERSECTION) != 0,
            flags: rs.flags,
            to_mask: rs.to_mask,
            rules: rs.rules,
            ..TransitSwitch::default()
        };
        // Step 3: a Subway "to" rule sets the access point.
        if sw.flags & 4 != 0 {
            self.set_access_point(&mut sw, lot_rect);
        }
        // Step 4: the mode mask.
        let mask = sw.mode_mask();
        // Step 5: the grids, the table, the next ID.
        let id = self.s.next_switch_id;
        self.write_switch_grids(lot_rect, id, mask);
        *self.s.switches.entry(id, TransitSwitch::default) = sw;
        self.s.next_switch_id = id.wrapping_add(1);
    }

    /// §4.7.1 step 3: the first subway access-point occupant's cell in the
    /// lot rectangle, else the NW cell of the building's bounding cells.
    fn set_access_point(&self, sw: &mut TransitSwitch, lot_rect: CellRect) {
        let h = self.h;
        let (ax, az) = match h.subway_occupants_in_rect(lot_rect).first() {
            Some(&occ) => {
                let p = h.occupant_position(occ);
                h.position_to_cell(p[0], p[2])
            }
            None => {
                let b = h.occupant_bounding_cells(sw.building);
                (b.x1, b.z1)
            }
        };
        sw.has_access_point = true;
        sw.access_x = ax as u8;
        sw.access_z = az as u8;
    }

    /// `ReadTransitSwitchRules` (§4.7.2): the lot at the NW corner of the
    /// building's bounding cells gives the facing. With no lot no rules are
    /// read *[quirk]*: `None`, and the caller's rules and flags stay.
    /// Otherwise the `⌊count / 4⌋` rules of the raw bytes (an absent
    /// property gives none), their "to" mask and the flags from all of them
    /// (TN19).
    pub(crate) fn read_switch_rules(&self, building: u64) -> Option<RuleSet> {
        let h = self.h;
        let b = h.occupant_bounding_cells(building);
        let lot = h.lot_at(b.x1, b.z1, false)?;
        let facing = h.lot_facing(lot);
        let bytes = h
            .occupant_property(building, prop::SWITCH_RULES)
            .map(|v| rule_bytes(&v))
            .unwrap_or_default();
        Some(read_rules(facing, &rule_entries(&bytes)))
    }

    /// DoPostCityInit steps 1 and 2 (§1, §4.7.2).
    ///
    /// Step 1, the re-read (TN3), for every switch in table order: the "to"
    /// mask is zeroed; with no lot at the building's NW cell the rules and
    /// flags stay as they were (the saved ones) and the mask stays 0;
    /// otherwise the rules are **replaced** by the property's *[choice: the
    /// original appends, so a loaded switch holds its rules twice]*, and the
    /// mask and flags are rebuilt from them. The mode-mask grid is not
    /// touched.
    ///
    /// *[choice]* (TN25) The flags come from the re-read rules. The original
    /// takes them from the first n entries of its appended list, which after
    /// a load are the saved rules, so the two agree whenever the building's
    /// `0xE90E25A1` is unchanged since the save. When it changed, the
    /// difference is wider than the flags: the original keeps the m old
    /// rules in front of the n new ones, so §4.7.4 still builds the old
    /// rules' connections (a new rule writing the same nibble wins), every
    /// later save writes the old rules back, and its flags (old rules, plus
    /// the first n − m new ones when m < n) can make the access-point walk
    /// disagree. §4.7.2 worked example: saved `81 F0 07 07`, `82 F0 07 07`
    /// (el train), exemplar now `81 F0 06 06`, `82 F0 06 06` (subway). The
    /// original holds all four rules, flags `0x08` (flag 4 clear: no access
    /// point) and builds both the el-train and the subway connections;
    /// FreeCity4 holds the two subway rules, flags `0x04`, sets the NW-cell
    /// access point when the save had none, and builds only the subway
    /// connections. FreeCity4 follows the current property throughout.
    ///
    /// Step 2, the access points (TN4), each switch on its own: flag 4 clear
    /// → no access point (the cell bytes stay); flag 4 set with an access
    /// point → kept; flag 4 set without one → the NW cell of the building's
    /// bounding cells, with no occupant search *[quirk]*.
    ///
    /// A switch with building 0 (TN23: a default inserted by a §4.7.3 or
    /// §4.7.4 lookup, or a loaded switch whose building reference was null)
    /// *[choice]*: not re-read, so its rules, flags and "to" mask stay; flag
    /// 4 clear → no access point, as for any switch; flag 4 set → "has access
    /// point" and the cell stay as they are, with no building to take a NW
    /// cell from. The original checks neither walk's building: a null one
    /// crashes it, and an inserted switch holds a stale stack value as its
    /// building (and stale load, capacity and flags).
    pub(crate) fn reread_switches(&mut self) {
        let ids = self.s.switches.keys();
        for &id in &ids {
            let Some(building) = self.s.switches.get(id).map(|sw| sw.building) else {
                continue;
            };
            if building == 0 {
                continue;
            }
            let rs = self.read_switch_rules(building);
            if let Some(sw) = self.s.switches.get_mut(id) {
                sw.to_mask = 0;
                if let Some(rs) = rs {
                    sw.rules = rs.rules;
                    sw.to_mask = rs.to_mask;
                    sw.flags = rs.flags;
                }
            }
        }
        let h = self.h;
        for &id in &ids {
            let Some(sw) = self.s.switches.get_mut(id) else {
                continue;
            };
            if sw.flags & 4 == 0 {
                sw.has_access_point = false;
            } else if !sw.has_access_point && sw.building != 0 {
                let b = h.occupant_bounding_cells(sw.building);
                sw.has_access_point = true;
                sw.access_x = b.x1 as u8;
                sw.access_z = b.z1 as u8;
            }
        }
    }

    /// Writes the switch ID and mode-mask grids over a rectangle.
    fn write_switch_grids(&mut self, rect: CellRect, id: u32, mask: u8) {
        for x in rect.x1..=rect.x2 {
            for z in rect.z1..=rect.z2 {
                if let Some(g) = &self.s.switch_id {
                    g.set(x, z, id);
                }
                if let Some(g) = &self.s.switch_mode {
                    g.set(x, z, mask);
                }
            }
        }
    }

    /// `DoLotDeactivated`, the network part (§4.7.5, then §4.9.4).
    pub(crate) fn lot_deactivated(&mut self, lot: u64) {
        let rect = self.h.lot_bounding_rect(lot);
        // Step 1: the switch ID at the lot rectangle's NW cell.
        let id = self.s.switch_id_at(rect.x1, rect.z1);
        if let Some(building) = self.s.switches.get(id).map(|sw| sw.building) {
            if building != 0 {
                self.h.release(building);
            }
            self.s.switches.remove(id);
            self.write_switch_grids(rect, 0, 0);
            self.do_connections_changed(CellRect::new(
                rect.x1.saturating_sub(1),
                rect.z1.saturating_sub(1),
                rect.x2.saturating_add(1),
                rect.z2.saturating_add(1),
            ));
        }
        // Step 2: ferry terminal removal.
        self.ferry_lot_deactivated(rect);
    }

    /// The switch phase of `DoConnectionsChanged` over R1 (§4.7.4): cells in
    /// rectangle order (x outer, z inner), sides 0…3, rules in stored order.
    pub(crate) fn apply_switches(&mut self, r1: CellRect) {
        for x in r1.x1..=r1.x2 {
            for z in r1.z1..=r1.z2 {
                let id = self.s.switch_id_at(x, z);
                if id == 0 {
                    continue;
                }
                // Looked up with insert-if-missing *[quirk]*.
                let rules = self.s.switches.entry(id, TransitSwitch::default).rules.clone();
                self.apply_switch_cell(x, z, id, &rules);
            }
        }
    }

    fn apply_switch_cell(&mut self, x: i32, z: i32, id: u32, rules: &[Rule]) {
        let s = &mut *self.s;
        // Step 1: every travel type except Subway: conn 0, switch 0xFFFF.
        if let Some(c) = s.conn_mut(x, z) {
            for t in (0..travel::COUNT).filter(|&t| t != travel::SUBWAY) {
                c.conn[t] = 0;
                c.switch[t] = 0xFFFF;
            }
        }
        // Step 2: each side whose neighbour N is in the city.
        for d in 0..side::COUNT {
            let (nx, nz) = (x + DX[d], z + DZ[d]);
            if !s.in_city(nx, nz) {
                continue;
            }
            if s.switch_id_at(nx, nz) == id {
                // Same switch: for each kind-0 rule with side[d],
                // SetConnection(cell, k, d, to) for every k with side[k].
                for r in rules.iter().filter(|r| r.kind == 0 && r.side[d]) {
                    for k in (0..side::COUNT).filter(|&k| r.side[k]) {
                        s.set_connection(x, z, k, d, usize::from(r.to));
                    }
                }
                continue;
            }
            let ns = super::side_of((x, z), (nx, nz));
            for r in rules.iter().filter(|r| r.side[d]) {
                let (from, to) = (usize::from(r.from), usize::from(r.to));
                if r.kind == 1 {
                    // Kind 1 (entering the switch).
                    if has_record_of_net(s, to, nx, nz) {
                        if from == to {
                            for k in 0..side::COUNT {
                                s.set_connection(x, z, k, d, from);
                            }
                        } else {
                            set_switch_nibble(s, x, z, from, d, r.to);
                        }
                    }
                    // Then SetConnection(N, ns, k, to) for each k ≠ ns with an
                    // exit k from some entry in N's conn[to].
                    for k in exits_through(s, nx, nz, to, ns) {
                        s.set_connection(nx, nz, ns, k, to);
                    }
                } else {
                    // Kind 0 (leaving the switch).
                    if has_record_of_net(s, from, nx, nz) {
                        if from != to {
                            set_switch_nibble(s, nx, nz, from, ns, r.to);
                        } else {
                            let entries: Vec<usize> = match s.conn_at(nx, nz) {
                                Some(c) if from < travel::COUNT => {
                                    (0..side::COUNT).filter(|&j| c.exits(from, j) != 0).collect()
                                }
                                _ => Vec::new(),
                            };
                            for j in entries {
                                s.set_connection(nx, nz, j, ns, from);
                            }
                        }
                    }
                    if from == travel::CAR {
                        for k in exits_through(s, nx, nz, travel::CAR, ns) {
                            s.set_connection(nx, nz, ns, k, travel::CAR);
                        }
                    }
                }
            }
        }
    }
}

/// Whether N has a record of `net(t)`; a travel type past the table has
/// none [choice].
fn has_record_of_net(s: &super::NetworkState, t: usize, x: i32, z: i32) -> bool {
    NET_OF_TRAVEL.get(t).is_some_and(|&n| s.network_info(n, x, z).is_some())
}

/// The sides k ≠ `ns` that `conn[t]` of (x, z) has as an exit from some
/// entry side.
fn exits_through(s: &super::NetworkState, x: i32, z: i32, t: usize, ns: usize) -> Vec<usize> {
    match s.conn_at(x, z) {
        Some(c) if t < travel::COUNT => (0..side::COUNT)
            .filter(|&k| k != ns && c.has_exit_through(t, k))
            .collect(),
        _ => Vec::new(),
    }
}

/// Sets nibble `d` of `switch[from]` at (x, z) to `to` (§4.7.4); the last
/// write wins.
fn set_switch_nibble(s: &mut super::NetworkState, x: i32, z: i32, from: usize, d: usize, to: u8) {
    if from >= travel::COUNT {
        return;
    }
    if let Some(c) = s.conn_mut(x, z) {
        let to = u16::from(to & 0xF);
        c.switch[from] = (c.switch[from] & !(0xF << (4 * d))) | (to << (4 * d));
    }
}

impl NetworkModel {
    /// Lot activated (§3, `0xE8BDA2E7` with data2 = 3; §4.7.1, §4.9.1).
    /// Returns whether a transit switch was created; see
    /// [`NetworkModel::lot_activated_outcome`] for whether step 7 follows.
    pub fn lot_activated(&self, lot: u64) -> bool {
        self.lot_activated_outcome(lot).switch_created()
    }

    /// Lot activated, with the full outcome (§4.7.1): the simulator runs its
    /// step 7 only when [`LotActivation::continues`] (TN1).
    pub fn lot_activated_outcome(&self, lot: u64) -> LotActivation {
        self.run(|cx| cx.lot_activated(lot))
    }

    /// Lot deactivated (§3, `0xE8BDA2E7` with data1 = 3; §4.7.5, §4.9.4).
    pub fn lot_deactivated(&self, lot: u64) {
        self.run(|cx| cx.lot_deactivated(lot))
    }

    /// `GetTransitSwitches` (slot 28): every switch building in table order.
    /// The glue AddRefs each into the caller's list.
    pub fn get_transit_switches(&self) -> Vec<u64> {
        self.s.borrow().switches.iter().map(|(_, sw)| sw.building).collect()
    }

    /// The lookup of `GetTransitSwitchQueryData(x, z, …)` (slot 35, §7.1): a
    /// switch by the switch ID grid, else the first ferry terminal (list
    /// order) whose lot cell lies in the lot at (x, z).
    pub fn transit_switch_query_target(&self, x: i32, z: i32) -> Option<super::SwitchQueryTarget> {
        let id = self.s.borrow().switch_id_at(x, z);
        if id != 0 && self.s.borrow().switches.contains(id) {
            return Some(super::SwitchQueryTarget::Switch(id));
        }
        let lot = self.host.lot_at(x, z, false)?;
        let r = self.host.lot_bounding_rect(lot);
        let s = self.s.borrow();
        s.ferries
            .iter()
            .position(|t| {
                let (lx, lz) = (i32::from(t.lot_x), i32::from(t.lot_z));
                lx >= r.x1 && lx <= r.x2 && lz >= r.z1 && lz <= r.z2
            })
            .map(super::SwitchQueryTarget::Ferry)
    }
}

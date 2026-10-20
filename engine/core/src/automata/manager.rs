// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Components 43–46, the automata managers (§4, §5): the base record
//! (version 6), the traffic automata manager record of the vehicle and
//! pedestrian managers (version 3), the aircraft (version 5) and watercraft
//! (version 6) records, and the manager slots other specs use (§7.3, §9).

use std::rc::Rc;

use crate::stream::{InStream, OutStream, RefValue, ResKey};

use super::automaton::{Automaton, AutomatonRecord, PedestrianRecord, VehicleRecord};
use super::codec::{
    get_byte_vec, get_n_refs, get_ref_checked, get_refs, get_vec3, put_byte_vec, put_vec3, ref_lists_equal, refs_equal,
    same_ref, Vec3,
};
use super::{
    CLSID_AIRCRAFT_MANAGER, CLSID_PEDESTRIAN_MANAGER, CLSID_VEHICLE_MANAGER, CLSID_WATERCRAFT_MANAGER, IID_AUTOMATON,
    IID_LOT, IID_NETWORK_OCCUPANT, IID_OCCUPANT, U_DRIVE_IT_VIEW,
};

/// The base record's current version (§4; Read accepts ≤ 6).
pub const MANAGER_BASE_VERSION: u16 = 6;
/// The traffic automata manager's version (§5.1; Read accepts 2 or 3).
pub const TRAFFIC_MANAGER_VERSION: u16 = 3;
/// The incident list's leading `u16` (§5.1 field 5).
pub const INCIDENT_LIST_TAG: u16 = 1;
/// The aircraft manager's version (§5.2; above 5 fails).
pub const AIRCRAFT_MANAGER_VERSION: u16 = 5;
/// The watercraft manager's version (§5.3).
pub const WATERCRAFT_MANAGER_VERSION: u16 = 6;
/// `SetMaxCount` clamps to 0–5000 (§7.3).
pub const MAX_COUNT_LIMIT: i32 = 5000;

/// Which of the four managers (§0 table).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagerKind {
    /// 43, `0xA98F9D3B`.
    Vehicle,
    /// 44, `0xC98F9D17`.
    Pedestrian,
    /// 45, `0xC98F9D4A`.
    Aircraft,
    /// 46, `0xA98F9D28`.
    Watercraft,
}

impl ManagerKind {
    pub fn class_id(self) -> u32 {
        match self {
            ManagerKind::Vehicle => CLSID_VEHICLE_MANAGER,
            ManagerKind::Pedestrian => CLSID_PEDESTRIAN_MANAGER,
            ManagerKind::Aircraft => CLSID_AIRCRAFT_MANAGER,
            ManagerKind::Watercraft => CLSID_WATERCRAFT_MANAGER,
        }
    }

    pub fn from_class(class: u32) -> Option<ManagerKind> {
        Some(match class {
            CLSID_VEHICLE_MANAGER => ManagerKind::Vehicle,
            CLSID_PEDESTRIAN_MANAGER => ManagerKind::Pedestrian,
            CLSID_AIRCRAFT_MANAGER => ManagerKind::Aircraft,
            CLSID_WATERCRAFT_MANAGER => ManagerKind::Watercraft,
            _ => return None,
        })
    }

    /// The traffic manager's discarded `u32` (§5.1 field 3): vehicle 1,
    /// pedestrian 2 in every save.
    pub fn traffic_tag(self) -> u32 {
        match self {
            ManagerKind::Pedestrian => 2,
            _ => 1,
        }
    }

    fn is_traffic(self) -> bool {
        matches!(self, ManagerKind::Vehicle | ManagerKind::Pedestrian)
    }
}

/// The base record (§4).
#[derive(Clone, Debug)]
pub struct ManagerBase {
    /// The version as read (6 for a new manager). Versions 5 and 6 have the
    /// same layout; below 5 there is no group map. Written back unchanged
    /// unless the group map is not empty, which needs version 5 or more
    /// *[choice]*.
    pub version: u16,
    pub max_count: i32,
    pub active: bool,
    /// The time-of-day scaling flag *[inferred]*.
    pub time_scaling: bool,
    /// The manager's automata (read with `0xA9B40F05`).
    pub automata: Vec<RefValue>,
    /// The group instance counts (slot 31), version ≥ 5.
    pub group_counts: Vec<(u32, u32)>,
}

impl Default for ManagerBase {
    fn default() -> Self {
        ManagerBase {
            version: MANAGER_BASE_VERSION,
            max_count: 0,
            active: false,
            time_scaling: false,
            automata: Vec::new(),
            group_counts: Vec::new(),
        }
    }
}

impl PartialEq for ManagerBase {
    fn eq(&self, o: &Self) -> bool {
        self.version == o.version
            && self.max_count == o.max_count
            && self.active == o.active
            && self.time_scaling == o.time_scaling
            && ref_lists_equal(&self.automata, &o.automata)
            && self.group_counts == o.group_counts
    }
}

/// Whether a manager's list entry is written: an emergency vehicle is not
/// (§12 quirk 3, *[choice]*).
fn saved_automaton(r: &RefValue) -> bool {
    match r {
        RefValue::Object(o) => !o
            .clone()
            .as_any_rc()
            .downcast::<Automaton>()
            .map(|a| a.is_emergency())
            .unwrap_or(false),
        _ => true,
    }
}

impl ManagerBase {
    fn written_version(&self) -> u16 {
        if self.version < 5 && !self.group_counts.is_empty() {
            MANAGER_BASE_VERSION
        } else {
            self.version
        }
    }

    /// Write (§4). Always true *[quirk]* (§12 item 4 for Read; §4 for Write).
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        let v = self.written_version();
        out.put_u16(v);
        out.put_i32(self.max_count);
        out.put_bool(self.active);
        out.put_bool(self.time_scaling);
        let saved: Vec<&RefValue> = self.automata.iter().filter(|r| saved_automaton(r)).collect();
        out.put_u32(saved.len() as u32);
        for r in saved {
            out.put_ref(r);
        }
        if v >= 5 {
            out.put_u32(self.group_counts.len() as u32);
            for &(g, n) in &self.group_counts {
                out.put_u32(g);
                out.put_u32(n);
            }
        }
        true
    }

    /// Read (§4): `None` on a version above 6 or a short stream. (The
    /// original returns true without checking the stream error; the managers
    /// here report a short read, so the round trip can be checked
    /// *[choice]*, AM11.)
    pub fn read(inp: &mut dyn InStream) -> Option<ManagerBase> {
        let version = inp.get_u16()?;
        ManagerBase::read_after_version(version, inp)
    }

    fn read_after_version(version: u16, inp: &mut dyn InStream) -> Option<ManagerBase> {
        if version > MANAGER_BASE_VERSION {
            return None;
        }
        let max_count = inp.get_i32()?;
        let active = inp.get_bool()?;
        let time_scaling = inp.get_bool()?;
        let automata = get_refs(inp, IID_AUTOMATON)?;
        let mut group_counts = Vec::new();
        if version >= 5 {
            let m = inp.get_u32()?;
            for _ in 0..m {
                group_counts.push((inp.get_u32()?, inp.get_u32()?));
            }
        }
        Some(ManagerBase {
            version,
            max_count,
            active,
            time_scaling,
            automata,
            group_counts,
        })
    }
}

/// A traffic incident (an accident, §5.1).
#[derive(Clone, Debug)]
pub struct Incident {
    pub x: i32,
    pub z: i32,
    pub kind: u32,
    pub values: [i32; 8],
    /// `ref` network occupant (read with `0xA821EF94`).
    pub network: RefValue,
    pub flag: bool,
    pub last: u32,
}

impl PartialEq for Incident {
    fn eq(&self, o: &Self) -> bool {
        self.x == o.x
            && self.z == o.z
            && self.kind == o.kind
            && self.values == o.values
            && refs_equal(&self.network, &o.network)
            && self.flag == o.flag
            && self.last == o.last
    }
}

impl Incident {
    fn write(&self, out: &mut dyn OutStream) {
        out.put_i32(self.x);
        out.put_i32(self.z);
        out.put_u32(self.kind);
        for v in &self.values {
            out.put_i32(*v);
        }
        out.put_ref(&self.network);
        out.put_bool(self.flag);
        out.put_u32(self.last);
    }

    fn read(inp: &mut dyn InStream) -> Option<Incident> {
        let x = inp.get_i32()?;
        let z = inp.get_i32()?;
        let kind = inp.get_u32()?;
        let mut values = [0i32; 8];
        for v in &mut values {
            *v = inp.get_i32()?;
        }
        Some(Incident {
            x,
            z,
            kind,
            values,
            network: get_ref_checked(inp, IID_NETWORK_OCCUPANT)?,
            flag: inp.get_bool()?,
            last: inp.get_u32()?,
        })
    }
}

/// The vehicle and pedestrian managers' part (§5.1).
#[derive(Clone, Debug, PartialEq)]
pub struct TrafficPart {
    /// The version as read (3 for a new manager); version 2 has no incident
    /// list. Written back unchanged unless there are incidents *[choice]*.
    pub version: u16,
    /// Field 3, discarded by Read; written as the kind's constant (vehicle 1,
    /// pedestrian 2) *[choice]* unless a read gave another value, which is
    /// kept so the record re-encodes byte-identically.
    pub tag: u32,
    /// Field 4: the ID at the list cursor, kept as read (§5.1; what the IDs of
    /// the list nodes are is open, AM3).
    pub cursor: u32,
    /// Field 5's leading `u16` (version 3), kept as read.
    pub incident_tag: u16,
    pub incidents: Vec<Incident>,
}

/// One route element of a ferry entry (§5.3).
#[derive(Clone, Debug)]
pub struct RouteElement {
    pub from_lot: RefValue,
    pub to_lot: RefValue,
    /// 0, 1, otherwise −1 (§5.3); stored as read.
    pub kind: i32,
    pub words: [u16; 4],
    pub flag: bool,
    pub a: i16,
    pub b: i16,
    pub bytes1: Vec<u8>,
    pub bytes2: Vec<u8>,
}

impl PartialEq for RouteElement {
    fn eq(&self, o: &Self) -> bool {
        refs_equal(&self.from_lot, &o.from_lot)
            && refs_equal(&self.to_lot, &o.to_lot)
            && self.kind == o.kind
            && self.words == o.words
            && self.flag == o.flag
            && self.a == o.a
            && self.b == o.b
            && self.bytes1 == o.bytes1
            && self.bytes2 == o.bytes2
    }
}

/// A ferry entry (§5.3 field 5).
#[derive(Clone, Debug)]
pub struct FerryEntry {
    pub lot: RefValue,
    pub occupant: RefValue,
    pub route: Vec<RouteElement>,
}

impl PartialEq for FerryEntry {
    fn eq(&self, o: &Self) -> bool {
        refs_equal(&self.lot, &o.lot) && refs_equal(&self.occupant, &o.occupant) && self.route == o.route
    }
}

/// The kind-specific part of a manager record.
#[derive(Clone, Debug)]
pub enum ManagerPart {
    Traffic(TrafficPart),
    /// §5.2: the airport pairs (occupant, automaton) and the (`vec3`, `u8`)
    /// list.
    Aircraft {
        pairs: Vec<(RefValue, RefValue)>,
        spots: Vec<(Vec3, u8)>,
    },
    /// §5.3: (lot, occupant) pairs, occupants, ferry entries.
    Watercraft {
        lots: Vec<(RefValue, RefValue)>,
        occupants: Vec<RefValue>,
        ferries: Vec<FerryEntry>,
    },
}

fn pairs_equal(a: &[(RefValue, RefValue)], b: &[(RefValue, RefValue)]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| refs_equal(&x.0, &y.0) && refs_equal(&x.1, &y.1))
}

impl PartialEq for ManagerPart {
    fn eq(&self, o: &Self) -> bool {
        match (self, o) {
            (ManagerPart::Traffic(a), ManagerPart::Traffic(b)) => a == b,
            (ManagerPart::Aircraft { pairs, spots }, ManagerPart::Aircraft { pairs: p2, spots: s2 }) => {
                pairs_equal(pairs, p2) && spots == s2
            }
            (
                ManagerPart::Watercraft {
                    lots,
                    occupants,
                    ferries,
                },
                ManagerPart::Watercraft {
                    lots: l2,
                    occupants: o2,
                    ferries: f2,
                },
            ) => pairs_equal(lots, l2) && ref_lists_equal(occupants, o2) && ferries == f2,
            _ => false,
        }
    }
}

/// An `s32` count (§5.3): a negative count reads no entries *[choice]*.
fn get_s32_count(inp: &mut dyn InStream) -> Option<usize> {
    Some(inp.get_i32()?.max(0) as usize)
}

impl RouteElement {
    fn write(&self, out: &mut dyn OutStream) {
        out.put_ref(&self.from_lot);
        out.put_ref(&self.to_lot);
        out.put_i32(self.kind);
        for w in &self.words {
            out.put_u16(*w);
        }
        out.put_bool(self.flag);
        out.put_i16(self.a);
        out.put_i16(self.b);
        put_byte_vec(out, &self.bytes1);
        put_byte_vec(out, &self.bytes2);
    }

    fn read(inp: &mut dyn InStream) -> Option<RouteElement> {
        Some(RouteElement {
            from_lot: get_ref_checked(inp, IID_LOT)?,
            to_lot: get_ref_checked(inp, IID_LOT)?,
            kind: inp.get_i32()?,
            words: [inp.get_u16()?, inp.get_u16()?, inp.get_u16()?, inp.get_u16()?],
            flag: inp.get_bool()?,
            a: inp.get_i16()?,
            b: inp.get_i16()?,
            bytes1: get_byte_vec(inp)?,
            bytes2: get_byte_vec(inp)?,
        })
    }
}

/// A whole manager record: the record version (aircraft and watercraft: the
/// outer `u16`, or the base's own when the record is a bare old base), the
/// base, and the kind's part.
#[derive(Clone, Debug, PartialEq)]
pub struct ManagerRecord {
    pub kind: ManagerKind,
    /// Aircraft / watercraft: the version as read (below 5 / 6 the record is a
    /// bare base record of that version). Traffic managers: unused (their
    /// version is in [`TrafficPart`]).
    pub version: u16,
    pub base: ManagerBase,
    pub part: ManagerPart,
}

impl ManagerRecord {
    /// The record of a new manager of `kind`.
    pub fn new(kind: ManagerKind) -> ManagerRecord {
        let (version, part) = match kind {
            ManagerKind::Vehicle | ManagerKind::Pedestrian => (
                TRAFFIC_MANAGER_VERSION,
                ManagerPart::Traffic(TrafficPart {
                    version: TRAFFIC_MANAGER_VERSION,
                    tag: kind.traffic_tag(),
                    cursor: 0,
                    incident_tag: INCIDENT_LIST_TAG,
                    incidents: Vec::new(),
                }),
            ),
            ManagerKind::Aircraft => (
                AIRCRAFT_MANAGER_VERSION,
                ManagerPart::Aircraft {
                    pairs: Vec::new(),
                    spots: Vec::new(),
                },
            ),
            ManagerKind::Watercraft => (
                WATERCRAFT_MANAGER_VERSION,
                ManagerPart::Watercraft {
                    lots: Vec::new(),
                    occupants: Vec::new(),
                    ferries: Vec::new(),
                },
            ),
        };
        ManagerRecord {
            kind,
            version,
            base: ManagerBase::default(),
            part,
        }
    }

    /// The version a write uses: the read one, raised to the current one when
    /// the kind's part holds data the old layout cannot carry *[choice]*.
    fn written_version(&self) -> u16 {
        match &self.part {
            ManagerPart::Traffic(t) => {
                if t.version < 3 && !t.incidents.is_empty() {
                    TRAFFIC_MANAGER_VERSION
                } else {
                    t.version
                }
            }
            ManagerPart::Aircraft { pairs, spots } => {
                if self.version < AIRCRAFT_MANAGER_VERSION && (!pairs.is_empty() || !spots.is_empty()) {
                    AIRCRAFT_MANAGER_VERSION
                } else {
                    self.version
                }
            }
            ManagerPart::Watercraft {
                lots,
                occupants,
                ferries,
            } => {
                if self.version < WATERCRAFT_MANAGER_VERSION
                    && (!lots.is_empty() || !occupants.is_empty() || !ferries.is_empty())
                {
                    WATERCRAFT_MANAGER_VERSION
                } else {
                    self.version
                }
            }
        }
    }

    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        if out.error() != 0 {
            return false;
        }
        let v = self.written_version();
        match &self.part {
            ManagerPart::Traffic(t) => {
                out.put_u16(v);
                self.base.write(out);
                out.put_u32(t.tag);
                out.put_u32(t.cursor);
                if v >= 3 {
                    out.put_u16(t.incident_tag);
                    out.put_u32(t.incidents.len() as u32);
                    for i in &t.incidents {
                        i.write(out);
                    }
                }
            }
            ManagerPart::Aircraft { pairs, spots } => {
                if v < AIRCRAFT_MANAGER_VERSION {
                    // A bare base record of the old version (§5.2).
                    let mut b = self.base.clone();
                    b.version = v;
                    b.write(out);
                } else {
                    out.put_u16(v);
                    self.base.write(out);
                    out.put_u32(pairs.len() as u32);
                    for (o, a) in pairs {
                        out.put_ref(o);
                        out.put_ref(a);
                    }
                    out.put_u32(spots.len() as u32);
                    for (p, b) in spots {
                        put_vec3(out, p);
                        out.put_u8(*b);
                    }
                }
            }
            ManagerPart::Watercraft {
                lots,
                occupants,
                ferries,
            } => {
                if v < WATERCRAFT_MANAGER_VERSION {
                    let mut b = self.base.clone();
                    b.version = v;
                    b.write(out);
                } else {
                    out.put_u16(v);
                    self.base.write(out);
                    out.put_i32(lots.len() as i32);
                    for (l, o) in lots {
                        out.put_ref(l);
                        out.put_ref(o);
                    }
                    out.put_i32(occupants.len() as i32);
                    for o in occupants {
                        out.put_ref(o);
                    }
                    out.put_i32(ferries.len() as i32);
                    for f in ferries {
                        out.put_ref(&f.lot);
                        out.put_ref(&f.occupant);
                        out.put_i32(f.route.len() as i32);
                        for e in &f.route {
                            e.write(out);
                        }
                    }
                }
            }
        }
        out.error() == 0
    }

    /// Reads a record of `kind`. `None` on a version the reader rejects or a
    /// short stream.
    pub fn read(kind: ManagerKind, inp: &mut dyn InStream) -> Option<ManagerRecord> {
        let version = inp.get_u16()?;
        if kind.is_traffic() {
            // §5.1: Read accepts 2 or 3.
            if version != 2 && version != 3 {
                return None;
            }
            let base = ManagerBase::read(inp)?;
            let tag = inp.get_u32()?;
            let cursor = inp.get_u32()?;
            let (incident_tag, incidents) = if version >= 3 {
                let t = inp.get_u16()?;
                let n = inp.get_u32()?;
                let mut v = Vec::new();
                for _ in 0..n {
                    v.push(Incident::read(inp)?);
                }
                (t, v)
            } else {
                (INCIDENT_LIST_TAG, Vec::new())
            };
            return Some(ManagerRecord {
                kind,
                version,
                base,
                part: ManagerPart::Traffic(TrafficPart {
                    version,
                    tag,
                    cursor,
                    incident_tag,
                    incidents,
                }),
            });
        }
        let current = if kind == ManagerKind::Aircraft {
            AIRCRAFT_MANAGER_VERSION
        } else {
            WATERCRAFT_MANAGER_VERSION
        };
        if version > current {
            // §5.2: a version above 5 fails; §5.3 is read the same way
            // *[choice]* (the spec gives no upper check for the watercraft, AM9).
            return None;
        }
        if version < current {
            // The whole record is a base record of that version.
            let base = ManagerBase::read_after_version(version, inp)?;
            let mut r = ManagerRecord::new(kind);
            r.version = version;
            r.base = base;
            return Some(r);
        }
        let base = ManagerBase::read(inp)?;
        let part = if kind == ManagerKind::Aircraft {
            let n = inp.get_u32()?;
            let mut pairs = Vec::new();
            for _ in 0..n {
                pairs.push((
                    get_ref_checked(inp, IID_OCCUPANT)?,
                    get_ref_checked(inp, IID_AUTOMATON)?,
                ));
            }
            let m = inp.get_u32()?;
            let mut spots = Vec::new();
            for _ in 0..m {
                spots.push((get_vec3(inp)?, inp.get_u8()?));
            }
            ManagerPart::Aircraft { pairs, spots }
        } else {
            let n = get_s32_count(inp)?;
            let mut lots = Vec::new();
            for _ in 0..n {
                lots.push((get_ref_checked(inp, IID_LOT)?, get_ref_checked(inp, IID_OCCUPANT)?));
            }
            let n = get_s32_count(inp)?;
            let occupants = get_n_refs(inp, n, IID_OCCUPANT)?;
            let n = get_s32_count(inp)?;
            let mut ferries = Vec::new();
            for _ in 0..n {
                let lot = get_ref_checked(inp, IID_LOT)?;
                let occupant = get_ref_checked(inp, IID_OCCUPANT)?;
                let k = get_s32_count(inp)?;
                let mut route = Vec::new();
                for _ in 0..k {
                    route.push(RouteElement::read(inp)?);
                }
                ferries.push(FerryEntry { lot, occupant, route });
            }
            ManagerPart::Watercraft {
                lots,
                occupants,
                ferries,
            }
        };
        Some(ManagerRecord {
            kind,
            version,
            base,
            part,
        })
    }
}

/// The seven arguments of pedestrian / vehicle manager slot 15
/// `CreateAutomaton` (§7.2; Q5 answered by re-integrate1). Only the group and
/// the position matter to the simulation; the rest are passed straight on to
/// the host's call, which is where they are interpreted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CreateArgs {
    /// Argument 1, the automata group ID (selects the exemplar and the class).
    pub group: u32,
    /// Argument 2, the position, city metres (`SetPosition` after creation).
    pub position: Vec3,
    /// Argument 3, the automata flags. Bit 2 = self-controlling; `0x80`
    /// lets the creation through when the manager is full.
    pub flags: u32,
    /// Argument 4, the model key (`key3`). An all-zero key means the model
    /// comes from an exemplar chosen for the group.
    pub model_key: ResKey,
    /// Argument 5, the callback target (a host token); 0 = null.
    pub callback: u64,
    /// Argument 6, the priority; `0xFF` forces the creation.
    pub priority: u8,
    /// Argument 7, the orientation (`const vec3*`); `None` = null.
    pub orientation: Option<Vec3>,
}

/// One automata manager (`cISC4AutomataManager`, components 43–46).
pub struct AutomataManager {
    rec: ManagerRecord,
    paused: bool,
    loaded: bool,
    old_version: bool,
    hard_limit: i32,
    initialized: bool,
}

impl AutomataManager {
    pub fn new(kind: ManagerKind) -> AutomataManager {
        AutomataManager {
            rec: ManagerRecord::new(kind),
            paused: false,
            loaded: false,
            old_version: false,
            hard_limit: 0,
            initialized: false,
        }
    }

    pub fn kind(&self) -> ManagerKind {
        self.rec.kind
    }

    pub fn record(&self) -> &ManagerRecord {
        &self.rec
    }

    pub fn record_mut(&mut self) -> &mut ManagerRecord {
        &mut self.rec
    }

    /// Slot 3 `Init`: the saved max count is applied with `SetMaxCount`
    /// (§4 field 2).
    pub fn init(&mut self) -> bool {
        let n = self.rec.base.max_count;
        self.set_max_count(n);
        self.initialized = true;
        true
    }

    /// Slot 4 `Shutdown`.
    pub fn shutdown(&mut self) -> bool {
        self.initialized = false;
        true
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Slot 5 `Activate(bool)`.
    pub fn activate(&mut self, on: bool) {
        self.rec.base.active = on;
    }

    /// Slot 6 `IsActive`.
    pub fn is_active(&self) -> bool {
        self.rec.base.active
    }

    /// Slot 9 `Pause` (U-Drive-It, §9).
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Slot 10 `Resume`.
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Slot 11 `IsPaused`.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Slot 13 `IsDrivingMode`: true when the current view is the U-Drive-It
    /// view `0x2B709676` (§7.3).
    pub fn is_driving_mode(&self, current_view: u32) -> bool {
        current_view == U_DRIVE_IT_VIEW
    }

    /// Slot 12 `GetAutomataCount`.
    pub fn automata_count(&self) -> usize {
        self.rec.base.automata.len()
    }

    pub fn automata(&self) -> &[RefValue] {
        &self.rec.base.automata
    }

    /// Slot 7 `AddAutomaton`: appended; false when it is already listed.
    pub fn add_automaton(&mut self, a: RefValue) -> bool {
        if a.is_null() || self.rec.base.automata.iter().any(|x| same_ref(x, &a)) {
            return false;
        }
        self.rec.base.automata.push(a);
        true
    }

    /// Slot 8 `RemoveAutomaton`: false when it is not listed.
    pub fn remove_automaton(&mut self, a: &RefValue) -> bool {
        let before = self.rec.base.automata.len();
        self.rec.base.automata.retain(|x| !same_ref(x, a));
        self.rec.base.automata.len() != before
    }

    /// Slot 23 `SetMaxCount(n)` (§7.3): clamp to 0–5000, remove the automata
    /// beyond it (the newest first *[choice]*, AM4), set the hard limit to
    /// `trunc(n × 1.25)` (a `double` multiply). Returns the removed automata.
    pub fn set_max_count(&mut self, n: i32) -> Vec<RefValue> {
        let n = n.clamp(0, MAX_COUNT_LIMIT);
        self.rec.base.max_count = n;
        self.hard_limit = (f64::from(n) * 1.25) as i32;
        let keep = n as usize;
        if self.rec.base.automata.len() > keep {
            self.rec.base.automata.split_off(keep)
        } else {
            Vec::new()
        }
    }

    pub fn max_count(&self) -> i32 {
        self.rec.base.max_count
    }

    pub fn hard_limit(&self) -> i32 {
        self.hard_limit
    }

    /// Slot 15 `CreateAutomaton` (§7.2), stubbed: the pedestrian manager
    /// makes a pedestrian and the vehicle manager a vehicle at `position`,
    /// tagged with the group, and lists it; null when the manager is at its
    /// hard limit *[choice]* (AM4), and always for the aircraft and watercraft
    /// managers (their automata are Q4). How automata are chosen and spawned
    /// is outside this spec (§0).
    pub fn create_automaton(&mut self, args: CreateArgs) -> Option<Rc<Automaton>> {
        let rec = match self.rec.kind {
            ManagerKind::Pedestrian => AutomatonRecord::Pedestrian(PedestrianRecord::new()),
            ManagerKind::Vehicle => AutomatonRecord::Vehicle(VehicleRecord::new()),
            _ => return None,
        };
        if self.rec.base.automata.len() as i64 >= i64::from(self.hard_limit) {
            return None;
        }
        let a = Automaton::from_record(rec);
        a.set_group(args.group);
        a.set_position(args.position);
        self.rec.base.automata.push(RefValue::Object(a.clone()));
        Some(a)
    }

    /// Slot 31 `TrackAutomataGroupInstances(group, delta)`: the count moves by
    /// `delta` (not below 0 *[choice]*, AM12); a new group is appended.
    pub fn track_group_instances(&mut self, group: u32, delta: i32) {
        let counts = &mut self.rec.base.group_counts;
        match counts.iter_mut().find(|(g, _)| *g == group) {
            Some((_, n)) => *n = (i64::from(*n) + i64::from(delta)).max(0) as u32,
            None => counts.push((group, delta.max(0) as u32)),
        }
    }

    /// Slot 32 `GetInstancesOfGroup`.
    pub fn instances_of_group(&self, group: u32) -> u32 {
        self.rec
            .base
            .group_counts
            .iter()
            .find(|(g, _)| *g == group)
            .map(|&(_, n)| n)
            .unwrap_or(0)
    }

    /// The occupant-removed handler (§10): an occupant that leaves the city
    /// takes its airport pairs and its ferry entries with it (and the
    /// watercraft lists' entries that name it).
    pub fn occupant_removed(&mut self, occupant: &RefValue) {
        match &mut self.rec.part {
            ManagerPart::Aircraft { pairs, .. } => pairs.retain(|(o, _)| !same_ref(o, occupant)),
            ManagerPart::Watercraft {
                lots,
                occupants,
                ferries,
            } => {
                lots.retain(|(_, o)| !same_ref(o, occupant));
                occupants.retain(|o| !same_ref(o, occupant));
                ferries.retain(|f| !same_ref(&f.occupant, occupant));
            }
            ManagerPart::Traffic(_) => {}
        }
    }

    /// Set by Read (§4).
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Read saw a base version below 6 (§4).
    pub fn is_old_version(&self) -> bool {
        self.old_version
    }

    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        self.rec.write(out)
    }

    /// Read: on a rejected version or a short stream nothing changes and the
    /// result is false *[choice]* (see [`ManagerBase::read`]).
    pub fn read(&mut self, inp: &mut dyn InStream) -> bool {
        match ManagerRecord::read(self.rec.kind, inp) {
            Some(r) => {
                self.old_version = r.base.version < MANAGER_BASE_VERSION;
                self.loaded = true;
                self.rec = r;
                inp.error() == 0
            }
            None => false,
        }
    }
}

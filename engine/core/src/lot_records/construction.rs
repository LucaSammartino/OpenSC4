//! The construction occupant (`cSC4ConstructionOccupant`, class `0xA97F909E`):
//! save record (lot-records.md §9, §9.1) and the simulation-relevant life
//! cycle (lot-records §9.2, lot-developer.md §6.7: `Init`, `SetLot`,
//! `DoDailySim`, completion, `Shutdown`, the animation tick's crane state).
//!
//! Owned here: the record `Write` / `Read` with the version rules, the static
//! tuning (§6.7.2), `RebuildModel`'s geometry (§6.7.3), the timing values and
//! the `SetLot` order (§6.7.4), the daily state machine and completion
//! (§6.7.5), `SetBuildingCompletion`, the posted messages (§6.7.9), the draws
//! of `CreateLotTexture`, the spark count and scatter, and the crane's state
//! steps (§6.7.6, §6.7.7).
//!
//! Placement, resource tables and manager cleanup are owned here too.
//! Explicitly unspecified visual operations are listed in `Unsupported`;
//! see construction/README.md for the owner closure and open questions.

mod placement;
pub use placement::*;

use crate::automata::OccupantBase;
use crate::network_occupants::Xform;
use crate::props::round_half_even_f64;
use crate::rng::{CrtRand, RzRandom};
use crate::stream::{InStream, OutStream, RefValue};
use crate::variant::{mem, Elem, Variant};
use crate::x87::r24;

use super::{get_box, get_vec3, put_box, put_vec3, Bounds, OccupantRecord, IID_BUILDING_OCCUPANT, IID_LOT};

pub const CLSID_CONSTRUCTION_OCCUPANT: u32 = 0xA97F_909E;
/// §2.3 field 5.
pub const OCCUPANT_TYPE_CONSTRUCTION: u32 = 0x096E_763C;
/// The version `Write` writes (§9 #1).
pub const CONSTRUCTION_RECORD_VERSION: u16 = 5;
/// The oldest version `Read` accepts (§9).
pub const CONSTRUCTION_OLDEST_VERSION: u16 = 4;

/// Prop references (§9 #10, #11).
pub const IID_PROP: u32 = 0xE979_3A65;
/// Effect-prop references (§9 #12, #13).
pub const IID_EFFECT_PROP: u32 = 0x2977_AA49;
/// Crew automaton references (§9 #15).
pub const IID_AUTOMATON: u32 = 0xA9B4_0F05;

// ---- messages (lr §9.2, ld §6.7.1, §6.7.9) ---------------------------------------------------------

pub const MSG_SIM_NEW_DAY: u32 = 0x6695_6814;
pub const MSG_ANIMATION_TICK: u32 = 0x86AD_10EE;
pub const MSG_VISIBILITY_TEST: u32 = 0x06CD_B65B;
pub const MSG_MODEL_AVAILABLE: u32 = 0xA6B7_9602;
pub const MSG_VIEW_REFRESH: u32 = 0x26D3_1EC1;
/// Segment start, data1 = 1.
pub const MSG_SEGMENT_START: u32 = 0x49E7_B1E2;
/// Construction on (data1 = 1, `SetLot`) / off (data1 = 0, `Shutdown`).
pub const MSG_CONSTRUCTION: u32 = 0xC9E7_B1E8;
/// The building's completion changed (IGZUnknown = the building).
pub const MSG_COMPLETION_CHANGED: u32 = 0x2977_AE2F;

/// The four message-server subscriptions of `Init`, in lr §9.2 order
/// (answers OP15): the animation tick `0x86AD10EE` is **not** subscribed;
/// the tick manager calls `DoMessage` directly. `Shutdown` unsubscribes
/// the same four.
pub const SUBSCRIPTIONS: [u32; 4] = [
    MSG_SIM_NEW_DAY,
    MSG_MODEL_AVAILABLE,
    MSG_VISIBILITY_TEST,
    MSG_VIEW_REFRESH,
];

/// Occupant group property (§9 #2) and the group `Init` adds to it.
pub const PROP_OCCUPANT_GROUPS: u32 = 0xAA1D_D396;
pub const OCCUPANT_GROUP_CONSTRUCTION: u32 = 0x5007;
/// Posted (queued) by `AddOccupantGroup` when it appends a group.
pub const MSG_OCCUPANT_GROUP_ADDED: u32 = 0xEBFF_18A3;

/// `k` of `AnimateCrane`'s duration: f32 `0x3E22F983` = 1/(2π) (`0x00AA2098`).
pub const CRANE_K_BITS: u32 = 0x3E22_F983;

/// Animation-tick manager priority (ld §6.7.1): bucket 1, every second frame.
pub const ANIMATION_TICK_PRIORITY: u32 = 0x80;

/// Lot state of a completed construction (`lot.SetState(3, 0, 0)`, §6.7.5).
pub const LOT_STATE_GROWN: u32 = 3;

/// Base instance of the construction lot texture (§6.7.7 step 6).
pub const LOT_TEXTURE_BASE: u32 = 0xA990_C100;

// ---- record (§9, §9.1) ------------------------------------------------------------------------------

/// The 19 scalars of §9 #14 in Write order, with their lr §9.1 roles
/// (object-frame offsets; answers OP13/OP14). These are the occupant's live
/// life-cycle and crane fields: there is no separate unsaved copy, so a
/// loaded construction resumes from them (lr §9.2, last bullet).
///
/// `state` and `crane_state` are int32 in memory, written as their low byte
/// and read sign-extended. `a`, `b`, `c` hold the signed timing values
/// (ld §6.7.4) as their u32 bits.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ConstructionScalars {
    /// 14a `+0x140` daily state (ld §6.7.5, 0…4).
    pub state: i32,
    /// 14b `+0x144` `T0`, the sim day of `SetLot`.
    pub t0: u32,
    /// 14b `+0x148` unused: 0 from the constructor, only written and read.
    pub unused_148: u32,
    /// 14b `+0x14C` `nextDay`.
    pub next_day: u32,
    /// 14b `+0x150` `N` (segments, ≥ 1 once `RebuildModel` ran).
    pub segments: u32,
    /// 14b `+0x154` `counter` (current segment).
    pub counter: u32,
    /// 14c `+0x158` scaffold span `N · segH`.
    pub span: f32,
    /// 14c `+0x15C` `startY`.
    pub start_y: f32,
    /// 14d `+0x160…+0x16C` `A`, `B`, `C`, `E`.
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub e: u32,
    /// 14d `+0x170` `msAcc`.
    pub ms_acc: u32,
    /// 14e `+0x174` crane state (0 swing, 1 pause, 2 pick target).
    pub crane_state: i32,
    /// 14f `+0x178`, `+0x17C` crane swing limits min / max (radians).
    pub swing_min: f32,
    pub swing_max: f32,
    /// 14g `+0x180` unused: false from the constructor, only written and read.
    pub unused_180: bool,
    /// 14h `+0x184`, `+0x188` crane accumulator / duration (ms).
    pub crane_acc: u32,
    pub crane_duration: u32,
    /// 14i `+0x18C`, `+0x190` crane target / previous target angle.
    pub crane_target: f32,
    pub crane_previous: f32,
    /// 14j `+0x124` crane type (boom-radius table index).
    pub crane_type: u8,
}

/// One crew entry (§9 #15): two bytes and the automaton reference.
#[derive(Clone, Debug)]
pub struct CrewEntry {
    pub first: bool,
    pub second: bool,
    pub automaton: RefValue,
}

/// `cSC4ConstructionOccupant` record (§9).
#[derive(Clone, Debug)]
pub struct ConstructionRecord {
    pub base: OccupantBase,
    /// #3 / #4: the **live** lot and building references `SetLot` holds
    /// (`+0x68`/`+0x6C`, lr §9.1; there is no separate saved copy). `Read`
    /// replaces them.
    pub lot: RefValue,
    pub building: RefValue,
    /// #5.
    pub transform: Xform,
    /// #6 `+0xA8`: world bounds, the box `GetPosition` reads.
    pub world_bounds: Bounds,
    /// #7 `+0xC0`: the scaffold box `RebuildModel` stores (used by the
    /// crew, roof props and sparks).
    pub local_bounds: Bounds,
    /// `+0x104` != 0: a live crane model exists (`CreateCrane`). #8 is
    /// written from this; it is never read back into it.
    pub has_crane: bool,
    /// `+0xD8`: the load-only has-crane flag `Read` stores #8 into.
    pub loaded_has_crane: bool,
    /// `+0xD9`: refresh pending; `Read` sets it, the `0x26D31EC1` refresh
    /// clears it (lr §9.2).
    pub refresh_pending: bool,
    /// #9.
    pub crane_position: [f32; 3],
    /// #10, #11, #12.
    pub ground_props: Vec<RefValue>,
    pub roof_props: Vec<RefValue>,
    pub effects: Vec<RefValue>,
    /// #13.
    pub current_effect: RefValue,
    /// #14.
    pub scalars: ConstructionScalars,
    /// #15.
    pub crew: Vec<CrewEntry>,
}

fn put_ref_list(out: &mut dyn OutStream, list: &[RefValue]) {
    out.put_u32(list.len() as u32);
    for r in list {
        out.put_ref(r);
    }
}

/// `u32 n + n × ref`. The list is replaced, not appended to (the spec does
/// not say; literal reading of "count then refs"). No pre-allocation from n.
fn get_ref_list(inp: &mut dyn InStream, iid: u32) -> Option<Vec<RefValue>> {
    let n = inp.get_u32()?;
    let mut v = Vec::new();
    for _ in 0..n {
        v.push(inp.get_ref_as(iid)?);
    }
    Some(v)
}

impl ConstructionRecord {
    /// The result of the version check of `Read` (§9): `None` = rejected.
    /// Versions above 5 are read as 5 *[quirk]*.
    pub fn accepts(version: u16) -> Option<u16> {
        (version >= CONSTRUCTION_OLDEST_VERSION).then_some(version)
    }

    fn read_fields(&mut self, inp: &mut dyn InStream, version: u16) -> Option<bool> {
        self.lot = inp.get_ref_as(IID_LOT)?;
        self.building = inp.get_ref_as(IID_BUILDING_OCCUPANT)?;
        if version == CONSTRUCTION_OLDEST_VERSION {
            // Version 4: the fixed 56-byte legacy transform (`0x005DA930`,
            // NO15); the rest of the layout is the v5 one (lr §9).
            self.transform.legacy_load_into(inp)?;
        } else {
            self.transform.read_into(inp)?;
        }
        self.world_bounds = get_box(inp)?;
        self.local_bounds = get_box(inp)?;
        // #8 goes to the load-only flag `+0xD8`, never to the live crane
        // (`+0x104`); `Read` marks the refresh pending (`+0xD9`, lr §9.1).
        self.loaded_has_crane = inp.get_bool()?;
        self.refresh_pending = true;
        self.crane_position = get_vec3(inp)?;
        self.ground_props = get_ref_list(inp, IID_PROP)?;
        self.roof_props = get_ref_list(inp, IID_PROP)?;
        self.effects = get_ref_list(inp, IID_EFFECT_PROP)?;
        self.current_effect = inp.get_ref_as(IID_EFFECT_PROP)?;
        let s = &mut self.scalars;
        s.state = i32::from(inp.get_i8()?);
        s.t0 = inp.get_u32()?;
        s.unused_148 = inp.get_u32()?;
        s.next_day = inp.get_u32()?;
        s.segments = inp.get_u32()?;
        s.counter = inp.get_u32()?;
        s.span = inp.get_f32()?;
        s.start_y = inp.get_f32()?;
        s.a = inp.get_u32()?;
        s.b = inp.get_u32()?;
        s.c = inp.get_u32()?;
        s.e = inp.get_u32()?;
        s.ms_acc = inp.get_u32()?;
        s.crane_state = i32::from(inp.get_i8()?);
        s.swing_min = inp.get_f32()?;
        s.swing_max = inp.get_f32()?;
        s.unused_180 = inp.get_bool()?;
        s.crane_acc = inp.get_u32()?;
        s.crane_duration = inp.get_u32()?;
        s.crane_target = inp.get_f32()?;
        s.crane_previous = inp.get_f32()?;
        s.crane_type = inp.get_u8()?;
        let n = inp.get_u32()?;
        let mut crew = Vec::new();
        for _ in 0..n {
            let first = inp.get_bool()?;
            let second = inp.get_bool()?;
            let automaton = inp.get_ref_as(IID_AUTOMATON)?;
            crew.push(CrewEntry {
                first,
                second,
                automaton,
            });
        }
        self.crew = crew;
        Some(true)
    }
}

impl OccupantRecord for ConstructionRecord {
    const CLASS_ID: u32 = CLSID_CONSTRUCTION_OCCUPANT;
    fn new() -> Self {
        // Fields without a specified constructor default are 0 / null
        // [choice]; the base uses the occupant constructor (flags 4).
        ConstructionRecord {
            base: OccupantBase::new(OCCUPANT_TYPE_CONSTRUCTION),
            lot: RefValue::Null,
            building: RefValue::Null,
            transform: Xform::default(),
            world_bounds: ([0.0; 3], [0.0; 3]),
            local_bounds: ([0.0; 3], [0.0; 3]),
            has_crane: false,
            loaded_has_crane: false,
            refresh_pending: false,
            crane_position: [0.0; 3],
            ground_props: Vec::new(),
            roof_props: Vec::new(),
            effects: Vec::new(),
            current_effect: RefValue::Null,
            scalars: ConstructionScalars::default(),
            crew: Vec::new(),
        }
    }
    fn base(&self) -> &OccupantBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut OccupantBase {
        &mut self.base
    }
    fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u16(CONSTRUCTION_RECORD_VERSION);
        self.base.write(out);
        out.put_ref(&self.lot);
        out.put_ref(&self.building);
        self.transform.write(out);
        put_box(out, &self.world_bounds);
        put_box(out, &self.local_bounds);
        out.put_bool(self.has_crane);
        put_vec3(out, &self.crane_position);
        put_ref_list(out, &self.ground_props);
        put_ref_list(out, &self.roof_props);
        put_ref_list(out, &self.effects);
        out.put_ref(&self.current_effect);
        let s = &self.scalars;
        out.put_i8(s.state as i8);
        for v in [s.t0, s.unused_148, s.next_day, s.segments, s.counter] {
            out.put_u32(v);
        }
        out.put_f32(s.span);
        out.put_f32(s.start_y);
        for v in [s.a, s.b, s.c, s.e, s.ms_acc] {
            out.put_u32(v);
        }
        out.put_i8(s.crane_state as i8);
        out.put_f32(s.swing_min);
        out.put_f32(s.swing_max);
        out.put_bool(s.unused_180);
        out.put_u32(s.crane_acc);
        out.put_u32(s.crane_duration);
        out.put_f32(s.crane_target);
        out.put_f32(s.crane_previous);
        out.put_u8(s.crane_type);
        out.put_u32(self.crew.len() as u32);
        for c in &self.crew {
            out.put_bool(c.first);
            out.put_bool(c.second);
            out.put_ref(&c.automaton);
        }
        out.error() == 0
    }
    /// §9: < 4 → false before the base; 4 → the v5 layout with the 56-byte
    /// legacy transform (NO15); ≥ 5 read with the v5 layout.
    fn read(&mut self, inp: &mut dyn InStream) -> bool {
        let Some(v) = inp.get_u16() else { return false };
        let Some(version) = Self::accepts(v) else {
            return false;
        };
        match OccupantBase::read(inp) {
            Some(b) => self.base = b,
            None => return false,
        }
        match self.read_fields(inp, version) {
            Some(false) => false,
            _ => inp.error() == 0,
        }
    }
}

// ---- static tuning (ld §6.7.2) ----------------------------------------------------------------------

/// The "Construction" exemplar (type, group, instance).
pub const TUNING_EXEMPLAR: (u32, u32, u32) = (0x6534_284A, 0x2989_FD57, 0x8989_DBE3);

/// The static tuning `StaticInit` reads (ld §6.7.2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstructionTuning {
    pub segment_height: f32,
    pub post_width: f32,
    pub scaffold_gap: f32,
    pub plank_width: f32,
    /// Stored as `r24(1 / v)`; no code default in the spec (`None`).
    pub plank_texture_scale: Option<f32>,
    pub scaffold_texture_scale: Option<f32>,
    pub girder_texture_scale: Option<f32>,
    /// `grow`: `r24(1 / v)` when present, else 1.0.
    pub growth_rate: f32,
    /// `remove`: `r24(1 / v)` when present, else 2.0.
    pub removal_rate: f32,
    pub crane_post_width: f32,
    pub crane_post_gap: f32,
    pub crane_height_above_scaffold: f32,
    /// Milliseconds.
    pub crane_rotation_ms: f32,
}

impl Default for ConstructionTuning {
    /// The code defaults.
    fn default() -> Self {
        ConstructionTuning {
            segment_height: 3.0,
            post_width: 1.0,
            scaffold_gap: 1.0,
            plank_width: 2.0,
            plank_texture_scale: None,
            scaffold_texture_scale: None,
            girder_texture_scale: None,
            growth_rate: 1.0,
            removal_rate: 2.0,
            crane_post_width: 1.5,
            crane_post_gap: 0.5,
            crane_height_above_scaffold: 1.0,
            crane_rotation_ms: 6000.0,
        }
    }
}

impl ConstructionTuning {
    /// Reads the tuning from the exemplar's float properties; a missing
    /// property keeps the code default.
    pub fn from_properties(mut get: impl FnMut(u32) -> Option<f32>) -> Self {
        let mut t = ConstructionTuning::default();
        let inv = |v: f32| r24(1.0 / f64::from(v)) as f32;
        let mut set = |id: u32, slot: &mut f32| {
            if let Some(v) = get(id) {
                *slot = v;
            }
        };
        set(0x8989_EC22, &mut t.segment_height);
        set(0x8989_EC23, &mut t.post_width);
        set(0x8989_EC24, &mut t.scaffold_gap);
        set(0x8989_EC25, &mut t.plank_width);
        let mut tex = [None; 3];
        let mut rates = [None; 2];
        for (i, id) in [0x8989_EC26u32, 0x8989_EC27, 0x8989_EC28].into_iter().enumerate() {
            let mut v = f32::NAN;
            set(id, &mut v);
            tex[i] = (!v.is_nan()).then(|| inv(v));
        }
        for (i, id) in [0x8989_EC29u32, 0x8989_EC2A].into_iter().enumerate() {
            let mut v = f32::NAN;
            set(id, &mut v);
            rates[i] = (!v.is_nan()).then(|| inv(v));
        }
        set(0x8989_EC2B, &mut t.crane_post_width);
        set(0x8989_EC2C, &mut t.crane_post_gap);
        set(0x8989_EC2D, &mut t.crane_height_above_scaffold);
        set(0x8989_EC2E, &mut t.crane_rotation_ms);
        [t.plank_texture_scale, t.scaffold_texture_scale, t.girder_texture_scale] = tex;
        if let Some(g) = rates[0] {
            t.growth_rate = g;
        }
        if let Some(r) = rates[1] {
            t.removal_rate = r;
        }
        t
    }
}

/// The once-per-process statics of the class: the tuning and the file-local
/// generator plus the C `rand()` its iterators use (ld §6.7.7). The original
/// reseeds the generator from the wall clock in `StaticInit`; FreeCity4 seeds
/// it explicitly *[choice, ld §6.7.7 "Determinism"]*.
#[derive(Clone, Debug)]
pub struct ConstructionStatics {
    pub tuning: ConstructionTuning,
    pub rng: RzRandom,
    pub crt: CrtRand,
    pub cranes: [CraneSize; 3],
    pub ground: Vec<PropSize>,
    pub roof: Vec<PropSize>,
}

// ---- geometry and timing (ld §6.7.3, §6.7.4) --------------------------------------------------------

/// `RebuildModel`'s values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstructionGeometry {
    pub ground: f32,
    pub start_y: f32,
    pub top_y: f32,
    /// `f` before the floor.
    pub f: f32,
    /// N, the number of segments (≥ 1).
    pub segments: u32,
    pub scaffold: Bounds,
}

/// `floor` as the round idiom: round-half-even of `r24(f − 0.5)`, then +1
/// when `r + 1 = f`.
fn floor_idiom(f: f64) -> i64 {
    let r = round_half_even_f64(r24(f - 0.5));
    if r + 1.0 == f {
        (r + 1.0) as i64
    } else {
        r as i64
    }
}

/// ld §6.7.3. `altitude(x, z)` is terrain slot 13; a NaN altitude keeps the
/// current ground value. Corners are taken (min.x, min.z), (max.x, min.z),
/// (min.x, max.z), (max.x, max.z); the order does not change a minimum.
pub fn rebuild_model(
    bbox: &Bounds,
    tuning: &ConstructionTuning,
    mut altitude: impl FnMut(f32, f32) -> f32,
) -> ConstructionGeometry {
    let (mn, mx) = bbox;
    let mut ground = mn[1];
    for (x, z) in [(mn[0], mn[2]), (mx[0], mn[2]), (mn[0], mx[2]), (mx[0], mx[2])] {
        let a = altitude(x, z);
        if a < ground {
            ground = a;
        }
    }
    let start_y = r24(f64::from(mn[1]) + 0.1) as f32;
    let top_y = mx[1];
    let f = r24(r24(f64::from(top_y) - f64::from(start_y)) / f64::from(tuning.segment_height));
    let mut n = floor_idiom(f) + 1;
    if n == 0 {
        n = 1;
    }
    let g = tuning.scaffold_gap;
    ConstructionGeometry {
        ground,
        start_y,
        top_y,
        f: f as f32,
        segments: n as u32,
        scaffold: ([mn[0] - g, ground, mn[2] - g], [mx[0] + g, top_y, mx[2] + g]),
    }
}

/// The timing values of ld §6.7.4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstructionTiming {
    pub d: u32,
    pub a: i64,
    pub b: i64,
    pub c: i64,
    pub e: u32,
}

/// `min(round(r24(x · rate)), 1)` *[quirk: min, not max]*.
fn phase_days(x: f64, rate: f32) -> i64 {
    (round_half_even_f64(r24(x * f64::from(rate))) as i64).min(1)
}

/// ld §6.7.4. `construction_days` is the building's property `0x499AFA38`:
/// `None` when the property is absent (D = 30); a null BDS reads a zero
/// profile, which the caller passes as `Some(0)` (D = 10).
pub fn timing(
    geom: &ConstructionGeometry,
    tuning: &ConstructionTuning,
    construction_days: Option<u8>,
) -> ConstructionTiming {
    let d = match construction_days {
        None => 30,
        Some(0) => 10,
        Some(v) => u32::from(v),
    };
    let a = phase_days(
        r24(f64::from(geom.start_y) - f64::from(geom.ground)),
        tuning.growth_rate,
    );
    let b = phase_days(f64::from(tuning.segment_height), tuning.growth_rate);
    let c = phase_days(r24(f64::from(geom.top_y) - f64::from(geom.ground)), tuning.removal_rate);
    let n = i64::from(geom.segments);
    // max_signed on 32-bit values, then an unsigned division.
    let rest = (i64::from(d) - b * n - a - c) as i32;
    let m = rest.max(n as i32) as u32;
    ConstructionTiming {
        d,
        a,
        b,
        c,
        e: m / geom.segments,
    }
}

// ---- draws (ld §6.7.7) ------------------------------------------------------------------------------

/// `CreateSparks`: 1, 2 or 3 sparks for `ext < 20`, `< 30`, otherwise, with
/// `ext := max(xmax − xmin, zmax − zmin)` of the scaffold box (record #7);
/// y is excluded (ld §6.7.7, answers OP16).
pub fn spark_count(bbox: &Bounds) -> usize {
    let e = f32::max(bbox.1[0] - bbox.0[0], bbox.1[2] - bbox.0[2]);
    if e < 20.0 {
        1
    } else if e < 30.0 {
        2
    } else {
        3
    }
}

/// A linear iterator reset in table mode: 0..n − 1 shuffled with `n − 1`
/// C `rand()` calls.
pub fn reset_iterator(crt: &mut CrtRand, n: usize) -> Vec<usize> {
    let mut t: Vec<usize> = (0..n).collect();
    crt.random_shuffle(&mut t);
    t
}

/// The spark scatter of state 1 → 2: the side iterator over 4 sides is
/// reset, then per spark one side and its position (x / z first, then y).
pub fn scatter_sparks(st: &mut ConstructionStatics, bbox: &Bounds, sparks: usize) -> Vec<[f32; 3]> {
    let sides = reset_iterator(&mut st.crt, 4);
    let ([x0, y0, z0], [x1, y1, z1]) = *bbox;
    let mut gf = |a: f32, b: f32| st.rng.range_gaussian_fast(f64::from(a), f64::from(b)) as f32;
    (0..sparks)
        .map(|i| {
            // More than 4 sparks cannot happen (at most 3); the iterator
            // wraps literally by index.
            let (x, z) = match sides[i % 4] {
                0 => (gf(x0, x1), z0),
                1 => (x1, gf(z0, z1)),
                2 => (gf(x0, x1), z1),
                _ => (x0, gf(z0, z1)),
            };
            [x, gf(y0, y1), z]
        })
        .collect()
}

/// `CreateLotTexture`: x outer, z inner over the lot's cell rect
/// `(x1, z1, x2, z2)` inclusive; `usable(x, z)` = inside the city and not
/// water. Returns `(x, z, texture instance)` in draw order.
pub fn lot_texture(
    rng: &mut RzRandom,
    rect: [i32; 4],
    mut usable: impl FnMut(i32, i32) -> bool,
) -> Vec<(i32, i32, u32)> {
    let mut out = Vec::new();
    for x in rect[0]..=rect[2] {
        for z in rect[1]..=rect[3] {
            if usable(x, z) {
                out.push((x, z, LOT_TEXTURE_BASE + 0x100 * rng.uniform_u32(4)));
            }
        }
    }
    out
}

// ---- the crane's animation state (ld §6.7.6) --------------------------------------------------------

/// What `AttemptAddCrane` produced when it created a crane model. The fit
/// itself (sizes, sides, `DivideLot` areas, `asin`) is core geometry; the
/// core stores the results in the record: type → #14j `+0x124`, position →
/// #9 `+0x108`, swing limits `[c − h, c + h]` → #14f `+0x178`/`+0x17C`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CraneFit {
    pub crane_type: u8,
    pub position: [f32; 3],
    pub swing_min: f32,
    pub swing_max: f32,
}

/// `duration := round_f32(T · |Δ| · k)` (`0x005CD645…0x005CD669`): the x87
/// product `T · |Δ|`, times `k` (f32 `0x3E22F983`), plus 12582912.0 stored
/// as f32, then the bits minus `0x4B400000` (round half to even). The x87
/// extended product is taken in f64 here.
pub fn crane_duration(t: f32, delta: f64) -> u32 {
    let k = f64::from(f32::from_bits(CRANE_K_BITS));
    let q = f64::from(t) * delta * k;
    ((q + 12_582_912.0) as f32).to_bits().wrapping_sub(0x4B40_0000)
}

/// `AnimateCrane(ms)` (`0x005CD570`) on the record's crane fields (#14e,
/// #14f, #14h, #14i). `t` is the tunable sfCraneTimeOfFullRotation.
pub fn animate_crane(s: &mut ConstructionScalars, ms: u32, rng: &mut RzRandom, t: f32) {
    s.crane_acc = s.crane_acc.wrapping_add(ms);
    if s.crane_acc < s.crane_duration {
        return;
    }
    s.crane_acc = 0;
    match s.crane_state {
        0 => {
            s.crane_state = 1;
            s.crane_duration = 1000 + rng.uniform_u32(1000);
        }
        // 1 → 2 without a draw; the duration drawn by 0 → 1 stays.
        1 => s.crane_state = 2,
        2 => {
            let (mn, mx) = (f64::from(s.swing_min), f64::from(s.swing_max));
            let mid = r24(r24(r24(mx - mn) * 0.5) + mn);
            let old = s.crane_target;
            let new = if f64::from(old) < mid {
                rng.range_f64(mid, mx)
            } else {
                rng.range_f64(mn, mid)
            } as f32;
            s.crane_previous = old;
            s.crane_target = new;
            let delta = (f64::from(new) - f64::from(s.crane_previous)).abs();
            s.crane_duration = crane_duration(t, delta);
            s.crane_state = 0;
        }
        // ld §6.7.6 names states 0…2 only; another (loaded) value is left
        // as is [literal].
        _ => {}
    }
}

// ---- the life cycle ---------------------------------------------------------------------------------

/// The services the construction occupant calls (all through the game's
/// interfaces in the original). Visual steps the spec leaves open are
/// delegated whole.
pub trait ConstructionHost {
    /// The occupant base `Init` (`0x005E8C90`, lr §2.4), run unconditionally
    /// by `Init` once its own `IsInitialized` is false.
    fn occupant_base_init(&mut self);
    /// Message server slot 0x14 / its unsubscribe (target = the `+4`
    /// message-target sub-object).
    fn subscribe(&mut self, msg: u32);
    fn unsubscribe(&mut self, msg: u32);
    /// Animation-tick manager `AddTickableObject(this, priority)` / removal.
    fn add_tickable(&mut self, priority: u32);
    fn remove_tickable(&mut self);
    /// Occupant slot 15: posts `0xEBFF18A3` queued with the occupant and the
    /// group ID (`AddOccupantGroup`, lr §9.2).
    fn post_occupant_group_added(&mut self, group: u32);
    /// `SendConstructionMessage(type, data1)`: queued, IGZUnknown = this.
    fn post_construction_message(&mut self, msg: u32, data1: u8);
    /// Simulator slot 8 `GetSimDateNumber`.
    fn sim_date(&mut self) -> u32;
    /// Lot slot 48 `GetBuilding` (null → `None`).
    fn lot_building(&mut self, lot: &RefValue) -> Option<RefValue>;
    /// The building occupant's bounding box (occupant slot 10).
    fn building_bounds(&mut self, building: &RefValue) -> Bounds;
    /// Terrain slot 13.
    fn altitude(&mut self, x: f32, z: f32) -> f32;
    /// Building property `0x499AFA38` (see [`timing`]).
    fn construction_days(&mut self, building: &RefValue) -> Option<u8>;
    /// Building slots 9 / 10.
    fn building_completion(&mut self, building: &RefValue) -> f32;
    fn set_building_completion(&mut self, building: &RefValue, p: f32);
    /// Posts `0x2977AE2F` queued with IGZUnknown = the building.
    fn post_completion_changed(&mut self, building: &RefValue);
    /// Lot slot 34 `SetState(state, a, b)`.
    fn set_lot_state(&mut self, lot: &RefValue, state: u32, a: u32, b: u32);
    /// Lot slot 14: the lot's cell rect x1, z1, x2, z2.
    fn lot_cell_rect(&mut self, lot: &RefValue) -> [i32; 4];
    /// Inside the city and not water (terrain slot 34).
    fn cell_usable(&mut self, x: i32, z: i32) -> bool;
    /// One construction lot-texture cell.
    fn set_lot_texture(&mut self, x: i32, z: i32, instance: u32);
    /// City slot 96; conversion remains a foreign city service.
    fn cell_corner(&mut self, x: i32, z: i32) -> [f32; 3];
    /// Building uint8 property 0xAA83558F; absent is zero.
    fn crane_policy(&mut self, building: &RefValue) -> u8;
    fn create_prop(&mut self, type_id: u32) -> Option<RefValue>;
    fn place_prop(&mut self, prop: &RefValue, position: [f32; 3], orientation: u8);
    fn render_detail(&mut self) -> u32;
    fn create_automaton(&mut self, type_id: u32, position: [f32; 3], roof: bool) -> Option<RefValue>;
    fn worker_direction(&mut self, automaton: &RefValue, direction: [f32; 2]);
    /// Foreign visual-effect creation; its parameters are unspecified (LD38).
    fn create_dust_clouds(&mut self);
    /// `CreateSparks`: create `count` spark effects; their references form
    /// the effect vector (record #12).
    fn create_sparks(&mut self, count: usize) -> Vec<RefValue>;
    /// `FUN_005CBE10(angle)`: poses the crane boom (only that is specified).
    fn pose_crane(&mut self, angle: f32);
    /// `CreateCrane(x, z, type)` of the refresh after a load; true when the
    /// crane model exists afterwards (`+0x104` != 0).
    fn create_crane(&mut self, x: f32, z: f32, crane_type: u8) -> bool;
    /// Spark effect `i`: slot 8(v).
    fn spark_slot8(&mut self, i: usize, v: u32);
    /// Spark effect `i`: slot 9(&pos) then slot 7(1).
    fn place_spark(&mut self, i: usize, pos: [f32; 3]);
    /// State 0/2 with `counter > N`: roof props removed (prop manager slot
    /// 29), the scaffold switched to removal.
    fn start_scaffold_removal(&mut self);
    fn occupant_base_shutdown(&mut self);
    /// Release texture bindings, free model buffers, then release crane model.
    fn release_texture_bindings(&mut self);
    fn free_model_buffers(&mut self);
    fn release_crane_model(&mut self);
    /// Prop manager slot 29; invoked before the owner drops its reference.
    fn remove_prop(&mut self, prop: &RefValue);
    /// Automata manager slot 8, second argument zero.
    fn remove_automaton(&mut self, automaton: &RefValue);
    /// Framework grid acquisition and render-state packing; IDs not specified (LD38).
    fn prepare_render_state(&mut self);
    /// Foreign visibility-test object and configuration resolved by the host.
    fn visibility_result(&mut self) -> bool;
    /// Dwords at +4 of the message slot 10 and 15 objects.
    fn model_message_keys(&mut self) -> (u32, u32);
    /// Renderer slots 28 / 29.
    fn add_to_renderer(&mut self);
    fn remove_from_renderer(&mut self);
    /// Occupant manager slot 21 `RemoveOccupant(this + 8, 0, 0)`.
    fn remove_from_occupant_manager(&mut self);
}

/// Daily state (ld §6.7.5).
pub const STATE_DONE: i32 = 4;

/// The live construction occupant. The saved fields are the live ones
/// (lr §9.1): lot / building (#3/#4), boxes (#6/#7), the crane flag and
/// position, the effect vector and every life-cycle value (#14) live in
/// [`ConstructionRecord`], so a loaded occupant resumes its schedule. Only
/// `RebuildModel`'s unsaved values (`ground`, `topY`) and the renderer bit
/// are kept here.
#[derive(Clone, Debug)]
pub struct ConstructionOccupant {
    pub rec: ConstructionRecord,
    /// The occupant's own `IsInitialized` (occupant slot 5).
    pub initialised: bool,
    pub geometry: Option<ConstructionGeometry>,
    pub scaffold_height: f32,
    /// Byte +0x37 bit 0x02, "in the renderer".
    pub in_renderer: bool,
    pub areas: [Bounds; 8],
    pub model_dirty: bool,
    pub unsupported: Vec<Unsupported>,
}

impl Default for ConstructionOccupant {
    fn default() -> Self {
        Self::from_record(ConstructionRecord::new())
    }
}

impl ConstructionOccupant {
    /// An occupant around a (possibly loaded) record; `Init` has not run.
    pub fn from_record(rec: ConstructionRecord) -> Self {
        ConstructionOccupant {
            rec,
            initialised: false,
            geometry: None,
            scaffold_height: 0.0,
            in_renderer: false,
            areas: [([0.0; 3], [0.0; 3]); 8],
            model_dirty: false,
            unsupported: vec![
                Unsupported::ModelGeometry,
                Unsupported::ScaffoldHeight,
                Unsupported::Draw,
                Unsupported::RotationChange,
                Unsupported::PhaseInterpolation,
            ],
        }
    }

    /// `Init` (`0x005D1300`, lr §9.2): true at once when already
    /// initialised; the occupant base `Init` unconditionally; `StaticInit` is
    /// the caller's [`ConstructionStatics`]; the four subscriptions; the tick
    /// at `0x80`; last `AddOccupantGroup(0x5007)`. The 6 × 5 sim-grid slots
    /// and the render-state pack are not used by anything specified here.
    pub fn init(&mut self, host: &mut dyn ConstructionHost) -> bool {
        if self.initialised {
            return true;
        }
        host.occupant_base_init();
        // Literal: the base Init is what makes IsInitialized true (lr §2.4).
        self.initialised = true;
        for m in SUBSCRIPTIONS {
            host.subscribe(m);
        }
        host.add_tickable(ANIMATION_TICK_PRIORITY);
        host.prepare_render_state();
        self.add_occupant_group(host, OCCUPANT_GROUP_CONSTRUCTION);
        true
    }

    /// `AddOccupantGroup(group)` (occupant slot 23, `0x005E9350`): appends
    /// `group` to the u32-array property `0xAA1DD396` when absent, writes the
    /// property back and posts `0xEBFF18A3`; otherwise nothing. Returns
    /// whether it appended (the caller ignores it). An absent property reads
    /// as an empty array, and the write goes to the occupant's own holder
    /// [literal].
    pub fn add_occupant_group(&mut self, host: &mut dyn ConstructionHost, group: u32) -> bool {
        let holder = &mut self.rec.base.holder;
        let mut items = holder.get(PROP_OCCUPANT_GROUPS).map(|v| v.items()).unwrap_or_default();
        if items.iter().any(|e| e.as_i64() as u32 == group) {
            return false;
        }
        items.push(Elem::U32(group));
        holder.own.add(PROP_OCCUPANT_GROUPS, Variant::array(mem::UINT32, items));
        host.post_occupant_group_added(group);
        true
    }

    /// `SetBuildingCompletion(p)` (`0x005CA8D0`).
    pub fn set_building_completion(&mut self, host: &mut dyn ConstructionHost, p: f32) {
        let b = self.rec.building.clone();
        if !b.is_null() && host.building_completion(&b) != p {
            host.set_building_completion(&b, p);
            host.post_completion_changed(&b);
        }
    }

    /// `RebuildModel` (ld §6.7.3) into the record: scaffold box #7, `N`,
    /// span `r24(N · segH)`, `startY`, world box widened by post width (OP17).
    fn rebuild_model(&mut self, host: &mut dyn ConstructionHost, st: &ConstructionStatics) -> ConstructionGeometry {
        let bbox = host.building_bounds(&self.rec.building);
        let geom = rebuild_model(&bbox, &st.tuning, |x, z| host.altitude(x, z));
        let (mut lo, mut hi) = geom.scaffold;
        for axis in [0, 2] {
            lo[axis] -= st.tuning.post_width;
            hi[axis] += st.tuning.post_width;
        }
        self.rec.world_bounds = (lo, hi);
        self.rec.local_bounds = geom.scaffold;
        let s = &mut self.rec.scalars;
        s.segments = geom.segments;
        s.span = r24(f64::from(geom.segments) * f64::from(st.tuning.segment_height)) as f32;
        s.start_y = geom.start_y;
        self.geometry = Some(geom);
        geom
    }

    /// `SetLot(lot)` (ld §6.7.4, lr §9.2).
    pub fn set_lot(&mut self, host: &mut dyn ConstructionHost, st: &mut ConstructionStatics, lot: RefValue) {
        self.init(host);
        self.rec.building = RefValue::Null;
        self.rec.lot = lot;
        let Some(b) = host.lot_building(&self.rec.lot) else {
            return;
        };
        self.rec.building = b.clone();
        let geom = self.rebuild_model(host, st);
        let rect = host.lot_cell_rect(&self.rec.lot);
        let world = (
            host.cell_corner(rect[0], rect[1]),
            host.cell_corner(rect[2].wrapping_add(1), rect[3].wrapping_add(1)),
        );
        let bbox = host.building_bounds(&b);
        self.areas = divide_lot(world, bbox, st.tuning.scaffold_gap);
        self.attempt_crane(host, st, bbox);
        self.ground_props(host, st);
        self.crew(host, st);
        self.roof_props(host, st);
        host.create_dust_clouds();
        self.rec.effects = host.create_sparks(spark_count(&geom.scaffold));
        let rect = host.lot_cell_rect(&self.rec.lot);
        for (x, z, inst) in lot_texture(&mut st.rng, rect, |x, z| host.cell_usable(x, z)) {
            host.set_lot_texture(x, z, inst);
        }
        let t0 = host.sim_date();
        let t = timing(&geom, &st.tuning, host.construction_days(&b));
        let s = &mut self.rec.scalars;
        s.t0 = t0;
        (s.a, s.b, s.c, s.e) = (t.a as i32 as u32, t.b as i32 as u32, t.c as i32 as u32, t.e);
        s.state = 0;
        s.counter = 0;
        s.ms_acc = 0;
        s.next_day = t0.wrapping_add(s.a);
        self.set_building_completion(host, 0.0);
        self.scaffold_height = r24(f64::from(geom.ground) + 0.001) as f32;
        if self.rec.has_crane {
            let s = &mut self.rec.scalars;
            s.crane_target = s.swing_min;
            s.crane_previous = s.swing_max;
            s.crane_acc = 1;
            s.crane_duration = 0;
            s.crane_state = 2;
            host.pose_crane(crane_model_angle(s.swing_min));
        }
        host.post_construction_message(MSG_SEGMENT_START, 1);
        host.post_construction_message(MSG_CONSTRUCTION, 1);
        host.add_to_renderer();
        self.in_renderer = true;
        self.rec.base.flags |= 2;
    }

    /// `DoDailySim` (ld §6.7.5); the date comes from the simulator, never
    /// from the message. Reads only `state`, `nextDay`, `counter`, `N`, `B`,
    /// `C`, `E` and the saved scaffold box / effects, so a loaded occupant
    /// resumes without a recomputation (lr §9.2).
    pub fn do_daily_sim(&mut self, host: &mut dyn ConstructionHost, st: &mut ConstructionStatics) {
        if self.rec.building.is_null() || self.rec.scalars.state == STATE_DONE {
            return;
        }
        let today = host.sim_date();
        let s = &mut self.rec.scalars;
        if today < s.next_day {
            return;
        }
        s.ms_acc = 0;
        let sparks = self.rec.effects.len();
        match s.state {
            0 | 2 => {
                s.counter = s.counter.wrapping_add(1);
                let removal = s.counter > s.segments;
                if removal {
                    for prop in self.rec.roof_props.drain(..) {
                        host.remove_prop(&prop);
                    }
                    host.start_scaffold_removal();
                } else {
                    host.post_construction_message(MSG_SEGMENT_START, 1);
                }
                for i in 0..sparks {
                    host.spark_slot8(i, 1);
                }
                let s = &mut self.rec.scalars;
                if removal {
                    s.state = 3;
                    s.next_day = today.wrapping_add(s.c);
                } else {
                    s.state = 1;
                    s.next_day = today.wrapping_add(s.b);
                }
            }
            1 => {
                s.state = 2;
                s.next_day = today.wrapping_add(s.e);
                let scaffold = self.rec.local_bounds;
                for (i, p) in scatter_sparks(st, &scaffold, sparks).into_iter().enumerate() {
                    host.place_spark(i, p);
                }
            }
            3 => self.complete(host),
            // ld §6.7.5 lists 0…4 only; another (loaded) value does nothing
            // [literal].
            _ => {}
        }
    }

    /// Completion (ld §6.7.5), then `Shutdown`.
    fn complete(&mut self, host: &mut dyn ConstructionHost) {
        self.rec.scalars.state = STATE_DONE;
        self.set_building_completion(host, 1.0);
        if self.in_renderer {
            host.remove_from_renderer();
            self.in_renderer = false;
            self.rec.base.flags &= !2;
        }
        host.remove_from_occupant_manager();
        let lot = self.rec.lot.clone();
        host.set_lot_state(&lot, LOT_STATE_GROWN, 0, 0);
        self.shutdown(host);
    }

    /// `Shutdown` (`0x005D0CE0`): posts `0xC9E7B1E8` (0) first, unsubscribes
    /// the four, unregisters the tick, releases the lot, building, props,
    /// effects and crew.
    pub fn shutdown(&mut self, host: &mut dyn ConstructionHost) {
        if !self.initialised {
            return;
        }
        host.post_construction_message(MSG_CONSTRUCTION, 0);
        host.occupant_base_shutdown();
        self.initialised = false;
        self.rec.lot = RefValue::Null;
        self.rec.building = RefValue::Null;
        for m in SUBSCRIPTIONS {
            host.unsubscribe(m);
        }
        host.remove_tickable();
        host.release_texture_bindings();
        host.free_model_buffers();
        host.release_crane_model();
        self.rec.has_crane = false;
        for prop in self
            .rec
            .ground_props
            .drain(..)
            .chain(self.rec.roof_props.drain(..))
            .chain(self.rec.effects.drain(..))
        {
            host.remove_prop(&prop);
        }
        // Literal unconditional current-effect removal, including null (§6.7.14).
        host.remove_prop(&self.rec.current_effect);
        self.rec.current_effect = RefValue::Null;
        for crew in self.rec.crew.drain(..) {
            host.remove_automaton(&crew.automaton);
        }
    }

    /// `DoAnimationTick(ms)`: accumulates `msAcc` and runs `AnimateCrane` in
    /// states 0–2 when a crane model exists. The scaffold / completion
    /// interpolation is visual and not specified to the formula; it is not
    /// done here.
    pub fn do_animation_tick(&mut self, st: &mut ConstructionStatics, ms: u32) {
        let s = &mut self.rec.scalars;
        s.ms_acc = s.ms_acc.wrapping_add(ms);
        if self.rec.has_crane && (0..=2).contains(&s.state) {
            animate_crane(s, ms, &mut st.rng, st.tuning.crane_rotation_ms);
        }
    }

    /// The `0x26D31EC1` refresh after a load (lr §9.2): only when `Read` set
    /// the pending flag. `RebuildModel`; `CreateCrane` from the saved
    /// position x / z and type when the load-only flag (#8) is set;
    /// `AddToRenderer`; `DoAnimationTick(0)` with state 2 shown as 1 around
    /// the call; clear the flag. The crane model exists only afterwards.
    pub fn refresh(&mut self, host: &mut dyn ConstructionHost, st: &mut ConstructionStatics) {
        if !self.rec.refresh_pending {
            return;
        }
        self.rebuild_model(host, st);
        if self.rec.loaded_has_crane {
            let p = self.rec.crane_position;
            self.rec.has_crane = host.create_crane(p[0], p[2], self.rec.scalars.crane_type);
        }
        host.add_to_renderer();
        self.in_renderer = true;
        let was2 = self.rec.scalars.state == 2;
        self.rec.base.flags |= 2;
        if was2 {
            self.rec.scalars.state = 1;
        }
        self.animation_tick(host, st, 0);
        if was2 {
            self.rec.scalars.state = 2;
        }
        self.rec.refresh_pending = false;
    }

    /// `DoMessage` (object `+4` frame): SimNewDay → `DoDailySim` (data1
    /// unused); the tick (delivered directly by the tick manager) with
    /// `ms := data2 / 1000`, unsigned and truncating; `0x26D31EC1` → the
    /// refresh; visibility and model messages resolve foreign message objects
    /// through the host and perform owner state changes. Returns true for all.
    pub fn do_message(
        &mut self,
        host: &mut dyn ConstructionHost,
        st: &mut ConstructionStatics,
        msg: u32,
        data2: u32,
    ) -> bool {
        match msg {
            MSG_SIM_NEW_DAY => self.do_daily_sim(host, st),
            MSG_ANIMATION_TICK => self.animation_tick(host, st, data2 / 1000),
            MSG_VIEW_REFRESH => self.refresh(host, st),
            MSG_VISIBILITY_TEST => {
                let visible = host.visibility_result();
                self.visibility_test(host, visible);
            }
            MSG_MODEL_AVAILABLE => {
                let (first, second) = host.model_message_keys();
                self.model_available(first, second);
            }
            _ => {}
        }
        true
    }

    /// `GetPosition`: the saved world box (#6) centre, `(min + max) * 0.5f`
    /// per axis.
    pub fn position(&self) -> [f32; 3] {
        let (a, b) = self.rec.world_bounds;
        [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5, (a[2] + b[2]) * 0.5]
    }

    pub fn bounding_box(&self) -> Bounds {
        self.rec.world_bounds
    }

    /// Resolved visibility test from the foreign renderer (§9.2).
    pub fn visibility_test(&mut self, host: &mut dyn ConstructionHost, visible: bool) {
        if visible != self.in_renderer {
            if visible {
                host.add_to_renderer();
            } else {
                host.remove_from_renderer();
            }
            self.in_renderer = visible;
        }
        if visible {
            self.rec.base.flags |= 2;
        } else {
            self.rec.base.flags &= !2;
        }
    }

    /// Message slots 10/15 return objects; their +4 dwords are compared.
    pub fn model_available(&mut self, first: u32, second: u32) {
        if first != second {
            self.model_dirty = true;
        }
    }

    /// Phase interpolation is open (LD36); crane smoothstep is settled.
    pub fn animation_tick(&mut self, host: &mut dyn ConstructionHost, st: &mut ConstructionStatics, ms: u32) {
        self.do_animation_tick(st, ms);
        let s = self.rec.scalars;
        if self.rec.has_crane && (0..=2).contains(&s.state) && s.crane_state == 0 {
            let t = s.crane_acc as f32 / s.crane_duration as f32;
            let ease = t * t * (3.0 - 2.0 * t);
            let angle = s.crane_previous + (s.crane_target - s.crane_previous) * ease;
            host.pose_crane(crane_model_angle(angle));
        }
    }
}

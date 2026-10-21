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
//! Not owned (host callbacks, the spec leaves them open, §6.7.10): the crane
//! fit geometry (`AttemptAddCrane`), `DivideLot`, ground / roof props, the
//! construction crew, dust clouds, the scaffold model and the animation's
//! scaffold / completion interpolation. Questions OP13–OP16 cover the gaps
//! that touch this module (`docs/specs/questions/occupants-props.md`).

use crate::automata::OccupantBase;
use crate::network_occupants::Xform;
use crate::props::round_half_even_f64;
use crate::rng::{CrtRand, RzRandom};
use crate::stream::{InStream, OutStream, RefValue};
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

/// The message-server subscriptions of `Init`, in the order lr §9.2 lists
/// them. ld §6.7.1 lists four (without the tick, which the tick manager
/// delivers directly); the core follows lr's five, which `Shutdown`
/// unsubscribes ("all five") — question OP15.
pub const SUBSCRIPTIONS: [u32; 5] = [
    MSG_SIM_NEW_DAY,
    MSG_ANIMATION_TICK,
    MSG_VISIBILITY_TEST,
    MSG_MODEL_AVAILABLE,
    MSG_VIEW_REFRESH,
];

/// Animation-tick manager priority (ld §6.7.1): bucket 1, every second frame.
pub const ANIMATION_TICK_PRIORITY: u32 = 0x80;

/// Lot state of a completed construction (`lot.SetState(3, 0, 0)`, §6.7.5).
pub const LOT_STATE_GROWN: u32 = 3;

/// Base instance of the construction lot texture (§6.7.7 step 6).
pub const LOT_TEXTURE_BASE: u32 = 0xA990_C100;

// ---- record (§9, §9.1) ------------------------------------------------------------------------------

/// The 19 unnamed scalars of §9 #14, in Write order (§9.1 offsets). Their
/// roles are only partly known (§16 Q3); they are kept as data. The `u32`
/// groups are written as the stored bits, so a field the life cycle treats
/// as f32 (§9.2 `+0x158`/`+0x15C`) round-trips unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ConstructionScalars {
    /// 14a `+0x134` s8.
    pub effect_index: i8,
    /// 14b `+0x138…+0x148`.
    pub progress: [u32; 5],
    /// 14c `+0x14C`, `+0x150`.
    pub segment: [f32; 2],
    /// 14d `+0x154…+0x164`.
    pub placement: [u32; 5],
    /// 14e `+0x168` s8.
    pub flag: i8,
    /// 14f `+0x16C`, `+0x170`.
    pub crane_interp: [f32; 2],
    /// 14g `+0x174` bool.
    pub crane_flag: bool,
    /// 14h `+0x178`, `+0x17C`.
    pub crane_swing: [u32; 2],
    /// 14i `+0x180`, `+0x184`.
    pub crane_phase: [f32; 2],
    /// 14j `+0x118` u8.
    pub tail: u8,
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
    /// #3 / #4: the **saved** lot and building references (`+0x5C`/`+0x60`);
    /// the live copies (`+0x68`/`+0x6C`) are [`ConstructionOccupant::lot`] /
    /// `building`. Which code fills the saved pair is not specified (OP14).
    pub lot: RefValue,
    pub building: RefValue,
    /// #5.
    pub transform: Xform,
    /// #6, #7.
    pub world_bounds: Bounds,
    pub local_bounds: Bounds,
    /// #8: written from the live "crane model exists" state.
    pub has_crane: bool,
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
        self.has_crane = inp.get_bool()?;
        self.crane_position = get_vec3(inp)?;
        self.ground_props = get_ref_list(inp, IID_PROP)?;
        self.roof_props = get_ref_list(inp, IID_PROP)?;
        self.effects = get_ref_list(inp, IID_EFFECT_PROP)?;
        self.current_effect = inp.get_ref_as(IID_EFFECT_PROP)?;
        let s = &mut self.scalars;
        s.effect_index = inp.get_i8()?;
        for v in &mut s.progress {
            *v = inp.get_u32()?;
        }
        for v in &mut s.segment {
            *v = inp.get_f32()?;
        }
        for v in &mut s.placement {
            *v = inp.get_u32()?;
        }
        s.flag = inp.get_i8()?;
        for v in &mut s.crane_interp {
            *v = inp.get_f32()?;
        }
        s.crane_flag = inp.get_bool()?;
        for v in &mut s.crane_swing {
            *v = inp.get_u32()?;
        }
        for v in &mut s.crane_phase {
            *v = inp.get_f32()?;
        }
        s.tail = inp.get_u8()?;
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
        out.put_i8(s.effect_index);
        s.progress.iter().for_each(|&v| {
            out.put_u32(v);
        });
        s.segment.iter().for_each(|&v| {
            out.put_f32(v);
        });
        s.placement.iter().for_each(|&v| {
            out.put_u32(v);
        });
        out.put_i8(s.flag);
        s.crane_interp.iter().for_each(|&v| {
            out.put_f32(v);
        });
        out.put_bool(s.crane_flag);
        s.crane_swing.iter().for_each(|&v| {
            out.put_u32(v);
        });
        s.crane_phase.iter().for_each(|&v| {
            out.put_f32(v);
        });
        out.put_u8(s.tail);
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
        let Some(version) = Self::accepts(v) else { return false };
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

/// `CreateSparks`: 1, 2 or 3 sparks for a maximum box extent < 20, < 30,
/// otherwise. The extent is taken over the scaffold box's x, y and z (the
/// spec does not name the box or the axes: OP16).
pub fn spark_count(bbox: &Bounds) -> usize {
    let e = (0..3).map(|i| bbox.1[i] - bbox.0[i]).fold(f32::MIN, f32::max);
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

/// The swing limits and the duration constant of state 2 → 0. The angle
/// range and the constant `k` (`0x00AA2098`) are not given by any spec
/// (OP16); the caller supplies them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CraneSwing {
    pub min: f32,
    pub max: f32,
    pub k: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CraneAnim {
    pub state: u32,
    pub acc: u32,
    pub duration: u32,
    /// The current swing target. Its value before the first 2 → 0 step is
    /// not specified; 0 [choice].
    pub target: f32,
}

impl CraneAnim {
    /// `SetLot`'s crane reset: state 2, accumulator 1, duration 0.
    pub fn after_set_lot() -> Self {
        CraneAnim {
            state: 2,
            acc: 1,
            duration: 0,
            target: 0.0,
        }
    }

    /// `AnimateCrane(ms)`.
    pub fn animate(&mut self, ms: u32, rng: &mut RzRandom, swing: &CraneSwing) {
        self.acc = self.acc.wrapping_add(ms);
        if self.acc < self.duration {
            return;
        }
        self.acc = 0;
        match self.state {
            0 => {
                self.state = 1;
                self.duration = 1000 + rng.uniform_u32(1000);
            }
            // 1 → 2 without a draw; the duration is not changed (literal).
            1 => self.state = 2,
            _ => {
                let (mn, mx) = (f64::from(swing.min), f64::from(swing.max));
                let mid = r24(r24(r24(mx - mn) * 0.5) + mn);
                let prev = self.target;
                let t = if f64::from(prev) < mid {
                    rng.range_f64(mid, mx)
                } else {
                    rng.range_f64(mn, mid)
                } as f32;
                self.target = t;
                let delta = (f64::from(t) - f64::from(prev)).abs();
                self.duration = round_half_even_f64(delta * 6000.0 * swing.k) as u32;
                self.state = 0;
            }
        }
    }
}

// ---- the life cycle ---------------------------------------------------------------------------------

/// The services the construction occupant calls (all through the game's
/// interfaces in the original). Visual steps the spec leaves open are
/// delegated whole.
pub trait ConstructionHost {
    /// Message server slot 0x14 / its unsubscribe.
    fn subscribe(&mut self, msg: u32);
    fn unsubscribe(&mut self, msg: u32);
    /// Animation-tick manager `AddTickableObject(this, priority)` / removal.
    fn add_tickable(&mut self, priority: u32);
    fn remove_tickable(&mut self);
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
    /// The visual placement steps of `SetLot` the spec leaves open, in
    /// order; the host makes their draws on `st`. `AttemptAddCrane`'s result
    /// is whether a crane model exists.
    fn divide_lot(&mut self, st: &mut ConstructionStatics);
    fn attempt_add_crane(&mut self, st: &mut ConstructionStatics) -> bool;
    fn add_ground_props(&mut self, st: &mut ConstructionStatics);
    fn create_construction_crew(&mut self, st: &mut ConstructionStatics, scaffold: &Bounds);
    fn add_roof_props(&mut self, st: &mut ConstructionStatics, scaffold: &Bounds);
    fn create_dust_clouds(&mut self);
    /// `CreateSparks`: create `count` spark effects.
    fn create_sparks(&mut self, count: usize);
    /// Spark effect `i`: slot 8(v).
    fn spark_slot8(&mut self, i: usize, v: u32);
    /// Spark effect `i`: slot 9(&pos) then slot 7(1).
    fn place_spark(&mut self, i: usize, pos: [f32; 3]);
    /// State 0/2 with `counter > N`: roof props removed (prop manager slot
    /// 29), the scaffold switched to removal.
    fn remove_roof_props(&mut self);
    fn start_scaffold_removal(&mut self);
    /// Renderer slots 28 / 29.
    fn add_to_renderer(&mut self);
    fn remove_from_renderer(&mut self);
    /// Occupant manager slot 21 `RemoveOccupant(this + 8, 0, 0)`.
    fn remove_from_occupant_manager(&mut self);
    /// The crane's swing limits, when a crane exists.
    fn crane_swing(&mut self) -> CraneSwing;
}

/// Daily state (ld §6.7.5).
pub const STATE_DONE: u32 = 4;

/// The live construction occupant: the record plus the unsaved life-cycle
/// state. Where the life-cycle values live in the record's scalars is only
/// partly specified (§9.2 vs §9.1 types, OP13), so they are kept apart and a
/// loaded occupant does not resume its schedule.
#[derive(Clone, Debug)]
pub struct ConstructionOccupant {
    pub rec: ConstructionRecord,
    pub initialised: bool,
    /// Live lot / building (`+0x68`/`+0x6C`).
    pub lot: RefValue,
    pub building: Option<RefValue>,
    /// Live world box (`+0xA0…+0xB4`), the building's box.
    pub live_bounds: Bounds,
    pub geometry: Option<ConstructionGeometry>,
    pub timing: Option<ConstructionTiming>,
    pub t0: u32,
    pub state: u32,
    pub counter: u32,
    pub next_day: u32,
    pub ms_acc: u32,
    pub scaffold_height: f32,
    pub crane: Option<CraneAnim>,
    pub sparks: usize,
    /// Byte +0x37 bit 0x02, "in the renderer".
    pub in_renderer: bool,
}

impl Default for ConstructionOccupant {
    fn default() -> Self {
        ConstructionOccupant {
            rec: ConstructionRecord::new(),
            initialised: false,
            lot: RefValue::Null,
            building: None,
            live_bounds: ([0.0; 3], [0.0; 3]),
            geometry: None,
            timing: None,
            t0: 0,
            state: 0,
            counter: 0,
            next_day: 0,
            ms_acc: 0,
            scaffold_height: 0.0,
            crane: None,
            sparks: 0,
            in_renderer: false,
        }
    }
}

impl ConstructionOccupant {
    /// `Init` (`0x005D1300`): subscribes, registers the tick at `0x80`.
    /// `StaticInit` is the caller's [`ConstructionStatics`]. The 20 sim-grid
    /// slots it pre-sizes are not used by anything specified here.
    pub fn init(&mut self, host: &mut dyn ConstructionHost) -> bool {
        if self.initialised {
            return true;
        }
        self.initialised = true;
        for m in SUBSCRIPTIONS {
            host.subscribe(m);
        }
        host.add_tickable(ANIMATION_TICK_PRIORITY);
        true
    }

    /// `SetBuildingCompletion(p)` (`0x005CA8D0`).
    pub fn set_building_completion(&mut self, host: &mut dyn ConstructionHost, p: f32) {
        if let Some(b) = &self.building {
            if host.building_completion(b) != p {
                host.set_building_completion(b, p);
                host.post_completion_changed(b);
            }
        }
    }

    /// `SetLot(lot)` (ld §6.7.4).
    pub fn set_lot(&mut self, host: &mut dyn ConstructionHost, st: &mut ConstructionStatics, lot: RefValue) {
        self.init(host);
        self.building = None;
        self.lot = lot;
        let Some(b) = host.lot_building(&self.lot) else { return };
        self.building = Some(b.clone());
        // RebuildModel.
        let bbox = host.building_bounds(&b);
        self.live_bounds = bbox;
        let geom = rebuild_model(&bbox, &st.tuning, |x, z| host.altitude(x, z));
        self.geometry = Some(geom);
        host.divide_lot(st);
        let crane = host.attempt_add_crane(st);
        host.add_ground_props(st);
        host.create_construction_crew(st, &geom.scaffold);
        host.add_roof_props(st, &geom.scaffold);
        host.create_dust_clouds();
        self.sparks = spark_count(&geom.scaffold);
        host.create_sparks(self.sparks);
        let rect = host.lot_cell_rect(&self.lot);
        for (x, z, inst) in lot_texture(&mut st.rng, rect, |x, z| host.cell_usable(x, z)) {
            host.set_lot_texture(x, z, inst);
        }
        self.t0 = host.sim_date();
        let days = host.construction_days(&b);
        let t = timing(&geom, &st.tuning, days);
        self.timing = Some(t);
        self.state = 0;
        self.counter = 0;
        self.ms_acc = 0;
        self.next_day = (i64::from(self.t0) + t.a) as u32;
        self.set_building_completion(host, 0.0);
        self.scaffold_height = r24(f64::from(geom.ground) + 0.001) as f32;
        self.crane = crane.then(CraneAnim::after_set_lot);
        self.rec.has_crane = crane;
        host.post_construction_message(MSG_SEGMENT_START, 1);
        host.post_construction_message(MSG_CONSTRUCTION, 1);
        host.add_to_renderer();
        self.in_renderer = true;
    }

    /// `DoDailySim` (ld §6.7.5); the date comes from the simulator, never
    /// from the message.
    pub fn do_daily_sim(&mut self, host: &mut dyn ConstructionHost, st: &mut ConstructionStatics) {
        let (Some(_), Some(geom), Some(t)) = (&self.building, self.geometry, self.timing) else {
            return;
        };
        if self.state == STATE_DONE {
            return;
        }
        let today = host.sim_date();
        if today < self.next_day {
            return;
        }
        self.ms_acc = 0;
        match self.state {
            0 | 2 => {
                self.counter += 1;
                let removal = self.counter > geom.segments;
                if removal {
                    host.remove_roof_props();
                    host.start_scaffold_removal();
                } else {
                    host.post_construction_message(MSG_SEGMENT_START, 1);
                }
                for i in 0..self.sparks {
                    host.spark_slot8(i, 1);
                }
                if removal {
                    self.state = 3;
                    self.next_day = (i64::from(today) + t.c) as u32;
                } else {
                    self.state = 1;
                    self.next_day = (i64::from(today) + t.b) as u32;
                }
            }
            1 => {
                for (i, p) in scatter_sparks(st, &geom.scaffold, self.sparks).into_iter().enumerate() {
                    host.place_spark(i, p);
                }
                self.state = 2;
                self.next_day = today.wrapping_add(t.e);
            }
            _ => self.complete(host),
        }
    }

    /// Completion (ld §6.7.5), then `Shutdown`.
    fn complete(&mut self, host: &mut dyn ConstructionHost) {
        self.state = STATE_DONE;
        self.set_building_completion(host, 1.0);
        if self.in_renderer {
            host.remove_from_renderer();
            self.in_renderer = false;
        }
        host.remove_from_occupant_manager();
        host.set_lot_state(&self.lot, LOT_STATE_GROWN, 0, 0);
        self.shutdown(host);
    }

    /// `Shutdown` (`0x005D0CE0`): posts `0xC9E7B1E8` (0) first, unsubscribes,
    /// unregisters the tick, releases the lot, building, props, effects and
    /// crew.
    pub fn shutdown(&mut self, host: &mut dyn ConstructionHost) {
        host.post_construction_message(MSG_CONSTRUCTION, 0);
        for m in SUBSCRIPTIONS {
            host.unsubscribe(m);
        }
        host.remove_tickable();
        self.lot = RefValue::Null;
        self.building = None;
        self.rec.ground_props.clear();
        self.rec.roof_props.clear();
        self.rec.effects.clear();
        self.rec.current_effect = RefValue::Null;
        self.rec.crew.clear();
    }

    /// `DoAnimationTick(ms)`: accumulates `msAcc` and runs the crane in
    /// states 0–2. The scaffold / completion interpolation is visual and not
    /// specified to the formula; it is not done here.
    pub fn do_animation_tick(&mut self, host: &mut dyn ConstructionHost, st: &mut ConstructionStatics, ms: u32) {
        self.ms_acc = self.ms_acc.wrapping_add(ms);
        if self.state <= 2 {
            if let Some(c) = &mut self.crane {
                let swing = host.crane_swing();
                c.animate(ms, &mut st.rng, &swing);
            }
        }
    }

    /// `DoMessage`: SimNewDay → `DoDailySim` (data1 unused); the tick with
    /// `ms := data2 / 1000` (ld §6.7.1; lr §9.2 says data3, OP15). The
    /// visual messages are not handled here. Returns true for all.
    pub fn do_message(
        &mut self,
        host: &mut dyn ConstructionHost,
        st: &mut ConstructionStatics,
        msg: u32,
        data2: u32,
    ) -> bool {
        match msg {
            MSG_SIM_NEW_DAY => self.do_daily_sim(host, st),
            MSG_ANIMATION_TICK => self.do_animation_tick(host, st, data2 / 1000),
            _ => {}
        }
        true
    }

    /// `GetPosition`: the live box centre, `(min + max) * 0.5f` per axis.
    pub fn position(&self) -> [f32; 3] {
        let (a, b) = self.live_bounds;
        [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5, (a[2] + b[2]) * 0.5]
    }
}

//! Construction occupant tests: lot-records §9 record rules and
//! lot-developer §16 tests 20, 21, 23, 24, 25.

use super::construction::*;
use super::tests::{read_obj, rec, token, write_obj};
use super::*;
use crate::network_occupants::Xform;
use crate::rng::{CrtRand, FixedSeeds, RzRandom};
use crate::stream::RefValue;

fn host_ref(id: u64, class_id: u32) -> RefValue {
    RefValue::Host { token: id, class_id }
}

fn sample() -> ConstructionRecord {
    let mut r = ConstructionRecord::new();
    r.base.flags = 5;
    r.base.cell_rect = [64, 65, 66, 67];
    r.lot = host_ref(7, CLSID_LOT);
    r.building = host_ref(8, CLSID_BUILDING_OCCUPANT);
    r.transform = Xform::default();
    r.world_bounds = ([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]);
    r.local_bounds = ([-1.5, 0.0, -2.5], [1.5, 3.0, 2.5]);
    r.has_crane = true;
    r.crane_position = [10.0, 20.0, 30.0];
    r.ground_props = vec![host_ref(11, 0x2977_AA47), host_ref(12, 0x2977_AA47)];
    r.roof_props = vec![host_ref(13, 0x2977_AA47)];
    r.effects = vec![host_ref(14, 0x2977_AA49)];
    r.current_effect = host_ref(14, 0x2977_AA49);
    r.scalars = ConstructionScalars {
        effect_index: -1,
        progress: [1, 2, 3, 2_451_545, 2_451_560],
        segment: [0.25, 6.0],
        placement: [4, 0x42C8_3333, 0x42C8_0000, 6, 7],
        flag: 3,
        crane_interp: [12.5, 1000.0],
        crane_flag: true,
        crane_swing: [9, 10],
        crane_phase: [0.5, 1.0],
        tail: 0xAB,
    };
    r.crew = vec![
        CrewEntry {
            first: true,
            second: false,
            automaton: host_ref(21, 0x896E_75AF),
        },
        CrewEntry {
            first: false,
            second: true,
            automaton: RefValue::Null,
        },
    ];
    r
}

fn sample_bytes() -> Vec<u8> {
    write_obj(&RefValue::Object(RecordObject::new(sample())))
}

#[test]
fn record_layout_and_round_trip() {
    let b = sample_bytes();
    assert_eq!(&b[..2], &[5, 0]);
    // §2.3 base: 14 bytes + an empty exemplar holder (u16 2, u16 2, u32 0,
    // 3 × u32 parent key) = 34; then the refs, xform mask 0, two boxes …
    let (o, ok, rem) = read_obj(construction::CLSID_CONSTRUCTION_OCCUPANT, &b);
    assert!(ok);
    assert_eq!(rem, 0);
    let r: ConstructionRecord = rec(&o);
    assert_eq!(token(&r.lot), Some((7, CLSID_LOT)));
    assert_eq!(token(&r.building), Some((8, CLSID_BUILDING_OCCUPANT)));
    assert_eq!(r.ground_props.len(), 2);
    assert_eq!(r.crew.len(), 2);
    assert!(r.crew[0].first && !r.crew[0].second && r.crew[1].automaton.is_null());
    assert_eq!(r.scalars, sample().scalars);
    assert_eq!(write_obj(&o), b);
    // The tail: u8 0xAB, crew count 2, {1, 0, ref 21}, {0, 1, null}.
    let tail = &b[b.len() - sample_tail_len()..];
    assert_eq!(tail[0], 0xAB);
    assert_eq!(&tail[1..5], &[2, 0, 0, 0]);
    assert_eq!(&tail[5..7], &[1, 0]);
    assert_eq!(&tail[15..17], &[0, 1]);
    assert_eq!(&tail[17..21], &[0, 0, 0, 0]);
}

#[test]
fn bool_fields_read_nonzero_and_write_one() {
    let mut b = sample_bytes();
    let pos = b.len() - sample_tail_len() + 5; // crew entry 0, first bool
    b[pos] = 7;
    let (o, ok, _) = read_obj(construction::CLSID_CONSTRUCTION_OCCUPANT, &b);
    assert!(ok);
    let w = write_obj(&o);
    assert_eq!(w[pos], 1);
}

fn sample_tail_len() -> usize {
    21
}

#[test]
fn version_rules() {
    let b = sample_bytes();
    // < 4: false before the base, 2 bytes consumed.
    let mut v3 = b.clone();
    v3[0] = 3;
    let (_, ok, rem) = read_obj(construction::CLSID_CONSTRUCTION_OCCUPANT, &v3);
    assert!(!ok);
    assert_eq!(rem, b.len() - 2);
    // > 5: read as 5, written back as 5.
    let mut v9 = b.clone();
    v9[0] = 9;
    let (o, ok, rem) = read_obj(construction::CLSID_CONSTRUCTION_OCCUPANT, &v9);
    assert!(ok);
    assert_eq!(rem, 0);
    assert_eq!(write_obj(&o), b);
    // 4: the legacy transform (NO15) stops the read after the references.
    let mut v4 = b.clone();
    v4[0] = 4;
    let (o, ok, rem) = read_obj(construction::CLSID_CONSTRUCTION_OCCUPANT, &v4);
    assert!(!ok);
    let r: ConstructionRecord = rec(&o);
    assert_eq!(token(&r.building), Some((8, CLSID_BUILDING_OCCUPANT)));
    assert!(r.ground_props.is_empty());
    assert!(rem > 0);
    // Short payload: false.
    let (_, ok, _) = read_obj(construction::CLSID_CONSTRUCTION_OCCUPANT, &b[..b.len() - 1]);
    assert!(!ok);
}

#[test]
fn tuning_shipped_and_defaults() {
    let shipped = |id: u32| -> Option<f32> {
        Some(match id {
            0x8989_EC22 => 3.0,
            0x8989_EC24 => 2.0,
            0x8989_EC26 => 15.0,
            0x8989_EC27 | 0x8989_EC28 => 25.0,
            0x8989_EC29 => 1.0,
            0x8989_EC2A => 0.5,
            0x8989_EC2D => 3.0,
            _ => return None,
        })
    };
    let t = ConstructionTuning::from_properties(shipped);
    assert_eq!(t.growth_rate, 1.0);
    assert_eq!(t.removal_rate, 2.0);
    assert_eq!(t.scaffold_gap, 2.0);
    assert_eq!(t.plank_texture_scale, Some(1.0 / 15.0));
    assert_eq!(t.girder_texture_scale, Some(0.04));
    assert_eq!(t.crane_height_above_scaffold, 3.0);
    assert_eq!(t.post_width, 1.0);
    let d = ConstructionTuning::from_properties(|_| None);
    assert_eq!(d, ConstructionTuning::default());
    assert_eq!((d.growth_rate, d.removal_rate, d.plank_texture_scale), (1.0, 2.0, None));
}

fn shipped_tuning() -> ConstructionTuning {
    ConstructionTuning {
        scaffold_gap: 2.0,
        ..ConstructionTuning::default()
    }
}

/// (max.y, corner altitude, D, f bits or 0, N, A, B, C, E)
type Row = (f32, Option<f32>, Option<u8>, u32, u32, i64, i64, i64, u32);

const TEST20: [Row; 9] = [
    (110.0, None, None, 0x4053_3335, 4, 0, 1, 1, 6),
    (110.0, Some(99.0), None, 0, 4, 1, 1, 1, 6),
    (110.0, Some(99.5), None, 0, 4, 1, 1, 1, 6),
    (110.0, None, Some(0), 0, 4, 0, 1, 1, 1),
    (110.0, None, Some(255), 0, 4, 0, 1, 1, 62),
    (160.0, None, None, 0x419F_BBBC, 20, 0, 1, 1, 1),
    (100.25, None, None, 0, 1, 0, 1, 0, 29),
    (103.125, None, None, 0, 2, 0, 1, 1, 13),
    (103.0, None, None, 0, 1, 0, 1, 1, 28),
];

fn geom_for(row: &Row) -> ConstructionGeometry {
    let bbox = ([0.0, 100.0, 0.0], [10.0, row.0, 10.0]);
    let mut first = true;
    rebuild_model(&bbox, &shipped_tuning(), |_, _| {
        let a = if first { row.1.unwrap_or(100.0) } else { 100.0 };
        first = false;
        a
    })
}

#[test]
fn test20_construction_timing() {
    for row in &TEST20 {
        let g = geom_for(row);
        assert_eq!(g.start_y.to_bits(), 0x42C8_3333, "{row:?}");
        if row.3 != 0 {
            assert_eq!(g.f.to_bits(), row.3, "{row:?}");
        }
        let t = timing(&g, &shipped_tuning(), row.2);
        assert_eq!(
            (g.segments, t.a, t.b, t.c, t.e),
            (row.4, row.5, row.6, row.7, row.8),
            "{row:?}"
        );
    }
    let g = geom_for(&TEST20[2]);
    assert_eq!(g.ground, 99.5);
    assert_eq!(g.scaffold, ([-2.0, 99.5, -2.0], [12.0, 110.0, 12.0]));
    // A NaN altitude keeps the current ground.
    let g = rebuild_model(&([0.0, 100.0, 0.0], [1.0, 110.0, 1.0]), &shipped_tuning(), |_, _| {
        f32::NAN
    });
    assert_eq!(g.ground, 100.0);
}

// ---- a recording host -------------------------------------------------------------------------------

#[derive(Default)]
struct Host {
    day: u32,
    bbox: Bounds,
    corner: Option<f32>,
    days: Option<u8>,
    completion: f32,
    crane: bool,
    log: Vec<String>,
}

impl Host {
    fn new(max_y: f32, corner: Option<f32>, days: Option<u8>) -> Self {
        Host {
            bbox: ([0.0, 100.0, 0.0], [10.0, max_y, 10.0]),
            corner,
            days,
            ..Host::default()
        }
    }
    fn say(&mut self, s: String) {
        self.log.push(s);
    }
}

impl ConstructionHost for Host {
    fn subscribe(&mut self, msg: u32) {
        self.say(format!("sub {msg:08X}"));
    }
    fn unsubscribe(&mut self, msg: u32) {
        self.say(format!("unsub {msg:08X}"));
    }
    fn add_tickable(&mut self, p: u32) {
        self.say(format!("tick+ {p:X}"));
    }
    fn remove_tickable(&mut self) {
        self.say("tick-".into());
    }
    fn post_construction_message(&mut self, msg: u32, d: u8) {
        let day = self.day;
        self.say(format!("post {msg:08X} {d} day {day}"));
    }
    fn sim_date(&mut self) -> u32 {
        self.day
    }
    fn lot_building(&mut self, lot: &RefValue) -> Option<RefValue> {
        (!lot.is_null()).then(|| host_ref(2, CLSID_BUILDING_OCCUPANT))
    }
    fn building_bounds(&mut self, _: &RefValue) -> Bounds {
        self.bbox
    }
    fn altitude(&mut self, x: f32, z: f32) -> f32 {
        if x == 0.0 && z == 0.0 {
            self.corner.unwrap_or(100.0)
        } else {
            100.0
        }
    }
    fn construction_days(&mut self, _: &RefValue) -> Option<u8> {
        self.days
    }
    fn building_completion(&mut self, _: &RefValue) -> f32 {
        self.completion
    }
    fn set_building_completion(&mut self, _: &RefValue, p: f32) {
        self.completion = p;
        self.say(format!("completion {p}"));
    }
    fn post_completion_changed(&mut self, _: &RefValue) {
        self.say("post 2977AE2F".into());
    }
    fn set_lot_state(&mut self, _: &RefValue, s: u32, a: u32, b: u32) {
        let day = self.day;
        self.say(format!("lot state {s} {a} {b} day {day}"));
    }
    fn lot_cell_rect(&mut self, _: &RefValue) -> [i32; 4] {
        [3, 4, 4, 5]
    }
    fn cell_usable(&mut self, _: i32, _: i32) -> bool {
        false
    }
    fn set_lot_texture(&mut self, x: i32, z: i32, i: u32) {
        self.say(format!("texture {x} {z} {i:08X}"));
    }
    fn divide_lot(&mut self, _: &mut ConstructionStatics) {
        self.say("divide".into());
    }
    fn attempt_add_crane(&mut self, _: &mut ConstructionStatics) -> bool {
        self.say("crane?".into());
        self.crane
    }
    fn add_ground_props(&mut self, _: &mut ConstructionStatics) {
        self.say("ground props".into());
    }
    fn create_construction_crew(&mut self, _: &mut ConstructionStatics, _: &Bounds) {
        self.say("crew".into());
    }
    fn add_roof_props(&mut self, _: &mut ConstructionStatics, _: &Bounds) {
        self.say("roof props".into());
    }
    fn create_dust_clouds(&mut self) {
        self.say("dust".into());
    }
    fn create_sparks(&mut self, n: usize) {
        self.say(format!("sparks {n}"));
    }
    fn spark_slot8(&mut self, i: usize, v: u32) {
        self.say(format!("spark {i} slot8 {v}"));
    }
    fn place_spark(&mut self, i: usize, _: [f32; 3]) {
        self.say(format!("spark {i} placed"));
    }
    fn remove_roof_props(&mut self) {
        let day = self.day;
        self.say(format!("roof props removed day {day}"));
    }
    fn start_scaffold_removal(&mut self) {
        self.say("scaffold removal".into());
    }
    fn add_to_renderer(&mut self) {
        self.say("render+".into());
    }
    fn remove_from_renderer(&mut self) {
        self.say("render-".into());
    }
    fn remove_from_occupant_manager(&mut self) {
        self.say("occupant manager remove".into());
    }
    fn crane_swing(&mut self) -> CraneSwing {
        CraneSwing {
            min: 0.0,
            max: 1.0,
            k: 1.0,
        }
    }
}

fn statics() -> ConstructionStatics {
    let seeds = FixedSeeds {
        wall_clock: 1,
        process_clock: 1,
    };
    ConstructionStatics {
        tuning: shipped_tuning(),
        rng: RzRandom::new(12345, &seeds),
        crt: CrtRand::default(),
    }
}

/// Runs SetLot on day 0 then SimNewDay from `first` on; returns the host.
fn run(mut h: Host, first: u32) -> (Host, ConstructionOccupant) {
    let mut st = statics();
    let mut c = ConstructionOccupant::default();
    c.set_lot(&mut h, &mut st, host_ref(1, CLSID_LOT));
    h.log.clear();
    for day in first..400 {
        h.day = day;
        // data2 of SimNewDay is not used; pass junk.
        c.do_message(&mut h, &mut st, MSG_SIM_NEW_DAY, 0xDEAD_BEEF);
        if c.state == STATE_DONE {
            break;
        }
    }
    (h, c)
}

fn days_of(h: &Host, prefix: &str) -> Vec<u32> {
    h.log
        .iter()
        .filter(|l| l.starts_with(prefix))
        .map(|l| l.rsplit(' ').next().unwrap().parse().unwrap())
        .collect()
}

#[test]
fn test21_construction_schedule() {
    let (h, c) = run(Host::new(110.0, None, None), 1);
    assert_eq!(days_of(&h, "post 49E7B1E2"), vec![1, 8, 15, 22]);
    assert_eq!(days_of(&h, "roof props removed"), vec![29]);
    assert_eq!(days_of(&h, "lot state 3 0 0"), vec![30]);
    assert_eq!(c.state, STATE_DONE);
    let (h, _) = run(Host::new(110.0, None, None), 0);
    assert_eq!(days_of(&h, "lot state 3 0 0"), vec![29]);
    let cases: [(f32, Option<f32>, Option<u8>, u32); 8] = [
        (110.0, Some(99.0), None, 30),
        (110.0, Some(99.5), None, 30),
        (110.0, None, Some(0), 10),
        (110.0, None, Some(255), 254),
        (160.0, None, None, 42),
        (100.25, None, None, 32),
        (103.125, None, None, 30),
        (103.0, None, None, 31),
    ];
    for (my, corner, d, done) in cases {
        let (h, _) = run(Host::new(my, corner, d), 1);
        assert_eq!(days_of(&h, "lot state 3 0 0"), vec![done], "{my} {corner:?} {d:?}");
    }
    let (h, _) = run(Host::new(110.0, Some(99.0), None), 0);
    assert_eq!(days_of(&h, "lot state 3 0 0"), vec![30]);
    let (h, _) = run(Host::new(110.0, None, Some(0)), 1);
    assert_eq!(days_of(&h, "post 49E7B1E2"), vec![1, 3, 5, 7]);
    assert_eq!(days_of(&h, "roof props removed"), vec![9]);
}

#[test]
fn set_lot_order_and_completion_order() {
    let mut h = Host::new(110.0, None, None);
    h.day = 5;
    h.completion = 0.5;
    let mut st = statics();
    let mut c = ConstructionOccupant::default();
    c.set_lot(&mut h, &mut st, host_ref(1, CLSID_LOT));
    let l = h.log.join("|");
    assert!(l.starts_with(
        "sub 66956814|sub 86AD10EE|sub 06CDB65B|sub A6B79602|sub 26D31EC1|tick+ 80|divide|crane?|ground props|crew|roof props|dust|sparks 1|completion 0|post 2977AE2F|post 49E7B1E2 1 day 5|post C9E7B1E8 1 day 5|render+"
    ), "{l}");
    assert_eq!((c.t0, c.next_day, c.state, c.counter), (5, 5, 0, 0));
    assert_eq!(c.scaffold_height.to_bits(), (100.001f32).to_bits());
    assert!(c.crane.is_none() && !c.rec.has_crane);
    // Completion order on the last day.
    let mut last = Vec::new();
    for day in 6..60 {
        h.day = day;
        h.log.clear();
        c.do_daily_sim(&mut h, &mut st);
        if c.state == STATE_DONE {
            last = h.log.clone();
            break;
        }
    }
    assert_eq!(
        last.join("|"),
        "completion 1|post 2977AE2F|render-|occupant manager remove|lot state 3 0 0 day 35|post C9E7B1E8 0 day 35|unsub 66956814|unsub 86AD10EE|unsub 06CDB65B|unsub A6B79602|unsub 26D31EC1|tick-"
    );
    assert!(c.lot.is_null() && c.building.is_none());
    // Done: further days change nothing.
    h.log.clear();
    h.day = 100;
    c.do_daily_sim(&mut h, &mut st);
    assert!(h.log.is_empty());
}

#[test]
fn no_building_means_no_schedule() {
    let mut h = Host::new(110.0, None, None);
    let mut st = statics();
    let mut c = ConstructionOccupant::default();
    c.set_lot(&mut h, &mut st, RefValue::Null);
    assert_eq!(h.log.len(), 6); // Init only
    h.day = 10;
    c.do_daily_sim(&mut h, &mut st);
    assert_eq!(h.log.len(), 6);
}

#[test]
fn set_building_completion_posts_only_on_change() {
    let mut h = Host::new(110.0, None, None);
    let mut c = ConstructionOccupant {
        building: Some(host_ref(2, CLSID_BUILDING_OCCUPANT)),
        ..ConstructionOccupant::default()
    };
    c.set_building_completion(&mut h, 0.0);
    assert!(h.log.is_empty());
    c.set_building_completion(&mut h, 0.25);
    assert_eq!(h.log, vec!["completion 0.25".to_string(), "post 2977AE2F".to_string()]);
}

#[test]
fn test23_lot_texture() {
    let mut st = statics();
    let mut n = 0;
    let t = lot_texture(&mut st.rng, [1, 1, 2, 2], |_, _| {
        n += 1;
        true
    });
    assert_eq!(
        t,
        vec![
            (1, 1, 0xA990_C100),
            (1, 2, 0xA990_C400),
            (2, 1, 0xA990_C200),
            (2, 2, 0xA990_C400)
        ]
    );
    // A water / outside cell draws nothing.
    let mut st = statics();
    let t = lot_texture(&mut st.rng, [1, 1, 2, 2], |x, z| (x, z) != (1, 1));
    assert_eq!(t[0], (1, 2, 0xA990_C100));
}

#[test]
fn test24_spark_scatter() {
    let mut st = statics();
    let p = scatter_sparks(&mut st, &([0.0, 100.0, 0.0], [20.0, 110.0, 30.0]), 1);
    assert_eq!(p.len(), 1);
    assert_eq!(p[0][0].to_bits(), 0x4138_7420);
    assert_eq!(p[0][2], 0.0);
    assert_eq!(p[0][1].to_bits(), 0x42CC_C8DD);
    // Three rand() consumed: the next is the fourth of a fresh CRT.
    let mut fresh = CrtRand::default();
    for _ in 0..3 {
        fresh.rand();
    }
    assert_eq!(st.crt, fresh);
    assert_eq!(reset_iterator(&mut CrtRand::default(), 4), vec![0, 1, 3, 2]);
}

#[test]
fn spark_counts() {
    assert_eq!(spark_count(&([0.0; 3], [19.9, 5.0, 3.0])), 1);
    assert_eq!(spark_count(&([0.0; 3], [20.0, 5.0, 3.0])), 2);
    assert_eq!(spark_count(&([0.0; 3], [2.0, 30.0, 3.0])), 3);
}

#[test]
fn test25_crane_pause() {
    let mut st = statics();
    let mut c = CraneAnim {
        state: 0,
        acc: 0,
        duration: 0,
        target: 0.0,
    };
    let swing = CraneSwing {
        min: 0.0,
        max: 1.0,
        k: 1.0,
    };
    c.animate(16, &mut st.rng, &swing);
    assert_eq!((c.state, c.duration, c.acc), (1, 1048, 0));
    c.animate(1000, &mut st.rng, &swing);
    assert_eq!(c.state, 1);
    c.animate(48, &mut st.rng, &swing);
    assert_eq!((c.state, c.duration), (2, 1048));
    // SetLot's reset swings on the first tick (acc 1 ≥ duration 0).
    let mut c = CraneAnim::after_set_lot();
    c.animate(0, &mut st.rng, &swing);
    assert_eq!(c.state, 0);
    assert!(c.target >= 0.5 && c.target <= 1.0);
}

#[test]
fn animation_tick_message_divides_by_1000() {
    let mut h = Host::new(110.0, None, None);
    h.crane = true;
    let mut st = statics();
    let mut c = ConstructionOccupant::default();
    c.set_lot(&mut h, &mut st, host_ref(1, CLSID_LOT));
    assert!(c.rec.has_crane);
    assert!(c.do_message(&mut h, &mut st, MSG_ANIMATION_TICK, 16_700));
    assert_eq!(c.ms_acc, 16);
    assert_eq!(c.crane.unwrap().state, 0);
}

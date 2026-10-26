// SPDX-License-Identifier: GPL-3.0-or-later
//! landvalue §6.1.6 / §16.32–35: controlled deliveries, not native capture.

use super::*;
use fc4_core::landvalue::msg;

const OLD_ALTITUDE: f32 = 228.07106;
// Shortest float32 spellings of the logged 8.65548706 and 13.0744085.
const OLD_SLOPE: f32 = 8.655_487;
const NEW_SLOPE: f32 = 13.074_409;

fn fixture(zoned: bool) -> (LandValueSim, Rc<RefCell<Data>>) {
    let mut d = Data::new();
    d.city.cells_x = 16;
    d.city.cells_z = 8;
    d.land_default = false;
    d.land_rects = vec![([1, 1, 1, 1], true), ([3, 1, 3, 1], true)];
    d.altitudes = HashMap::from([((1, 1), OLD_ALTITUDE), ((3, 1), 300.0)]);
    d.slope_input = Some(OLD_SLOPE);
    if zoned {
        d.zones.push(([0, 0, 3, 1], 1));
    }
    let d = Rc::new(RefCell::new(d));
    let mut sim = LandValueSim::new(Box::new(FakeHost(d.clone())));
    let mut tuning = shipped_tuning();
    tuning.water = Some(vec![0, 0]);
    assert!(sim.init(Some(&tuning)));
    for _ in 0..3 {
        sim.step(u32::MAX);
    }
    assert_eq!(sim.pipeline_state(), 3);
    assert_eq!(sim.scratch().get(0, 0), 1);
    assert_eq!(sim.scratch().get(1, 0), 1);
    d.borrow_mut().reads.clear();
    (sim, d)
}

fn edit(sim: &mut LandValueSim, d: &Rc<RefCell<Data>>) {
    let before = sim.snapshot();
    d.borrow_mut().altitudes.insert((1, 1), 270.0);
    d.borrow_mut().slope_input = Some(NEW_SLOPE);
    sim.handle_message(msg::TERRAIN_HEIGHTS_CHANGED, 0);
    assert_eq!(sim.pipeline_state(), before.pipeline_state);
    assert_eq!(sim.snapshot().sum, before.sum);
    assert_eq!(sim.snapshot().min_alt, before.min_alt);
    assert!(sim.snapshot().dirty);
}

fn bytes(d: &Rc<RefCell<Data>>, grid: u64) -> [u8; 2] {
    let d = d.borrow();
    let AnyGrid::U8(g) = &d.grids[&grid].grid else {
        panic!("not a byte grid")
    };
    [g.get(0, 0), g.get(1, 0)]
}

fn slope(d: &Rc<RefCell<Data>>, sim: &LandValueSim) -> Vec<f32> {
    let d = d.borrow();
    let AnyGrid::F32(g) = &d.grids[&sim.slope_grid()].grid else {
        panic!("not a slope grid")
    };
    g.values()
}

fn finish(sim: &mut LandValueSim, d: &Rc<RefCell<Data>>, expected: [u8; 2], average: u8) {
    for _ in 0..12 {
        if sim.pipeline_state() == 9 {
            break;
        }
        sim.step(u32::MAX);
    }
    assert_eq!(sim.pipeline_state(), 9);
    assert_eq!(bytes(d, sim.intrinsic_grid()), expected);
    assert_eq!(bytes(d, sim.total_grid()), expected);
    assert_eq!(sim.average_land_value(), average);
}

#[test]
fn lv10_edit_after_statistics_or_slope_and_ordinary_monthly_refresh() {
    for after_slope in [false, true] {
        let (mut sim, d) = fixture(true);
        sim.step(u32::MAX);
        let st = sim.snapshot();
        assert_eq!((st.sum, st.count, st.mean_alt, st.mode_alt), (528, 2, 264.0, 240.0));
        assert_eq!((st.min_alt, st.max_alt), (OLD_ALTITUDE, 300.0));
        assert_eq!(
            (st.altitude.pivot, st.altitude.high_range, st.altitude.top_index),
            (240.0, 60.0, 100)
        );
        if after_slope {
            sim.step(u32::MAX);
            assert_eq!(sim.pipeline_state(), 4);
        }
        edit(&mut sim, &d);
        sim.handle_message(msg::SIM_NEW_MONTH, 0);
        assert_eq!(sim.pipeline_state(), if after_slope { 4 } else { 5 });
        assert!(sim.snapshot().dirty);
        finish(&mut sim, &d, [22, 51], 36);
        assert_eq!(
            slope(&d, &sim),
            vec![if after_slope { OLD_SLOPE } else { NEW_SLOPE }; 2]
        );
        assert_eq!((sim.snapshot().mean_alt, sim.mode_altitude()), (264.0, 240.0));

        sim.handle_message(msg::SIM_NEW_MONTH, 0);
        assert_eq!(sim.pipeline_state(), 0);
        assert!(!sim.snapshot().dirty);
        // Month delivery selects state 0; it has not recomputed anything yet.
        assert_eq!(bytes(&d, sim.total_grid()), [22, 51]);
        for _ in 0..4 {
            sim.step(u32::MAX);
        }
        let st = sim.snapshot();
        assert_eq!((st.sum, st.mean_alt, st.mode_alt), (570, 285.0, 272.0));
        assert_eq!((st.min_alt, st.max_alt, st.altitude.low_range), (270.0, 300.0, 2.0));
        finish(&mut sim, &d, [1, 51], 26);
        assert_eq!(slope(&d, &sim), vec![NEW_SLOPE; 2]);
    }
}

#[test]
fn lv10_state3_read_order_and_partial_scan_keep_processed_contributions() {
    let (mut sim, d) = fixture(true);
    sim.step(0); // one tract only, A at its old altitude
    let reads = d.borrow().reads.clone();
    let altitude = reads.iter().position(|r| *r == "altitude").unwrap();
    assert!(altitude > 0 && reads[..altitude].iter().all(|r| *r == "land"));
    assert_eq!(&reads[altitude..], &["altitude", "zone"]);
    assert_eq!((sim.snapshot().sum, sim.snapshot().count), (228, 1));
    edit(&mut sim, &d);
    sim.step(u32::MAX);
    assert_eq!((sim.snapshot().sum, sim.snapshot().mean_alt), (528, 264.0));
    finish(&mut sim, &d, [22, 51], 36);
}

#[test]
fn lv10_fallback_retains_first_scan_extrema_and_zeroes_mean_before_returning() {
    let (mut sim, d) = fixture(false);
    sim.debug_set(None, None, None, Some(999.0));
    sim.step(u32::MAX);
    let first = sim.snapshot();
    assert_eq!(first.pipeline_state, 3);
    assert!(first.ignore_zoning);
    assert_eq!((first.sum, first.count, first.mean_alt), (0, 0, 0.0));
    assert_eq!((first.min_alt, first.max_alt), (OLD_ALTITUDE, 300.0));
    assert!(first.histogram.iter().all(|n| *n == 0));
    let first_scan_reads = d.borrow().reads.clone();
    edit(&mut sim, &d);
    d.borrow_mut().reads.clear();
    sim.step(u32::MAX);
    let second = sim.snapshot();
    assert_eq!(
        (second.sum, second.count, second.mean_alt, second.mode_alt),
        (570, 2, 285.0, 272.0)
    );
    assert_eq!((second.min_alt, second.max_alt), (OLD_ALTITUDE, 300.0));
    assert_eq!(second.altitude.low_range.to_bits(), (272.0 - OLD_ALTITUDE).to_bits());
    assert_eq!(second.altitude.high_range, 28.0);
    assert_eq!(((270.0 - OLD_ALTITUDE) / second.altitude.low_range * 50.0) as i32, 47);
    assert!(first_scan_reads.contains(&"zone"));
    assert!(!d.borrow().reads.contains(&"zone")); // ignoreZoning short circuits
    d.borrow_mut().zones.push(([0, 0, 3, 1], 1)); // state-8 selection
    finish(&mut sim, &d, [10, 51], 30);
    // Dropping the first scan would instead give A=1: not an eligible replay.
    sim.handle_message(msg::SIM_NEW_MONTH, 0);
    finish(&mut sim, &d, [1, 51], 26);
}

#[test]
fn lv10_written_slope_and_intrinsic_cells_survive_edits_between_slices() {
    let (mut sim, d) = fixture(true);
    sim.step(u32::MAX); // statistics
    sim.step(0); // first of two slope cells
    edit(&mut sim, &d);
    sim.step(u32::MAX);
    assert_eq!(slope(&d, &sim), vec![OLD_SLOPE, NEW_SLOPE]);
    d.borrow_mut().reads.clear();
    sim.step(0); // A intrinsic at new altitude, with retained statistics
    assert_eq!(&d.borrow().reads, &["altitude"]);
    d.borrow_mut().altitudes.insert((1, 1), OLD_ALTITUDE);
    d.borrow_mut().altitudes.insert((3, 1), 270.0);
    sim.handle_message(msg::TERRAIN_HEIGHTS_CHANGED, 0);
    finish(&mut sim, &d, [22, 22], 22);

    let (mut sim, d) = fixture(true);
    for _ in 0..3 {
        sim.step(u32::MAX); // 3, 5, 4 completed before edit
    }
    assert_eq!(sim.pipeline_state(), 6);
    edit(&mut sim, &d);
    finish(&mut sim, &d, [1, 51], 26);
    assert_eq!(slope(&d, &sim), vec![OLD_SLOPE; 2]);
}

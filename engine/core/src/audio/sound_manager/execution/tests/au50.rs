// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
#[test]
fn au50_last_match_time_boundary_count_gate_and_preinsertion_gain() {
    let mut m = manager();
    let mut h = Trace {
        play: true,
        ..Trace::default()
    };
    let mut ids = InstanceIdCounter::new();
    m.elapsed = 900;
    assert!(m.start(7, 1, 0, &mut ids, &mut h));
    m.elapsed = 980;
    assert!(m.start(7, 2, 0, &mut ids, &mut h));
    m.elapsed = 1000;
    assert!(!m.start(7, 3, 0, &mut ids, &mut h));
    m.instances.get_mut(&1).unwrap().timestamp = 980;
    m.instances.get_mut(&2).unwrap().timestamp = 900;
    m.tuning = SoundManagerTuning::shipped();
    assert!(m.start(7, 3, 0, &mut ids, &mut h));
    for s in m.instances.values() {
        assert_eq!(s.gain, 950.0 / 1024.0);
        assert_eq!(s.raw.map(f32::to_bits), [0; 3]);
        assert_eq!(s.average.map(f32::to_bits), [0; 3]);
    }
    m.elapsed = 1050;
    assert!(!m.start(7, 4, 0, &mut ids, &mut h));
    m.one_shots = 10;
    assert!(m.start(8, 4, 0, &mut ids, &mut h));
    assert_eq!(m.one_shots, 11);
    // Duplicate-ID removal precedes rejection; a failed new start never restores it.
    assert!(!m.start(7, 4, 0, &mut ids, &mut h));
    assert!(!m.instances.contains_key(&4));
    m = manager();
    m.elapsed = 950;
    assert!(m.start(7, 1, 0, &mut ids, &mut h));
    m.elapsed = 1000;
    assert!(m.start(7, 2, 0, &mut ids, &mut h));
}
#[test]
fn au50_loop_link_promotion_preserves_child_state_without_play() {
    let mut m = manager();
    let mut h = Trace {
        play: true,
        looping: true,
        ..Trace::default()
    };
    let mut ids = InstanceIdCounter::new();
    for id in 1..=3 {
        assert!(m.start(7, id, 0, &mut ids, &mut h));
    }
    assert_eq!(m.instances[&1].children, [2, 3]);
    assert_eq!(m.independent_loops, 1);
    assert_eq!(h.calls.iter().filter(|c| c.1 == 0x14).count(), 1);
    let child = m.instances.get_mut(&2).unwrap();
    child.holder = 110;
    child.buffer = 120;
    child.gain = 0.25;
    child.pause_count = 2;
    h.calls.clear();
    h.retains.clear();
    h.releases.clear();
    assert!(m.stop(1, true, &mut h));
    let child = &m.instances[&2];
    assert_eq!((child.id, child.gain, child.pause_count), (2, 0.25, 2));
    assert_eq!((child.holder, child.buffer, child.master), (10, 20, None));
    assert_eq!(child.children, [3]);
    assert_eq!(m.instances[&3].master, Some(2));
    assert!(h.retains.contains(&10) && h.retains.contains(&20));
    assert!(h.releases.contains(&110) && h.releases.contains(&120));
    assert!(!h.calls.iter().any(|c| matches!(c.1, 0x14 | 0x20 | 0x24)));
}
#[test]
fn au50_payload_ownership_order_and_distinct_worker_results() {
    let mut m = manager();
    let mut h = Trace {
        payload: true,
        ..Trace::default()
    };
    let mut ids = InstanceIdCounter::new();
    h.data
        .extend([(0x24, 99), (0x38, 16777217), (0x4C, (-3i32) as u32 as u64), (0x60, 4)]);
    assert!(m.do_event(Some(1), 0xCA45_190F, &mut ids, &mut h));
    assert!(!m.position(99, [0.0; 3], &mut h));
    assert_eq!(
        h.calls.iter().filter(|c| c.0 == 50).map(|c| c.1).collect::<Vec<_>>(),
        [0x60, 0x4C, 0x38, 0x24]
    );
    assert_eq!((h.retains.clone(), h.releases.clone()), (vec![50], vec![50]));
    h.calls.clear();
    h.data.insert(0x38, 1);
    h.data.insert(0x48, 0);
    assert!(m.do_event(Some(1), 0xEA45_1903, &mut ids, &mut h));
    assert!(m.instances.is_empty());
    assert_eq!(
        h.calls.iter().filter(|c| c.0 == 50).map(|c| c.1).collect::<Vec<_>>(),
        [0x48, 0x38, 0x24]
    );
    h.play = true;
    m.start(8, 1, 0, &mut ids, &mut h);
    h.data.insert(0x24, 1);
    h.data.insert(0x38, 16777217);
    m.do_event(Some(1), 0xCA45_190F, &mut ids, &mut h);
    assert_eq!(m.instances[&1].raw, [16777216.0, -3.0, 4.0]);
}
#[test]
fn au50_distance_simulation_pause_resume_and_refresh_order() {
    let mut m = manager();
    let mut h = Trace {
        play: true,
        looping: true,
        model: 0x7DCD_ED3D,
        ..Trace::default()
    };
    let mut ids = InstanceIdCounter::new();
    m.start(7, 1, 1, &mut ids, &mut h);
    m.position(1, [100.0, 0.0, 0.0], &mut h);
    assert_eq!(m.instances[&1].pause_count, 1);
    assert_eq!(m.instances[&1].flags & 8, 8);
    h.paused = true;
    m.update_all(&mut h);
    assert_eq!(m.instances[&1].pause_count, 2);
    m.position(1, [1.0, 0.0, 0.0], &mut h);
    assert_eq!(m.instances[&1].pause_count, 1);
    h.paused = false;
    h.calls.clear();
    h.writes.clear();
    m.update_all(&mut h);
    assert_eq!(m.instances[&1].pause_count, 0);
    assert_eq!(h.writes[0].1, 0x15);
    assert!(h.calls.contains(&(10, 0x14, vec![1, 0, 512, 0])));
    assert_eq!((m.elapsed, m.updates), (0, 0));
    assert!(m.unsupported.is_empty());
    let a = policy::average_position(
        [0.0, 0.0, 1.0],
        [0.0; 3],
        &[([1.0, 0.0, 1.0], 1024), ([-1.0, 0.0, 1.0], 1024)],
        0.0,
    );
    assert!(a[0].abs() < 1e-7);
    assert!(a[2] > 2.0 && a[2] < 3.0);
    let a = policy::average_position(
        [0.0, 0.0, 1.0],
        [0.0; 3],
        &[([1.0, 0.0, 1.0], 1024), ([-1.0, 0.0, 1.0], 0)],
        0.0,
    );
    assert!((a[0] - 1.0).abs() < 1e-6 && (a[2] - 1.0).abs() < 1e-6);
    let singular = policy::average_position([0.0, 0.0, 1.0], [0.0; 3], &[([0.0, 0.0, 1.0], 1024)], 0.0);
    assert!(singular.iter().any(|v| v.is_nan()));
    h.calls.clear();
    for expected in 1..=3 {
        assert_eq!(m.pause_for_distance(1, true, &mut h), expected);
    }
    assert_eq!(h.calls.iter().filter(|c| c.1 == 0x20).count(), 1);
    for expected in (0..3).rev() {
        assert_eq!(m.resume(1, &mut h), expected);
    }
    assert_eq!(h.calls.iter().filter(|c| c.1 == 0x14).count(), 1);
}

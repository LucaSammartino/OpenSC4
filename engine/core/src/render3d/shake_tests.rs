// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Default)]
struct Clock {
    now: f32,
    restarts: usize,
    seeds: Vec<u32>,
    draws: usize,
}

impl ShakeServices for Clock {
    fn restart_shake_timer(&mut self) {
        self.now = 0.0;
        self.restarts += 1;
    }
    fn shake_time(&self) -> f32 {
        self.now
    }
    fn seed_shake_random(&mut self, seed: u32) {
        self.seeds.push(seed);
    }
    fn shake_random_uniform(&mut self, min: f64, max: f64) -> f64 {
        assert_eq!((min, max), (-10.0, 10.0));
        self.draws += 1;
        1.0
    }
}

fn parameters(amplitude: f32) -> ShakeParameters {
    ShakeParameters {
        duration: 2.0,
        fade_out: 0.0,
        amplitude: vec![amplitude, 0.0],
        rate: vec![60.0, 60.0],
        aspect: 1.0,
        pattern: 0,
    }
}

fn frame(zoom: i32) -> ShakeFrame {
    ShakeFrame {
        camera_center: [0.0; 2],
        camera_radius_scale: 1.0 / 640.0,
        zoom,
    }
}

#[test]
fn shake_s1_parameter_handles_front_insertion_and_timer_restart() {
    let mut r = Render3d::new(&mut tests::Host::default());
    let mut host = Clock::default();
    let mut p = parameters(5.0);
    let first = r.start_screen_shake(&mut host, &p, None).unwrap();
    host.now = 0.5;
    let second = r.start_screen_shake(&mut host, &p, None).unwrap();
    p.amplitude[0] = 99.0;
    assert_eq!((first, second), (0, 1));
    assert_eq!(host.restarts, 1);
    assert_eq!(r.screen_shakes().iter().map(|n| n.handle).collect::<Vec<_>>(), [1, 0]);
    assert_eq!(r.screen_shakes()[0].start, 0.5);
    assert_eq!(r.screen_shakes()[0].parameters.amplitude[0], 5.0);
    r.stop_screen_shake(&host, first);
    r.stop_screen_shake(&host, second);
    assert_eq!(r.start_screen_shake(&mut host, &p, None), Ok(2));
    assert_eq!(host.restarts, 2);
    assert_eq!(r.screen_shakes()[0].start, 0.0);
}

#[test]
fn shake_s2_interpolation_strict_expiry_and_first_frame_phase() {
    let mut r = Render3d::new(&mut tests::Host::default());
    let mut host = Clock::default();
    let mut patterns = ShakePatterns::default();
    r.start_screen_shake(&mut host, &parameters(5.0), None).unwrap();
    host.now = 1.0;
    r.update_screen_shake(&mut host, &mut patterns, frame(4), true);
    assert_eq!(r.shake_offsets(), [2.0; 2]);
    assert_eq!(r.screen_shakes()[0].phase, 60.0);
    r.start_screen_shake(&mut host, &parameters(5.0), None).unwrap();
    host.now = 1.25;
    r.update_screen_shake(&mut host, &mut patterns, frame(4), true);
    assert_eq!(r.screen_shakes()[0].phase, 15.0);
    host.now = 2.0;
    r.update_screen_shake(&mut host, &mut patterns, frame(4), true);
    assert_eq!(r.screen_shakes().len(), 2);
    host.now = 2.001;
    r.update_screen_shake(&mut host, &mut patterns, frame(4), true);
    assert_eq!(r.screen_shakes().len(), 1);
    assert_eq!(host.seeds, [88]);
    assert_eq!(host.draws, 128);
}

#[test]
fn shake_s3_at_radius_samples_but_beyond_radius_freezes_phase() {
    let mut r = Render3d::new(&mut tests::Host::default());
    let mut host = Clock::default();
    let mut patterns = ShakePatterns::default();
    r.start_screen_shake(&mut host, &parameters(5.0), Some([1.0, 0.0]))
        .unwrap();
    r.start_screen_shake(&mut host, &parameters(5.0), Some([1.01, 0.0]))
        .unwrap();
    host.now = 1.0;
    r.update_screen_shake(&mut host, &mut patterns, frame(4), true);
    assert_eq!(r.screen_shakes()[0].phase, 0.0);
    assert_eq!(r.screen_shakes()[1].phase, 60.0);
    assert_eq!(r.shake_offsets(), [0.0; 2]);
}

#[test]
fn shake_s4_zoom_scaling_and_s5_half_even_rounding() {
    for (amplitude, zoom, expected) in [
        (5.0, 4, 2.0),
        (7.0, 4, 4.0),
        (-5.0, 4, -2.0),
        (10.0, 3, 3.0),
        (10.0, 5, 5.0),
        (10.0, 1, 1.0),
    ] {
        let mut r = Render3d::new(&mut tests::Host::default());
        let mut host = Clock::default();
        r.start_screen_shake(&mut host, &parameters(amplitude), None).unwrap();
        host.now = 1.0;
        r.update_screen_shake(&mut host, &mut ShakePatterns::default(), frame(zoom), true);
        assert_eq!(r.shake_offsets(), [expected; 2]);
    }
}

#[test]
fn shake_s6_preference_freezes_nodes_and_zeroes_offsets() {
    let mut r = Render3d::new(&mut tests::Host::default());
    let mut host = Clock::default();
    r.start_screen_shake(&mut host, &parameters(5.0), None).unwrap();
    host.now = 3.0;
    let mut patterns = ShakePatterns::default();
    r.state.shake_offsets = [4, 5];
    r.update_screen_shake(&mut host, &mut patterns, frame(4), false);
    assert_eq!(r.shake_offsets(), [0.0; 2]);
    assert_eq!(r.screen_shakes().len(), 1);
    assert_eq!(r.screen_shakes()[0].phase, 0.0);
    assert!(patterns.tables().is_none());
    r.update_screen_shake(&mut host, &mut patterns, frame(4), true);
    assert!(r.screen_shakes().is_empty());
}

#[test]
fn shake_stop_immediate_or_pull_start_back_without_pushing_forward() {
    let mut r = Render3d::new(&mut tests::Host::default());
    let mut host = Clock::default();
    let mut p = parameters(5.0);
    let first = r.start_screen_shake(&mut host, &p, None).unwrap();
    p.duration = 10.0;
    p.fade_out = 2.0;
    let fading = r.start_screen_shake(&mut host, &p, None).unwrap();
    host.now = 1.0;
    r.stop_screen_shake(&host, first);
    r.stop_screen_shake(&host, fading);
    r.stop_screen_shake(&host, 123);
    assert_eq!(r.screen_shakes().len(), 1);
    assert_eq!(r.screen_shakes()[0].start, -7.0);
    host.now = 2.0;
    r.stop_screen_shake(&host, fading);
    assert_eq!(r.screen_shakes()[0].start, -7.0);
}

#[test]
fn shake_patterns_once_per_process_sine_aspect_and_other_pattern() {
    let mut patterns = ShakePatterns::default();
    let mut host = Clock::default();
    let mut r = Render3d::new(&mut tests::Host::default());
    let mut p = parameters(1.0);
    p.amplitude = vec![1.0];
    p.rate = vec![16.0];
    p.pattern = 1;
    p.aspect = 2.0;
    r.start_screen_shake(&mut host, &p, None).unwrap();
    host.now = 1.0;
    r.update_screen_shake(&mut host, &mut patterns, frame(4), true);
    assert_eq!(r.shake_offsets(), [0.0, 20.0]);
    let sine = &patterns.tables().unwrap()[1];
    assert_eq!(sine[0], [0.0; 2]);
    assert_eq!(sine[16], [0.0, 10.0]);
    let mut second = Render3d::new(&mut tests::Host::default());
    p.pattern = 9;
    second.start_screen_shake(&mut host, &p, None).unwrap();
    second.update_screen_shake(&mut host, &mut patterns, frame(4), true);
    assert_eq!(second.shake_offsets(), [0.0; 2]);
    assert_eq!(host.seeds, [88]);
    assert_eq!(host.draws, 128);
}

#[test]
fn shake_empty_curve_rejected_without_timer_or_handle_effects() {
    let mut r = Render3d::new(&mut tests::Host::default());
    let mut host = Clock::default();
    let mut p = parameters(1.0);
    p.rate.clear();
    assert_eq!(r.start_screen_shake(&mut host, &p, None), Err(ShakeError::EmptyCurve));
    assert_eq!(host.restarts, 0);
    assert!(r.screen_shakes().is_empty());
}

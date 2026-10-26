// SPDX-License-Identifier: GPL-3.0-or-later
//! Screen-shake list, patterns and sampling (render3d §§6.3, 10).
use super::Render3d;

pub trait ShakeServices {
    fn restart_shake_timer(&mut self);
    fn shake_time(&self) -> f32;
    fn seed_shake_random(&mut self, seed: u32);
    fn shake_random_uniform(&mut self, min: f64, max: f64) -> f64;
}

/// One instance per process, supplied by the application like BackingStore.
#[derive(Debug, Default)]
pub struct ShakePatterns {
    tables: Option<[[[f32; 2]; 64]; 2]>,
}

impl ShakePatterns {
    fn initialize(&mut self, host: &mut impl ShakeServices) {
        if self.tables.is_some() {
            return;
        }
        let mut tables = [[[0.0; 2]; 64]; 2];
        host.seed_shake_random(88);
        for pair in &mut tables[0] {
            for value in pair {
                *value = host.shake_random_uniform(-10.0, 10.0) as f32;
            }
        }
        for (i, pair) in tables[1].iter_mut().enumerate() {
            // SPEC-GAP: portable Rust uses f64 for the specified x87 intermediates.
            pair[1] = ((i as f64 * 0.098_174_77_f32 as f64).sin() * 10.0) as f32;
        }
        self.tables = Some(tables);
    }

    pub fn tables(&self) -> Option<&[[[f32; 2]; 64]; 2]> {
        self.tables.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShakeParameters {
    pub duration: f32,
    pub fade_out: f32,
    pub amplitude: Vec<f32>,
    pub rate: Vec<f32>,
    pub aspect: f32,
    pub pattern: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShakeNode {
    pub handle: u32,
    pub start: f32,
    pub phase: f32,
    pub position: Option<[f32; 2]>,
    pub parameters: ShakeParameters,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShakeError {
    EmptyCurve,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShakeFrame {
    pub camera_center: [f32; 2],
    pub camera_radius_scale: f32,
    pub zoom: i32,
}

#[derive(Debug, Default)]
pub(super) struct Shakes {
    nodes: Vec<ShakeNode>,
    next_handle: u32,
    previous_time: f32,
}

impl Shakes {
    pub(super) fn clear(&mut self) {
        self.nodes.clear();
    }
}

fn sample(curve: &[f32], u: f32) -> f32 {
    if curve.len() == 1 {
        return curve[0];
    }
    let x = (curve.len() - 1) as f32 * u;
    let index = x.floor() as usize;
    let r = x - index as f32;
    if r <= 0.0 {
        curve[index]
    } else {
        curve[index] + (curve[index + 1] - curve[index]) * r
    }
}

impl Render3d {
    pub fn screen_shakes(&self) -> &[ShakeNode] {
        &self.shakes.nodes
    }

    pub fn start_screen_shake(
        &mut self,
        host: &mut impl ShakeServices,
        parameters: &ShakeParameters,
        position: Option<[f32; 2]>,
    ) -> Result<u32, ShakeError> {
        if parameters.amplitude.is_empty() || parameters.rate.is_empty() {
            // SPEC-GAP: reject empty curves instead of native out-of-bounds access.
            return Err(ShakeError::EmptyCurve);
        }
        if self.shakes.nodes.is_empty() {
            host.restart_shake_timer();
            self.shakes.previous_time = 0.0;
        }
        let handle = self.shakes.next_handle;
        self.shakes.next_handle = handle.wrapping_add(1);
        self.shakes.nodes.insert(
            0,
            ShakeNode {
                handle,
                start: host.shake_time(),
                phase: 0.0,
                position,
                parameters: parameters.clone(),
            },
        );
        Ok(handle)
    }

    pub fn stop_screen_shake(&mut self, host: &impl ShakeServices, handle: u32) {
        let Some(index) = self.shakes.nodes.iter().position(|node| node.handle == handle) else {
            return;
        };
        let node = &mut self.shakes.nodes[index];
        if node.parameters.fade_out == 0.0 {
            self.shakes.nodes.remove(index);
        } else {
            let start = host.shake_time() + (node.parameters.fade_out - node.parameters.duration);
            if start < node.start {
                node.start = start;
            }
        }
    }

    /// The preference gate in Draw freezes the list as well as zeroing offsets.
    pub fn update_screen_shake(
        &mut self,
        host: &mut impl ShakeServices,
        patterns: &mut ShakePatterns,
        frame: ShakeFrame,
        enabled: bool,
    ) {
        self.state.shake_offsets = [0; 2];
        if !enabled {
            return;
        }
        patterns.initialize(host);
        if self.shakes.nodes.is_empty() {
            return;
        }
        let tables = patterns.tables.as_ref().unwrap();
        let radius = frame.camera_radius_scale * 640.0;
        let inv = (1.0 / (radius as f64 * radius as f64)) as f32;
        let now = host.shake_time();
        let dt = now - self.shakes.previous_time;
        self.shakes.previous_time = now;
        let mut sum = [0.0_f64; 2];
        self.shakes.nodes.retain_mut(|node| {
            let t = now - node.start;
            let p = &node.parameters;
            if t > p.duration {
                return false;
            }
            let mut w = 1.0;
            if let Some([px, pz]) = node.position {
                let dx = px as f64 - frame.camera_center[0] as f64;
                let dz = pz as f64 - frame.camera_center[1] as f64;
                w = (1.0 - (dx * dx + dz * dz) * inv as f64) as f32;
                if w < 0.0 {
                    return true;
                }
            }
            let u = t / p.duration;
            let amplitude = sample(&p.amplitude, u);
            let rate = sample(&p.rate, u);
            let ax = amplitude * w / p.aspect;
            let ay = amplitude * w * p.aspect;
            node.phase += rate * t.min(dt);
            let f = node.phase / 64.0;
            let x = 63.0 * (f - f.floor());
            let index = x.floor() as usize;
            let r = x - index as f32;
            if let Some(table) = usize::try_from(p.pattern).ok().and_then(|i| tables.get(i)) {
                let [mut x, mut y] = table[index].map(f64::from);
                if r > 0.0 {
                    x = r as f64 * table[index + 1][0] as f64 + ((1.0 - r) * table[index][0]) as f64;
                    y = ((1.0 - r) * table[index][1]) as f64 + (r * table[index + 1][1]) as f64;
                }
                sum[0] += x * ax as f64;
                sum[1] += y * ay as f64;
            }
            true
        });
        let mut scale = 1.0_f64;
        for _ in 0..4_i32.saturating_sub(frame.zoom) {
            scale *= 0.6_f32 as f64;
        }
        self.state.shake_offsets = [
            (((sum[0] * scale + 12_582_912.0) as f32).to_bits() as i32).wrapping_sub(0x4B40_0000),
            ((((sum[1] * scale) as f32 + 12_582_912.0).to_bits()) as i32).wrapping_sub(0x4B40_0000),
        ];
    }
}

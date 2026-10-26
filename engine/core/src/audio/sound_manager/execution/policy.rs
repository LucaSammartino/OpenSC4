// SPDX-License-Identifier: GPL-3.0-or-later
//! audio §8.3.2 and audio-owner-closure §7.
use super::{Host, Manager};
impl Manager {
    pub(super) fn admission(&self, sound: u32, h: &mut impl Host) -> (bool, u32) {
        let matches: Vec<_> = self
            .list
            .iter()
            .map(|id| &self.instances[id])
            .filter(|s| s.sound == sound)
            .collect();
        let mut minimum = i32::MAX;
        for s in &matches {
            minimum = minimum.min(s.timestamp.wrapping_sub(self.elapsed) as i32);
        }
        if let Some(s) = matches.last() {
            h.retain(s.pointer);
            let reject = minimum != i32::MAX
                && h.property(s.buffer, 0x33, 0) == 0
                && self.elapsed.wrapping_sub(s.timestamp) < 50;
            h.release(s.pointer);
            if reject {
                return (false, 0);
            }
            h.retain(s.pointer);
            let reject = h.property(s.buffer, 0x33, 0) == 0
                && (self.one_shots >= self.tuning.one_shot || matches.len() as u32 >= self.tuning.voice_gate);
            h.release(s.pointer);
            if reject {
                return (false, matches.len() as u32);
            }
        }
        (true, matches.len() as u32)
    }
    pub(super) fn curve(&self, x: f32) -> f64 {
        let points = &self.tuning.curve;
        let mut previous = points[0];
        for &p in &points[1..] {
            if p.0 <= previous.0 {
                break;
            }
            if x <= p.0 {
                return f64::from(previous.1)
                    + (f64::from(x) - f64::from(previous.0)) / (f64::from(p.0) - f64::from(previous.0))
                        * (f64::from(p.1) - f64::from(previous.1));
            }
            previous = p;
        }
        f64::from(previous.1)
    }
    pub(super) fn update_position(&mut self, id: u32, h: &mut impl Host) {
        let s = &self.instances[&id];
        if !s.children.is_empty() {
            let geometry = match h.property(s.buffer, 0x39, 0) {
                0x7DCD_ED3D => Some(h.listener().unwrap_or(([0.0; 3], [0.0; 3]))),
                0x0A63_A543 => h.view_geometry(),
                _ => None,
            };
            if let Some((l, t)) = geometry {
                let d = std::array::from_fn(|i| t[i] - l[i]);
                let ids: Vec<_> = std::iter::once(id).chain(s.children.iter().copied()).collect();
                let mut entries = Vec::new();
                for id in ids {
                    let s = &self.instances[&id];
                    entries.push((s.raw, h.property(s.buffer, 0x15, 0) as i32));
                }
                let inner = s.radii[0];
                let average = average_position(d, l, &entries, inner);
                self.instances.get_mut(&id).expect("instance").average = average;
            }
        }
        Self::write_position(&self.instances[&id], h);
    }
    fn pause(&mut self, id: u32, soft: bool, h: &mut impl Host) {
        let s = self.instances.get_mut(&id).expect("instance");
        s.pause_count = s.pause_count.wrapping_add(1);
        if s.pause_count == 1 && s.master.is_none() {
            Self::completion(s, false, h);
            h.call(s.holder, if soft { 0x20 } else { 0x24 }, &[]);
        }
    }
    pub fn pause_for_distance(&mut self, id: u32, on: bool, h: &mut impl Host) -> u32 {
        if on {
            self.instances.get_mut(&id).expect("instance").flags |= 8;
            self.pause(id, true, h);
            self.instances[&id].pause_count
        } else {
            self.instances.get_mut(&id).expect("instance").flags &= !8;
            self.resume(id, h)
        }
    }
    pub fn resume(&mut self, id: u32, h: &mut impl Host) -> u32 {
        let s = self.instances.get_mut(&id).expect("instance");
        s.pause_count = s.pause_count.wrapping_sub(1);
        let count = s.pause_count;
        if count == 0 && s.master.is_none() {
            Self::completion(s, true, h);
            let view = h.query(s.holder, 0xBB1B_E367);
            if let Some(v) = view {
                h.call(v, 0x30, &[]);
            }
            h.set_property(s.buffer, 0x10, 3, 0);
            h.call(s.holder, 0x14, &[u64::from(s.id), u64::from(s.volume), 512, 0]);
            self.update_position(id, h);
            self.refresh_volume(id, h);
            if let Some(v) = view {
                h.release(v);
            }
        }
        count
    }
    pub fn should_pause_distance(&self, id: u32, h: &mut impl Host) -> bool {
        let s = &self.instances[&id];
        let model = h.property(s.buffer, 0x39, 0);
        if s.master.is_some() || matches!(model, 0x7DCD_ED2D | 0x0A63_A543) {
            return false;
        }
        let l = h.listener().expect("unchecked listener").0;
        let p = if s.children.is_empty() { s.raw } else { s.average };
        length(std::array::from_fn(|i| p[i] - l[i])) >= f64::from(s.radii[1])
    }
    pub(super) fn distance_policy(&mut self, id: u32, h: &mut impl Host) {
        let s = &self.instances[&id];
        if s.master.is_some() || h.property(s.buffer, 0x33, 0) == 0 || h.property(s.buffer, 0x39, 0) != 0x7DCD_ED3D {
            return;
        }
        let paused = s.flags & 8 != 0;
        let should = self.should_pause_distance(id, h);
        if should && !paused {
            self.instances.get_mut(&id).expect("instance").flags |= 8;
            self.pause(id, true, h);
        }
        if !should && paused {
            self.instances.get_mut(&id).expect("instance").flags &= !8;
            self.resume(id, h);
        }
    }
    pub(super) fn simulation_policy(&mut self, id: u32, h: &mut impl Host) {
        let s = &self.instances[&id];
        let paused = s.flags & 0x10 != 0;
        let should = s.flags & 0x20 != 0 && h.simulation_paused().unwrap_or(false);
        if should && !paused {
            self.instances.get_mut(&id).expect("instance").flags |= 0x10;
            self.pause(id, true, h);
        }
        if !should && paused {
            self.instances.get_mut(&id).expect("instance").flags &= !0x10;
            self.resume(id, h);
        }
    }
}
fn length(v: [f32; 3]) -> f64 {
    let [x, y, z] = v.map(f64::from);
    (z * z + y * y + x * x).sqrt()
}
fn normalize(v: [f32; 3]) -> [f32; 3] {
    let n = length(v);
    v.map(|x| (f64::from(x) / n) as f32)
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f64 {
    f64::from(a[0]) * f64::from(b[0]) + f64::from(a[2]) * f64::from(b[2]) + f64::from(a[1]) * f64::from(b[1])
}
pub fn average_position(d: [f32; 3], l: [f32; 3], entries: &[([f32; 3], i32)], inner: f32) -> [f32; 3] {
    let d = normalize(d);
    let mut weight = 0.0f32;
    let mut sum = [0.0f32; 3];
    let mut nearest = [0.0f32; 3];
    let mut minimum = f32::MAX;
    for &(p, volume) in entries {
        let r = std::array::from_fn(|i| p[i] - l[i]);
        let dist = length(r);
        if dist < f64::from(minimum) {
            minimum = dist as f32;
            nearest = p;
        }
        let a = ((f64::from(r[0]) / dist) * f64::from(d[0])
            + (f64::from(r[2]) / dist) * f64::from(d[2])
            + (f64::from(r[1]) / dist) * f64::from(d[1]))
        .abs() as f32;
        let w = f64::from(volume) / 1024.0 / (1.0 - f64::from(a)) * f64::from(a);
        weight = (f64::from(weight) + w) as f32;
        for i in 0..3 {
            sum[i] = (f64::from(sum[i]) + f64::from(p[i]) * w) as f32;
        }
    }
    let mut a = sum.map(|v| (f64::from(v) / f64::from(weight)) as f32);
    let u = normalize(std::array::from_fn(|i| a[i] - l[i]));
    let c = normalize(std::array::from_fn(|i| nearest[i] - l[i]));
    if dot(u, d).abs() - dot(c, d).abs() > 0.0 {
        let shift = f64::from(minimum.max(inner)) - f64::from(inner);
        for i in 0..3 {
            a[i] = (f64::from(a[i]) + f64::from(u[i]) * shift) as f32;
        }
    }
    a
}

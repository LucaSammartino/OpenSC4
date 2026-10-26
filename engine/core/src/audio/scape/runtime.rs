// SPDX-License-Identifier: GPL-3.0-or-later
use super::{ScapeGlobals, ScapeGroup, ScapeHost, ScapeManager};

#[derive(Clone, Debug)]
pub struct Definition {
    pub min: u32,
    pub max: u32,
    pub extra: u32,
    pub sound: u32,
    pub flag: u8,
    pub f: f32,
}
#[derive(Clone, Debug)]
pub struct Playing {
    pub definition: Definition,
    pub instance: u32,
}
#[derive(Clone, Debug)]
pub struct Member {
    pub key: u32,
    pub position: [f32; 3],
}
#[derive(Clone, Debug)]
pub struct Cluster {
    pub average: [f32; 3],
    pub sum: [f32; 3],
    pub count: u32,
    pub adds: u32,
    pub radius: f32,
    pub members: Vec<Member>,
    pub playing: Vec<Playing>,
    pub countdown: u32,
    pub channels: [i32; 2],
    pub targets: [i32; 2],
    pub cache: i32,
}
impl Cluster {
    fn new(radius: f32) -> Self {
        Self {
            average: [0.0; 3],
            sum: [0.0; 3],
            count: 0,
            adds: 0,
            radius,
            members: Vec::new(),
            playing: Vec::new(),
            countdown: 0,
            channels: [0; 2],
            targets: [1024; 2],
            cache: 0,
        }
    }
    fn add_position(&mut self, p: [f32; 3]) {
        for (i, v) in p.into_iter().enumerate() {
            self.sum[i] = if self.count == 0 { v } else { self.sum[i] + v };
            self.average[i] = (f64::from(self.sum[i]) / f64::from(self.count.wrapping_add(1))) as f32;
        }
        self.count = self.count.wrapping_add(1);
    }
    fn subtract(&mut self, p: [f32; 3]) {
        for (i, v) in p.into_iter().enumerate() {
            self.sum[i] -= v;
            self.average[i] = (f64::from(self.sum[i]) / f64::from(self.count)) as f32;
        }
        self.count = self.count.wrapping_sub(1);
    }
}
fn distance(a: [f32; 3], b: [f32; 3]) -> f64 {
    a.into_iter()
        .zip(b)
        .map(|(x, y)| (f64::from(x) - f64::from(y)).powi(2))
        .sum::<f64>()
        .sqrt()
}
fn post(g: &ScapeGlobals, h: &mut impl ScapeHost, event: u32, args: [u32; 4]) {
    if let Some(system) = g.sound_system {
        h.sound_event(system, event, args);
    }
}
fn stop(c: &mut Cluster, g: &ScapeGlobals, h: &mut impl ScapeHost) {
    for p in c.playing.drain(..) {
        post(g, h, 0x6A45_1908, [p.instance, 0, 0, 0]);
    }
}
fn choose(
    c: &mut Cluster,
    defs: &[Definition],
    temporary: Option<&Definition>,
    next: &mut u32,
    g: &ScapeGlobals,
    h: &mut impl ScapeHost,
) {
    for d in temporary.into_iter().chain(defs) {
        let index = c.playing.iter().position(|p| p.definition.sound == d.sound);
        if d.min <= c.members.len() as u32 && c.members.len() as u32 <= d.max {
            if index.is_none() {
                // SPEC-GAP: portable instance tokens replace private-copy addresses at the foreign boundary.
                let instance = *next;
                *next = next.wrapping_add(1);
                c.cache = c.channels[0].wrapping_mul(c.channels[1]) / 1024;
                c.playing.push(Playing {
                    definition: d.clone(),
                    instance,
                });
                post(g, h, 0xEA45_1903, [d.sound, instance, 0, 0]);
                post(g, h, 0x4A45_1915, [instance, c.cache as u32, 0, 0]);
            }
        } else if let Some(i) = index {
            let p = c.playing.remove(i);
            post(g, h, 0x6A45_1908, [p.instance, 0, 0, 0]);
        }
    }
    for p in &c.playing {
        post(
            g,
            h,
            0xCA45_190F,
            [
                p.instance,
                crate::x87::ftol32(f64::from(c.average[0])) as u32,
                crate::x87::ftol32(f64::from(c.average[1])) as u32,
                crate::x87::ftol32(f64::from(c.average[2])) as u32,
            ],
        );
    }
}
fn insert(
    s: &mut ScapeGroup,
    member: Option<Member>,
    position: [f32; 3],
    temporary: Option<&Definition>,
    next: &mut u32,
    g: &ScapeGlobals,
    h: &mut impl ScapeHost,
) {
    let mut best = None;
    let mut nearest = f64::from(f32::MAX);
    for (i, c) in s.clusters.iter().enumerate() {
        let d = distance(position, c.average);
        if d <= f64::from(c.radius) && d < nearest {
            best = Some(i);
            nearest = d;
        }
    }
    let i = best.unwrap_or_else(|| {
        s.clusters.push(Cluster::new(s.radius));
        s.clusters.len() - 1
    });
    let c = &mut s.clusters[i];
    if let Some(member) = member {
        c.members.push(member);
    }
    c.add_position(position);
    c.adds = c.adds.wrapping_add(1);
    choose(c, &s.definitions, temporary, next, g, h);
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryFault {
    NativeGridDomain,
    NativeNontermination,
}
impl ScapeManager {
    pub fn create_scape(&mut self, name: &str, id: u32, flags: u32, radius: f32) -> bool {
        if self.groups.contains_key(&id) {
            return false;
        }
        self.register_group(
            id,
            ScapeGroup {
                resource: 0,
                flags,
                lifetime: 0,
                instances: Vec::new(),
                name: name.into(),
                radius,
                total: 0,
                definitions: Vec::new(),
                clusters: Vec::new(),
            },
        );
        if flags & 1 != 0 {
            self.add_sound(id, id, 0, 0, 1, 0x7FFF_FFFF);
        }
        true
    }
    pub fn add_sound(&mut self, id: u32, sound: u32, flag: u32, extra: u32, min: u32, max: u32) -> bool {
        let Some(s) = self.groups.get_mut(&id) else {
            return false;
        };
        if s.definitions.iter().any(|d| d.sound == sound) {
            return false;
        }
        s.definitions.push(Definition {
            min,
            max,
            extra,
            sound,
            flag: flag as u8,
            f: 50.0,
        });
        true
    }
    pub fn remove_sound(&mut self, id: u32, sound: u32) -> bool {
        let Some(s) = self.groups.get_mut(&id) else {
            return false;
        };
        let Some(i) = s.definitions.iter().position(|d| d.sound == sound) else {
            return false;
        };
        s.definitions.remove(i);
        true
    }
    #[allow(clippy::too_many_arguments)]
    pub fn add_emanation(
        &mut self,
        id: u32,
        sound: u32,
        flag: u32,
        key: u32,
        p: [f32; 3],
        f: f32,
        g: &ScapeGlobals,
        h: &mut impl ScapeHost,
    ) -> bool {
        if !self.groups.contains_key(&id) {
            self.create_scape("nameless", id, 1, 50.0);
            return self.add_emanation(id, 0, 0, key, p, f, g, h);
        }
        let s = self.groups.get_mut(&id).expect("scape");
        let tracked = s.flags & 4 == 0;
        if tracked && s.clusters.iter().any(|c| c.members.iter().any(|m| m.key == key)) {
            return false;
        }
        let d = Definition {
            min: 1,
            max: 0x7FFF_FFFF,
            extra: 0,
            sound,
            flag: flag as u8,
            f,
        };
        insert(
            s,
            tracked.then_some(Member { key, position: p }),
            p,
            (sound != 0).then_some(&d),
            &mut self.next_instance,
            g,
            h,
        );
        s.total = s.total.wrapping_add(1);
        true
    }
    pub fn remove_emanation(&mut self, id: u32, key: u32, g: &ScapeGlobals, h: &mut impl ScapeHost) -> bool {
        let Some(s) = self.groups.get_mut(&id) else {
            return false;
        };
        for i in 0..s.clusters.len() {
            let c = &mut s.clusters[i];
            if let Some(j) = c.members.iter().position(|m| m.key == key) {
                let m = c.members.remove(j);
                c.subtract(m.position);
                s.total = s.total.wrapping_sub(1);
                choose(c, &s.definitions, None, &mut self.next_instance, g, h);
                if c.members.is_empty() && g.sound_system.is_some() {
                    stop(c, g, h);
                    s.clusters.remove(i);
                }
                return true;
            }
        }
        false
    }
    pub fn move_emanation(
        &mut self,
        _id: u32,
        key: u32,
        p: [f32; 3],
        radius: f32,
        g: &ScapeGlobals,
        h: &mut impl ScapeHost,
    ) -> bool {
        for id in &self.order {
            let s = self.groups.get_mut(id).expect("scape");
            for i in 0..s.clusters.len() {
                let c = &mut s.clusters[i];
                if let Some(j) = c.members.iter().position(|m| m.key == key) {
                    c.subtract(c.members[j].position);
                    c.add_position(p);
                    c.members[j].position = p;
                    choose(c, &s.definitions, None, &mut self.next_instance, g, h);
                    if distance(p, c.average) <= f64::from(radius) + f64::from(c.radius) {
                        return true;
                    }
                    let member = c.members.remove(j);
                    if c.members.is_empty() && g.sound_system.is_some() {
                        stop(c, g, h);
                        s.clusters.remove(i);
                    }
                    insert(s, Some(member), p, None, &mut self.next_instance, g, h);
                    return true;
                }
            }
        }
        true
    }
    pub fn clear_scape(&mut self, id: u32, g: &ScapeGlobals, h: &mut impl ScapeHost) -> bool {
        let Some(s) = self.groups.get_mut(&id) else {
            return false;
        };
        s.clusters.retain_mut(|c| {
            let keep = s.flags & 0x10 != 0 && c.count != 0;
            c.members.clear();
            c.count = 0;
            if !keep {
                stop(c, g, h);
            }
            keep
        });
        s.total = 0;
        true
    }
    pub fn clear_flags(
        &mut self,
        mask: u32,
        keep: bool,
        g: &ScapeGlobals,
        h: &mut impl ScapeHost,
    ) -> Result<bool, QueryFault> {
        for id in &self.order {
            let s = self.groups.get_mut(id).expect("scape");
            if s.flags & mask != mask {
                continue;
            }
            for c in &mut s.clusters {
                c.members.clear();
                c.count = 0;
                if !keep {
                    if g.sound_system.is_none() {
                        return Err(QueryFault::NativeNontermination);
                    }
                    stop(c, g, h);
                }
            }
            if !keep {
                s.clusters.clear();
            }
        }
        Ok(true)
    }
    pub fn on_tick(&mut self, now: u32, g: &ScapeGlobals, h: &mut impl ScapeHost) -> bool {
        let dt = now.wrapping_sub(self.elapsed);
        self.elapsed = now;
        let mut i = 0;
        while i < self.pending.len() {
            let Some(c) = self.pending[i].as_mut() else {
                return false;
            };
            c.countdown = c.countdown.saturating_sub(dt);
            if c.countdown == 0 {
                stop(c, g, h);
                self.pending.remove(i);
            } else {
                i += 1;
            }
        }
        for id in &self.order {
            for c in &mut self.groups.get_mut(id).expect("scape").clusters {
                for i in 0..2 {
                    let d = c.targets[i].wrapping_sub(c.channels[i]);
                    if d != 0 {
                        let step = if d / 7 == 0 { d } else { d / 7 };
                        c.channels[i] = c.channels[i].wrapping_add(step);
                        for p in &c.playing {
                            post(g, h, 0x4A45_1915, [p.instance, c.channels[i] as u32, 0, 0]);
                        }
                    }
                }
            }
        }
        true
    }
    pub fn positions(&self, id: u32, out: &mut [[f32; 3]], count: &mut i32, total: &mut u32) -> bool {
        let Some(s) = self.groups.get(&id) else { return false };
        if s.clusters.len() as i32 > *count {
            return false;
        }
        assert!(out.len() >= s.clusters.len(), "required output storage");
        for (o, c) in out.iter_mut().zip(&s.clusters) {
            *o = c.average;
        }
        *count = s.clusters.len() as i32;
        *total = s.total;
        true
    }
    pub fn grid(
        &self,
        id: u32,
        rect: [f32; 4],
        out: &mut [[f32; 3]],
        count: &mut i32,
        total: &mut u32,
        filter: bool,
    ) -> Result<bool, QueryFault> {
        let Some(s) = self.groups.get(&id) else {
            *count = 0;
            return Ok(false);
        };
        grid(&s.clusters, s.total, rect, out, count, total, filter)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn near(
        &mut self,
        a: u32,
        b: u32,
        rect: [f32; 4],
        out: &mut [[f32; 3]],
        count: &mut i32,
        total: &mut u32,
        filter: bool,
    ) -> Result<bool, QueryFault> {
        let (Some(sa), Some(sb)) = (self.groups.get(&a), self.groups.get(&b)) else {
            *count = 0;
            return Ok(false);
        };
        let radius = f64::from(sa.radius) + f64::from(sb.radius);
        let bs: Vec<_> = sb.clusters.iter().map(|c| c.average).collect();
        let sa = self.groups.get_mut(&a).expect("scape");
        for c in &mut sa.clusters {
            c.adds = 0;
        }
        let mut selected = Vec::new();
        for p in bs {
            for c in &mut sa.clusters {
                if c.adds == 0 && distance(p, c.average) < radius {
                    c.adds = 1;
                    selected.push(c.clone());
                }
            }
        }
        grid(&selected, selected.len() as u32, rect, out, count, total, filter)
    }
    pub fn priority(&self) -> u32 {
        0x8000_0000
    }
    pub fn on_idle(&self) -> bool {
        false
    }
    pub fn destroy(&mut self, h: &mut impl ScapeHost) {
        // Listed scapes are not cleared/stopped; native destruction leaks their contents.
        self.groups.clear();
        self.order.clear();
        self.pending.clear();
        if let Some(p) = self.sound_manager.take() {
            h.release(p);
        }
        if let Some(p) = self.sound_system.take() {
            h.release(p);
        }
    }
    pub fn release(&mut self, h: &mut impl ScapeHost) -> u32 {
        if self.refcount > 1 {
            self.refcount -= 1;
            self.refcount
        } else {
            self.destroy(h);
            self.refcount = 0;
            0
        }
    }
}
fn grid(
    clusters: &[Cluster],
    full_total: u32,
    r: [f32; 4],
    out: &mut [[f32; 3]],
    count: &mut i32,
    total: &mut u32,
    filter: bool,
) -> Result<bool, QueryFault> {
    if clusters.is_empty() {
        *count = 0;
        *total = 0;
        return Ok(false);
    }
    *total = full_total;
    let inside = |p: [f32; 3]| !filter || (p[0] >= r[0] && p[0] <= r[2] && p[2] >= r[1] && p[2] <= r[3]);
    if clusters.len() as i32 <= *count {
        *count = 0;
        for c in clusters.iter().filter(|c| inside(c.average)) {
            out[*count as usize] = c.average;
            *count += 1;
        }
        return Ok(true);
    }
    let n = crate::x87::ftol32(f64::from(*count).sqrt());
    if n <= 0 {
        return Err(QueryFault::NativeGridDomain);
    }
    let w = crate::x87::ftol32((f64::from(r[2]) - f64::from(r[0])) / f64::from(n));
    let h = crate::x87::ftol32((f64::from(r[3]) - f64::from(r[1])) / f64::from(n));
    if w == 0 || h == 0 {
        return Err(QueryFault::NativeGridDomain);
    }
    let size = n
        .checked_mul(n)
        .filter(|v| *v > 0)
        .ok_or(QueryFault::NativeGridDomain)?;
    let mut cells = vec![([0.0f32; 3], 0u32); size as usize];
    for c in clusters.iter().filter(|c| inside(c.average)) {
        let row = crate::x87::ftol32((f64::from(c.average[2]) - f64::from(r[1])) / f64::from(h));
        let col = crate::x87::ftol32((f64::from(c.average[0]) - f64::from(r[0])) / f64::from(w));
        let i = row.wrapping_mul(n).wrapping_add(col);
        if i < size {
            if i < 0 {
                return Err(QueryFault::NativeGridDomain);
            }
            let (sum, num) = &mut cells[i as usize];
            for (s, v) in sum.iter_mut().zip(c.average) {
                *s += v;
            }
            *num = num.wrapping_add(1);
        }
    }
    *count = 0;
    for (i, (sum, num)) in cells.into_iter().enumerate() {
        if num != 0 {
            out[i] = sum.map(|v| (f64::from(v) / f64::from(num as i32)) as f32);
            *count += 1;
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests;

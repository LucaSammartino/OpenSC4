// SPDX-License-Identifier: GPL-3.0-or-later
//! Foreign owner execution, audio §§8.3.1/10.6 (AU46/AU49).
//! Unlike the legacy fixture, these paths do not synthesize successful playback.
use super::owner::{event_route, EventRoute};
use crate::audio::{data::SoundManagerTuning, instance_id::InstanceIdCounter};
use crate::x87::ftol32;
use std::collections::{BTreeMap, BTreeSet};

mod policy;

pub trait Host {
    fn service(&mut self, id: u32, iid: u32) -> Option<u64>;
    fn message_server(&mut self) -> Option<u64>;
    fn retain(&mut self, object: u64);
    fn release(&mut self, object: u64);
    fn call(&mut self, object: u64, slot: u32, arguments: &[u64]) -> u64;
    fn query(&mut self, object: u64, iid: u32) -> Option<u64>;
    fn property(&mut self, object: u64, key: u32, flags: u32) -> u32;
    fn set_property(&mut self, object: u64, key: u32, value: i32, flags: u32) -> bool;
    fn preferences(&mut self) -> (bool, u32);
    fn tuning(&mut self) -> Option<SoundManagerTuning>;
    fn get_sound(&mut self, manager: u64, sound: u32, iid: u32) -> Option<u64>;
    fn allocate_instance(&mut self) -> u64;
    fn construct_instance(&mut self, allocation: u64) -> u64;
    fn create_event(&mut self, system: u64, event: u32, iid: u32, data: [u32; 4]) -> Option<u64>;
    fn payload(&mut self, event: u64) -> Option<u64>; // borrowed
    fn enumerate_system(&mut self, system: u64) -> Vec<Option<u64>>; // borrowed sounds
    fn listener(&mut self) -> Option<([f32; 3], [f32; 3])>;
    fn view_geometry(&mut self) -> Option<([f32; 3], [f32; 3])>;
    fn simulation_paused(&mut self) -> Option<bool>;
}

#[derive(Clone, Debug)]
pub struct Sound {
    pub pointer: u64,
    pub sound: u32,
    pub id: u32,
    pub holder: u64,
    pub buffer: u64,
    pub manager: u64,
    pub volume: u32,
    pub gain: f32,
    pub raw: [f32; 3],
    pub average: [f32; 3],
    pub flags: u32,
    pub pause_count: u32,
    pub current: f32,
    pub target: f32,
    pub rate: f32,
    pub radii: [f32; 2],
    pub master: Option<u32>,
    pub children: Vec<u32>,
    pub timestamp: u32,
}
/// Legacy diagnostic names retained for source compatibility; AU50 emits none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Unsupported {
    StartVoicePolicy,
    LinkedPosition,
    DistancePause,
    SimulationPause,
    Resume,
}

#[derive(Debug)]
pub struct Manager {
    pub refcount: u32,
    pub position_sentinel: f32,
    pub unsupported: BTreeSet<Unsupported>,
    pub pointer: u64,
    pub system: Option<u64>,
    pub sound_manager: Option<u64>,
    pub master_volume: u32,
    pub elapsed: u32,
    pub updates: u32,
    pub countdown: u32,
    pub flags: u8,
    pub one_shots: u32,
    pub independent_loops: u32,
    pub tuning: SoundManagerTuning,
    pub instances: BTreeMap<u32, Sound>,
    pub list: Vec<u32>,
}
pub const SUBSCRIPTIONS: [u32; 7] = [
    0x28E9_397B,
    0x28E9_397C,
    0x26D3_1EC2,
    0x8A4A_69C7,
    0x4348_B111,
    0xAA7F_B7E0,
    0x4A7F_B7E9,
];

impl Manager {
    pub fn add_ref(&mut self) -> u32 {
        self.refcount = self.refcount.wrapping_add(1);
        self.refcount
    }
    pub fn release(&mut self, host: &mut impl Host) -> u32 {
        if (self.refcount as i32) > 1 {
            self.refcount -= 1;
            return self.refcount;
        }
        self.destroy(host);
        self.refcount = 0;
        0
    }
    pub fn construct(pointer: u64) -> Self {
        Self {
            refcount: 0,
            position_sentinel: 0.0,
            unsupported: BTreeSet::new(),
            pointer,
            system: None,
            sound_manager: None,
            master_volume: 1024,
            elapsed: 0,
            updates: 0,
            countdown: 0,
            flags: 0,
            one_shots: 0,
            independent_loops: 0,
            tuning: SoundManagerTuning::default(),
            instances: BTreeMap::new(),
            list: Vec::new(),
        }
    }
    pub fn query_interface(&mut self, iid: u32) -> bool {
        if !matches!(iid, 1 | 0x4A47_DC0D) {
            return false;
        }
        self.refcount = self.refcount.wrapping_add(1);
        true
    }
    fn acquire(holder: &mut Option<u64>, value: u64, host: &mut impl Host) {
        host.retain(value);
        if let Some(old) = holder.replace(value) {
            host.release(old);
        }
        host.release(value);
    }
    pub fn init(&mut self, host: &mut impl Host) -> bool {
        let Some(system) = host.service(0x23CE_CBA0, 0x1060_77D2) else {
            return false;
        };
        Self::acquire(&mut self.system, system, host);
        let Some(manager) = host.service(0x1AF8_5BFB, 0x3AF8_5BB7) else {
            return false;
        };
        Self::acquire(&mut self.sound_manager, manager, host);
        self.tuning = host.tuning().unwrap_or_default();
        host.call(system, 0x3C, &[self.pointer]);
        host.call(system, 0x78, &[self.pointer + 4, 20]);
        if let Some(messages) = host.message_server() {
            for m in SUBSCRIPTIONS {
                host.call(messages, 0x14, &[self.pointer + 12, u64::from(m)]);
            }
        }
        let (enabled, volume) = host.preferences();
        self.master_volume = if enabled { volume } else { 0 };
        self.update_all(host);
        true
    }
    pub fn shutdown(&mut self, host: &mut impl Host) -> bool {
        self.post(0xCAA4_4CA7, [1, 0, 0, 0], host);
        if let Some(system) = self.system {
            host.call(system, 0xBC, &[]);
        }
        if let Some(messages) = host.message_server() {
            for m in SUBSCRIPTIONS.into_iter().rev() {
                host.call(messages, 0x18, &[self.pointer + 12, u64::from(m)]);
            }
        }
        if let Some(system) = self.system.take() {
            host.call(system, 0x7C, &[self.pointer + 4]);
            host.call(system, 0x40, &[self.pointer]);
            host.release(system);
        }
        if let Some(manager) = self.sound_manager.take() {
            host.release(manager);
        }
        true
    }
    pub fn destroy(&mut self, host: &mut impl Host) {
        for (_, s) in std::mem::take(&mut self.instances) {
            host.release(s.buffer);
            host.release(s.holder);
            host.release(s.manager);
            host.release(s.pointer);
        }
        self.list.clear();
        if let Some(system) = self.system.take() {
            host.release(system);
        }
        if let Some(manager) = self.sound_manager.take() {
            host.release(manager);
        }
    }
    pub fn post(&self, event: u32, data: [u32; 4], host: &mut impl Host) -> bool {
        let Some(system) = self.system else {
            return false;
        };
        let Some(event) = host.create_event(system, event, 0x5B52_3BA4, data) else {
            return false;
        };
        host.call(system, 0x20, &[event]);
        host.release(event);
        true
    }
    pub fn completion(sound: &Sound, enabled: bool, host: &mut impl Host) -> bool {
        let Some(view) = host.query(sound.holder, 0xE9D0_0A9A) else {
            return false;
        };
        host.call(view, 0x0C, &[if enabled { sound.manager + 8 } else { 0 }]);
        host.release(view);
        true
    }
    pub fn start(
        &mut self,
        sound: u32,
        mut id: u32,
        flags: u32,
        ids: &mut InstanceIdCounter,
        host: &mut impl Host,
    ) -> bool {
        if self.instances.contains_key(&id) {
            self.stop(id, true, host);
        }
        if id == 0 {
            id = ids.generate();
        }
        let (admitted, matching) = self.admission(sound, host);
        if !admitted {
            return false;
        }
        let Some(manager) = self.sound_manager else {
            return false;
        };
        let Some(holder) = host.get_sound(manager, sound, 0x2060_77D2) else {
            return false;
        };
        let Some(buffer) = host.query(holder, 0xFD58_4467) else {
            host.release(holder);
            return false;
        };
        let allocation = host.allocate_instance();
        assert_ne!(allocation, 0, "required pool allocation");
        let pointer = host.construct_instance(allocation);
        assert_ne!(pointer, 0, "required instance constructor");
        let radius = f64::from(host.property(buffer, 0x38, 0) as i32) * f64::from(0.01f32) * f64::from(100.0f32);
        host.retain(self.pointer);
        let mut s = Sound {
            pointer,
            sound,
            id,
            holder,
            buffer,
            manager: self.pointer,
            volume: 0,
            gain: 1.0,
            raw: [self.position_sentinel; 3],
            average: [self.position_sentinel; 3],
            flags: if flags & 1 != 0 { 0x20 } else { 0 },
            pause_count: 0,
            current: 0.0,
            target: 0.0,
            rate: 0.0,
            radii: [radius as f32, (radius * f64::from(4.0f32)) as f32],
            master: None,
            children: Vec::new(),
            timestamp: self.elapsed,
        };
        Self::completion(&s, true, host);
        let master = self.list.iter().copied().find(|id| {
            let s = &self.instances[id];
            s.sound == sound && s.master.is_none()
        });
        if let Some(master) = master {
            let m = &self.instances[&master];
            let master_pointer = m.pointer;
            host.retain(master_pointer);
            if host.property(m.buffer, 0x33, 0) != 0 {
                s.master = Some(master);
                self.instances.get_mut(&master).expect("master").children.push(id);
            }
            host.release(master_pointer);
        }
        if s.master.is_none() {
            host.set_property(buffer, 0x10, 3, 0);
        }
        if s.master.is_none() && host.call(holder, 0x14, &[u64::from(id), u64::from(self.master_volume), 512, 0]) == 0 {
            host.release(buffer);
            host.release(holder);
            host.release(self.pointer);
            host.release(pointer);
            return false;
        }
        host.retain(pointer); // container reference; constructor temporary is dropped below
        let one_shot = host.property(buffer, 0x33, 0) == 0;
        if s.master.is_none() && one_shot {
            self.one_shots = self.one_shots.wrapping_add(1);
        } else if s.master.is_none() {
            self.independent_loops = self.independent_loops.wrapping_add(1);
        }
        self.instances.insert(id, s);
        self.list.push(id);
        if matching != 0 && one_shot {
            let gain = (self.curve(matching as i32 as f32) / 1024.0) as f32;
            for i in self.list.clone() {
                if self.instances[&i].sound == sound {
                    self.instances.get_mut(&i).expect("instance").gain = gain;
                    self.refresh_volume(i, host);
                }
            }
        }
        self.simulation_policy(id, host);
        host.release(pointer);
        true
    }
    pub fn stop(&mut self, id: u32, hard: bool, host: &mut impl Host) -> bool {
        let Some(mut s) = self.instances.remove(&id) else {
            return false;
        };
        self.list.retain(|i| *i != id);
        host.retain(s.pointer);
        let looping = host.property(s.buffer, 0x33, 0) != 0;
        if s.children.is_empty() {
            Self::completion(&s, false, host);
            if host.call(s.holder, 0x1C, &[]) != 0 {
                if hard {
                    host.call(s.holder, 0x24, &[]);
                } else if looping {
                    host.call(s.holder, 0x20, &[]);
                }
            }
        } else {
            let first = s.children.remove(0);
            if let Some(child) = self.instances.get_mut(&first) {
                child.master = None;
                child.children = std::mem::take(&mut s.children);
                if child.holder != s.holder {
                    host.retain(s.holder);
                    host.release(child.holder);
                }
                if child.buffer != s.buffer {
                    host.retain(s.buffer);
                    host.release(child.buffer);
                }
                child.holder = s.holder;
                child.buffer = s.buffer;
            }
            let children = self
                .instances
                .get(&first)
                .map(|s| s.children.clone())
                .unwrap_or_default();
            for id in children {
                if let Some(child) = self.instances.get_mut(&id) {
                    child.master = Some(first);
                }
            }
            host.release(s.holder);
            host.release(s.buffer);
            s.holder = 0;
            s.buffer = 0;
            self.update_position(first, host);
        }
        if let Some(master) = s.master {
            if let Some(master) = self.instances.get_mut(&master) {
                master.children.retain(|c| *c != id);
            }
        }
        if !looping {
            self.one_shots = self.one_shots.wrapping_sub(1);
        } else if s.master.is_none() {
            self.independent_loops = self.independent_loops.wrapping_sub(1);
        }
        if s.buffer != 0 {
            host.release(s.buffer);
        }
        if s.holder != 0 {
            host.release(s.holder);
        }
        host.release(s.manager);
        host.release(s.pointer);
        host.release(s.pointer);
        true
    }
    pub fn completed(&mut self, object: u64, host: &mut impl Host) {
        if let Some(view) = host.query(object, 0xFD58_4467) {
            let id = host.property(view, 0x34, 0);
            self.stop(id, true, host);
            host.release(view);
        }
    }
    pub fn reset(&mut self, id: u32, host: &mut impl Host) -> bool {
        let Some(s) = self.instances.get(&id) else {
            return false;
        };
        host.retain(s.pointer);
        if let Some(reset) = host.query(s.holder, 0xBB1B_E367) {
            host.call(reset, 0x30, &[]);
            if let Some(buffer) = host.query(reset, 0xFD58_4467) {
                let v = host.property(buffer, 0x15, 0);
                host.set_property(buffer, 0x15, v as i32, 0);
                host.release(buffer);
            }
            host.release(reset);
        }
        host.release(s.pointer);
        true
    }
    fn write_volume(s: &Sound, host: &mut impl Host) {
        host.set_property(
            s.buffer,
            0x15,
            ftol32(f64::from(s.volume as i32) * f64::from(s.gain)),
            0,
        );
    }
    fn write_position(s: &Sound, host: &mut impl Host) {
        let p = if s.children.is_empty() { s.raw } else { s.average };
        for (i, v) in p.into_iter().enumerate() {
            host.set_property(
                s.buffer,
                0x20 + i as u32,
                ftol32(f64::from(v)),
                if i < 2 { 1 } else { 0 },
            );
        }
    }
    pub fn volume(&mut self, id: u32, volume: u32, host: &mut impl Host) -> bool {
        let Some(s) = self.instances.get_mut(&id) else {
            return false;
        };
        s.volume = volume;
        self.refresh_volume(id, host);
        self.instances.get_mut(&id).expect("instance").flags |= 1;
        true
    }
    fn refresh_volume(&mut self, id: u32, host: &mut impl Host) {
        let s = &self.instances[&id];
        Self::write_volume(s, host);
        if !s.children.is_empty() || s.master.is_some() {
            self.update_position(id, host);
        }
    }
    pub fn position(&mut self, id: u32, position: [f32; 3], host: &mut impl Host) -> bool {
        let Some(s) = self.instances.get_mut(&id) else {
            return false;
        };
        host.retain(s.pointer);
        s.raw = position;
        let master = s.master;
        let pointer = s.pointer;
        self.update_position(id, host);
        if let Some(master) = master {
            self.update_position(master, host);
        }
        self.distance_policy(id, host);
        self.instances.get_mut(&id).expect("instance").flags |= 2;
        host.release(pointer);
        true
    }
    pub fn focus(&mut self, gain: bool, host: &mut impl Host) -> bool {
        if self.system.is_none() {
            return false;
        }
        for id in self.list.clone() {
            if gain {
                self.resume(id, host);
            } else {
                let s = self.instances.get_mut(&id).expect("list entry");
                s.pause_count = s.pause_count.wrapping_add(1);
                if s.pause_count == 1 && s.master.is_none() {
                    Self::completion(s, false, host);
                    host.call(s.holder, 0x24, &[]);
                }
            }
        }
        if gain {
            let (enabled, volume) = host.preferences();
            self.master_volume = if enabled { volume } else { 0 };
            self.countdown = 50;
        } else {
            self.master_volume = 0;
        }
        true
    }
    pub fn focus_message(&self, message: u64, host: &mut impl Host) -> bool {
        if self.system.is_none() {
            return false;
        }
        let Some(view) = host.query(message, 0x6529_7976) else {
            return false;
        };
        let gain = host.call(view, 0x24, &[]) != 0;
        let result = self.post(0x6A95_CBA2, [u32::from(gain), 0, 0, 0], host);
        host.release(view);
        result
    }
    pub fn enumerate(&self, payload: u64, host: &mut impl Host) -> bool {
        let object = host.call(payload, 0x80, &[]); // acquired output
        if object == 0 {
            return false;
        }
        let Some(callback) = host.query(object, 0xEA4F_C425) else {
            host.release(object);
            return false;
        };
        for id in &self.list {
            if host.call(callback, 0x0C, &[self.instances[id].pointer]) == 0 {
                break;
            }
        }
        host.release(callback);
        host.release(object);
        true
    }
    pub fn update_all(&mut self, host: &mut impl Host) -> bool {
        for id in self.list.clone() {
            let s = &self.instances[&id];
            Self::write_volume(s, host);
            if s.master.is_some() || !s.children.is_empty() {
                self.update_position(id, host);
            }
            self.distance_policy(id, host);
            self.simulation_policy(id, host);
        }
        true
    }
    pub fn do_message(&mut self, message: Option<(u32, u64)>, host: &mut impl Host) -> bool {
        let Some((kind, pointer)) = message else {
            return false;
        };
        match kind {
            0x28E9_397B => true,
            0x28E9_397C | 0x26D3_1EC2 => self.post(0xCAA4_4CA7, [0; 4], host),
            0x4348_B111 => self.focus_message(pointer, host),
            0x4A7F_B7E9 | 0x8A4A_69C7 | 0xAA7F_B7E0 => self.post(0x4A8C_52C8, [0; 4], host),
            0xC53D_10AA => {
                self.tuning = host.tuning().unwrap_or_default();
                true
            }
            _ => false,
        }
    }
    pub fn update(&mut self, dt: u32, host: &mut impl Host) {
        self.elapsed = self.elapsed.wrapping_add(dt);
        if self.countdown != 0 {
            self.countdown = (self.countdown as i32).min(50).wrapping_sub(1) as u32;
            if let Some(system) = self.system {
                for sound in host.enumerate_system(system).into_iter().flatten() {
                    if let Some(view) = host.query(sound, 0x2060_77D2) {
                        let saved = host.call(view, 0x50, &[]);
                        host.call(view, 0x54, &[0]);
                        host.call(view, 0x54, &[saved]);
                        host.release(view);
                    }
                }
            }
        }
        self.updates = self.updates.wrapping_add(1);
        for id in self.list.clone() {
            let s = self.instances.get_mut(&id).expect("list entry");
            if s.current != s.target {
                let difference = f64::from(s.target) - f64::from(s.current);
                let step = f64::from(dt as f32) * f64::from(s.rate);
                let step = if difference > 0.0 {
                    step.min(difference)
                } else {
                    step.max(difference)
                };
                s.current = (f64::from(s.current) + step) as f32;
                s.volume = ftol32(f64::from(s.current)) as u32;
                Self::write_volume(s, host);
                if s.master.is_some() || !s.children.is_empty() {
                    self.update_position(id, host);
                }
            }
        }
        self.updates = self.updates.wrapping_sub(1);
    }
    pub fn do_event(
        &mut self,
        event: Option<u64>,
        kind: u32,
        ids: &mut InstanceIdCounter,
        host: &mut impl Host,
    ) -> bool {
        let route = event_route(kind);
        if route == EventRoute::Ignore {
            return false;
        }
        if route == EventRoute::UpdateAll {
            return self.update_all(host);
        }
        let payload = event.and_then(|e| host.payload(e));
        if route == EventRoute::Focus && self.system.is_none() {
            return false;
        }
        if matches!(
            route,
            EventRoute::Reset | EventRoute::Volume | EventRoute::Focus | EventRoute::Start | EventRoute::Position
        ) && payload.is_none()
        {
            return false;
        }
        let p = payload.expect("required event payload");
        match route {
            EventRoute::Reset => {
                host.retain(p);
                let id = host.call(p, 0x24, &[]) as u32;
                self.reset(id, host);
                host.release(p);
                true
            }
            EventRoute::Volume => {
                host.retain(p);
                let v = host.call(p, 0x38, &[]) as u32;
                let id = host.call(p, 0x24, &[]) as u32;
                self.volume(id, v, host);
                host.release(p);
                true
            }
            EventRoute::Focus => {
                host.retain(p);
                let gain = host.call(p, 0x24, &[]) != 0;
                self.focus(gain, host);
                host.release(p);
                false
            }
            EventRoute::StopAll => {
                let hard = host.call(p, 0x24, &[]) != 0;
                while let Some(id) = self.list.first().copied() {
                    self.stop(id, hard, host);
                }
                false
            }
            EventRoute::Enumerate => self.enumerate(p, host),
            EventRoute::Start => {
                host.retain(p);
                let flags = if host.call(p, 0x48, &[]) != 0 {
                    host.call(p, 0x4C, &[]) as u8 as u32
                } else {
                    0
                };
                let id = host.call(p, 0x38, &[]) as u32;
                let sound = host.call(p, 0x24, &[]) as u32;
                self.start(sound, id, flags, ids, host);
                host.release(p);
                true
            }
            EventRoute::Stop => {
                let id = host.call(p, 0x24, &[]) as u32;
                let hard = host.call(p, 0x38, &[]) != 0;
                self.stop(id, hard, host);
                true
            }
            EventRoute::Position => {
                host.retain(p);
                let z = host.call(p, 0x60, &[]) as i32 as f32;
                let y = host.call(p, 0x4C, &[]) as i32 as f32;
                let x = host.call(p, 0x38, &[]) as i32 as f32;
                let id = host.call(p, 0x24, &[]) as u32;
                self.position(id, [x, y, z], host);
                host.release(p);
                true
            }
            EventRoute::UpdateAll | EventRoute::Ignore => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod au50;
    #[derive(Default)]
    struct Trace {
        calls: Vec<(u64, u32, Vec<u64>)>,
        writes: Vec<(u64, u32, i32, u32)>,
        releases: Vec<u64>,
        retains: Vec<u64>,
        missing_manager: bool,
        play: bool,
        payload: bool,
        callbacks: usize,
        enumeration: usize,
        data: BTreeMap<u32, u64>,
        looping: bool,
        model: u32,
        paused: bool,
    }
    impl Host for Trace {
        fn listener(&mut self) -> Option<([f32; 3], [f32; 3])> {
            Some(([0.0; 3], [0.0, 0.0, 1.0]))
        }
        fn view_geometry(&mut self) -> Option<([f32; 3], [f32; 3])> {
            None
        }
        fn simulation_paused(&mut self) -> Option<bool> {
            Some(self.paused)
        }
        fn service(&mut self, id: u32, _: u32) -> Option<u64> {
            if id == 0x23CE_CBA0 {
                Some(100)
            } else {
                (!self.missing_manager).then_some(200)
            }
        }
        fn message_server(&mut self) -> Option<u64> {
            None
        }
        fn retain(&mut self, o: u64) {
            self.retains.push(o);
        }
        fn release(&mut self, o: u64) {
            self.releases.push(o);
        }
        fn call(&mut self, o: u64, s: u32, a: &[u64]) -> u64 {
            self.calls.push((o, s, a.to_vec()));
            if o == 10 && s == 0x14 {
                return u64::from(self.play);
            }
            if o == 10 && s == 0x1C {
                return 1;
            }
            if o == 31 && s == 0x0C {
                self.callbacks += 1;
                return u64::from(self.callbacks < 2);
            }
            if o == 50 && s == 0x80 {
                return 60;
            }
            if o == 50 {
                return *self.data.get(&s).unwrap_or(&0);
            }
            1
        }
        fn query(&mut self, _: u64, iid: u32) -> Option<u64> {
            match iid {
                0xFD58_4467 => Some(20),
                0xE9D0_0A9A => Some(30),
                0xEA4F_C425 => Some(31),
                0x2060_77D2 => Some(10),
                _ => None,
            }
        }
        fn property(&mut self, _: u64, key: u32, _: u32) -> u32 {
            if key == 0x38 {
                7
            } else if key == 0x33 {
                u32::from(self.looping)
            } else if key == 0x39 {
                self.model
            } else if key == 0x15 {
                1024
            } else {
                0
            }
        }
        fn set_property(&mut self, o: u64, k: u32, v: i32, f: u32) -> bool {
            self.writes.push((o, k, v, f));
            false
        }
        fn preferences(&mut self) -> (bool, u32) {
            (true, 900)
        }
        fn tuning(&mut self) -> Option<SoundManagerTuning> {
            None
        }
        fn get_sound(&mut self, _: u64, _: u32, iid: u32) -> Option<u64> {
            assert_eq!(iid, 0x2060_77D2);
            Some(10)
        }
        fn allocate_instance(&mut self) -> u64 {
            40
        }
        fn construct_instance(&mut self, _: u64) -> u64 {
            40
        }
        fn create_event(&mut self, _: u64, _: u32, iid: u32, _: [u32; 4]) -> Option<u64> {
            assert_eq!(iid, 0x5B52_3BA4);
            Some(70)
        }
        fn payload(&mut self, _: u64) -> Option<u64> {
            self.payload.then_some(50)
        }
        fn enumerate_system(&mut self, _: u64) -> Vec<Option<u64>> {
            self.enumeration += 1;
            vec![None, Some(10)]
        }
    }
    fn manager() -> Manager {
        let mut m = Manager::construct(1000);
        m.system = Some(100);
        m.sound_manager = Some(200);
        m
    }
    #[test]
    fn au46_partial_init_null_and_unknown_payloads_and_enumeration() {
        let mut m = Manager::construct(1000);
        let mut h = Trace {
            missing_manager: true,
            ..Trace::default()
        };
        assert!(!m.init(&mut h));
        assert_eq!(m.system, Some(100));
        assert_eq!(m.sound_manager, None);
        m.shutdown(&mut h);
        assert_eq!(m.system, None);
        assert_eq!(h.releases.iter().filter(|p| **p == 100).count(), 2);
        let mut ids = InstanceIdCounter::new();
        assert!(!m.do_event(None, 0x4A4D_35E0, &mut ids, &mut h));
        assert!(!m.do_event(Some(1), 0x4A45_1915, &mut ids, &mut h));
        h.payload = true;
        assert!(m.do_event(Some(1), 0x4A4D_35E0, &mut ids, &mut h));
        assert!(m.do_event(Some(1), 0x4A45_1915, &mut ids, &mut h));
        m = manager();
        h.play = true;
        for id in 1..=3 {
            m.elapsed += 50;
            assert!(m.start(8, id, 0, &mut ids, &mut h));
        }
        assert!(m.enumerate(50, &mut h));
        assert_eq!(h.callbacks, 2);
    }
    #[test]
    fn au49_play_failure_property_radii_completion_soft_stop_and_position() {
        let mut m = manager();
        let mut h = Trace::default();
        let mut ids = InstanceIdCounter::new();
        assert!(!m.start(8, 1, 255, &mut ids, &mut h));
        assert!(m.instances.is_empty());
        assert!(m.list.is_empty());
        h.play = true;
        assert!(m.start(8, 1, 255, &mut ids, &mut h));
        let s = &m.instances[&1];
        let radius = f64::from(7) * f64::from(0.01f32) * 100.0;
        assert_eq!(s.radii, [radius as f32, (radius * 4.0) as f32]);
        assert_eq!(s.flags, 0x20);
        assert!(h.calls.contains(&(10, 0x14, vec![1, 1024, 512, 0])));
        assert!(h.calls.contains(&(30, 0x0C, vec![1008])));
        assert!(!m.position(99, [0.0; 3], &mut h));
        assert!(m.position(1, [-1.8, 2.9, 3.1], &mut h));
        assert_eq!(
            &h.writes[h.writes.len() - 3..],
            [(20, 0x20, -1, 1), (20, 0x21, 2, 1), (20, 0x22, 3, 0)]
        );
        h.calls.clear();
        assert!(m.stop(1, false, &mut h));
        assert!(!h.calls.iter().any(|(_, slot, _)| matches!(slot, 0x20 | 0x24)));
        assert!(h.calls.contains(&(30, 0x0C, vec![0])));
        assert!(!m.stop(1, true, &mut h));
    }
    #[test]
    fn au46_signed_volume_two_stage_ramp_and_fifty_focus_callbacks() {
        let mut m = manager();
        let mut h = Trace {
            play: true,
            payload: true,
            ..Trace::default()
        };
        let mut ids = InstanceIdCounter::new();
        m.start(8, 1, 0, &mut ids, &mut h);
        m.instances.get_mut(&1).unwrap().gain = 0.5;
        assert!(m.volume(1, (-3i32) as u32, &mut h));
        assert_eq!(m.instances[&1].volume, (-3i32) as u32);
        assert_eq!(h.writes.last(), Some(&(20, 0x15, -1, 0)));
        assert_eq!(m.instances[&1].flags & 1, 1);
        let s = m.instances.get_mut(&1).unwrap();
        s.current = 3.9;
        s.target = 5.0;
        s.rate = 0.01;
        m.update(1, &mut h);
        assert!((m.instances[&1].current - 3.91).abs() < 0.00001);
        assert_eq!(h.writes.last(), Some(&(20, 0x15, 1, 0)));
        h.data.insert(0x24, 1);
        assert!(!m.do_event(Some(1), 0x6A95_CBA2, &mut ids, &mut h));
        assert_eq!(m.countdown, 50);
        for _ in 0..51 {
            m.update(1, &mut h);
        }
        assert_eq!(h.enumeration, 50);
        assert_eq!(m.countdown, 0);
        assert_eq!(m.updates, 0);
        assert!(m.unsupported.is_empty());
    }
}

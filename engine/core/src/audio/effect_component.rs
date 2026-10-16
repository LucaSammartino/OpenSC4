// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The effects system's sound component (`docs/specs/audio.md` §9).
//! Section references (`§n`) point at that spec.
//!
//! The effect component type is **7** (§9). Its description class is
//! [`SoundDescription`](crate::audio::data::SoundDescription); `GetType` is
//! `MOV EAX, 7; RET`, which is what makes it type 7. The runtime component's
//! vtable is `0x00AA1118` and its object size is `0x30` (§9.1).
//!
//! The component speaks to **`cIGZSndSys`** (global `0x00B43CB8`), **not** to
//! the message server (§9.5). AEMS is not on this path.
//!
//! **The instance handle is the component's own `this` pointer** — `Start`
//! sets `+0x20 = this` and every later event carries that value as the key
//! (§9.2).

use crate::audio::data::SoundDescription;
use crate::audio::{EVT_SET_POSITION, EVT_START_SOUND, EVT_STOP_SOUND, RUNTIME_FLAG_IGNORED, SOUND_COMPONENT_SIZE};

/// The effects component type of `cSC4SoundDescription::GetType` (§9).
pub const EFFECT_COMPONENT_TYPE: u32 = crate::audio::EFFECT_TYPE_SOUND;

/// The object size of the runtime component (§9.1): `0x30` = 48 bytes.
pub const SOUND_COMPONENT_OBJECT_SIZE: usize = SOUND_COMPONENT_SIZE;

/// The nine-slot vtable `0x00AA1118` (§9.1). Slots 5 and 6 share a no-op.
pub const VTABLE_SOUND_COMPONENT: u32 = 0x00AA_1118;

/// A sound component (§9.1).
///
/// The offsets below are the original's; the fields are named after them so a
/// vector can assert on the layout of §9.1.
#[derive(Clone, Debug, PartialEq)]
pub struct SoundComponent {
    /// `+0x0C` — the description (AddRef'd; Released in the dtor).
    pub description: SoundDescription,
    /// `+0x10`, `+0x14`, `+0x18` — the world position x, y, z.
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// `+0x1C` — **started**: the `IsActive` byte (§9.1, §9.3).
    pub started: bool,
    /// `+0x20` — the instance handle, set to `this` at `Start` (§9.2).
    pub instance: u32,
    /// `+0x24` — the elapsed seconds.
    pub elapsed: f32,
    /// `+0x28` — the next position-update time.
    pub next_update: f32,
    /// `+0x2C` — the cached `cIGZSndSys*` (present after `Start`).
    pub snd_sys_present: bool,
    /// The events posted to the sound system, in order.
    pub events: Vec<(u32, u32, i32, i32, i32)>,
}

impl SoundComponent {
    /// A component as the description factory builds it: not started, no
    /// position, no sound system cached (§9.1).
    pub fn new(description: SoundDescription) -> Self {
        SoundComponent {
            description,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            started: false,
            instance: 0,
            elapsed: 0.0,
            next_update: 0.0,
            snd_sys_present: false,
            events: Vec::new(),
        }
    }

    /// `IsActive` (slot 2, `0x00580B50`, `mov al,[ecx+0x1C]; ret`): the
    /// `+0x1C` started byte. It stays true until `Stop`, so a non-ignored
    /// sound with `length == 0` pins its effect forever (§9.3, §14.16,
    /// conformance test 24).
    pub fn is_active(&self) -> bool {
        self.started
    }

    /// `Start(mode)` (slot 0, `0x005BE430`, `ret 4`): **mode is never read**
    /// (§9.1). No-op if `+0x1C != 0` (§9.2).
    ///
    /// Caches the `cIGZSndSys` service into `+0x2C`, zeroes `+0x24` and
    /// `+0x28`, sets `+0x20 = this`, posts `0xEA451903` with the description's
    /// `+0x10` sound id and `this` as instance, then sets `+0x1C = 1`.
    pub fn start(&mut self, mode: u32, this_addr: u32, snd_sys_present: bool) {
        let _ = mode; // §9.1: the mode is never read.
        if self.started {
            return;
        }
        self.snd_sys_present = snd_sys_present;
        self.elapsed = 0.0;
        self.next_update = 0.0;
        self.instance = this_addr;
        self.events
            .push((EVT_START_SOUND, self.description.sound_id, this_addr as i32, 0, 0));
        self.started = true;
    }

    /// `Update(dt seconds)` (slot 4, `0x005BE620`, `ret 4`, §9.2).
    ///
    /// Adds `dt` to `+0x24`. If the description's `+0x14` (the reciprocal of
    /// the location-update period) is > 0 and `+0x24` has passed `+0x28`, it
    /// sets `+0x28 = +0x24 + period` and posts `0xCA45190F` with **integer**
    /// coordinates (`ftol` on each, §14.15). If the description's `+0x18` (max
    /// length) is > 0 and `+0x24` has passed it, it calls `Stop(1)`.
    pub fn update(&mut self, dt: f32) {
        if !self.started {
            return;
        }
        self.elapsed += dt;
        let period = if self.description.inv_location_rate > 0.0 {
            1.0 / self.description.inv_location_rate
        } else {
            0.0
        };
        if period > 0.0 && self.elapsed >= self.next_update {
            self.next_update = self.elapsed + period;
            // §14.15: the position is quantised to whole units.
            self.events.push((
                EVT_SET_POSITION,
                self.instance,
                ftol(self.x),
                ftol(self.y),
                ftol(self.z),
            ));
        }
        if self.description.length > 0 && self.elapsed > self.description.length as f32 {
            self.stop(1);
        }
    }

    /// `Stop(mode)` (slot 1, `0x005BE4A0`, `ret 4`, §9.2).
    ///
    /// No-op if `+0x1C == 0`. Sets `+0x1C = 0` **before** the null check on
    /// `+0x2C` (§14.14), posts `0x6A451908`, then clears and releases `+0x2C`.
    /// The `mode` argument is **unread**, so a hard and a soft stop of a sound
    /// component are identical (§14.14, conformance test 23).
    pub fn stop(&mut self, mode: u32) {
        let _ = mode; // §14.14: never read.
        if !self.started {
            return;
        }
        // §14.14: cleared before the sound-system check.
        self.started = false;
        self.events.push((EVT_STOP_SOUND, self.instance, 0, 0, 0));
        self.snd_sys_present = false;
    }

    /// `SetTransform(local, parent)` (slot 3, `0x005BE4E0`, `ret 8`).
    pub fn set_transform(&mut self, x: f32, y: f32, z: f32) {
        self.x = x;
        self.y = y;
        self.z = z;
    }
}

/// The CRT `_ftol` truncation §14.15 applies before posting the position.
/// NaN truncates to 0 (foundations §18.1).
pub fn ftol(v: f32) -> i32 {
    if v.is_nan() {
        0
    } else {
        v.trunc() as i32
    }
}

/// The ignore bit of the component record `+0x10` (§9.3): bit 0 means "this
/// component is not considered when deciding whether the effect has finished".
///
/// In `CreateComponentFromDescription` (`0x00591730`):
/// `if (rec+0x10 & 1) runtime+0x40 |= 4; else runtime+0x40 &= ~4;` and
/// `CheckFinished` (`0x005BFD70`) skips any component whose runtime flag bit 2
/// is set (conformance test 24).
pub fn runtime_flags_for(record_flags: u32, current: u32) -> u32 {
    if record_flags & crate::audio::IGNORE_BIT != 0 {
        current | RUNTIME_FLAG_IGNORED
    } else {
        current & !RUNTIME_FLAG_IGNORED
    }
}

/// `CheckFinished` (`0x005BFD70`, §9.3): skip a component whose runtime flag
/// bit 2 is set. Returns `true` when the component is **considered**.
pub fn check_finished_considers(runtime_flags: u32) -> bool {
    runtime_flags & RUNTIME_FLAG_IGNORED == 0
}

/// The four shipped type-7 components that do **not** have the ignore bit set
/// (§9.3, measured from the effect directory's sound section: 303 type-7
/// components, 299 with bit 0 set).
pub const IGNORE_BIT_EXCEPTIONS: [(&str, u32, f32, u32); 4] = [
    // name, sound id, period, length
    ("researchsparksfx_slave", 0x6A92_274B, 0.5, 50),
    ("tornado_active", 0x4A89_0A7C, 0.5, 0),
    ("volcano_active", 0x4A89_0A7C, 0.5, 0),
    ("earthquake_active", 0x4A89_0A7C, 0.5, 0),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::data::SoundDescription;

    fn desc(sound_id: u32, rate: f32, length: u32) -> SoundDescription {
        let mut d = SoundDescription::new();
        d.sound_id = sound_id;
        d.inv_location_rate = rate;
        d.length = length;
        d
    }

    /// Conformance test 23: `Start` posts `0xEA451903` with the description's
    /// sound id and `this` as the instance key.
    #[test]
    fn start_posts_the_start_event() {
        let mut c = SoundComponent::new(desc(0x00CA_FE01, 0.5, 0));
        c.start(0, 0x00B0_1234, true);
        assert_eq!(c.events.len(), 1);
        assert_eq!(c.events[0].0, EVT_START_SOUND);
        assert_eq!(c.events[0].1, 0x00CA_FE01, "the description's +0x10 sound id");
        assert_eq!(c.events[0].2, 0x00B0_1234, "this as the instance key");
        assert_eq!(c.instance, 0x00B0_1234);
        assert!(c.is_active());
    }

    /// Conformance test 23: `Start` is a no-op when already started, and the
    /// mode is unread either way (§9.1).
    #[test]
    fn start_ignores_its_mode_and_is_idempotent() {
        let mut a = SoundComponent::new(desc(1, 0.5, 0));
        a.start(0, 0x10, true);
        let mut b = SoundComponent::new(desc(1, 0.5, 0));
        b.start(0xFFFFFFFF, 0x10, true);
        assert_eq!(a.events, b.events, "the mode is never read");
        let n = a.events.len();
        a.start(7, 0x20, true);
        assert_eq!(a.events.len(), n, "a second Start is a no-op");
    }

    /// Conformance test 23: `Update` posts `0xCA45190F` with **integer**
    /// coordinates once per `desc+0x14` seconds.
    #[test]
    fn update_posts_integer_positions_on_a_period() {
        let mut c = SoundComponent::new(desc(1, 1.0 / 0.5, 0)); // period 0.5 s
        c.set_transform(1.7, -2.2, 3.9);
        c.start(0, 0x42, true);
        c.events.clear();

        // §9.2 literally: `Start` zeroes `+0x28`, so the first `Update` that
        // runs with `elapsed >= +0x28` posts at once and sets
        // `+0x28 = elapsed + period`. From then on it is once per period.
        c.update(0.4);
        assert_eq!(c.events.len(), 1, "+0x28 starts at 0, so the first Update posts");
        assert_eq!(c.events[0].0, EVT_SET_POSITION);
        assert_eq!(c.events[0].1, 0x42);
        assert_eq!(c.events[0].2, 1, "ftol(1.7)");
        assert_eq!(c.events[0].3, -2, "ftol(-2.2)");
        assert_eq!(c.events[0].4, 3, "ftol(3.9)");

        c.update(0.2); // elapsed 0.6, next 0.9
        assert_eq!(c.events.len(), 1, "inside the period");
        c.update(0.5); // elapsed 1.1, past 0.9
        assert_eq!(c.events.len(), 2, "once per period after the first");
        c.update(0.5); // elapsed 1.6, next 1.4 -> due
        assert_eq!(c.events.len(), 3);
    }

    /// Conformance test 23: `Update` calls `Stop(1)` once `elapsed > length`
    /// when that length is non-zero.
    #[test]
    fn update_self_stops_at_the_max_length() {
        let mut c = SoundComponent::new(desc(1, 0.0, 50));
        c.start(0, 0x42, true);
        c.events.clear();
        c.update(49.0);
        assert!(c.is_active(), "still running at t = 49");
        c.update(2.0);
        assert!(!c.is_active(), "stopped once t > 50");
        assert_eq!(c.events.last().unwrap().0, EVT_STOP_SOUND);
    }

    /// §9.2 / §14.16: a length of 0 never self-stops.
    #[test]
    fn zero_length_never_self_stops() {
        let mut c = SoundComponent::new(desc(1, 0.0, 0));
        c.start(0, 0x42, true);
        c.update(1e9);
        assert!(c.is_active(), "a length of 0 pins the effect forever");
    }

    /// Conformance test 23: `Stop` posts `0x6A451908`, ignores its mode and
    /// clears `+0x1C` before checking whether the sound system is present.
    #[test]
    fn stop_ignores_mode_and_clears_first() {
        let mut c = SoundComponent::new(desc(1, 0.5, 0));
        c.start(0, 0x42, true);
        c.events.clear();
        c.stop(0);
        assert!(!c.is_active(), "+0x1C is cleared");
        assert_eq!(c.events.len(), 1);
        assert_eq!(c.events[0].0, EVT_STOP_SOUND);
        assert_eq!(c.events[0].1, 0x42);
        assert!(!c.snd_sys_present, "the cached service is released");

        // The mode makes no difference.
        let mut d = SoundComponent::new(desc(1, 0.5, 0));
        d.start(0, 0x42, true);
        d.events.clear();
        d.stop(1);
        assert_eq!(d.events, c.events);
    }

    /// Conformance test 23: `Stop` is a no-op when not started, and clears
    /// `+0x1C` even when there is no sound system (§14.14).
    #[test]
    fn stop_without_a_sound_system_still_clears() {
        let mut c = SoundComponent::new(desc(1, 0.5, 0));
        c.start(0, 0x42, false);
        c.stop(1);
        assert!(!c.is_active());
    }

    /// Conformance test 24: `IsActive` is the started byte and holds until
    /// `Stop`.
    #[test]
    fn is_active_is_the_started_byte() {
        let mut c = SoundComponent::new(desc(1, 0.5, 0));
        assert!(!c.is_active());
        c.start(0, 1, true);
        assert!(c.is_active());
        c.stop(0);
        assert!(!c.is_active());
    }

    /// Conformance test 24: the ignore bit becomes runtime flag bit 2 and
    /// `CheckFinished` skips those components.
    #[test]
    fn ignore_bit_sets_runtime_flag_bit_2() {
        assert_eq!(runtime_flags_for(1, 0), 0x04);
        assert_eq!(runtime_flags_for(0, 0), 0);
        assert_eq!(runtime_flags_for(1, 0x01), 0x05);
        assert_eq!(runtime_flags_for(0, 0x04), 0, "cleared when the bit is off");

        assert!(!check_finished_considers(0x04), "an ignored component is skipped");
        assert!(check_finished_considers(0x00));
        assert!(check_finished_considers(0x03), "only bit 2 matters");
    }

    /// §9.3: 299 of 303 type-7 components have the ignore bit set; these four
    /// do not.
    #[test]
    fn ignore_bit_exceptions() {
        assert_eq!(IGNORE_BIT_EXCEPTIONS.len(), 4);
        // Only `researchsparksfx_slave` self-stops (length 50).
        let slave = IGNORE_BIT_EXCEPTIONS[0];
        assert_eq!(slave.3, 50);
        for e in &IGNORE_BIT_EXCEPTIONS[1..] {
            assert_eq!(e.3, 0, "{} has length 0", e.0);
        }
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4AudioEventHandler`, CLSID `0xBDB6171E` (`docs/specs/audio.md` §7).
//! Section references (`§n`) point at that spec.
//!
//! Object size `0x50` (80 bytes). The handler is what turns an automata SFX
//! message into a sound-system event; the sound manager never sees the
//! automata messages themselves *[compat]* (§8, §14.11).

use std::collections::BTreeSet;

mod owner;
pub use owner::*;

use crate::audio::{
    EVT_SET_POSITION, EVT_START_SOUND, EVT_STOP_SOUND, HANDLER_TIMER_OFFSET, HANDLER_WINDOW_OFFSET, MSG_AUTOMATA_A,
    MSG_AUTOMATA_B, MSG_AUTOMATA_PLAY_SFX, MSG_AUTOMATA_STOP_SFX, MSG_POST_CITY_INIT, MSG_PRE_CITY_SHUTDOWN, RAND_MAX,
    RATE_LIMIT_MIN_MS, RATE_LIMIT_PAYLOAD, RATE_LIMIT_SPAN,
};

/// The object size of `cSC4AudioEventHandler` (§7).
pub const EVENT_HANDLER_OBJECT_SIZE: usize = 0x50;

/// The types `DoMessage` routes to `HandleAutomataMessages` (§7.2) — **and no
/// other type** (conformance test 12).
pub fn automata_message_types() -> [u32; 4] {
    [
        MSG_AUTOMATA_PLAY_SFX,
        MSG_AUTOMATA_A,
        MSG_AUTOMATA_STOP_SFX,
        MSG_AUTOMATA_B,
    ]
}

/// The 11 types subscribed only while a city is loaded, by
/// `HandleCityInitShutdownMessages` (`0x004EE970`, §7.2). `0x26D31EC1` adds
/// them and `0x26D31EC2` removes exactly this set.
pub fn city_message_types() -> [u32; 11] {
    [
        0xE946_71EB,
        0xE946_71ED,
        0x8946_71F6,
        MSG_AUTOMATA_A,
        MSG_AUTOMATA_PLAY_SFX,
        MSG_AUTOMATA_STOP_SFX,
        0x49F1_0111,
        0xC9F1_011A,
        MSG_AUTOMATA_B,
        0x89F1_0866,
        0xA9F1_0E45,
    ]
}

/// The 13 types `Init` subscribes on the main target (§7.2).
pub fn init_message_types() -> [u32; 13] {
    [
        MSG_POST_CITY_INIT,
        MSG_PRE_CITY_SHUTDOWN,
        0xC53D_10AA,
        0x26D3_1EC4,
        0x26D3_1EC5,
        0x4A7F_B7E9,
        0x2A3A_D653,
        0x0A61_1B34,
        0x2A61_1B51,
        0xAA61_1B6E,
        0xAA65_0860,
        0xAA65_0861,
        0x4348_B111,
    ]
}

/// What `HandleAutomataMessages` posted to the sound system (§7.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SndSysEvent {
    /// The sound-system event id: `0xEA451903`, `0x6A451908` or `0xCA45190F`.
    pub event: u32,
    /// The first argument (the sound id on start, the instance key otherwise).
    pub a: u32,
    /// The second argument (flags on start, 0 on stop).
    pub b: u32,
    /// The packed position on `0xCA45190F`; 0 otherwise.
    pub c: u32,
    /// The packed position's second word on `0xCA45190F`; 0 otherwise.
    pub d: u32,
}

/// The handler's rate limiter (§7.3, §14.5).
///
/// The limiter state is on the **handler**, not on the automaton, so it
/// throttles all matching sounds game-wide *[quirk]* (§14.5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RateLimiter {
    /// The timer at `+0x30`: whether it is running.
    pub running: bool,
    /// The elapsed time on that timer, in ms.
    pub elapsed_ms: u32,
    /// The window at `+0x48`, in ms.
    pub window_ms: u32,
}

impl RateLimiter {
    /// A limiter as the handler is constructed: not running, no window.
    pub const fn new() -> Self {
        RateLimiter {
            running: false,
            elapsed_ms: 0,
            window_ms: 0,
        }
    }

    /// `HandleAutomataMessages`'s gate (`0x004EE73C`–`0x004EE75F`, §7.3).
    ///
    /// Returns `true` when the play must be **dropped**: the timer is running
    /// and `GetElapsedTime` is below `+0x48`. Otherwise the timer restarts and
    /// the window becomes `1000 + rand()*3000/0x7FFF`.
    pub fn gate(&mut self, rand: u32) -> bool {
        if self.running && self.elapsed_ms < self.window_ms {
            return true;
        }
        self.running = true;
        self.elapsed_ms = 0;
        self.window_ms = RATE_LIMIT_MIN_MS + (rand % (RAND_MAX + 1)) * RATE_LIMIT_SPAN / RAND_MAX;
        false
    }
}

/// `cSC4AudioEventHandler` (§7).
#[derive(Clone, Debug)]
pub struct AudioEventHandler {
    /// The set of types currently subscribed on the main target.
    pub subscribed: BTreeSet<u32>,
    /// Whether a city is loaded: the 11 city types are subscribed only then.
    pub city_loaded: bool,
    /// The rate limiter of §7.3.
    pub limiter: RateLimiter,
    /// `+0x1C` — the initialized flag (§7.1).
    pub initialized: bool,
    /// The events posted to the sound system, in order.
    pub events: Vec<SndSysEvent>,
    /// The sound system (`+0x14`); when null the handler releases and returns 0.
    pub snd_sys_present: bool,
    /// The modal target has a separate subscription set (§7.2).
    pub modal_subscribed: BTreeSet<u32>,
    /// Required capability at +0x18; availability is supplied by the foreign host.
    pub capability_present: bool,
    /// The radio service at +0x10.
    pub radio_present: bool,
    /// Whether this handler has registered with the foreign sound system.
    pub sound_registered: bool,
    /// Constructor-zero words at +0x20, +0x24 and +0x28 (§7.1.1).
    pub reference_count: u32,
    pub reserved_words: [u32; 2],
    /// Front is the most recently begun modal window (§7.2.3).
    pub modal_windows: Vec<u64>,
    /// Timer constructor selector, independent of runtime elapsed time.
    pub timer_clock_selector: u32,
}

impl Default for AudioEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEventHandler {
    /// An isolated handler model: not initialized, no subscriptions.
    ///
    /// The legacy play/stop helpers assume a sound system for conformance
    /// fixtures. Use `construct` and `init_with` to acquire foreign services.
    pub fn new() -> Self {
        AudioEventHandler {
            subscribed: BTreeSet::new(),
            city_loaded: false,
            limiter: RateLimiter::new(),
            initialized: false,
            events: Vec::new(),
            snd_sys_present: true,
            modal_subscribed: BTreeSet::new(),
            capability_present: false,
            radio_present: false,
            sound_registered: false,
            reference_count: 0,
            reserved_words: [0; 2],
            modal_windows: Vec::new(),
            timer_clock_selector: 4,
        }
    }

    /// `Init` (`0x004EFE80`, §7.2): subscribe the 13 main types.
    pub fn init(&mut self) -> u32 {
        for t in init_message_types() {
            self.subscribed.insert(t);
        }
        self.modal_subscribed.extend(MODAL_MESSAGE_TYPES);
        self.initialized = true;
        1
    }

    /// `HandleCityInitShutdownMessages` (`0x004EE970`, §7.2): `0x26D31EC1`
    /// **adds** the city set, `0x26D31EC2` **removes** exactly that set. So the
    /// automata SFX messages are only heard while a city is loaded *[compat]*
    /// (conformance test 12).
    pub fn handle_city_init_shutdown(&mut self, msg_type: u32) {
        let set = city_message_types();
        match msg_type {
            MSG_POST_CITY_INIT => {
                self.city_loaded = true;
                for t in set {
                    self.subscribed.insert(t);
                }
            }
            MSG_PRE_CITY_SHUTDOWN => {
                self.city_loaded = false;
                for t in set {
                    self.subscribed.remove(&t);
                }
            }
            _ => {}
        }
    }

    /// `DoMessage` (`0x004EF810`, §7.2): route the four automata types to
    /// `HandleAutomataMessages` and nothing else there (conformance test 12).
    /// Returns `Some(true/false)` when the type is handled, `None` otherwise.
    pub fn route_to_automata(msg_type: u32) -> Option<bool> {
        if automata_message_types().contains(&msg_type) {
            Some(true)
        } else {
            None
        }
    }

    /// `HandleAutomataMessages` (`0x004EE560`, §7.3) for a **play**
    /// (`0x8A385DD8`).
    ///
    /// Entry requires `+0x18` non-null and the message to answer QI
    /// `0x65297976`; if `+0x14` (`cIGZSndSys`) is null the handler releases and
    /// returns 0. The rate limiter runs only when the payload is `0xA9F14C47`
    /// *[quirk]* (§14.5).
    pub fn handle_play(&mut self, payload: u32, rand: u32, sound_id: u32, instance: u32) -> u32 {
        if !self.snd_sys_present {
            return 0;
        }
        // Step 1 of §7.3: the rate limiter, only for the one magic payload.
        if payload == RATE_LIMIT_PAYLOAD && self.limiter.gate(rand) {
            return 1; // the play is dropped
        }
        // Steps 2-3: position lookup, then two events.
        self.events.push(SndSysEvent {
            event: EVT_START_SOUND,
            a: sound_id,
            b: instance,
            c: 0,
            d: 0,
        });
        self.events.push(SndSysEvent {
            event: EVT_SET_POSITION,
            a: instance,
            b: 0,
            c: 0,
            d: 0,
        });
        1
    }

    /// `HandleAutomataMessages` for a **stop** (`0xAA396960`, §7.3): post
    /// `0x6A451908` with the payload's arguments.
    pub fn handle_stop(&mut self, sound_id: u32, instance: u32) -> u32 {
        if !self.snd_sys_present {
            return 0;
        }
        self.events.push(SndSysEvent {
            event: EVT_STOP_SOUND,
            a: sound_id,
            b: instance,
            c: 0,
            d: 0,
        });
        1
    }
}

/// The `+0x30` timer offset (§7.1).
pub fn timer_offset() -> u32 {
    HANDLER_TIMER_OFFSET
}

/// The `+0x48` window offset (§7.1).
pub fn window_offset() -> u32 {
    HANDLER_WINDOW_OFFSET
}

/// `1000 + rand()*3000/0x7FFF` (§7.3). With `rand` in `[0, 0x7FFF]` this spans
/// `[1000, 4000]`, **inclusive** of 4000 (§14.13).
pub fn rate_limit_window(rand: u32) -> u32 {
    RATE_LIMIT_MIN_MS + (rand % (RAND_MAX + 1)) * RATE_LIMIT_SPAN / RAND_MAX
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Conformance test 12: exactly the four automata types route there.
    #[test]
    fn routing_is_exactly_four_types() {
        let t = automata_message_types();
        assert_eq!(t.len(), 4);
        for x in t {
            assert_eq!(AudioEventHandler::route_to_automata(x), Some(true));
        }
        // A few neighbours that must not route there.
        for x in [
            0x26D3_1EC1u32,
            0x2A3A_D653,
            0xFA0C_6390,
            crate::audio::MSG_EXEMPLAR_CHANGED,
        ] {
            assert_eq!(AudioEventHandler::route_to_automata(x), None, "0x{x:08X}");
        }
    }

    /// Conformance test 12: `0x26D31EC1` adds the city set, `0x26D31EC2`
    /// removes exactly that set.
    #[test]
    fn city_subscriptions_come_and_go() {
        let mut h = AudioEventHandler::new();
        h.init();
        let before = h.subscribed.len();
        assert!(!h.city_loaded);

        h.handle_city_init_shutdown(MSG_POST_CITY_INIT);
        assert!(h.city_loaded);
        assert_eq!(h.subscribed.len(), before + 11);
        for t in city_message_types() {
            assert!(h.subscribed.contains(&t), "0x{t:08X} is subscribed");
        }

        h.handle_city_init_shutdown(MSG_PRE_CITY_SHUTDOWN);
        assert!(!h.city_loaded);
        assert_eq!(h.subscribed.len(), before);
        for t in city_message_types() {
            assert!(!h.subscribed.contains(&t), "0x{t:08X} is gone");
        }
        // The Init set survives (§7.2: Shutdown undoes Init, not the city set).
        for t in init_message_types() {
            assert!(h.subscribed.contains(&t), "0x{t:08X} stays");
        }
    }

    /// Conformance test 13: the rate limiter drops a repeat inside the window
    /// and only for the one magic payload.
    #[test]
    fn rate_limiter_gate() {
        let mut h = AudioEventHandler::new();
        // First play: not running, so it goes through and starts the timer.
        assert_eq!(h.handle_play(RATE_LIMIT_PAYLOAD, 0x1000, 5, 5), 1);
        assert_eq!(h.events.len(), 2, "start + set-position");
        assert!(h.limiter.running);
        let w = h.limiter.window_ms;
        assert!((1000..=4000).contains(&w), "window {w}");

        // A second play inside the window is dropped.
        h.limiter.elapsed_ms = w - 1;
        let n = h.events.len();
        assert_eq!(h.handle_play(RATE_LIMIT_PAYLOAD, 0x1000, 6, 6), 1);
        assert_eq!(h.events.len(), n, "the play is dropped");

        // Past the window it goes through and the window is redrawn.
        h.limiter.elapsed_ms = w + 1;
        assert_eq!(h.handle_play(RATE_LIMIT_PAYLOAD, 0x1000, 7, 7), 1);
        assert_eq!(h.events.len(), n + 2);
    }

    /// §14.5: the limiter only fires for `0xA9F14C47`.
    #[test]
    fn rate_limiter_only_for_one_payload() {
        let mut h = AudioEventHandler::new();
        h.handle_play(RATE_LIMIT_PAYLOAD, 0x1000, 1, 1);
        h.limiter.elapsed_ms = 0;
        let n = h.events.len();
        assert_eq!(h.handle_play(0x1234_5678, 0x1000, 2, 2), 1);
        assert_eq!(h.events.len(), n + 2, "another payload plays immediately");
    }

    /// §14.13: the window can be exactly 4000 ms.
    #[test]
    fn rate_limit_window_is_inclusive() {
        assert_eq!(rate_limit_window(0), 1000);
        assert_eq!(rate_limit_window(RAND_MAX), 4000);
    }

    /// Conformance test 13: two automata share one handler, one timer and one
    /// threshold.
    #[test]
    fn one_handler_one_timer() {
        let mut h = AudioEventHandler::new();
        h.handle_play(RATE_LIMIT_PAYLOAD, 0x2000, 1, 1);
        let w = h.limiter.window_ms;
        h.limiter.elapsed_ms = w - 1;
        let n = h.events.len();
        // A different automaton, same handler: still throttled.
        assert_eq!(h.handle_play(RATE_LIMIT_PAYLOAD, 0x2000, 99, 99), 1);
        assert_eq!(h.events.len(), n);
    }

    /// §7.3: with no sound system the handler releases and returns 0.
    #[test]
    fn no_sound_system_means_no_events() {
        let mut h = AudioEventHandler::new();
        h.snd_sys_present = false;
        assert_eq!(h.handle_play(RATE_LIMIT_PAYLOAD, 0, 1, 1), 0);
        assert_eq!(h.handle_stop(1, 1), 0);
        assert!(h.events.is_empty());
    }

    /// §7.3: the stop path posts `0x6A451908`.
    #[test]
    fn stop_posts_the_stop_event() {
        let mut h = AudioEventHandler::new();
        assert_eq!(h.handle_stop(0x00CA_FE01, 0x2A), 1);
        assert_eq!(h.events.len(), 1);
        assert_eq!(h.events[0].event, EVT_STOP_SOUND);
        assert_eq!(h.events[0].a, 0x00CA_FE01);
        assert_eq!(h.events[0].b, 0x2A);
    }
}

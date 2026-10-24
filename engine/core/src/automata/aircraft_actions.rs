// SPDX-License-Identifier: GPL-3.0-or-later
//! Settled aircraft action contracts. Adapters execute requests in returned order.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundRequest {
    Start(u32),
    End(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineNotification {
    Base(bool),
    Embedded { offset: u32, operation: u32, argument: u32 },
}

pub fn stopped_sounds() -> Vec<SoundRequest> {
    vec![
        SoundRequest::End(9),
        SoundRequest::End(8),
        SoundRequest::End(10),
        SoundRequest::Start(10),
    ]
}

/// Only the settled slot-7 fields; other base player state remains foreign.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerWeaponRecord {
    pub command: f32,
    pub ready: bool,
    pub repeat_timer: f32,
    pub uses: u32,
}

pub trait AircraftWeaponEffects {
    fn translate_base_commands(&mut self, player: &mut PlayerWeaponRecord);
    fn preliminary(&mut self);
    fn attempt(&mut self, name: &'static str, argument1: u32, argument2: u32) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponOutcome {
    NoRequest,
    NotReady,
    Failed,
    Used(&'static str),
    /// AF10/AF5 do not establish unordered command comparisons.
    UnsupportedNonFinite,
}

pub fn weapon_requests(
    player: &mut PlayerWeaponRecord,
    effects: &mut dyn AircraftWeaponEffects,
    names: &[&'static str],
) -> WeaponOutcome {
    effects.translate_base_commands(player);
    if !player.command.is_finite() {
        return WeaponOutcome::UnsupportedNonFinite;
    }
    if player.command < 0.0 {
        return WeaponOutcome::NoRequest;
    }
    effects.preliminary();
    if !player.ready {
        return WeaponOutcome::NotReady;
    }
    for &name in names {
        if effects.attempt(name, 0, 0) {
            player.repeat_timer = 0.0;
            player.ready = false;
            player.uses = player.uses.wrapping_add(1);
            return WeaponOutcome::Used(name);
        }
    }
    WeaponOutcome::Failed
}

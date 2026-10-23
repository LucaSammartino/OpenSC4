// SPDX-License-Identifier: GPL-3.0-or-later
//! Literal command arithmetic from watercraft §6.2; unresolved choices: WC8.

use super::{state, BoatId, DockManager, Event, Fleet, Gear, Result, Watercraft};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DriveCommand {
    pub throttle: f32,
    pub reverse: f32,
    pub left: bool,
    pub right: bool,
    pub fire: bool,
    pub skier_toggle: bool,
    pub tow: bool,
}

impl Default for DriveCommand {
    fn default() -> Self {
        Self {
            throttle: -1.0,
            reverse: -1.0,
            left: false,
            right: false,
            fire: false,
            skier_toggle: false,
            tow: false,
        }
    }
}

/// Foreign effect playback only, never a native boat command calculation.
/// Fire's return value says whether the named effect actually played.
pub trait PlayerEffects {
    fn play_fire(&mut self, name: &'static str) -> bool;
    /// Placement behind the boat is unresolved (WC6); this only controls the
    /// already acquired waterski effect's running state.
    fn waterski(&mut self, running: bool);
}

fn approach(value: f32, target: f32, amount: f32) -> f32 {
    if value < target {
        (value + amount).min(target)
    } else {
        (value - amount).max(target)
    }
}

impl Watercraft {
    /// The specified speed/steering/button portion, before tow handling and
    /// inherited command translation. Returns None without an AEMS sound set.
    pub fn translate_drive_commands(
        &mut self,
        dt: f32,
        command: &mut DriveCommand,
        effects: &mut dyn PlayerEffects,
    ) -> Option<Gear> {
        let (gear, input) = self.select_gear(command.throttle, command.reverse)?;
        self.player.gear = gear;
        // WC8: literal target = signed gear input, clamped to entries 6–7.
        // Braking ends at zero this tick; leftover time is not reused.
        let target = input.max(self.drive.0[6]).min(self.drive.0[7]);
        let braking = self.speed * target < 0.0 || gear == Gear::Neutral;
        let slowing = braking || self.speed.abs() > target.abs();
        let rate = dt
            * self.drive.0[10]
            * if slowing || gear == Gear::Reverse {
                self.drive.0[0]
            } else {
                1.0
            };
        self.speed = approach(self.speed, if braking { 0.0 } else { target }, rate);

        // WC8: left is negative, right positive; simultaneous buttons select
        // neutral return. Entry 5 multiplies the entry-2 counter-steer rate.
        let direction = i32::from(command.right) - i32::from(command.left);
        let scale = dt * self.drive.0[10] * self.drive.0[0];
        if direction == 0 {
            self.turn = approach(self.turn, 0.0, scale * self.drive.0[1]);
        } else {
            let direction = direction as f32;
            let boost = if self.turn * direction < 0.0 {
                self.drive.0[5]
            } else {
                1.0
            };
            self.turn += direction * scale * self.drive.0[2] * boost;
        }
        self.turn = self.turn.max(self.drive.0[8]).min(self.drive.0[9]);

        if command.fire && effects.play_fire("whalefire") {
            self.player.shots = self.player.shots.wrapping_add(1);
            command.fire = false;
        }
        if self.group.id == 0x4409 {
            // WC8: toggling at rest cannot start the effect. A stop clears
            // its running state; no behind-boat placement is guessed here.
            let running = if self.speed <= 0.0 {
                false
            } else if command.skier_toggle {
                !self.player.skier_running
            } else {
                self.player.skier_running
            };
            if running != self.player.skier_running {
                effects.waterski(running);
                self.player.skier_running = running;
            }
        }
        Some(gear)
    }
}

impl Fleet {
    /// Owner command ordering: arithmetic/buttons, tug button, then the base
    /// translation request. The inherited translation itself remains WC6.
    pub fn player_commands(
        &mut self,
        id: BoatId,
        dt: f32,
        command: &mut DriveCommand,
        effects: &mut dyn PlayerEffects,
        manager: &mut Option<&mut dyn DockManager>,
    ) -> Result<()> {
        let old_speed = self.boat(id)?.speed.abs();
        if self
            .boat_mut(id)?
            .translate_drive_commands(dt, command, effects)
            .is_some()
        {
            self.tow_button(id, command.tow, manager)?;
            if old_speed != self.boat(id)?.speed.abs() {
                self.boat_mut(id)?.events.push(Event::ResetSpeedTween);
            }
            self.boat_mut(id)?.events.push(Event::BaseCommandTranslation);
        }
        Ok(())
    }

    /// Geometry-aware dispatch for the two fully specified movement ticks.
    /// Other states retain the ordinary state dispatcher and its explicit gaps.
    pub fn state_tick_with_geometry(
        &mut self,
        id: BoatId,
        dt: f32,
        rectangle: [super::Cell; 4],
        terrain: &dyn super::Terrain,
        manager: &mut Option<&mut dyn DockManager>,
    ) -> Result<()> {
        let active = match self.boat(id)?.top_state() {
            Some(state::DOCK) => self.docking_tick(id, dt, terrain, manager)?,
            Some(state::UNDOCK) => self.undocking_tick(id, dt, rectangle, terrain, manager)?,
            _ => return self.state_tick(id, dt, manager),
        };
        if !active {
            self.pop_state(id, manager)?;
        }
        Ok(())
    }
}

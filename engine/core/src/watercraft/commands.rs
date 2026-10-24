// SPDX-License-Identifier: GPL-3.0-or-later
//! Float-channel commands and shared bookkeeping, watercraft §§6.2.1–6.2.3.

use super::{state, BoatId, DockManager, Event, Fleet, Gear, Result, Watercraft};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DriveCommand {
    pub throttle: f32,
    pub reverse: f32,
    pub left: f32,
    pub right: f32,
    pub fire: f32,
    pub skier_toggle: bool,
    pub tow: bool,
    /// Remaining channels in record order: 4, 5, 6, 8, 9, 10, 11, 12.
    pub other: [f32; 8],
    pub shared: [CommandChannel; 13],
    pub signed_command: f32,
    /// Drive-state +0x5C, copied to the boat after the ordered responses.
    pub steering: f32,
    pub rudder: [f32; 3],
    pub heading: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CommandChannel {
    pub previous: f32,
    pub edge: bool,
    pub duration: i32,
    pub limit: i32,
    pub armed: bool,
    pub timer: f32,
    pub interval: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpeedTween {
    pub start: f32,
    pub target: f32,
    pub duration: f32,
    pub elapsed: f32,
}

impl SpeedTween {
    pub fn sample(&self) -> f32 {
        if self.duration <= 0.0 {
            self.target
        } else {
            self.start + (self.elapsed / self.duration).clamp(0.0, 1.0) * (self.target - self.start)
        }
    }

    pub fn retarget(&mut self, target: f32) -> bool {
        if self.target == target {
            return false;
        }
        self.start = self.sample();
        self.target = target;
        self.elapsed = 0.0;
        self.duration = 0.0;
        true
    }
}

impl Default for DriveCommand {
    fn default() -> Self {
        Self {
            throttle: -1.0,
            reverse: -1.0,
            left: -1.0,
            right: -1.0,
            fire: -1.0,
            skier_toggle: false,
            tow: false,
            other: [-1.0; 8],
            shared: [CommandChannel::default(); 13],
            signed_command: 0.0,
            steering: 0.0,
            rudder: [0.0; 3],
            heading: 0.0,
        }
    }
}

/// Foreign effect playback only, never a native boat command calculation.
/// Fire's return value says whether the named effect actually played.
pub trait PlayerEffects {
    fn play_fire(&mut self, name: &'static str) -> bool;
    fn fire_model_hook(&mut self);
    fn has_waterski(&self) -> bool;
    fn waterski_transform(&mut self, transform: &super::Transform);
    fn waterski(&mut self, running: bool);
    /// AEMS dispatch for one channel: slot 6 followed by slot 5, event 0x14.
    fn command_event(&mut self, channel: usize, event: u32);
}

impl DriveCommand {
    pub fn values(&self) -> [f32; 13] {
        [
            self.throttle,
            self.reverse,
            self.right,
            self.left,
            self.other[0],
            self.other[1],
            self.other[2],
            self.fire,
            self.other[3],
            self.other[4],
            self.other[5],
            self.other[6],
            self.other[7],
        ]
    }

    pub fn shared_bookkeeping(&mut self, dt: f32, effects: &mut dyn PlayerEffects) {
        for (i, current) in self.values().into_iter().enumerate() {
            let c = &mut self.shared[i];
            let difference = c.previous - current;
            c.edge = difference > 0.0 && difference < 250.0;
            c.previous = current;
            if c.duration == -1 || c.duration > c.limit {
                if !c.armed {
                    if c.timer < c.interval {
                        c.timer += dt;
                    } else {
                        c.armed = true;
                        c.timer = 0.0;
                    }
                }
            } else {
                c.armed = false;
                if current != -1.0 {
                    effects.command_event(i, 0x14);
                }
            }
        }
        self.skier_toggle = self.shared[7].edge;
        self.tow = self.shared[9].edge;
    }
}

// The constants are stored as f32, then used by x87 transcendental operations.
// Evaluate the transcendental in f64 before storing its result in f32.
fn curve_constants() -> (f32, f32, f32, f32) {
    let a = 0.91_f32;
    let b = std::f32::consts::FRAC_PI_2;
    let n = -a * 0.5;
    (a, b, n, ((a as f64 * b as f64).tan()) as f32)
}

impl Watercraft {
    pub fn command_target_speed(&self, q: f32, gear: Gear) -> f32 {
        let p = &self.drive.0;
        let (a, b, n, positive) = curve_constants();
        let negative = (n as f64 * b as f64).atan() as f32;
        let x = q.max(p[30]).min(p[31]);
        let u = if x > 0.0 { x * a / p[31] } else { x * n / p[30] };
        let angle = u as f64 * b as f64;
        let w = if gear == Gear::Forward {
            angle.tan()
        } else {
            angle.atan()
        } as f32;
        let speed = if w > 0.0 {
            w * p[7] / positive
        } else {
            w * p[6] / negative
        };
        speed.max(p[6]).min(p[7])
    }

    pub fn inverse_speed_command(&self, speed: f32) -> f32 {
        let p = &self.drive.0;
        let (a, b, n, positive) = curve_constants();
        let negative = (n as f64 * b as f64).tan() as f32;
        let s = speed.max(p[6]).min(p[7]);
        let z = if s > 0.0 {
            s * positive / p[7]
        } else {
            s * negative / p[6]
        };
        let h = z.atan() * 0.636_619_75;
        let q = if h > 0.0 { h * p[31] / a } else { h * p[30] / n };
        q.max(p[30]).min(p[31])
    }
}

fn approach(value: f32, target: f32, amount: f32) -> f32 {
    if value < target {
        (value + amount).min(target)
    } else {
        (value - amount).max(target)
    }
}

impl Watercraft {
    /// Speed, fire, skier, steering, in specified order. Fleet completes tow,
    /// render/tween fields and shared bookkeeping. None means missing AEMS.
    pub fn translate_drive_commands(
        &mut self,
        dt: f32,
        command: &mut DriveCommand,
        effects: &mut dyn PlayerEffects,
    ) -> Option<Gear> {
        let (gear, input) = self.select_gear(command.throttle, command.reverse)?;
        self.player.gear = gear;
        command.signed_command = input;
        let p = &self.drive.0;
        let target = self.command_target_speed(input, gear).max(p[6]).min(p[7]);
        let a = dt * p[10];
        let b = a * p[0];
        if input != 0.0 {
            let sign = super::sign;
            if self.speed != 0.0 && sign(target) != sign(self.speed) {
                let old = self.speed;
                self.speed = approach(self.speed, target, b);
                if old > 0.0 {
                    command.reverse = p[32];
                } else {
                    command.throttle = p[33];
                }
            } else if sign(input) == sign(self.speed) && target.abs() <= self.speed.abs() {
                let inverse = self.inverse_speed_command(self.speed);
                if self.speed > 0.0 {
                    command.throttle = inverse;
                } else {
                    command.reverse = inverse;
                }
            } else {
                let step = if input < 0.0 && sign(input) != sign(self.speed) {
                    b
                } else {
                    a
                };
                self.speed = if (target - self.speed).abs() <= step {
                    target
                } else {
                    self.speed + sign(input) * step
                };
            }
        }
        if command.fire >= 0.0 {
            effects.fire_model_hook();
            if command.shared[7].armed && effects.play_fire("whalefire") {
                self.player.shots = self.player.shots.wrapping_add(1);
                command.shared[7].timer = 0.0;
                command.shared[7].armed = false;
            }
        }
        let mut group = Some(self.group.as_ref());
        let mut skier_group = false;
        while let Some(current) = group {
            skier_group |= current.id == 0x4409;
            group = current.parent.as_deref();
        }
        if skier_group && effects.has_waterski() {
            if command.skier_toggle {
                if self.player.skier_running {
                    effects.waterski(false);
                    self.player.skier_running = false;
                } else if self.speed > 0.0 {
                    effects.waterski_transform(&self.transform);
                    effects.waterski(true);
                    self.player.skier_running = true;
                }
            }
            if self.player.skier_running {
                if self.speed <= 0.0 {
                    effects.waterski(false);
                    self.player.skier_running = false;
                } else {
                    effects.waterski_transform(&self.transform);
                }
            }
        }
        let r = (dt * 0.001) * p[2];
        self.turn = command.steering;
        if command.left >= 0.0 && self.turn <= 0.0 {
            if self.turn < -p[1] * command.left {
                command.left = -self.turn / p[1] - dt;
            }
            self.turn = -p[1] * command.left;
        } else if self.turn < 0.0 {
            let step = if command.right > 0.0 {
                command.right = 0.0;
                r.max(r * p[5])
            } else {
                r
            };
            self.turn = (self.turn + step).min(0.0);
        }
        // WC9: literal §6.2.2 predicates; WC8-08's simultaneous-channel
        // expectation contradicts this ordered positive-T response.
        if command.right >= 0.0 && self.turn >= 0.0 {
            if p[1] * command.right < self.turn {
                command.right = self.turn / p[1] + dt;
            }
            self.turn = p[1] * command.right;
        } else if self.turn > 0.0 {
            let step = if command.left > 0.0 {
                command.left = 0.0;
                r.max(r * p[5])
            } else {
                r
            };
            self.turn = (self.turn - step).max(0.0);
        }
        self.turn = self.turn.max(p[8]).min(p[9]);
        command.steering = self.turn;
        Some(gear)
    }

    pub fn complete_drive_commands(&mut self, dt: f32, command: &mut DriveCommand, effects: &mut dyn PlayerEffects) {
        self.turn = command.steering.max(self.drive.0[8]).min(self.drive.0[9]);
        command.steering = self.turn;
        command.rudder = [-0.125 * self.turn, 0.0, std::f32::consts::TAU];
        let h = super::math::normalize(self.heading);
        let angle = h[2].acos();
        command.heading = if h[0] >= 0.0 {
            angle
        } else {
            std::f32::consts::TAU - angle
        };
        if self.player.speed_tween.retarget(self.speed.abs()) {
            self.events.push(Event::ResetSpeedTween);
        }
        command.shared_bookkeeping(dt, effects);
        self.events.push(Event::BaseCommandTranslation);
    }
}

impl Fleet {
    /// Owner command ordering: arithmetic/buttons, tug button, then the base
    /// shared bookkeeping. Command dt is not clamped to movement time.
    pub fn player_commands(
        &mut self,
        id: BoatId,
        dt: f32,
        command: &mut DriveCommand,
        effects: &mut dyn PlayerEffects,
        manager: &mut Option<&mut dyn DockManager>,
    ) -> Result<()> {
        if self
            .boat_mut(id)?
            .translate_drive_commands(dt, command, effects)
            .is_some()
        {
            self.tow_button(id, command.tow, manager)?;
            self.boat_mut(id)?.complete_drive_commands(dt, command, effects);
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
            Some(state::TOWED) => self.following_tick(id, dt, terrain)?,
            _ => return self.state_tick(id, dt, manager),
        };
        if !active {
            self.pop_state(id, manager)?;
        }
        Ok(())
    }
}

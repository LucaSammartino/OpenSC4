// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::automata::{SetStateArgs, StateExtra};
use crate::stream::ResKey;

impl TrafficAutomaton {
    /// The inherited state body shares Vehicle's portable base handler. The
    /// facade moves (never clones) the record and runtime, and handles path
    /// states locally so the traffic owner never acquires vehicle anchoring.
    pub fn state_tick(&mut self, dt_ms: f32, movement: Movement, world: &mut impl VehicleWorld) -> Result<bool> {
        let kind = self.record.base.states.last().map(|s| s.state_type).unwrap_or(0);
        if kind == 13 {
            return Ok(self.owner().blocked_wait_tick(dt_ms, movement.speed_scale, world));
        }
        // Station calls the base dispatcher after its path move (automata §6.5).
        if kind == 30 && tween_value(&self.record.base.f3[..4]) > 0.0 {
            self.owner().path_tick(dt_ms, movement, false, world)?;
        }
        let mut facade = Vehicle::default();
        facade.record.traffic = std::mem::take(&mut self.record);
        facade.traffic = std::mem::take(&mut self.runtime);
        facade.crash_elapsed_ms = facade.traffic.crash_elapsed_ms;
        // Only Vehicle overrides 14/21; the Traffic base has no such handlers.
        let result = if kind == 14 || kind == 21 {
            facade.base_countdown_tick(dt_ms, world);
            if facade.record.traffic.base.states.last().map(|s| s.state_type) == Some(kind) {
                Ok(false)
            } else {
                facade.dispatch_state(dt_ms, movement, world)
            }
        } else if kind == 30 {
            facade.base_countdown_tick(dt_ms, world);
            facade.dispatch_state(dt_ms, movement, world).map(|_| true)
        } else {
            facade.state_tick(dt_ms, movement, world)
        };
        self.record = facade.record.traffic;
        self.runtime = facade.traffic;
        self.runtime.crash_elapsed_ms = facade.crash_elapsed_ms;
        result
    }
}

impl Vehicle {
    pub(crate) fn base_countdown_tick(&mut self, dt_ms: f32, world: &mut impl VehicleWorld) {
        self.countdown(dt_ms, world);
    }
    pub fn plop_jump_tick(&mut self, dt_ms: f32, world: &mut impl VehicleWorld) -> bool {
        if self.record.traffic.base.automata_flags & REMOVE != 0 {
            return false;
        }
        let Some(state) = self.record.traffic.base.states.last_mut() else {
            return false;
        };
        let StateExtra::T0F([floor, velocity]) = &mut state.extra else {
            return false;
        };
        let mut position = self.record.traffic.base.vectors[0];
        let candidate = position[1] + dt_ms * *velocity;
        if candidate < *floor {
            position[1] = *floor;
        } else {
            position[1] = candidate;
            *velocity += f32::from_bits(0xB869_0455) * dt_ms;
        }
        self.record.traffic.base.vectors[0] = position;
        world.move_models(position, self.record.traffic.base.vectors[2]);
        true
    }

    pub fn attach_anim(
        &mut self,
        key: ResKey,
        selector: u16,
        handled: bool,
        timeout_ms: f32,
        world: &mut impl VehicleWorld,
    ) -> Result<bool> {
        if !self.push_state(
            19,
            SetStateArgs {
                timeout_ms,
                ..SetStateArgs::default()
            },
            world,
        )? {
            return Ok(false);
        }
        let model = world.added_model(key);
        let success = !model.is_null();
        if success {
            world.select_animation(&model, selector);
            world.animation_value(&model, 1, 0.0);
        }
        self.record.traffic.base.states.last_mut().expect("pushed").extra = StateExtra::AttachAnim {
            model,
            flag: handled,
            anim_selector: selector,
        };
        Ok(success)
    }

    pub fn attach_tick(&mut self, dt_ms: f32, world: &mut impl VehicleWorld) -> bool {
        if self.record.traffic.base.automata_flags & REMOVE != 0 {
            return false;
        }
        let Some(state) = self.record.traffic.base.states.last() else {
            return false;
        };
        let StateExtra::AttachAnim {
            model,
            flag,
            anim_selector,
        } = &state.extra
        else {
            return false;
        };
        let handled = *flag;
        if !model.is_null() {
            if let Some((value, frame, count)) = world.animation_status(model) {
                if value < 1.0 {
                    let next = value + dt_ms / (world.attach_tuning_ms() * 0.5);
                    world.animation_value(model, 1, next.min(1.0));
                }
                if ![2, 3].contains(anim_selector) && frame >= count - 1.0 {
                    self.pop_state(world);
                    return true;
                }
            }
        }
        handled
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::automata::{SetStateArgs, StateExtra};
use crate::stream::ResKey;

impl Vehicle {
    pub fn plop_jump_tick(&mut self, dt_ms: f32, world: &mut impl VehicleWorld) -> bool {
        if self.record.traffic.base.automata_flags & REMOVE != 0 { return false; }
        let Some(state) = self.record.traffic.base.states.last_mut() else { return false; };
        let StateExtra::T0F([floor, velocity]) = &mut state.extra else { return false; };
        let mut position = self.record.traffic.base.vectors[0];
        let candidate = position[1] + dt_ms * *velocity;
        if candidate < *floor { position[1] = *floor; }
        else { position[1] = candidate; *velocity += f32::from_bits(0xB869_0455) * dt_ms; }
        self.record.traffic.base.vectors[0] = position;
        world.move_models(position, self.record.traffic.base.vectors[2]);
        true
    }

    pub fn attach_anim(&mut self, key: ResKey, selector: u16, handled: bool, timeout_ms: f32,
        world: &mut impl VehicleWorld) -> Result<bool> {
        if !self.push_state(19, SetStateArgs { timeout_ms, ..SetStateArgs::default() }, world)? { return Ok(false); }
        let model = world.added_model(key);
        let success = !model.is_null();
        if success {
            world.select_animation(&model, selector);
            world.animation_value(&model, 1, 0.0);
        }
        self.record.traffic.base.states.last_mut().expect("pushed").extra =
            StateExtra::AttachAnim { model, flag: handled, anim_selector: selector };
        Ok(success)
    }

    pub fn attach_tick(&mut self, dt_ms: f32, world: &mut impl VehicleWorld) -> bool {
        if self.record.traffic.base.automata_flags & REMOVE != 0 { return false; }
        let Some(state) = self.record.traffic.base.states.last() else { return false; };
        let StateExtra::AttachAnim { model, flag, anim_selector } = &state.extra else { return false; };
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

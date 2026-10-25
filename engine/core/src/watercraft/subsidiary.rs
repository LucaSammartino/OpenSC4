// SPDX-License-Identifier: GPL-3.0-or-later
//! Settled inherited slots 83/103/104/107 (§§5.1.5–5.1.7).
use super::{math, Event, SpeedTween, Watercraft};
use crate::automata::Vec3;

/// Only model/renderer services, never completed owner arithmetic.
pub trait AnimatedModel {
    fn visibility(&mut self, translucent: bool, value: f32);
    fn advance(&mut self, time: f32);
    fn frame_count(&self) -> f32;
    fn frame(&self) -> f32;
}
pub trait AnimationService {
    fn slot17(&mut self) -> bool;
    fn slot15(&mut self) -> bool;
}
pub trait Lighting {
    fn light_model(&mut self, control: bool);
}

#[derive(Clone, Debug, Default)]
pub struct Subsidiary {
    pub visibility: SpeedTween,
    pub attachments_present: bool,
    pub last_lit_position: Vec3,
    pub last_lit_heading: Vec3,
}

impl Subsidiary {
    pub fn visibility_tick(&mut self, time: f32, models: &mut [Option<&mut dyn AnimatedModel>]) {
        let before = self.visibility.sample();
        if self.visibility.elapsed < self.visibility.duration {
            self.visibility.elapsed += time * 0.001_f32;
            if self.visibility.elapsed >= self.visibility.duration {
                self.visibility.elapsed = 0.0;
                self.visibility.duration = 0.0;
                self.visibility.start = self.visibility.target;
            }
        }
        let after = self.visibility.sample();
        if before != after {
            for model in models.iter_mut().flatten() {
                if after >= 1.0 {
                    model.visibility(false, 1.0);
                } else {
                    model.visibility(true, after);
                }
            }
        }
    }

    pub fn animation_tick(
        &mut self,
        time: f32,
        service: &mut dyn AnimationService,
        models: &mut [Option<&mut dyn AnimatedModel>],
        boat: &mut Watercraft,
    ) -> bool {
        if !service.slot17() && service.slot15() {
            return true;
        }
        for model in models.iter_mut().flatten() {
            model.advance(time);
        }
        if self.attachments_present && !models.is_empty() {
            let model = models[0].as_ref().expect("first attached animation model is required");
            if model.frame() >= model.frame_count() - 1.0 {
                boat.events.push(Event::Notify(1, 0));
            }
        }
        // Slot 103 and slot 100 share +0x1C0; publication consumes this clock.
        boat.models.event_counter += time;
        true
    }

    pub fn lighting_tick(
        &mut self,
        boat: &Watercraft,
        processed_model: bool,
        heading_threshold: Vec3,
        distance_squared: f32,
        lighting: Option<&mut dyn Lighting>,
    ) {
        self.lighting_at(
            boat.record.base.automata_flags,
            boat.inherited_position,
            boat.inherited_heading,
            processed_model,
            heading_threshold,
            distance_squared,
            lighting,
        );
    }

    /// Slot 107 with explicit inherited position/heading (§5.1.5).
    #[allow(clippy::too_many_arguments)] // Owner flags, inherited P/H and renderer configuration.
    pub fn lighting_at(
        &mut self,
        flags: u32,
        position: Vec3,
        heading: Vec3,
        processed_model: bool,
        heading_threshold: Vec3,
        distance_squared: f32,
        lighting: Option<&mut dyn Lighting>,
    ) {
        if flags & 0x100000 != 0 || !processed_model {
            return;
        }
        let Some(lighting) = lighting else {
            return;
        };
        let delta = math::sub(heading, self.last_lit_heading);
        let moved = math::sub(position, self.last_lit_position);
        if delta
            .iter()
            .zip(heading_threshold)
            .any(|(d, threshold)| d.abs() > threshold)
            || math::dot(moved, moved) > distance_squared
        {
            lighting.light_model(false);
            self.last_lit_heading = heading;
            self.last_lit_position = position;
        }
    }
}

impl Watercraft {
    /// Execute wake requests through the shared core controller. Stop has an
    /// immediate slot-104 refresh after every name, even when no name matches.
    pub fn update_wakes_with_controller(
        &mut self,
        time: f32,
        cell_size: f32,
        controller: &mut crate::model_reg_effects::ModelRegEffects<'_>,
        factory: &mut dyn crate::model_reg_effects::EffectFactory,
    ) {
        let first = self.events.len();
        self.update_wakes(time, cell_size);
        for event in &self.events[first..] {
            match event {
                Event::StartWake(name) => {
                    controller.start_named(name, 0, false, factory);
                }
                Event::StopWake(name) => controller.stop_named(name, 0),
                Event::EffectUpdate => {
                    self.update_model_effects(controller);
                }
                _ => unreachable!("wake helpers emit only wake and effect-update requests"),
            }
        }
    }

    /// Execute the shared core controller, including local REGP composition,
    /// ordered traversal and transform-before-speed parameter publication.
    pub fn update_model_effects(
        &self,
        controller: &mut crate::model_reg_effects::ModelRegEffects<'_>,
    ) -> crate::model_reg_effects::UpdateReport {
        controller.update_with_motion(0, self.inherited_heading, self.inherited_measured_speed)
    }

    /// Inherited animation's settled order. `state_tick` is an owner dispatch
    /// hook, not a host-provided movement result. Both time arguments survive.
    pub fn inherited_animation(
        &mut self,
        movement: f32,
        animation: f32,
        mut subsidiary: impl FnMut(&mut Self, u32, f32),
        mut state_tick: impl FnMut(&mut Self, f32) -> bool,
        scale: f32,
        adjust: impl FnMut(Vec3, &mut Vec3, u32),
    ) -> bool {
        self.advance_speed_tween(animation);
        subsidiary(self, 83, animation);
        subsidiary(self, 103, animation);
        if self.record.base.automata_flags & 2 != 0
            && !state_tick(self, animation)
            && self.record.base.automata_flags & 4 == 0
        {
            self.generic_seek(movement, scale, adjust);
        }
        self.events.push(Event::Notify(0, 0));
        true
    }
}

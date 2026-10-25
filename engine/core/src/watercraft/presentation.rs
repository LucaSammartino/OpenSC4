// SPDX-License-Identifier: GPL-3.0-or-later
//! Executes the owner's ordered presentation requests (watercraft §§5.1,
//! 5.1.1, 5.1.4–5.1.7, 3.8): wake start/stop, slot 104 effect updates,
//! slot 107 lighting and slot 100 model publication.
//!
//! Requests are executed in emission order, each with the inherited motion
//! captured when it was requested, so a later publication inside the same
//! tick cannot leak into an earlier slot-104/107 computation.

use crate::automata::Vec3;
use crate::model_reg_effects::{EffectFactory, ModelRegEffects};

use super::{AnimationServices, Event, ModelServices, Publication, Watercraft};

/// Inherited P (+0x40), H (+0x58) and measured speed (+0x94) at request time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresentationSnapshot {
    pub event: usize,
    pub position: Vec3,
    pub heading: Vec3,
    pub measured_speed: f32,
}

/// Optional executors. An absent executor leaves its requests as output
/// commands in `Watercraft::events` for an adapter.
#[derive(Default)]
pub struct PresentationServices<'a, 'effects> {
    /// Embedded effect controller (+0x18C) and its effect factory.
    pub effects: Option<(&'a mut ModelRegEffects<'effects>, &'a mut dyn EffectFactory)>,
    /// Model services for slot 100 publication (§3.8).
    pub models: Option<&'a mut dyn ModelServices>,
}

impl Watercraft {
    fn snapshot_now(&mut self) {
        let event = self.events.len();
        // Requests removed by a caller clearing `events` lose their snapshot.
        self.presentation_snapshots.retain(|snapshot| snapshot.event < event);
        self.presentation_snapshots.push(PresentationSnapshot {
            event,
            position: self.inherited_position,
            heading: self.inherited_heading,
            measured_speed: self.inherited_measured_speed,
        });
    }

    /// Slot 104 request.
    pub(super) fn request_effect_update(&mut self) {
        self.snapshot_now();
        self.events.push(Event::EffectUpdate);
    }

    /// Slot 107 request.
    pub(super) fn request_model_update(&mut self) {
        self.snapshot_now();
        self.events.push(Event::ModelUpdate);
    }

    fn snapshot_for(&self, event: usize) -> PresentationSnapshot {
        self.presentation_snapshots
            .iter()
            .rev()
            .find(|snapshot| snapshot.event == event)
            .copied()
            .unwrap_or(PresentationSnapshot {
                event,
                position: self.inherited_position,
                heading: self.inherited_heading,
                measured_speed: self.inherited_measured_speed,
            })
    }

    /// Execute requests `events[from..]` in order. Lighting (slot 107) always
    /// runs through `animation`; wakes/slot 104 need `services.effects` and
    /// slot 100 needs `services.models`.
    pub fn execute_presentation(
        &mut self,
        from: usize,
        animation: &mut AnimationServices<'_, '_>,
        services: &mut PresentationServices<'_, '_>,
    ) {
        let end = self.events.len();
        let mut index = from;
        while index < end {
            let event = self.events[index].clone();
            match event {
                Event::StartWake(name) => {
                    if let Some((controller, factory)) = services.effects.as_mut() {
                        controller.start_named(name, 0, false, &mut **factory);
                    }
                }
                Event::StopWake(name) => {
                    if let Some((controller, _)) = services.effects.as_mut() {
                        controller.stop_named(name, 0);
                    }
                }
                Event::EffectUpdate => {
                    if let Some((controller, _)) = services.effects.as_mut() {
                        let snapshot = self.snapshot_for(index);
                        controller.update_with_motion(0, snapshot.heading, snapshot.measured_speed);
                    }
                }
                Event::ModelUpdate => {
                    let snapshot = self.snapshot_for(index);
                    animation.subsidiary.lighting_at(
                        self.record.base.automata_flags,
                        snapshot.position,
                        snapshot.heading,
                        animation.processed_model,
                        animation.heading_threshold,
                        animation.distance_squared,
                        animation
                            .lighting
                            .as_mut()
                            .map(|lighting| &mut **lighting as &mut dyn super::Lighting),
                    );
                }
                Event::Publish(Publication::Update(full, partial)) => {
                    if let Some(models) = services.models.as_mut() {
                        self.publish_models(full, partial, &mut **models);
                    }
                }
                _ => {}
            }
            index += 1;
        }
        self.presentation_snapshots.retain(|snapshot| snapshot.event < from);
    }
}

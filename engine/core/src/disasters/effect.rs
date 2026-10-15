// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The effect disasters (§11, short form): an instance per visual effect
//! that the disaster menu places; the damage is the effects manager's.

use super::system::{Core, InstanceState};
use super::{instance_class, kind, msg, DisasterMessage};

#[derive(Clone, Debug, Default)]
pub(crate) struct EffectState {
    /// The visual effect (host token).
    pub effect: u64,
    pub position: (f32, f32),
    pub cell: (i32, i32),
}

impl Core {
    fn es(&mut self, id: u32) -> Option<&mut EffectState> {
        match &mut self.instances.get_mut(&id)?.state {
            InstanceState::Effect(e) => Some(e),
            _ => None,
        }
    }

    /// `StartDisasterEffect(effect, position, …)`: a new instance, `Init`ed
    /// (§11: the manager, the two subscriptions, `DisasterHasBegun`). It
    /// registers no location (none is given) and is in phase 1 while it runs
    /// [choice, Q3].
    pub(crate) fn effect_start(&mut self, effect: u64, x: f32, z: f32) -> Option<u32> {
        if !self.managers.contains_key(&kind::EFFECT) || effect == 0 {
            return None;
        }
        let cell = self.world_to_cell(x, z);
        let id = self.new_instance(
            kind::EFFECT,
            instance_class::EFFECT,
            InstanceState::Effect(Box::new(EffectState {
                effect,
                position: (x, z),
                cell,
            })),
        );
        self.add_instance_to_list(kind::EFFECT, id);
        self.subscribe(id, msg::EFFECT_FINISHED);
        self.subscribe(id, msg::EFFECT_MOVED);
        if let Some(i) = self.instances.get_mut(&id) {
            i.phase = 1;
        }
        self.disaster_has_begun(id);
        Some(id)
    }

    /// `0x2A7B582E` (moved: x, z) and `0x2A7B582D` (finished: shut down,
    /// which ends the disaster) for this instance's effect (void1).
    pub(crate) fn effect_message(&mut self, id: u32, m: &DisasterMessage) -> bool {
        let cell = self.world_to_cell(m.x, m.z);
        let Some(e) = self.es(id) else { return false };
        if m.void1 != e.effect {
            return false;
        }
        match m.msg_type {
            msg::EFFECT_MOVED => {
                e.position = (m.x, m.z);
                e.cell = cell;
                true
            }
            msg::EFFECT_FINISHED => {
                if let Some(i) = self.instances.get_mut(&id) {
                    i.phase = 3;
                }
                self.disaster_has_ended();
                self.instance_finish(id);
                true
            }
            _ => false,
        }
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
//! Executable inherited Init boundary (watercraft §§3, 3.5–3.8).
use std::sync::Arc;

use crate::model_reg_effects::{EffectFactory, ModelInstance, ModelRegEffects};

use super::{EffectResources, ModelServices, Result, Watercraft};

/// Foreign occupant and model interfaces only; model loading and effect
/// discovery/update are executed by the core, including their failure branches.
pub trait LifecycleServices {
    fn initialized(&mut self) -> bool;
    fn model_effect_instance(&mut self, instance: u64) -> Arc<dyn ModelInstance>;
    fn occupant_init(&mut self) -> bool;
}

pub struct InitializationServices<'a, 'effects> {
    pub models: &'a mut dyn ModelServices,
    pub lifecycle: &'a mut dyn LifecycleServices,
    pub controller: &'a mut ModelRegEffects<'effects>,
    pub factory: &'a mut dyn EffectFactory,
    pub effects: &'a mut dyn EffectResources,
}

impl Watercraft {
    /// Inherited Init's result is newly loaded models, even when occupant Init
    /// reports failure. The watercraft entry deliberately ignores that result.
    pub fn initialize_inherited(&mut self, services: &mut InitializationServices<'_, '_>) -> bool {
        if services.lifecycle.initialized() {
            return true;
        }
        let loaded = self.load_models(services.models);
        if loaded {
            if let Some(first) = self.models.entries.first() {
                let instance = services
                    .models
                    .instance(first.expect("first loaded model is required (§3.5)"));
                let effect_instance = services.lifecycle.model_effect_instance(instance);
                services.controller.set_model_instance(Some(effect_instance));
                services.controller.init(services.factory);
                self.update_model_effects(services.controller);
            }
        }
        services.lifecycle.occupant_init();
        loaded
    }

    /// Complete owner Init: base first, six boat effects second. This also
    /// preserves partial model loads when inherited Init returns false.
    pub fn initialize(&mut self, services: &mut InitializationServices<'_, '_>) -> Result<bool> {
        if services.lifecycle.initialized() {
            return Ok(true);
        }
        self.initialize_inherited(services);
        self.initialize_owner_runtime(self.inherited_position, self.inherited_heading, services.effects)
    }
}

/// Foreign inherited-Shutdown slots (`0x0050BA50`, §3.5). The owner executes
/// their order, the controller shutdown and model removal itself.
pub trait ShutdownServices {
    fn initialized(&mut self) -> bool;
    /// Slot 69.
    fn slot69(&mut self);
    /// Slot 111 with the current state.
    fn clean_current_state(&mut self);
    /// Clear/release +0xB0.
    fn release_b0(&mut self);
    /// Disable the active/visible registrations whose bits are set.
    fn disable_registrations(&mut self);
    fn slot127(&mut self);
    fn slot98(&mut self);
    /// Release the nonnull inherited effect-list references and empty the list.
    fn release_effect_list(&mut self);
    /// Clear/release +0x98, then zero +0x84, +0x7C and +0x88.
    fn release_98_and_zero(&mut self);
    /// Read and zero the +0x1D4 sound identifier.
    fn take_sound_identifier(&mut self) -> u32;
    fn sound_event(&mut self, event: u32, identifier: u32);
    fn occupant_shutdown(&mut self) -> bool;
}

pub const SHUTDOWN_SOUND_EVENT: u32 = 0x6A45_1908;

pub struct ShutdownStack<'a, 'effects> {
    pub models: &'a mut dyn ModelServices,
    pub lifecycle: &'a mut dyn ShutdownServices,
    pub controller: &'a mut ModelRegEffects<'effects>,
}

impl Watercraft {
    /// Inherited Shutdown: slot 69 and controller stop/unbind even when
    /// uninitialized, then the initialized-only teardown. Always true.
    pub fn shutdown_inherited(&mut self, services: &mut ShutdownStack<'_, '_>) -> bool {
        services.lifecycle.slot69();
        services.controller.shutdown();
        if !services.lifecycle.initialized() {
            return true;
        }
        services.lifecycle.clean_current_state();
        self.remove_models(services.models);
        services.lifecycle.release_b0();
        services.lifecycle.disable_registrations();
        services.lifecycle.slot127();
        services.lifecycle.slot98();
        services.lifecycle.release_effect_list();
        services.lifecycle.release_98_and_zero();
        let identifier = services.lifecycle.take_sound_identifier();
        if identifier != 0 {
            services.lifecycle.sound_event(SHUTDOWN_SOUND_EVENT, identifier);
        }
        services.lifecycle.occupant_shutdown();
        true
    }
}

impl super::Fleet {
    /// Complete Shutdown (`0x0056D0E0`): watercraft cleanup only when the
    /// inherited initialized predicate holds, then inherited Shutdown, whose
    /// result is returned.
    pub fn shutdown_complete(
        &mut self,
        id: super::BoatId,
        manager: &mut Option<&mut dyn super::DockManager>,
        services: &mut ShutdownStack<'_, '_>,
    ) -> Result<bool> {
        if services.lifecycle.initialized() {
            self.owner_shutdown(id, manager)?;
        }
        let boat = self.boat_mut(id)?;
        boat.events.push(super::Event::BaseShutdown);
        let result = boat.shutdown_inherited(services);
        boat.initialized = false;
        boat.shut_down = true;
        Ok(result)
    }
}

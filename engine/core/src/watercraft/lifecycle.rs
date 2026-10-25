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

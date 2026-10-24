// SPDX-License-Identifier: GPL-3.0-or-later
//! Watercraft §§3.6–3.8. Handles identify foreign interfaces; all branching,
//! transforms, vector insertion and reference transfers are computed here.
use super::{Event, Transform, Watercraft};
use crate::automata::Vec3;
use crate::stream::ResKey;

pub type ModelHandle = u64;

#[derive(Debug, Default)]
pub struct ModelStorage {
    /// Each nonnull entry owns the reference transferred by acquisition.
    pub entries: Vec<Option<ModelHandle>>,
    pub event_counter: f32,
}

/// Foreign interface operations only. Required services/true acquisitions must
/// be usable (§3.7); null is not a recoverable owner success path.
pub trait ModelServices {
    fn sprite_lookup(&mut self, key: ResKey, iid: u32, tail: [u32; 2], out: &mut Option<u64>) -> bool;
    fn sprite_create(&mut self, resource: u64, argument: u32, out: &mut Option<ModelHandle>) -> bool;
    fn mesh_construct(&mut self, argument: u32) -> ModelHandle;
    fn mesh_load(&mut self, model: ModelHandle, key: ResKey) -> bool;
    fn add_ref(&mut self, reference: u64);
    fn release(&mut self, reference: u64);
    fn instance(&mut self, model: ModelHandle) -> u64;
    fn bounds(&mut self, instance: u64) -> [Vec3; 2];
    fn read_transform(&mut self, instance: u64, out: &mut Transform);
    fn write_transform(&mut self, instance: u64, transform: &Transform);
    fn associate(&mut self, instance: u64);
    fn instance_mode(&mut self, instance: u64, mode: u32);
    fn primary_mode(&mut self, model: ModelHandle, first: u32, second: u32);
    fn update_view(&mut self, model: ModelHandle);
    fn auxiliary_selectors(&mut self, property: u32) -> Option<Vec<u32>>;
    fn property_key(&mut self, property: u32) -> Option<ResKey>;
    fn visibility(&mut self, flags: u32) -> bool;
    fn render_full(&mut self, instance: u64);
    fn render_partial(&mut self, instance: u64, flags: u32);
    // Mutable flags model the native reentrant changes; owner rereads them.
    fn occupant_update(&mut self, argument: u32, flags: &mut u8);
    fn owner70(&mut self, enabled: bool, flags: &mut u8);
    fn owner71(&mut self, enabled: bool, flags: &mut u8);
    fn owner83(&mut self, time: f32);
    fn owner106(&mut self);
}

pub fn acquire_model(key: ResKey, out: &mut Option<ModelHandle>, service: &mut dyn ModelServices) -> bool {
    match key.type_id {
        0x5AD0_E817 => {
            let mut resource = None;
            if !service.sprite_lookup(key, 0x9AD0_E7E3, [0; 2], &mut resource) {
                return false;
            }
            let resource = resource.expect("true sprite lookup requires a resource (§3.7)");
            let result = service.sprite_create(resource, 0, out);
            service.release(resource);
            result
        }
        0x29A5_D1EC => {
            let model = service.mesh_construct(1);
            service.add_ref(model);
            if service.mesh_load(model, key) {
                *out = Some(model);
                true
            } else {
                service.release(model);
                *out = None;
                false
            }
        }
        _ => false,
    }
}

impl Watercraft {
    pub fn position_model_instance(&self, instance: u64, service: &mut dyn ModelServices) {
        let mut transform = Transform::default();
        service.read_transform(instance, &mut transform);
        let [hx, hy, hz] = self.inherited_heading.map(f64::from);
        let q = -(hx * hx + (-hz) * (-hz)).sqrt();
        let (right, up) = if q < 0.0 {
            let s = 1.0 / q;
            let r = [((-hz) * s) as f32, (0.0 * s) as f32, (hx * s) as f32];
            let [rx, ry, rz] = r.map(f64::from);
            let u = [
                (rz * hy - ry * hz) as f32,
                (rx * hz - rz * hx) as f32,
                (ry * hx - rx * hy) as f32,
            ];
            (r, u)
        } else {
            ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0])
        };
        transform.basis = [right, up, self.inherited_heading];
        transform.translation = self.inherited_position;
        transform.revision = transform.revision.wrapping_add(2);
        transform.flags |= 5;
        service.write_transform(instance, &transform);
    }

    pub fn add_model_instance(&mut self, key: ResKey, service: &mut dyn ModelServices) -> Option<ModelHandle> {
        if key.instance == 0 {
            return None;
        }
        if self.record.base.model.instance == 0 {
            self.record.base.model = key;
            self.record.base.u2 = key.instance;
        }
        let restore = self.record.base.occupant.flags & 2 != 0;
        if restore {
            service.owner71(false, &mut self.record.base.occupant.flags);
        }
        let mut model = None;
        if acquire_model(key, &mut model, service) {
            let acquired = model.expect("successful model acquisition requires a model (§3.7)");
            let instance = service.instance(acquired);
            service.associate(instance);
            service.instance_mode(instance, 1);
            service.update_view(acquired);
            self.position_model_instance(instance, service);
            self.models.entries.push(Some(acquired));
            self.record.base.model_keys.push(key);
        }
        if restore {
            service.owner71(true, &mut self.record.base.occupant.flags);
        }
        service.owner106();
        model
    }

    pub fn load_auxiliary_model(
        &mut self,
        selector: u32,
        out: &mut Option<ModelHandle>,
        service: &mut dyn ModelServices,
    ) -> bool {
        let property = match selector {
            1 => 0x09C8_0417,
            2 => 0xC9E6_9AF6,
            3 => 0xEC21_4180,
            other => other,
        };
        let Some(key) = service.property_key(property) else {
            return false;
        };
        *out = self.add_model_instance(key, service);
        if let Some(model) = *out {
            service.add_ref(model);
            true
        } else {
            false
        }
    }

    pub fn load_models(&mut self, service: &mut dyn ModelServices) -> bool {
        let mut loaded = false;
        if self.models.entries.is_empty() {
            if self.record.base.model_keys.is_empty() && self.record.base.model.instance != 0 {
                self.record.base.model_keys.push(self.record.base.model);
            }
            // Output is initialized once, before the primary loop (§3.6).
            let mut model = None;
            for (index, key) in self.record.base.model_keys.clone().into_iter().enumerate() {
                if !acquire_model(key, &mut model, service) {
                    return false;
                }
                let acquired = model.expect("successful model acquisition requires a model (§3.6)");
                self.models.entries.push(Some(acquired));
                let instance = service.instance(acquired);
                if index == 0 {
                    let bounds = service.bounds(instance);
                    self.radius = (bounds[1][2] - bounds[0][2]) * 0.5;
                    if self.radius > 32.0 {
                        self.radius = 32.0;
                    }
                }
                service.primary_mode(acquired, 1, 0);
                self.position_model_instance(instance, service);
                service.associate(instance);
                service.instance_mode(instance, 3);
                service.update_view(acquired);
                loaded = true;
            }
            if let Some(selectors) = service.auxiliary_selectors(0xCC21_420D) {
                for selector in selectors.into_iter().filter(|s| matches!(s, 1..=3)) {
                    let mut output = None;
                    self.load_auxiliary_model(selector, &mut output, service);
                    if let Some(model) = output {
                        service.release(model);
                    }
                }
            }
        }
        service.owner83(0.0);
        service.owner106();
        loaded
    }

    pub fn publish_models(&mut self, full: bool, partial: bool, service: &mut dyn ModelServices) {
        if let Some(first) = self.models.entries.first() {
            let instance = service.instance(first.expect("first model must be nonnull (§3.8)"));
            self.occupant_bounds = service.bounds(instance);
        }
        if service.visibility(4) {
            if self.record.base.occupant.flags & 2 != 0 {
                for model in self.models.entries.iter().flatten() {
                    if full {
                        let instance = service.instance(*model);
                        service.render_full(instance);
                    } else if partial {
                        let instance = service.instance(*model);
                        service.render_partial(instance, 3);
                    }
                }
                if self.record.base.occupant.flags & 1 != 0 {
                    service.occupant_update(0, &mut self.record.base.occupant.flags);
                }
            } else {
                if self.record.base.occupant.flags & 1 == 0 {
                    service.owner70(true, &mut self.record.base.occupant.flags);
                } else {
                    service.occupant_update(0, &mut self.record.base.occupant.flags);
                }
                if self.record.base.occupant.flags & 1 != 0 {
                    service.owner71(true, &mut self.record.base.occupant.flags);
                }
            }
        }
        if self.models.event_counter > 100.0 {
            self.events.push(Event::Notify(9, 0));
            self.models.event_counter = 0.0;
        }
    }

    /// Release only references retained by the model vector, never instance
    /// references borrowed from a model. Lifecycle invokes this before base teardown.
    pub fn remove_models(&mut self, service: &mut dyn ModelServices) {
        for model in self.models.entries.drain(..).flatten() {
            service.release(model);
        }
    }
}

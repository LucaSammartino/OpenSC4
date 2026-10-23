// SPDX-License-Identifier: GPL-3.0-or-later
//! particles.md §17: stable value handles and the settled noncontact integrator.
//! Does not substitute a numeric solver for PT11. Existing raw collision/list
//! helpers remain in the parent and storage modules.
use crate::effects::particle_owners::Unsupported;
use std::collections::BTreeMap;

pub const SECTOR_WIDTH: f32 = 64.;
pub const SECTOR_RECIPROCAL: f32 = 1. / 64.;
pub type BodyId = u64;

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalDescription {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub height: f32,
    pub height_velocity: f32,
    pub angle: f32,
    pub angular_velocity: f32,
    pub acceleration: [f32; 2],
    pub friction: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Object {
    pub description: PhysicalDescription,
    pub axes: [f32; 2],
    pub live: bool,
    pub infinite_mass: bool,
}
#[derive(Clone, Debug, Default)]
pub struct Simulator {
    next: BodyId,
    pub objects: BTreeMap<BodyId, Object>,
    /// Link creation/numerics remain PT11; stored endpoint records allow the
    /// specified RemoveObject ownership rule without guessing joint forces.
    pub joints: Vec<[BodyId; 2]>,
    pub sectors: BTreeMap<u32, Vec<BodyId>>,
}

/// §17.3 literally says component-wise MAX. The older rigid-body-2d.md
/// helper uses MIN; keep that contract separate pending PT12.
pub fn primary_collision_sector(current: u32, a: u32, b: u32) -> bool {
    current == ((a & 0xffff_0000).max(b & 0xffff_0000) | (a & 0xffff).max(b & 0xffff))
}
impl Simulator {
    pub fn add_object(&mut self, description: &PhysicalDescription, infinite_mass: bool) -> BodyId {
        self.next += 1;
        self.objects.insert(
            self.next,
            Object {
                description: description.clone(),
                axes: [description.angle.cos(), description.angle.sin()],
                live: true,
                infinite_mass,
            },
        );
        self.next
    }
    /// Valid retained body ID is a precondition, as for native RemoveObject.
    pub fn remove_object(&mut self, id: BodyId) {
        self.joints.retain(|joint| !joint.contains(&id));
        for ids in self.sectors.values_mut() {
            ids.retain(|&body| body != id);
        }
        self.sectors.retain(|_, ids| !ids.is_empty());
        self.objects.get_mut(&id).expect("valid body handle").live = false;
    }
    pub fn remove_dead_objects(&mut self) {
        self.objects.retain(|_, body| body.live);
    }
    pub fn integrate_list(&mut self, infinite_mass: bool, dt: f32) -> Result<(), Unsupported> {
        for body in self
            .objects
            .values_mut()
            .filter(|b| b.live && b.infinite_mass == infinite_mass)
        {
            // Exact friction/wrapping and extent registration remain PT11.
            // Reject before making unsupported stores, rather than free flight.
            let d = &mut body.description;
            for i in 0..2 {
                d.position[i] =
                    (f64::from(d.position[i]) + f64::from(dt) * f64::from(d.velocity[i])) as f32;
            }
            d.height = (f64::from(d.height) + f64::from(dt) * f64::from(d.height_velocity)) as f32;
            d.angle = (f64::from(d.angle) + f64::from(dt) * f64::from(d.angular_velocity)) as f32;
            if d.angle != 0. || d.angular_velocity != 0. {
                return Err(Unsupported::PT11);
            }
            body.axes = [d.angle.cos(), d.angle.sin()];
            for i in 0..2 {
                d.velocity[i] = (f64::from(d.velocity[i])
                    + f64::from(dt) * f64::from(d.acceleration[i]))
                    as f32;
            }
            if d.friction {
                return Err(Unsupported::PT11);
            }
        }
        Ok(())
    }
    /// Only the no-contact domain is settled. Collision dispatch always follows
    /// both lists. Returning Unsupported does not erase prior integration.
    pub fn advance_time(&mut self, dt: f32) -> Result<(), Unsupported> {
        self.integrate_list(false, dt)?;
        self.integrate_list(true, dt)?;
        if self.objects.values().filter(|b| b.live).count() > 1 || !self.joints.is_empty() {
            return Err(Unsupported::PT11);
        }
        Ok(())
    }
}

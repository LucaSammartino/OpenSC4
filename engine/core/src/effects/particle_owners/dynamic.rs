// SPDX-License-Identifier: GPL-3.0-or-later
//! particles.md §16. Host-entry and body identities are deliberately distinct.
use super::Unsupported;
use crate::rigid_body_2d::simulator::{BodyId, PhysicalDescription, Simulator};
use std::collections::BTreeMap;

pub const MESSAGE: u32 = 0x86ad_10ee;
pub const MAX_STEP: f32 = 0.005;
pub type EntryId = u64;
pub type SectorKey = (i32, i32);
pub trait Foreign {
    fn retain_drawable(&mut self, drawable: u64);
    fn release_drawable(&mut self, drawable: u64);
    fn static_occupants(&mut self, key: SectorKey) -> Vec<u64>;
    fn reset_shading(&mut self);
}
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub body: Option<BodyId>,
    pub drawable: Option<u64>,
    pub radius: f32,
    /// None is the uninitialized FF800001 sentinel, not sector zero.
    pub sector: Option<SectorKey>,
    pub application_id: u32,
    pub transform: Option<[[f32; 4]; 3]>,
}
#[derive(Clone, Debug, Default)]
pub struct Sector {
    pub references: u32,
    pub static_bodies: Vec<BodyId>,
}
#[derive(Clone, Debug)]
pub struct DynamicParticles {
    pub bodies: Simulator,
    pub entries: BTreeMap<EntryId, Entry>,
    pub sectors: BTreeMap<SectorKey, Sector>,
    pub sector_world_size: f32,
    pub sector_reciprocal: f32,
    pub seconds: f32,
    /// Integration-call trace; diagnostic only, not a deferred-time accumulator.
    pub steps: Vec<f32>,
    next: EntryId,
}
impl DynamicParticles {
    pub fn new(terrain_cell_size: f32) -> Self {
        let size = terrain_cell_size * 4.;
        Self {
            bodies: Simulator::default(),
            entries: BTreeMap::new(),
            sectors: BTreeMap::new(),
            sector_world_size: size,
            sector_reciprocal: 1. / size,
            seconds: 0.,
            steps: Vec::new(),
            next: 0,
        }
    }
    pub fn add_particle(
        &mut self,
        description: &PhysicalDescription,
        drawable: Option<u64>,
        radius: f32,
        application_id: u32,
        foreign: &mut impl Foreign,
    ) -> EntryId {
        let body = self.bodies.add_object(description, false);
        self.next += 1;
        if let Some(drawable) = drawable {
            foreign.retain_drawable(drawable);
        }
        self.entries.insert(
            self.next,
            Entry {
                body: Some(body),
                drawable,
                radius,
                sector: None,
                application_id,
                transform: None,
            },
        );
        self.next
    }
    pub fn link_particles(&self, first: u32, second: u32) -> Result<bool, Unsupported> {
        let a = self.entries.values().find(|e| e.application_id == first);
        let b = self.entries.values().find(|e| e.application_id == second);
        if a.is_none() || b.is_none() {
            return Ok(false);
        }
        Err(Unsupported::PT11)
    }
    /// Settled refcount transition on explicit keys. Radius-to-key construction
    /// remains PT10, and is never guessed by this operation.
    pub fn activate_sector(&mut self, key: SectorKey, foreign: &mut impl Foreign) -> Result<(), Unsupported> {
        if let Some(sector) = self.sectors.get_mut(&key) {
            sector.references += 1;
            return Ok(());
        }
        if !foreign.static_occupants(key).is_empty() {
            return Err(Unsupported::PT10);
        }
        self.sectors.insert(
            key,
            Sector {
                references: 1,
                static_bodies: Vec::new(),
            },
        );
        Ok(())
    }
    pub fn deactivate_sector(&mut self, key: SectorKey) {
        let sector = self.sectors.get_mut(&key).expect("active sector");
        sector.references -= 1;
        if sector.references == 0 {
            for body in self.sectors.remove(&key).expect("active sector").static_bodies {
                self.bodies.remove_object(body);
            }
        }
    }
    /// Already-resolved neighborhood keys, all new activations precede ANY old
    /// deactivation so even overlapping static sectors stay referenced.
    pub fn move_neighborhood(
        &mut self,
        old: &[SectorKey],
        new: &[SectorKey],
        foreign: &mut impl Foreign,
    ) -> Result<(), Unsupported> {
        for &key in new {
            self.activate_sector(key, foreign)?;
        }
        for &key in old {
            self.deactivate_sector(key);
        }
        Ok(())
    }
    pub fn delete_particle(&mut self, id: EntryId, foreign: &mut impl Foreign) -> Result<(), Unsupported> {
        let Some(entry) = self.entries.get(&id) else {
            return Ok(());
        };
        // Native radius/key formula is not settled. Do not remove the entry
        // while leaking a neighborhood that cannot yet be computed.
        if entry.sector.is_some() {
            return Err(Unsupported::PT10);
        }
        let entry = self.entries.remove(&id).expect("present entry");
        if let Some(body) = entry.body {
            self.bodies.remove_object(body);
        }
        if let Some(drawable) = entry.drawable {
            foreign.release_drawable(drawable);
        }
        Ok(())
    }
    pub fn do_message(
        &mut self,
        id: u32,
        payload: impl FnOnce() -> u32,
        city_size: [f32; 2],
    ) -> Result<bool, Unsupported> {
        if id != MESSAGE {
            return Ok(false);
        }
        self.tick(payload(), city_size)?;
        Ok(true)
    }
    pub fn tick(&mut self, microseconds: u32, city_size: [f32; 2]) -> Result<(), Unsupported> {
        self.steps.clear();
        self.seconds = (f64::from(microseconds) * f64::from(1e-6_f32)) as f32;
        self.seconds = self.seconds.min(0.1);
        // Clear dead cached handles before reclaiming their payloads.
        for entry in self.entries.values_mut() {
            if let Some(id) = entry.body {
                if !self.bodies.objects[&id].live {
                    entry.body = None;
                }
            }
        }
        self.bodies.remove_dead_objects();
        let mut remaining = self.seconds;
        while remaining > 0. {
            let step = remaining.min(MAX_STEP);
            self.steps.push(step);
            self.bodies.advance_time(step)?;
            for entry in self.entries.values_mut() {
                if let Some(id) = entry.body {
                    let p = self.bodies.objects[&id].description.position;
                    if p[0] < 0. || p[1] < 0. || p[0] >= city_size[0] || p[1] >= city_size[1] {
                        self.bodies.remove_object(id);
                        entry.body = None;
                    } else {
                        return Err(Unsupported::PT10);
                    }
                }
            }
            remaining -= step;
        }
        // Zero time still reaches transform refresh. Construction of those
        // transforms from terrain normals is explicitly PT10.
        if self.entries.values().any(|e| e.body.is_some()) {
            return Err(Unsupported::PT10);
        }
        Ok(())
    }
    pub fn draw(&self, foreign: &mut impl Foreign) -> Result<bool, Unsupported> {
        let needs_shading = self.entries.values().any(|e| e.body.is_some() && e.drawable.is_some());
        foreign.reset_shading();
        if needs_shading {
            Err(Unsupported::PT10)
        } else {
            Ok(true)
        }
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
//! particles.md §§1,5.1,6,7.1: complete records in the existing emitter.
use super::{descriptions::ParticleDescriptionBase, Unsupported};
use crate::rng::RzRandom;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentKind {
    Ordinary,
    ModelBased,
    GeometrySourced,
}
pub fn component_kind(flags_b: u32, source_scale: f32) -> ComponentKind {
    if flags_b & 1 == 0 {
        ComponentKind::Ordinary
    } else if source_scale == 0. {
        ComponentKind::ModelBased
    } else {
        ComponentKind::GeometrySourced
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub alive: bool,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub field_18: f32,
    /// P+28,2C,30,34,38,3C,40.
    pub factors: [f32; 7],
}
#[derive(Clone, Debug, PartialEq)]
pub struct Motion {
    pub description: ParticleDescriptionBase,
    pub emission_box: [[f32; 3]; 2],
    pub rotation: [[f32; 3]; 3],
    pub translation: [f32; 3],
    pub scale: f32,
    pub component_velocity: [f32; 3],
    pub speed_scale: f32,
    pub size_scales: [f32; 2],
    pub environment_size: f32,
    pub force: [f32; 3],
    pub walk: [f32; 3],
    pub wind: [f32; 3],
    pub lead_axes: [[f32; 3]; 2],
    pub walk_delay: f32,
    pub preferred: Option<[f32; 3]>,
    pub inverse_rotation: [[f32; 3]; 3],
    pub inverse_translation: [f32; 3],
    pub inverse_scale: f32,
    pub city_extent: Option<[f32; 2]>,
    pub collision_effects: Option<[u8; 2]>,
}
impl Motion {
    pub fn new(description: &ParticleDescriptionBase) -> Result<Self, Unsupported> {
        let scalar = |at| description.value.get_scalar(at).ok_or(Unsupported::PT3);
        let vector = |at| Ok([scalar(at)?, scalar(at + 4)?, scalar(at + 8)?]);
        let force = vector(0xe0)?;
        Ok(Self {
            description: description.clone(),
            emission_box: [vector(0x50)?, vector(0x5c)?],
            rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            translation: [0.; 3],
            scale: 1.,
            component_velocity: [0.; 3],
            speed_scale: 1.,
            size_scales: [1.; 2],
            environment_size: 1.,
            force,
            walk: [0.; 3],
            wind: [0.; 3],
            lead_axes: [[1., 0., 0.], [0., 1., 0.]],
            walk_delay: 0.,
            preferred: None,
            inverse_rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            inverse_translation: [0.; 3],
            inverse_scale: 1.,
            city_extent: None,
            collision_effects: None,
        })
    }
    fn scalar(&self, at: u32) -> f32 {
        self.description
            .value
            .get_scalar(at)
            .expect("initialized description scalar")
    }
    pub fn flags(&self) -> u32 {
        self.description.value.words[&0]
    }
    /// Only the untraced helpers are refused; simple and ordinary explicit-Euler
    /// stepped physics share the same records and force accumulator.
    pub fn validate(&self) -> Result<(), Unsupported> {
        let flags = self.flags();
        if flags & (0x7c000 | 0x2000 | 0x10000000 | 0x8000000 | 0x100 | 0x400 | 0x800000) != 0 {
            return Err(Unsupported::PT1);
        }
        if self.description.value.words[&4] & 1 != 0 {
            return Err(Unsupported::PT1);
        }
        if !self
            .description
            .value
            .floats
            .get(&0x1a4)
            .map_or(true, Vec::is_empty)
        {
            return Err(Unsupported::PT1);
        }
        Ok(())
    }
    pub fn reset_force(&mut self) {
        self.walk = [0.; 3];
        self.force = [self.scalar(0xe0), self.scalar(0xe4), self.scalar(0xe8)];
    }
    pub fn configure_source(&mut self, height: f32, extent: [f32; 2]) -> Result<(), Unsupported> {
        if self.flags() & 0x1000 == 0 {
            return Ok(());
        }
        if self.flags() & 0xc000 == 0 {
            self.emission_box[0][1] += height;
            self.emission_box[1][1] += height;
        }
        if self.flags() & 0x2000 != 0 {
            return Err(Unsupported::PT1);
        }
        self.emission_box[0][0] = 0.;
        self.emission_box[0][2] = 0.;
        self.emission_box[1][0] = extent[0];
        self.emission_box[1][2] = extent[1];
        Ok(())
    }
    /// Runtime static baseline is a supplied primitive, never guessed as zero.
    pub fn configure_preferred(&mut self, baseline: [f32; 3]) {
        let direction = [self.scalar(0x18c), self.scalar(0x190), self.scalar(0x194)];
        self.preferred = direction
            .iter()
            .zip(baseline)
            .any(|(&a, b)| (f64::from(a) - f64::from(b)).abs() >= 1e-4)
            .then_some(direction);
    }
    pub fn resolve_collision_effects(&mut self, mut lookup: impl FnMut(&[u8]) -> u8) {
        if self.flags() & 0x100 == 0 {
            return;
        }
        let name = self
            .description
            .value
            .strings
            .get(&0x160)
            .cloned()
            .unwrap_or_default();
        let mut terrain = name.clone();
        terrain.extend_from_slice(b"_terrain");
        let mut water = name;
        water.extend_from_slice(b"_water");
        self.collision_effects = Some([lookup(&terrain), lookup(&water)]);
    }
    pub fn prepare_start(
        &mut self,
        rng: &mut RzRandom,
        sea_access: Option<u32>,
    ) -> Result<(), Unsupported> {
        self.reset_force();
        if self.flags() & 0x800000 != 0 {
            for i in 0..3 {
                self.walk[i] = rng.range_f64(
                    f64::from(self.scalar(0x30 + i as u32 * 4)),
                    f64::from(self.scalar(0x3c + i as u32 * 4)),
                ) as f32;
            }
            self.walk[1] = 0.;
            if self.flags() & 0x1000000 == 0 {
                self.walk_delay = 0.;
                // The immediately called random-walk helper is not specified.
                return Err(Unsupported::PT1);
            }
            let strength =
                rng.range_f64(f64::from(self.scalar(0x17c)), f64::from(self.scalar(0x180)));
            let length = self
                .walk
                .iter()
                .map(|&v| f64::from(v) * f64::from(v))
                .sum::<f64>()
                .sqrt();
            if length != 0. {
                for v in &mut self.walk {
                    *v = (f64::from(*v) * strength / length) as f32;
                }
            }
            self.walk_delay =
                rng.range_f64(f64::from(self.scalar(0x174)), f64::from(self.scalar(0x178))) as f32;
        }
        if self.flags() & 0x2000000 != 0 {
            let access = sea_access.ok_or(Unsupported::PT1)?;
            let direction = [
                (1, [0., 0., -1.]),
                (2, [-1., 0., 0.]),
                (8, [0., 0., 1.]),
                (4, [1., 0., 0.]),
            ]
            .into_iter()
            .find(|(bit, _)| access & bit != 0)
            .map(|(_, direction)| direction);
            self.preferred = direction.map(|v| {
                std::array::from_fn(|i| {
                    let row = self.inverse_rotation[i];
                    ((f64::from(row[0]) * v[0]
                        + f64::from(row[1]) * v[1]
                        + f64::from(row[2]) * v[2])
                        / f64::from(self.inverse_scale)) as f32
                })
            });
        }
        Ok(())
    }
    pub fn frame_force(&mut self) {
        for i in 0..3 {
            self.force[i] = (f64::from(self.scalar(0xe0 + i as u32 * 4))
                + f64::from(self.walk[i])
                + f64::from(self.scalar(0xec)) * f64::from(self.wind[i]))
                as f32;
        }
    }
    /// Life was drawn first by ParticleSimulator::create. Remaining draws are
    /// x,y,z position, x,y,z direction, speed, six variances, optional rotation.
    pub fn init(&self, rng: &mut RzRandom) -> Result<Record, Unsupported> {
        let draw =
            |rng: &mut RzRandom, a: f32, b: f32| rng.range_f64(f64::from(a), f64::from(b)) as f32;
        let mut p = [0.; 3];
        let mut v = [0.; 3];
        for (i, value) in p.iter_mut().enumerate() {
            *value = draw(rng, self.emission_box[0][i], self.emission_box[1][i]);
        }
        for (i, value) in v.iter_mut().enumerate() {
            *value = draw(
                rng,
                self.scalar(0x30 + i as u32 * 4),
                self.scalar(0x3c + i as u32 * 4),
            );
        }
        let speed = f64::from(draw(rng, self.scalar(0x48), self.scalar(0x4c)))
            * f64::from(self.speed_scale);
        let length = v
            .iter()
            .fold(0., |sum, &x| sum + f64::from(x) * f64::from(x))
            .sqrt();
        if length > 0. {
            for x in &mut v {
                *x = (f64::from(*x) * speed / length) as f32;
            }
        }
        if self.scalar(0xf0) > 0. {
            let d: [f64; 3] = std::array::from_fn(|i| {
                f64::from(p[i]) - f64::from(self.scalar(0xf4 + i as u32 * 4))
            });
            let q = (d[0] * d[0] + d[2] * d[2] + d[1] * d[1]) as f32;
            if q >= 0. && q.abs() >= 1e-4 {
                for i in 0..3 {
                    v[i] = (f64::from(v[i])
                        + d[i] * f64::from(self.scalar(0xf0)) / f64::from(q).sqrt())
                        as f32;
                }
            }
        }
        let transform = |value: [f32; 3], scale: f32, offset: [f32; 3]| {
            std::array::from_fn(|i| {
                let row = self.rotation[i].map(f64::from);
                ((((row[2] * f64::from(value[2]) + row[1] * f64::from(value[1]))
                    + row[0] * f64::from(value[0]))
                    * f64::from(scale))
                    + f64::from(offset[i])) as f32
            })
        };
        p = transform(p, self.scale, self.translation);
        v = transform(v, 1., self.component_velocity);
        if self.flags() & 0x7c000 != 0 {
            return Err(Unsupported::PT1);
        }
        if self.flags() & 0x20 != 0 {
            let mut factor = 1.;
            if self.scalar(0xdc) != 0. {
                let dot = |a: [f32; 3]| {
                    (0..3)
                        .map(|i| f64::from(v[i]) * f64::from(a[i]))
                        .sum::<f64>()
                };
                let a = dot(self.lead_axes[1]);
                let b = dot(self.lead_axes[0]);
                factor = (b * b + a * a).sqrt() / f64::from(self.scalar(0xdc));
            }
            let c = self.description.value.floats[&0xac][0];
            for i in 0..3 {
                p[i] = (f64::from(p[i])
                    + (((f64::from(v[i]) * 0.5) * factor) * f64::from(c)) * (1. / speed))
                    as f32;
            }
        }
        let mut factors = [0.; 7];
        for (index, at) in [
            (0, 0x68),
            (1, 0x6c),
            (3, 0x78),
            (4, 0x7c),
            (5, 0x80),
            (6, 0x84),
        ] {
            let variance = self.scalar(at);
            factors[index] =
                rng.range_f64(1. - f64::from(variance), 1. + f64::from(variance)) as f32;
        }
        factors[0] = (f64::from(factors[0])
            * f64::from(self.size_scales[0])
            * f64::from(self.size_scales[1])) as f32;
        let rotation = self.scalar(0x70);
        factors[2] = if rotation <= 0. {
            1.
        } else {
            draw(rng, -rotation, rotation)
        };
        if self.description.value.words[&8] & 1 == 0 {
            factors[0] *= self.environment_size;
        }
        if self.flags() & 0x400000 != 0 {
            factors[0] *= self.scale;
        }
        if self.description.value.bytes[&0xd5] == 1 {
            factors[2] += (-self.rotation[2][0]).atan2(self.rotation[0][0]) * 0.15915494;
        }
        Ok(Record {
            alive: true,
            position: p,
            velocity: v,
            field_18: 0.,
            factors,
        })
    }
    pub fn update(&self, record: &mut Record, dt: f32) -> Result<(), Unsupported> {
        if self.flags() & 0x18000d00 != 0 {
            return self.update_stepped(record, dt);
        }
        let old = record.velocity;
        let drag = 1. - f64::from(dt) * f64::from(self.scalar(0x100));
        for (i, old) in old.into_iter().enumerate() {
            let accelerated = (f64::from(old) + f64::from(dt) * f64::from(self.force[i])) as f32;
            record.velocity[i] = (f64::from(accelerated) * drag) as f32;
            let sum = record.velocity[i] + old;
            record.position[i] =
                (f64::from(record.position[i]) + f64::from(sum) * (f64::from(dt) * 0.5)) as f32;
        }
        Ok(())
    }
    fn update_stepped(&self, record: &mut Record, dt: f32) -> Result<(), Unsupported> {
        let mut remaining = dt;
        loop {
            let step = remaining.min(super::super::particles::RATE_SUBSTEP);
            if self.flags() & 0x10000000 != 0 {
                return Err(Unsupported::PT1);
            }
            for i in 0..3 {
                record.position[i] = (f64::from(record.position[i])
                    + f64::from(step) * f64::from(record.velocity[i]))
                    as f32;
                record.velocity[i] = (f64::from(record.velocity[i])
                    + f64::from(step) * f64::from(self.force[i]))
                    as f32;
            }
            if self.flags() & 0x8000000 != 0 {
                return Err(Unsupported::PT1);
            }
            let drag = 1. - f64::from(remaining) * f64::from(self.scalar(0x100));
            for v in &mut record.velocity {
                *v = (f64::from(*v) * drag) as f32;
            }
            let extent = self.city_extent.ok_or(Unsupported::PT1)?;
            let q: [f32; 3] = std::array::from_fn(|i| {
                let r = self.inverse_rotation[i].map(f64::from);
                let p = record.position.map(f64::from);
                let sum = if i == 1 {
                    (p[0] * r[0] + p[2] * r[2]) + p[1] * r[1]
                } else {
                    (p[2] * r[2] + p[1] * r[1]) + p[0] * r[0]
                } as f32;
                (sum * self.inverse_scale) + self.inverse_translation[i]
            });
            let inside = q[0] > 0. && q[2] > 0. && q[0] < extent[0] && q[2] < extent[1];
            if !inside && self.flags() & 0x800 != 0 {
                record.alive = false;
                return Ok(());
            }
            if inside && self.flags() & 0x500 != 0 {
                return Err(Unsupported::PT1);
            }
            remaining -= step;
            if remaining <= 0. {
                break;
            }
        }
        if self.description.value.words[&4] & 1 != 0 {
            return Err(Unsupported::PT1);
        }
        Ok(())
    }
}

/// Ordered event selection is settled even though effect-name composition is PT2.
pub fn birth_events(events: &[(Vec<u8>, f32)]) -> impl Iterator<Item = &[u8]> {
    events
        .iter()
        .take_while(|(_, age)| *age == 0.)
        .map(|(name, _)| name.as_slice())
}
pub fn crossing_events(
    events: &[(Vec<u8>, f32)],
    age: f32,
    dt: f32,
) -> impl Iterator<Item = &[u8]> {
    events
        .iter()
        .filter(move |(_, time)| {
            let difference = f64::from(age) - f64::from(*time);
            difference <= 0. && difference + f64::from(dt) > 0.
        })
        .map(|(name, _)| name.as_slice())
}

// SPDX-License-Identifier: GPL-3.0-or-later
//! Verified query supplements, city-tools-input §8.16. Normal finite domain;
//! malformed curves and exceptional numeric conversions remain CT19.
use super::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct ResponseCurve {
    pub knots: Vec<(f32, f32)>,
    /// Stored slope of each segment; never reconstructed from its endpoints.
    pub slopes: Vec<f32>,
}
impl ResponseCurve {
    pub fn evaluate(&self, input: f32) -> Option<f64> {
        let first = *self.knots.first()?;
        if !input.is_finite()
            || self.knots.iter().any(|(x, y)| !x.is_finite() || !y.is_finite())
            || self.knots.windows(2).any(|p| p[0].0 >= p[1].0)
            || self.slopes.len() != self.knots.len() - 1
            || self.slopes.iter().any(|s| !s.is_finite())
        {
            return None; // CT19: no invented malformed-property repair policy.
        }
        if input < first.0 {
            return Some(f64::from(first.1));
        }
        for (i, pair) in self.knots.windows(2).enumerate() {
            if input < pair[1].0 {
                // Widen stored binary32 operands; do not round every intermediate
                // to binary32 or fuse the multiply/add.
                return Some(
                    f64::from(pair[0].1) + (f64::from(input) - f64::from(pair[0].0)) * f64::from(self.slopes[i]),
                );
            }
        }
        self.knots.last().map(|p| f64::from(p.1))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BuildingDevelopment {
    pub purpose: u32,
    pub wealth: u32,
}
#[derive(Clone, Debug, Default)]
pub struct Developer {
    pub desirability: i16,
    pub demand: f32,
    pub curves: BTreeMap<(u8, u8), ResponseCurve>,
    pub thresholds: BTreeMap<u8, (i32, i32)>,
}
pub fn find_best_developer(world: &impl World, lot: &Lot) -> Option<(u32, Developer)> {
    if lot.building_type != 0 {
        let record = world.building_development(lot.building_type)?;
        if record.purpose == 0 {
            return None;
        }
        let kind = world.developer_type(record.purpose, record.wealth)?;
        return world.developer(kind, lot.centre).map(|d| (kind, d));
    }
    let candidates: &[u32] = match world.zone(lot.centre) {
        1..=3 => &[0x1010, 0x1020, 0x1030],
        4..=6 => &[0x3110, 0x3120, 0x3130, 0x3320, 0x3330],
        7 => &[0x4100],
        8 => &[0x4400, 0x4300],
        9 => &[0x4200, 0x4400, 0x4300],
        _ => &[],
    };
    let mut best = (-1000_i16, -1000_i32);
    let mut selected = None;
    for kind in candidates {
        if let Some(developer) = world.developer(*kind, lot.centre) {
            let pair = (developer.desirability, developer.demand as i32);
            if pair > best {
                best = pair;
                selected = Some((*kind, developer));
            }
        }
    }
    selected
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubjectiveRating {
    /// 0 low, 1 medium, 2 high, independently of the auxiliary numeric result.
    pub index: u8,
    pub auxiliary: Option<i32>,
}
impl SubjectiveRating {
    pub fn label(self) -> u32 {
        [0x4A5F_E266, 0x0A5F_E260, 0xAA5F_E249][usize::from(self.index)]
    }
}
pub fn developer_rating(world: &impl World, lot: &Lot, rating: u8) -> Option<SubjectiveRating> {
    let (_, d) = find_best_developer(world, lot)?;
    if rating == 6 && !d.curves.contains_key(&(6, 0)) {
        return Some(SubjectiveRating {
            index: 1,
            auxiliary: None,
        });
    }
    let parts: &[u8] = if rating == 0 { &[0, 1, 3] } else { &[0] };
    let mut value = 0.0;
    for part in parts {
        let input = if rating == 6 {
            let byte = world.commute_sample(lot.centre);
            f32::from(if byte == 0 { 255 } else { byte })
        } else {
            world.rating_input(rating, *part, lot.centre)
        };
        value += d.curves.get(&(rating, *part))?.evaluate(input)?;
    }
    let (min, max) = *d.thresholds.get(&rating)?;
    if rating == 6 {
        let v = value as i32;
        return Some(if v < min {
            SubjectiveRating {
                index: 2,
                auxiliary: Some(-v),
            }
        } else if v >= max {
            SubjectiveRating {
                index: 0,
                auxiliary: Some(v / 2),
            }
        } else {
            SubjectiveRating {
                index: 1,
                auxiliary: None,
            }
        });
    }
    Some(SubjectiveRating {
        index: if value < f64::from(min) {
            2
        } else if value >= f64::from(max) {
            0
        } else {
            1
        },
        auxiliary: None,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeterWindow {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub reference_left: Option<i32>,
}
pub fn wealth_geometry(mut window: MeterWindow, wealth: u32) -> MeterWindow {
    let left = window.reference_left.unwrap_or(window.left);
    window.reference_left = Some(left);
    let width = f64::from(window.right - left);
    let size = (f32::from_bits(0x3EAA_AAAB) * wealth as f32).min(1.0);
    window.left = left + (width - f64::from(size) * width) as i32;
    window
}

pub type ResourceKey = (u32, u32, u32);
pub const LOST_LUGGAGE_KEY: ResourceKey = (0x6534_284A, 0x67CD_5FA1, 0xCA27_0E9D);
pub fn airport_lost_luggage(world: &impl World, cell: Cell) -> Option<i32> {
    let properties = world.resource_properties(LOST_LUGGAGE_KEY)?;
    match properties.get(&0xAA60_E745) {
        Some(Property::Curve(curve)) => goofy_stat(curve, world.read(Read::MayorRating, Subject::Cell(cell))),
        None => Some(256),
        _ => None, // CT19: property-to-curve construction is not settled.
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NeighborRow {
    pub cell: Cell,
    pub direction: u8,
    pub offset: u32,
    pub kind: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NeighborPlan {
    pub rows: Vec<NeighborRow>,
    pub script: Script,
}
pub fn neighbor_plan(world: &impl World, occupant: &Occupant) -> Option<NeighborPlan> {
    let cell = occupant.cell;
    let last = world.city_size() - 1;
    if cell.x != 0 && cell.z != 0 && cell.x != last && cell.z != last {
        return None;
    }
    let mut kind = 1;
    if occupant.network_interface {
        for (network, mapped) in [
            (11, 10),
            (0, 2),
            (6, 8),
            (12, 12),
            (2, 3),
            (7, 6),
            (8, 7),
            (9, 11),
            (1, 1),
            (4, 5),
        ] {
            if occupant.network_types.contains(&network) {
                kind = mapped;
            }
        }
    } else if occupant.pole_interface {
        kind = 4;
    }
    let mut rows = Vec::new();
    for direction in 0..4 {
        if world.valid_neighbor_connection(cell, direction, kind) {
            rows.push(NeighborRow {
                cell,
                direction,
                offset: rows.len() as u32 * 0x200,
                kind,
            });
        }
    }
    let scripts = match kind {
        1 | 6 | 7 | 11 => [0xEA56_6A49, 0x4A5E_7ED3],
        2 | 3 | 8 | 10 | 12 => [0xCA56_6F94, 0xAA5E_14CC],
        4 => [0x4A56_6C14, 0x8A5E_7BD2],
        5 => [0x4A56_6D6E, 0x2A5E_7490],
        _ => return None,
    };
    let instance = *scripts.get(rows.len().checked_sub(1)?)?;
    Some(NeighborPlan {
        rows,
        script: Script::query(instance),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugPhase {
    ResidentialQueryData(Token),
    Building(Token),
    Population(Cell),
    Demand(Cell),
    EqHq(Cell),
    Cell(Cell),
}

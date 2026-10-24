// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

//! Portable nation owner, from nation.md §§1–8 and demand.md §11.2.
//! Platform effects are supplied through interfaces; this owner has no save record.

use crate::curve::ResponseCurve;
use std::collections::BTreeMap;

mod owner;
pub use owner::*;

pub const POST_CITY_INIT: u32 = 0x26D3_1EC1;
pub const PRE_CITY_SHUTDOWN: u32 = 0x26D3_1EC2;
pub const RESET_TUTORIAL_CITIES: u32 = 0xAC3B_3FA6;
pub const SUBSCRIPTIONS: [u32; 3] = [POST_CITY_INIT, PRE_CITY_SHUTDOWN, RESET_TUTORIAL_CITIES];
pub const NATION_EXEMPLAR: (u32, u32, u32) = (0x6534_284A, 0xE7E2_C2DB, 0x4A1F_6115);
pub const DEMAND_IDS_PROPERTY: u32 = 0x4A1F_629F;
pub const FIRST_CURVE_PROPERTY: u32 = 0x4A1F_62A0;
pub const DIFFICULTY_PROPERTY: u32 = 0x4A1F_6300;
pub const SHIPPED_DIFFICULTY: [f32; 3] = [0.06, 0.03, 0.01];

/// None means missing; Some(empty) means explicitly present and empty.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TuningProperties {
    pub demand_ids: Option<Vec<u32>>,
    pub curves: BTreeMap<u32, Vec<f32>>,
    pub difficulty: Option<Vec<f32>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EconomyTuning {
    curves: BTreeMap<u32, ResponseCurve>,
    difficulty: Vec<f32>,
}

impl EconomyTuning {
    pub fn from_properties(properties: Option<&TuningProperties>) -> Self {
        Self::reload(properties, Vec::new())
    }

    fn reload(properties: Option<&TuningProperties>, mut difficulty: Vec<f32>) -> Self {
        // §6.3: resize retains surviving difficulty values across shutdown/reinit.
        difficulty.resize(3, 0.0);
        let mut curves = BTreeMap::new();
        let ids = properties.and_then(|p| p.demand_ids.as_deref());
        if let Some(ids) = ids {
            for (i, &id) in ids.iter().enumerate() {
                let property = FIRST_CURVE_PROPERTY.wrapping_add(i as u32);
                let curve = properties
                    .and_then(|p| p.curves.get(&property))
                    .and_then(|values| ResponseCurve::from_flat(values))
                    .unwrap_or_default();
                curves.insert(id, curve);
            }
        } else {
            curves.insert(0, ResponseCurve::from_points(&[(0.0, 0.0)]));
        }
        if ids != Some(&[][..]) {
            difficulty = properties
                .and_then(|p| p.difficulty.clone())
                .unwrap_or_else(|| SHIPPED_DIFFICULTY.to_vec());
        }
        Self { curves, difficulty }
    }

    pub fn difficulty_modifiers(&self) -> &[f32] {
        &self.difficulty
    }

    pub fn get_economy_modifier(
        &self,
        demand_id: u32,
        date: EconomyDate,
        city_difficulty: Option<usize>,
    ) -> Result<f32, NationError> {
        // §2.1: missing curve precedes invalid difficulty; no city never indexes it.
        let curve = self
            .curves
            .get(&demand_id)
            .or_else(|| self.curves.get(&0))
            .ok_or(NationError::MissingCurve(demand_id))?;
        let value = curve.eval(date.unsigned_months() as f32);
        match city_difficulty {
            None => Ok(value),
            Some(index) => self
                .difficulty
                .get(index)
                .map(|difficulty| value + difficulty)
                .ok_or(NationError::InvalidDifficulty(index)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EconomyDate {
    pub sim_year: i32,
    pub sim_month: i32,
    pub start_year: i32,
    pub start_month: i32,
}

impl EconomyDate {
    pub fn unsigned_months(self) -> u32 {
        self.sim_year
            .wrapping_sub(self.start_year)
            .wrapping_mul(12)
            .wrapping_sub(self.start_month)
            .wrapping_add(self.sim_month) as u32
    }
}

#[cfg(test)]
mod tests;

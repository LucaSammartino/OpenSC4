// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

//! Application-owned nation state, from nation.md and demand.md §8.2/§11.2.
//! No city save record is owned by this object. See README.md for open contracts.

use crate::curve::ResponseCurve;
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

pub const POST_CITY_INIT: u32 = 0x26D3_1EC1;
pub const PRE_CITY_SHUTDOWN: u32 = 0x26D3_1EC2;
pub const RESET_TUTORIAL_CITIES: u32 = 0xAC3B_3FA6;
pub const SUBSCRIPTIONS: [u32; 3] = [POST_CITY_INIT, PRE_CITY_SHUTDOWN, RESET_TUTORIAL_CITIES];
pub const NATION_EXEMPLAR: (u32, u32, u32) = (0x6534_284A, 0xE7E2_C2DB, 0x4A1F_6115);
pub const DEMAND_IDS_PROPERTY: u32 = 0x4A1F_629F;
pub const FIRST_CURVE_PROPERTY: u32 = 0x4A1F_62A0;
pub const DIFFICULTY_PROPERTY: u32 = 0x4A1F_6300;
pub const SHIPPED_DIFFICULTY: [f32; 3] = [0.06, 0.03, 0.01];

/// Typed property input. None means missing; Some(empty) means present and empty.
/// Exemplar decoding belongs to the data-format/property adapter, not the nation.
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
    /// Implements the explicit FreeCity4 missing-data choices in demand §11.2.
    pub fn from_properties(properties: Option<&TuningProperties>) -> Self {
        let mut curves = BTreeMap::new();
        let ids = properties.and_then(|p| p.demand_ids.as_deref());
        if let Some(ids) = ids {
            for (i, &id) in ids.iter().enumerate() {
                // Property IDs use plain addition, including indices 10–12.
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
        // With an empty ID list the original never enters its difficulty-load
        // loop. Missing data otherwise uses the explicit FreeCity4 defaults.
        let difficulty = if ids == Some(&[][..]) {
            vec![0.0; 3]
        } else {
            properties
                .and_then(|p| p.difficulty.clone())
                .unwrap_or_else(|| SHIPPED_DIFFICULTY.to_vec())
        };
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
        // NQ3: safe errors replace the original's undefined pointer/index reads.
        let curve = self
            .curves
            .get(&demand_id)
            .or_else(|| self.curves.get(&0))
            .ok_or(NationError::MissingCurve(demand_id))?;
        // Curve evaluation already rounds each operation to f32 (§18).
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

/// Nation §2 uses wrapping 32-bit arithmetic, then an unsigned float conversion.
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

/// One shared owner per application process; Shutdown does not clear this cache.
/// Independent instances permit isolated tests without resetting process globals.
#[derive(Debug, Default)]
pub struct ProcessEconomy {
    tuning: OnceLock<Arc<EconomyTuning>>,
}

impl ProcessEconomy {
    pub fn global() -> &'static Self {
        static PROCESS: ProcessEconomy = ProcessEconomy {
            tuning: OnceLock::new(),
        };
        &PROCESS
    }

    pub fn load_once(&self, read: impl FnOnce() -> Option<TuningProperties>) -> Arc<EconomyTuning> {
        Arc::clone(
            self.tuning
                .get_or_init(|| Arc::new(EconomyTuning::from_properties(read().as_ref()))),
        )
    }
}

/// Owned interface handles belong to the future adapter; the core never Releases
/// borrowed game pointers. Activation is only through the region interface.
pub trait RegionActivation {
    fn activate(&mut self);
    fn deactivate(&mut self);
}

pub trait MessageSubscriptions {
    fn subscribe(&mut self, message: u32);
    fn unsubscribe(&mut self, message: u32);
}

/// Already discovered catalog, supplied by a future filesystem adapter (NQ1).
#[derive(Debug)]
pub struct RegionCatalog<R> {
    pub region_directory: String,
    pub download_directory: String,
    pub regions: Vec<R>,
    pub active_index: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedOperation {
    DiscoverRegions,
    PreferenceLookup,
    DeleteRegion,
    AddNewRegion,
    ResetTutorialCities,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NationError {
    Unsupported(UnsupportedOperation),
    InvalidRegionIndex(usize),
    InvalidDifficulty(usize),
    MissingCurve(u32),
    NotInitialized,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageOutcome {
    Ignored,
    TutorialReset,
}

/// Portable cSC4Nation. There is deliberately no serialization implementation:
/// nation §1 says this application object writes no city save record.
pub struct Nation<R> {
    catalog: RegionCatalog<R>,
    tuning: Option<Arc<EconomyTuning>>,
    subscribed: bool,
}

impl<R> Nation<R> {
    pub fn new(catalog: RegionCatalog<R>) -> Result<Self, NationError> {
        if let Some(index) = catalog.active_index {
            if index >= catalog.regions.len() {
                return Err(NationError::InvalidRegionIndex(index));
            }
        }
        Ok(Self {
            catalog,
            tuning: None,
            subscribed: false,
        })
    }

    /// Defined portion of Init: process tuning and message subscriptions.
    /// NQ1: filesystem discovery is a separate, explicitly unsupported operation.
    pub fn init(
        &mut self,
        process: &ProcessEconomy,
        read_tuning: impl FnOnce() -> Option<TuningProperties>,
        messages: &mut impl MessageSubscriptions,
    ) {
        self.tuning = Some(process.load_once(read_tuning));
        // Literal lifecycle choice: repeated Init does not duplicate subscriptions.
        if !self.subscribed {
            for message in SUBSCRIPTIONS {
                messages.subscribe(message);
            }
            self.subscribed = true;
        }
    }

    pub fn shutdown(&mut self, messages: &mut impl MessageSubscriptions) {
        if self.subscribed {
            for message in SUBSCRIPTIONS {
                messages.unsubscribe(message);
            }
            self.subscribed = false;
        }
        // NQ1: no catalog destruction rule is specified. Retain supplied state.
    }

    pub fn get_region_directory(&self) -> &str {
        &self.catalog.region_directory
    }

    pub fn get_region_download_directory(&self) -> &str {
        &self.catalog.download_directory
    }

    pub fn get_region_count(&self) -> usize {
        self.catalog.regions.len()
    }

    pub fn get_active_region_index(&self) -> Option<usize> {
        self.catalog.active_index
    }

    pub fn get_region(&self, index: usize) -> Result<&R, NationError> {
        // NQ2: reject invalid indices instead of an unchecked access.
        self.catalog
            .regions
            .get(index)
            .ok_or(NationError::InvalidRegionIndex(index))
    }

    pub fn get_active_region(&self) -> Option<&R> {
        self.catalog
            .active_index
            .and_then(|index| self.catalog.regions.get(index))
    }

    pub fn get_economy_modifier(
        &self,
        demand_id: u32,
        date: EconomyDate,
        city_difficulty: Option<usize>,
    ) -> Result<f32, NationError> {
        self.tuning
            .as_ref()
            .ok_or(NationError::NotInitialized)?
            .get_economy_modifier(demand_id, date, city_difficulty)
    }

    pub fn do_message(&mut self, message: u32) -> Result<MessageOutcome, NationError> {
        if message == RESET_TUTORIAL_CITIES {
            self.reset_tutorial_cities()?;
            Ok(MessageOutcome::TutorialReset)
        } else {
            Ok(MessageOutcome::Ignored)
        }
    }

    // NQ1: these names are specified, but their filesystem/preference contracts
    // are absent. Do not invent paths, copying rules, or mutation on failure.
    pub fn discover_regions(&mut self) -> Result<(), NationError> {
        Err(NationError::Unsupported(UnsupportedOperation::DiscoverRegions))
    }

    pub fn get_index_of_current_region_setting_in_preferences(&self) -> Result<usize, NationError> {
        Err(NationError::Unsupported(UnsupportedOperation::PreferenceLookup))
    }

    pub fn delete_region(&mut self, _index: usize) -> Result<(), NationError> {
        Err(NationError::Unsupported(UnsupportedOperation::DeleteRegion))
    }

    pub fn add_new_region(&mut self, _region: R) -> Result<usize, NationError> {
        Err(NationError::Unsupported(UnsupportedOperation::AddNewRegion))
    }

    pub fn reset_tutorial_cities(&mut self) -> Result<(), NationError> {
        Err(NationError::Unsupported(UnsupportedOperation::ResetTutorialCities))
    }
}

impl<R: RegionActivation> Nation<R> {
    pub fn set_active_region(&mut self, index: usize) -> Result<(), NationError> {
        // NQ2: validate before any effect; same-index calls still deactivate/activate.
        if index >= self.catalog.regions.len() {
            return Err(NationError::InvalidRegionIndex(index));
        }
        if let Some(old) = self.catalog.active_index {
            self.catalog.regions[old].deactivate();
        }
        self.catalog.regions[index].activate();
        self.catalog.active_index = Some(index);
        Ok(())
    }
}

#[cfg(test)]
mod tests;

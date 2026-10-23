// SPDX-License-Identifier: GPL-3.0-or-later
//! Raw collection/radius selection (§5). Equality and native ordering are QV7.
use super::{Provider, Unsupported};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CoverageRequest {
    ResidentialSlot45,
    FireSlot21 { level: f32 },
    PoliceSlot20 { level: f32 },
    Unspecified,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HighlightRequest {
    pub provider: Provider,
    pub building_slot: u8,
    pub coverage: CoverageRequest,
}

/// Only raw simulator primitive requests, never an external GetHighlights.
pub fn highlight_request(kind: u32) -> Result<HighlightRequest, Unsupported> {
    let (provider, building_slot, coverage) = match kind {
        1 => (
            Provider::Residential,
            47,
            CoverageRequest::ResidentialSlot45,
        ),
        2 => (
            Provider::Residential,
            49,
            CoverageRequest::ResidentialSlot45,
        ),
        3 => (
            Provider::Fire,
            22,
            CoverageRequest::FireSlot21 { level: 0.5 },
        ),
        4 => (
            Provider::Police,
            21,
            CoverageRequest::PoliceSlot20 { level: 0.5 },
        ),
        5 => return Err(Unsupported::CrimeHotspot),
        6 => (Provider::Pollution, 46, CoverageRequest::Unspecified),
        7 => (Provider::Power, 25, CoverageRequest::Unspecified),
        8 => (Provider::Plumbing, 28, CoverageRequest::Unspecified),
        9 => (Provider::Traffic, 28, CoverageRequest::None),
        _ => return Err(Unsupported::HighlightKind(kind)),
    };
    Ok(HighlightRequest {
        provider,
        building_slot,
        coverage,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildingKind {
    School,
    Hospital,
    FireStation,
    PoliceStation,
    GarbageBuilding,
    PowerPlant,
    WaterProducer,
    TransitSwitch,
}

/// Raw results of individual simulator coverage-radius primitives. These
/// are not normalized highlight records or a host GetHighlights outcome.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Coverage {
    pub residential: Option<f32>,
    pub fire_half: Option<f32>,
    pub police_half: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawBuilding {
    pub id: u64,
    pub kind: BuildingKind,
    pub coverage: Coverage,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HighlightRadius {
    Value(f32),
    /// Explicitly no radius for kind 9.
    None,
    /// The dash for kinds 6-8 does not define a native radius.
    Unspecified,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HighlightCandidate {
    pub id: u64,
    pub radius: HighlightRadius,
}

/// An observation, not a native ordered/deduplicated highlight set.
/// Vec order merely retains input enumeration. PartialEq is for inspecting
/// fixture data; MapView never uses it to decide native equality/dirty state.
#[derive(Clone, Debug, PartialEq)]
pub struct HighlightCollection {
    pub kind: u32,
    pub candidates: Vec<HighlightCandidate>,
}

pub fn collect_highlights(
    kind: u32,
    buildings: &[RawBuilding],
) -> Result<HighlightCollection, Unsupported> {
    highlight_request(kind)?;
    let building_kind = match kind {
        1 => BuildingKind::School,
        2 => BuildingKind::Hospital,
        3 => BuildingKind::FireStation,
        4 => BuildingKind::PoliceStation,
        5 => return Err(Unsupported::CrimeHotspot),
        6 => BuildingKind::GarbageBuilding,
        7 => BuildingKind::PowerPlant,
        8 => BuildingKind::WaterProducer,
        9 => BuildingKind::TransitSwitch,
        _ => return Err(Unsupported::HighlightKind(kind)),
    };
    let mut candidates = Vec::new();
    for building in buildings.iter().filter(|b| b.kind == building_kind) {
        let raw_radius = match kind {
            1 | 2 => building.coverage.residential,
            3 => building.coverage.fire_half,
            4 => building.coverage.police_half,
            _ => None,
        };
        let radius = match kind {
            1..=4 => {
                let radius = raw_radius.ok_or(Unsupported::HighlightRadius(kind))?;
                if !radius.is_finite() || radius < 0.0 {
                    return Err(Unsupported::HighlightRadius(kind));
                }
                HighlightRadius::Value(radius)
            }
            6..=8 => HighlightRadius::Unspecified,
            9 => HighlightRadius::None,
            _ => unreachable!(),
        };
        candidates.push(HighlightCandidate {
            id: building.id,
            radius,
        });
    }
    Ok(HighlightCollection { kind, candidates })
}

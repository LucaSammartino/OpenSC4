// SPDX-License-Identifier: GPL-3.0-or-later
//! Raw collection/radius selection (§5). Equality and native ordering are QV7.
use super::*;

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
        1 => (Provider::Residential, 47, CoverageRequest::ResidentialSlot45),
        2 => (Provider::Residential, 49, CoverageRequest::ResidentialSlot45),
        3 => (Provider::Fire, 22, CoverageRequest::FireSlot21 { level: 0.5 }),
        4 => (Provider::Police, 21, CoverageRequest::PoliceSlot20 { level: 0.5 }),
        5 => (Provider::Crime, 8, CoverageRequest::None),
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

pub fn collect_highlights(kind: u32, buildings: &[RawBuilding]) -> Result<HighlightCollection, Unsupported> {
    highlight_request(kind)?;
    let building_kind = match kind {
        1 => BuildingKind::School,
        2 => BuildingKind::Hospital,
        3 => BuildingKind::FireStation,
        4 => BuildingKind::PoliceStation,
        // The building-only helper cannot consume crime outputs. Owner polling
        // handles kind 5 through crime_hotspot, never through buildings.
        5 => {
            return Ok(HighlightCollection {
                kind,
                candidates: Vec::new(),
            })
        }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrimeHotspot {
    pub strength: u8,
    pub x: i32,
    pub z: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HighlightStyle {
    pub name: String,
    pub ring: u32,
    pub footprint: u32,
}

impl HighlightStyle {
    pub fn from_exemplar(raw: &RawExemplar) -> Result<Self, Unsupported> {
        let name = match raw.find(0x4a0b_47ea)? {
            None => String::new(),
            Some(RawValue::String(name)) => name.clone(),
            Some(_) => return Err(Unsupported::MalformedProperty(0x4a0b_47ea)),
        };
        Ok(Self {
            name,
            ring: raw.optional_u32(0x4a0b_47ee)?.unwrap_or(0),
            footprint: raw.optional_u32(0x4a0b_47ef)?.unwrap_or(0),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum VisualEffect {
    Transform {
        handle: u64,
        position: [f32; 3],
        scale: f32,
        identity_rotation: bool,
        flags: [u8; 2],
    },
    Start {
        handle: u64,
        argument: bool,
    },
    Stop {
        handle: u64,
        argument: bool,
    },
    Release(u64),
}

/// References supplied here are owned by the record. No borrowed reference is
/// released. Building collection/comparison remains QV7; these are the settled
/// per-record add/remove primitives, also used by hotspot reconciliation.
#[derive(Clone, Debug, PartialEq)]
pub struct HighlightRecord {
    pub occupant: Option<u64>,
    pub position: [f32; 3],
    pub radius: f32,
    pub effect: Option<u64>,
}

impl HighlightRecord {
    pub fn create_effect(
        &self,
        name: &str,
        scale: f32,
        renderer: &mut impl Renderer,
    ) -> Result<Option<u64>, OwnerError> {
        if name.is_empty() {
            return Ok(None);
        }
        let handle = renderer.create_visual_effect(name)?;
        if let Some(handle) = handle {
            renderer.visual_effect(VisualEffect::Transform {
                handle,
                position: self.position,
                scale,
                identity_rotation: true,
                flags: [2, if scale == 1.0 { 1 } else { 3 }],
            })?;
            renderer.visual_effect(VisualEffect::Start {
                handle,
                argument: false,
            })?;
        }
        Ok(handle)
    }

    pub fn add(&mut self, name: &str, renderer: &mut impl Renderer) -> Result<(), OwnerError> {
        if let Some(effect) = self.create_effect(name, self.radius * 2.0, renderer)? {
            if let Some(temporary) = self.effect.replace(effect) {
                renderer.visual_effect(VisualEffect::Release(temporary))?;
            }
        }
        if let Some(occupant) = self.occupant {
            renderer.highlight_occupant(occupant, true, 0)?;
        }
        Ok(())
    }

    pub fn remove(self, renderer: &mut impl Renderer) -> Result<(), OwnerError> {
        self.stop(renderer)?;
        self.release(renderer)
    }

    pub(super) fn stop(&self, renderer: &mut impl Renderer) -> Result<(), OwnerError> {
        if let Some(effect) = self.effect {
            renderer.visual_effect(VisualEffect::Stop {
                handle: effect,
                argument: false,
            })?;
        }
        Ok(())
    }

    pub(super) fn release(self, renderer: &mut impl Renderer) -> Result<(), OwnerError> {
        if let Some(effect) = self.effect {
            renderer.visual_effect(VisualEffect::Release(effect))?;
        }
        if let Some(occupant) = self.occupant {
            renderer.highlight_occupant(occupant, false, 0)?;
            renderer.release_occupant(occupant)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl ClipRect {
    fn intersect(self, other: Self) -> Self {
        Self {
            left: self.left.max(other.left),
            top: self.top.max(other.top),
            right: self.right.min(other.right),
            bottom: self.bottom.min(other.bottom),
        }
    }
}

pub fn blend_highlight(destination: u32, source: u32, weight: u32) -> u32 {
    let inverse = 256 - weight;
    let alpha = 255 - (((255 - (destination >> 24)) * inverse + 128) / 256);
    let mut result = alpha << 24;
    for shift in [0, 8, 16] {
        let d = (destination >> shift) & 255;
        let s = (source >> shift) & 255;
        result |= ((d * inverse + s * weight + 128) / 256) << shift;
    }
    result
}

/// §5.2 signed 32-bit products/divisions, with explicit native wrapping products.
pub fn footprint_rect(cells: [i32; 4], bitmap: [i32; 2], city: [i32; 2]) -> Result<ClipRect, Unsupported> {
    if city.iter().any(|&v| v <= 0) || bitmap.iter().any(|&v| v <= 0) {
        return Err(Unsupported::GridGeometry);
    }
    Ok(ClipRect {
        left: cells[0].wrapping_mul(bitmap[0]) / city[0],
        top: cells[1].wrapping_mul(bitmap[1]) / city[1],
        right: (cells[2].wrapping_add(1).wrapping_mul(bitmap[0]) / city[0]).wrapping_add(1),
        bottom: (cells[3].wrapping_add(1).wrapping_mul(bitmap[1]) / city[1]).wrapping_add(1),
    })
}

/// Finite ordinary coordinates only (invalid conversion domains remain QV10).
pub fn ring_arguments(width: usize, world_width: f32, record: &HighlightRecord) -> Result<[f32; 3], Unsupported> {
    if !world_width.is_finite() || world_width <= 0.0 {
        return Err(Unsupported::GridGeometry);
    }
    let q = width as f32 / world_width;
    Ok([record.position[0] * q, record.position[2] * q, record.radius * q])
}

pub fn paint_ring(
    pixels: &mut [u32],
    width: usize,
    height: usize,
    clip: ClipRect,
    args: [f32; 3],
    color: u32,
) -> Result<(), Unsupported> {
    if width.checked_mul(height) != Some(pixels.len()) {
        return Err(Unsupported::BufferLength);
    }
    if width > i32::MAX as usize || height > i32::MAX as usize {
        return Err(Unsupported::GridGeometry);
    }
    if args.iter().any(|v| !v.is_finite()) {
        return Err(Unsupported::NonFiniteFloat);
    }
    if args[2] <= 0.0 {
        return Ok(());
    }
    if args.iter().any(|v| v.abs() > (i32::MAX / 4) as f32) {
        return Err(Unsupported::FloatOutOfRange);
    }
    let clip = clip.intersect(ClipRect {
        left: 0,
        top: 0,
        right: width as i32,
        bottom: height as i32,
    });
    if clip.left >= clip.right || clip.top >= clip.bottom {
        return Ok(());
    }
    // [literal formula / numeric choice, QV14] Preserve every stated binary32
    // store, using binary64 between stores. The spec does not give the x87
    // precision-control/rounding mode; exact native edge-pixel agreement is not
    // claimed until that mode and operation-rounding sequence are specified.
    let [u, v, r] = args.map(f64::from);
    let cx = f64::from((u + 0.5) as f32);
    let cy = f64::from((v + 0.5) as f32);
    let radius_squared = f64::from(((r + 1.0) * (r + 1.0)) as f32);
    let first = ((v - r - 0.5).ceil() as i32).max(clip.top);
    let last = ((v + r + 0.5).floor() as i32).min(clip.bottom - 1);
    for y in first..=last {
        let dy = f64::from(y) - cy;
        let dy_squared = f64::from((dy * dy) as f32);
        let remainder = radius_squared - dy_squared;
        if remainder < 0.0 {
            continue;
        }
        let span = remainder.sqrt();
        let left = ((cx - span).ceil() as i32).max(clip.left);
        let right = ((cx + span).floor() as i32).min(clip.right - 1);
        for x in left..=right {
            let dx = f64::from(x) - cx;
            let error = ((dx * dx + dy_squared).sqrt() - r).abs();
            if error < 2.0 {
                let weight = 255 - (128.0 * error).floor() as u32;
                let index = y as usize * width + x as usize;
                pixels[index] = blend_highlight(pixels[index], color, weight);
            }
        }
    }
    Ok(())
}

impl MapView {
    pub fn retained_highlights(&self) -> &[HighlightRecord] {
        &self.retained_highlights
    }
    pub fn highlight_style(&self) -> &HighlightStyle {
        &self.highlight_style
    }

    pub(super) fn decorate_bitmap(
        &self,
        width: usize,
        height: usize,
        pixels: &mut [u32],
        renderer: &mut impl Renderer,
        simulators: &mut impl Simulators,
    ) -> Result<(), OwnerError> {
        decorate_highlights(
            &self.retained_highlights,
            &self.highlight_style,
            width,
            height,
            pixels,
            renderer,
            simulators,
        )
    }
}

/// Explicit records allow tests of settled decoration without inventing QV7's
/// building-set comparison. The owner supplies its actual retained hotspot set.
pub fn decorate_highlights(
    records: &[HighlightRecord],
    style: &HighlightStyle,
    width: usize,
    height: usize,
    pixels: &mut [u32],
    renderer: &mut impl Renderer,
    simulators: &mut impl Simulators,
) -> Result<(), OwnerError> {
    if width.checked_mul(height) != Some(pixels.len()) {
        return Err(Unsupported::BufferLength.into());
    }
    if width == 0 || height == 0 || width > i32::MAX as usize || height > i32::MAX as usize {
        return Err(Unsupported::GridGeometry.into());
    }
    let bounds = ClipRect {
        left: 0,
        top: 0,
        right: width as i32,
        bottom: height as i32,
    };
    if style.footprint != 0 {
        if let Some(clip) = renderer.acquire_bitmap(0x8040, width, height)? {
            let result = (|| {
                let city = simulators.city_cells()?;
                let city = [
                    i32::try_from(city[0]).map_err(|_| Unsupported::GridGeometry)?,
                    i32::try_from(city[1]).map_err(|_| Unsupported::GridGeometry)?,
                ];
                for record in records {
                    if let Some(occupant) = record.occupant {
                        let rect = footprint_rect(
                            simulators.occupant_cells(occupant)?,
                            [width as i32, height as i32],
                            city,
                        )?
                        .intersect(clip)
                        .intersect(bounds);
                        for y in rect.top..rect.bottom {
                            for x in rect.left..rect.right {
                                let index = y as usize * width + x as usize;
                                pixels[index] = blend_highlight(pixels[index], style.footprint, style.footprint >> 24);
                            }
                        }
                    }
                }
                Ok::<(), OwnerError>(())
            })();
            renderer.release_bitmap()?;
            result?;
        }
    }
    if style.ring != 0 {
        for record in records.iter().filter(|record| record.radius > 0.0) {
            let args = ring_arguments(width, simulators.city_world_width()?, record)?;
            if let Some(clip) = renderer.acquire_bitmap(0x8040, width, height)? {
                let result = paint_ring(pixels, width, height, clip, args, style.ring);
                renderer.release_bitmap()?;
                result?;
            }
        }
    }
    Ok(())
}

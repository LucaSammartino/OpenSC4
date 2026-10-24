// SPDX-License-Identifier: GPL-3.0-or-later
//! Embedded coverage helper, view3d-window §6l, including stale-cache quirks.
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Police,
    Fire,
    Health,
    Education,
}

impl Category {
    pub fn effect(self) -> &'static str {
        match self {
            Self::Police => "PlopMode_Police_Existing",
            Self::Fire => "PlopMode_Fire_Existing",
            Self::Health => "PlopMode_Health_Existing",
            Self::Education => "PlopMode_Education_Existing",
        }
    }
    pub fn radius_factor(self) -> f32 {
        match self {
            Self::Police | Self::Fire => 0.5,
            Self::Health | Self::Education => f32::from_bits(0x3F33_3333),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoverageTransform {
    pub position: [f32; 3],
    /// Identity orientation; uniform scaling of 2 * radius.
    pub scale: f32,
    pub translation_present: bool,
    pub scaling_present: bool,
}

pub trait CoverageHost {
    /// Missing simulator returns an empty list. Each element owns one reference.
    fn occupants(&mut self, category: Category) -> Vec<u32>;
    fn radius(&mut self, category: Category, occupant: u32, factor: f32) -> Option<f32>;
    fn position(&mut self, occupant: u32) -> Option<[f32; 3]>;
    fn release_occupant(&mut self, occupant: u32);
    fn create_effect(&mut self, name: &'static str) -> Option<u32>;
    fn transform(&mut self, effect: u32, transform: CoverageTransform);
    fn start(&mut self, effect: u32, argument: u32);
    fn running(&mut self, effect: u32) -> bool;
    fn stop(&mut self, effect: u32, argument: u32);
    fn release_effect(&mut self, effect: u32);
}

#[derive(Debug)]
struct Entry {
    effect: Option<u32>,
    radius: f32,
    seen: bool,
}

#[derive(Debug, Default)]
pub struct Coverage {
    pub categories: u32,
    entries: BTreeMap<u32, Entry>,
}

impl Coverage {
    fn release(host: &mut impl CoverageHost, effect: u32) {
        if host.running(effect) {
            host.stop(effect, 0);
        }
        host.release_effect(effect);
    }

    pub fn clear(&mut self, host: &mut impl CoverageHost) {
        for entry in std::mem::take(&mut self.entries).into_values() {
            if let Some(effect) = entry.effect {
                Self::release(host, effect);
            }
        }
    }

    pub fn set_categories(&mut self, host: &mut impl CoverageHost, categories: u32) {
        self.categories = categories;
        self.clear(host);
    }

    pub fn recalculate(&mut self, host: &mut impl CoverageHost) {
        for entry in self.entries.values_mut() {
            entry.seen = false;
        }
        for (bit, category) in [
            (1, Category::Police),
            (2, Category::Fire),
            (4, Category::Health),
            (8, Category::Education),
        ] {
            if self.categories & bit == 0 {
                continue;
            }
            for occupant in host.occupants(category) {
                let radius = host
                    .radius(category, occupant, category.radius_factor())
                    .unwrap_or(32.0);
                if let Some(position) = host.position(occupant) {
                    let scale = radius * 2.0;
                    let transform = CoverageTransform {
                        position,
                        scale,
                        translation_present: true,
                        scaling_present: scale != 1.0,
                    };
                    if let Some(entry) = self.entries.get_mut(&occupant) {
                        entry.seen = true;
                        if radius != entry.radius {
                            if let Some(effect) = entry.effect {
                                host.transform(effect, transform);
                            }
                        }
                    } else if let Some(effect) = host.create_effect(category.effect()) {
                        host.transform(effect, transform);
                        host.start(effect, 0);
                        self.entries.insert(
                            occupant,
                            Entry {
                                effect: Some(effect),
                                radius,
                                seen: true,
                            },
                        );
                    }
                }
                host.release_occupant(occupant);
            }
        }
        for entry in self.entries.values_mut().filter(|entry| !entry.seen) {
            if let Some(effect) = entry.effect.take() {
                Self::release(host, effect);
            }
        }
    }
}

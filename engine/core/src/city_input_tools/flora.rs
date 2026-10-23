// SPDX-License-Identifier: GPL-3.0-or-later
use super::{cell_at, finite, trunc_i32, Cell, Pixel, PrimitiveFailure, Resource, Unsupported, Vec3};

pub const FLORA_ID: u32 = 0x26D7_881C;
pub const FLORA_CURSOR: u32 = 0x66DB_057F;
pub const PLANT_SOUND: u32 = 0x49DC_0C90;
pub const FLORA_RESULT_SOUND: u32 = 0xAA62_C9D5;

/// Semantic indicator intents preserve int64 without guessing slot42 narrowing
/// or Release's unpublished clear flag (CT15). No native adapter is supplied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CostIndicator {
    Price(i64),
    Stroke { total: i64, price: i64 },
    ClearPositiveStroke,
    ClearShutdown { flag: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlantSound {
    Start { event: u32, sound: u32, instance: u32 },
    Position { event: u32, instance: u32, xyz: [i32; 3] },
}

pub trait FloraHost {
    fn pick_terrain(&mut self, mouse: Pixel) -> Option<Vec3>;
    fn funds(&mut self) -> i64;
    /// flora.md 6.5 rewrites the caller-owned position on BOTH result branches.
    fn insert_flora(&mut self, simulator: Resource, flora_type: u32, p: &mut Vec3, force: bool) -> bool;
    fn location_ok(&mut self, simulator: Resource, flora_type: u32, x: f32, z: f32, exclude: Option<Resource>) -> bool;
    fn is_water_vertex(&mut self, vertex: Cell) -> bool;
    fn new_sound_instance(&mut self) -> Result<u32, PrimitiveFailure>;
    fn plant_sound(&mut self, message: PlantSound);
    /// Base PlayResultSound slot11 (1,id,0,0,0), NOT a sound-event post.
    fn result_sound(&mut self, id: u32);
    fn withdraw(&mut self, price: i64) -> bool;
    fn cost_indicator(&mut self, intent: CostIndicator);
    fn transient_aura(&mut self, x: f32, z: f32, kind: u32);
    /// Release exactly the explicitly acquired obligation transferred at init.
    fn release_simulator(&mut self, simulator: Resource);
}

/// Constructor fields not initialized by published Init are supplied, not
/// invented as zero/false. Unknown hover validity stays None until a hover hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FloraInputState {
    pub pressed: bool,
    pub hover_valid: Option<bool>,
    pub out_of_money: bool,
}

#[derive(Debug)]
pub struct Flora {
    flora_type: u32,
    simulator: Option<Resource>,
    price: i64,
    total: i64,
    input: FloraInputState,
}

impl Flora {
    /// Partial tool Init: static price and one acquired simulator identity are
    /// supplied primitives; no native base Init or constructor is simulated.
    /// On invalid price the caller retains its acquisition (CT14).
    pub fn init(
        flora_type: u32,
        price: i64,
        simulator: Resource,
        input: FloraInputState,
        host: &mut impl FloraHost,
    ) -> Result<Self, Unsupported> {
        if price <= 0 {
            return Err(Unsupported::NonpositivePrice);
        }
        host.cost_indicator(CostIndicator::Price(price));
        Ok(Self {
            flora_type,
            simulator: Some(simulator),
            price,
            total: 0,
            input,
        })
    }

    pub fn input(&self) -> FloraInputState {
        self.input
    }

    pub fn total(&self) -> i64 {
        self.total
    }

    fn simulator(&self) -> Result<Resource, Unsupported> {
        self.simulator.ok_or(Unsupported::InactiveLifetime)
    }

    pub fn press(&mut self, host: &mut impl FloraHost, mouse: Pixel) -> Result<bool, Unsupported> {
        self.simulator()?;
        if let Some(p) = host.pick_terrain(mouse) {
            // Published hit latches pressed even when unaffordable (or the
            // checked portable planting domain subsequently reports residual).
            self.input.pressed = true;
            if host.funds() >= self.price {
                self.plant(host, p, true)?;
            }
        }
        Ok(true)
    }

    pub fn mouse_move(&mut self, host: &mut impl FloraHost, mouse: Pixel) -> Result<bool, Unsupported> {
        let simulator = self.simulator()?;
        if let Some(p) = host.pick_terrain(mouse) {
            if self.input.pressed {
                if host.funds() < self.price {
                    self.input.out_of_money = true;
                } else {
                    self.plant(host, p, false)?;
                }
            } else {
                let p = finite(p)?;
                self.input.hover_valid = Some(host.location_ok(simulator, self.flora_type, p.x, p.z, None));
            }
        }
        // Sticky until Release, including misses and later affordable moves.
        if self.input.out_of_money {
            host.result_sound(FLORA_RESULT_SOUND);
        }
        Ok(true)
    }

    fn plant(&mut self, host: &mut impl FloraHost, p: Vec3, press: bool) -> Result<(), Unsupported> {
        let simulator = self.simulator()?;
        let mut p = finite(p)?;
        let planted = host.insert_flora(simulator, self.flora_type, &mut p, false);
        let p = finite(p)?;
        if planted {
            let xyz = [trunc_i32(p.x)?, trunc_i32(p.y)?, trunc_i32(p.z)?];
            let instance = host.new_sound_instance()?;
            host.plant_sound(PlantSound::Start {
                event: 0xEA45_1903,
                sound: PLANT_SOUND,
                instance,
            });
            host.plant_sound(PlantSound::Position {
                event: 0xCA45_190F,
                instance,
                xyz,
            });
            let withdrawn = host.withdraw(self.price);
            if !press || withdrawn {
                // CT14: completed insertion/sound/withdraw prefix is retained
                // on overflow; no fabricated wrapping or recovery behavior.
                self.total = self
                    .total
                    .checked_add(self.price)
                    .ok_or(Unsupported::ArithmeticOverflow)?;
                host.cost_indicator(CostIndicator::Stroke {
                    total: self.total,
                    price: self.price,
                });
            }
            // CT15: literal successful-insertion branch, even failed press
            // withdrawal; position includes insertion's published rewrite.
            host.transient_aura(p.x, p.z, 1);
        } else if host.is_water_vertex(cell_at(p)?) {
            host.result_sound(FLORA_RESULT_SOUND);
        }
        Ok(())
    }

    pub fn release(&mut self, host: &mut impl FloraHost) -> Result<bool, Unsupported> {
        self.simulator()?;
        self.input.pressed = false;
        host.cost_indicator(if self.total > 0 {
            CostIndicator::ClearPositiveStroke
        } else {
            CostIndicator::Stroke {
                total: self.price,
                price: self.price,
            }
        });
        self.input.out_of_money = false;
        self.total = 0;
        Ok(true)
    }

    pub fn shutdown(&mut self, host: &mut impl FloraHost) -> Result<(), Unsupported> {
        let simulator = self.simulator.take().ok_or(Unsupported::InactiveLifetime)?;
        host.release_simulator(simulator);
        host.cost_indicator(CostIndicator::ClearShutdown { flag: true });
        Ok(())
    }
}

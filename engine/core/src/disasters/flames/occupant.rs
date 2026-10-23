// SPDX-License-Identifier: GPL-3.0-or-later
use super::{
    get_points_on_model, EffectFactory, Flame, FlameInitial, ModelInstance, PlacementError, Stage, VisualEffect,
};
use crate::camera::Vec3;
use crate::rng::{RzRandom, SeedSource};

pub const FLORA_TYPE: u32 = 0x7475_8926;
pub const FIRE_OCCUPANT_GROUP: u32 = 0x5005;

/// Foreign occupant base operations only. No flame decisions are delegated.
pub trait OccupantInit {
    fn init(&mut self);
    fn join_group(&mut self, group: u32);
}

/// Raw target/view inputs plus the persistent fire-code RNG. The counts are the
/// fire occupant's existing +0x38/+0x3c fields, not a provider-computed placement.
pub struct Placement<'a> {
    pub instances: &'a [ModelInstance<'a>],
    pub bounds: [Vec3; 2],
    pub target_type: u32,
    pub view_level: u32,
    pub seeds: &'a dyn SeedSource,
    pub fire_random: &'a mut RzRandom,
}

/// Visual portion of the fire occupant (§§3–4). Other fire-occupant contracts
/// live in disasters.md §5.11; this component makes no complete-owner claim.
pub struct FireFlames<E: VisualEffect> {
    top_count: u32,
    side_count: u32,
    initial: FlameInitial,
    stage: i32,
    initialized: bool,
    on_model: bool,
    flames: Vec<Flame<E>>,
}

impl<E: VisualEffect> FireFlames<E> {
    /// Visual composition only: use FlameInitial::native with the required
    /// shared position word for fresh allocations. No full FireOccupant claim.
    pub fn new(top_count: u32, side_count: u32, initial: FlameInitial) -> Self {
        Self {
            top_count,
            side_count,
            initial,
            stage: -1,
            initialized: false,
            on_model: false,
            flames: Vec::new(),
        }
    }

    pub fn stage(&self) -> Option<Stage> {
        Stage::new(self.stage as u32)
    }
    pub fn raw_stage(&self) -> i32 {
        self.stage
    }
    pub fn shutdown(&mut self) -> bool {
        if !self.initialized {
            return true;
        }
        for flame in &mut self.flames {
            flame.shutdown();
        }
        self.flames.clear();
        true
    }
    /// Native direct destruction has no effect-pointee walk. Transfer creation
    /// references to the caller instead of representing it as Rust RAII Drop.
    pub fn destroy_storage(self) -> Vec<E> {
        self.flames.into_iter().filter_map(Flame::into_effect).collect()
    }
    pub fn on_model(&self) -> bool {
        self.on_model
    }
    pub fn flames(&self) -> &[Flame<E>] {
        &self.flames
    }

    /// §3. The only published caller creates when empty (§4). FF2: reject
    /// unsupported populated calls explicitly instead of guessing append/reset.
    pub fn create_flames(&mut self, placement: &mut Placement<'_>) -> Result<(), PlacementError> {
        if !self.flames.is_empty() {
            return Err(PlacementError::AlreadyPopulated);
        }
        let count = self
            .top_count
            .checked_add(self.side_count)
            .ok_or(PlacementError::CountOverflow)?;
        if let Some(points) = get_points_on_model(count, placement.instances, placement.seeds)? {
            self.on_model = true;
            for point in points {
                let mut flame = Flame::from_initial(self.initial);
                flame.place_model(point.position, point.wall, point.side, placement.view_level)?;
                self.flames.push(flame);
            }
        } else {
            self.on_model = false;
            let [min, max] = placement.bounds;
            // The inclusive loop is intentional: T+S+1 flames, also for T=S=0.
            for index in 0..=count {
                let mut position = [(min[0] + max[0]) * 0.5, min[1], (min[2] + max[2]) * 0.5];
                let (mut wall, mut side) = (self.initial.wall, self.initial.side);
                if placement.target_type == FLORA_TYPE {
                    position[1] = min[1];
                } else if index < self.top_count {
                    wall = false;
                    position[1] = (max[1] - 1.0).max(min[1]);
                    if self.top_count > 1 {
                        position[0] = placement
                            .fire_random
                            .range_gaussian(f64::from(min[0]), f64::from(max[0]))
                            as f32;
                        position[2] = placement
                            .fire_random
                            .range_gaussian(f64::from(min[2]), f64::from(max[2]))
                            as f32;
                    }
                } else {
                    wall = true;
                    side = placement.fire_random.uniform_u32(4);
                    // FF4: draw the free horizontal coordinate before y.
                    match side {
                        0 | 2 => {
                            position[0] = if side == 0 { max[0] } else { min[0] };
                            position[2] = placement.fire_random.range_f64(f64::from(min[2]), f64::from(max[2])) as f32;
                        }
                        _ => {
                            position[2] = if side == 1 { min[2] } else { max[2] };
                            position[0] = placement.fire_random.range_f64(f64::from(min[0]), f64::from(max[0])) as f32;
                        }
                    }
                    position[1] = placement.fire_random.range_f64(f64::from(min[1]), f64::from(max[1])) as f32;
                }
                let mut flame = Flame::from_initial(self.initial);
                flame.place(position, wall, side, placement.view_level)?;
                self.flames.push(flame);
            }
        }
        Ok(())
    }

    /// §4: store the changed stage before creating, then update every flame.
    pub fn set_stage(
        &mut self,
        stage: Stage,
        placement: &mut Placement<'_>,
        factory: &mut impl EffectFactory<Effect = E>,
    ) -> Result<(), PlacementError> {
        self.set_raw_stage(stage.value() as i32, placement, factory)
    }

    pub fn set_raw_stage(
        &mut self,
        stage: i32,
        placement: &mut Placement<'_>,
        factory: &mut impl EffectFactory<Effect = E>,
    ) -> Result<(), PlacementError> {
        if self.stage != stage {
            self.stage = stage;
            if Stage::new(stage as u32).is_none() {
                return Err(PlacementError::Unsupported(super::Unsupported::EffectTable));
            }
            if self.flames.is_empty() {
                self.create_flames(placement)?;
            }
            self.update_all(factory)?;
        }
        Ok(())
    }

    /// §4: base Init, group membership, then Update(stage) for existing flames.
    /// Settled FF2: this does not itself create flames.
    pub fn init(
        &mut self,
        base: &mut impl OccupantInit,
        factory: &mut impl EffectFactory<Effect = E>,
    ) -> Result<(), PlacementError> {
        base.init();
        base.join_group(FIRE_OCCUPANT_GROUP);
        self.initialized = true;
        self.update_all(factory)
    }

    fn update_all(&mut self, factory: &mut impl EffectFactory<Effect = E>) -> Result<(), PlacementError> {
        if !self.flames.is_empty() {
            let stage = self.stage().ok_or(PlacementError::UnpublishedInitialStage)?;
            for flame in &mut self.flames {
                flame.update(stage, factory)?;
            }
        }
        Ok(())
    }

    /// §4: only model flames, with a complete new set of exactly the same size.
    /// No effect recreation, visibility restoration, or box placement on failure.
    pub fn view_update(&mut self, placement: &mut Placement<'_>) -> Result<(), PlacementError> {
        if self.on_model {
            let count = u32::try_from(self.flames.len()).map_err(|_| PlacementError::CountOverflow)?;
            if let Some(points) = get_points_on_model(count, placement.instances, placement.seeds)? {
                if points.len() == self.flames.len() {
                    for (flame, point) in self.flames.iter_mut().zip(points) {
                        flame.place(point.position, point.wall, point.side, placement.view_level)?;
                    }
                }
            }
        }
        Ok(())
    }
}

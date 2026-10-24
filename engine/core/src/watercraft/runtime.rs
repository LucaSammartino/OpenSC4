// SPDX-License-Identifier: GPL-3.0-or-later
//! Executable owner orchestration. Providers expose foreign geometry/effects,
//! never a completed watercraft movement, target, or crash calculation.
use super::*;
use crate::automata::{State, Vec3};

/// Cell conversion and inherited city segment adjustment are foreign services.
pub trait SpatialServices {
    fn centre(&self, cell: Cell) -> Vec3;
    fn adjust_segment(&mut self, from: Vec3, to: &mut Vec3, mask: u32);
}

pub struct MotionServices<'a, T> {
    pub terrain: &'a T,
    pub queries: &'a mut dyn CollisionQueries,
    pub spatial: &'a mut dyn SpatialServices,
    pub ai_collisions: bool,
}

pub struct DriveServices<'a> {
    pub command: &'a mut DriveCommand,
    pub effects: &'a mut dyn PlayerEffects,
    /// Result of the inherited slot-31 removal predicate, not a drive result.
    pub removal_predicate: bool,
    pub cell: Cell,
}

/// Primitive tow-connection interface. The owner computes its refresh policy.
pub trait RopeEffect {
    fn set_transform(&mut self, transform: &Transform);
    fn running(&self) -> bool;
    fn start(&mut self, control: bool);
    fn stop(&mut self, control: bool);
}

#[derive(Clone, Copy, Debug)]
pub struct AnimationTimes {
    pub movement: f32,
    pub animation: f32,
    pub scale: f32,
    pub cell_size: f32,
}

pub struct AnimationServices<'a, 'model> {
    pub subsidiary: &'a mut Subsidiary,
    pub models: &'a mut [Option<&'model mut dyn AnimatedModel>],
    pub service: &'a mut dyn AnimationService,
    pub rope: Option<&'a mut dyn RopeEffect>,
    pub lighting: Option<&'a mut dyn Lighting>,
    pub processed_model: bool,
    pub heading_threshold: Vec3,
    pub distance_squared: f32,
}

impl DriveCommand {
    /// Base drive's channel step, before virtual translation (§6.2.3).
    pub fn advance_channels(&mut self, dt: f32) {
        let advance = |value: &mut f32| {
            if *value >= 0.0 {
                *value = (*value as f64 + dt as f64).min(f32::MAX as f64) as f32;
            }
        };
        advance(&mut self.throttle);
        advance(&mut self.reverse);
        advance(&mut self.right);
        advance(&mut self.left);
        advance(&mut self.fire);
        for value in &mut self.other {
            advance(value);
        }
    }
}

impl Watercraft {
    pub fn refresh_rope(&self, rope: Option<&mut dyn RopeEffect>) {
        if self.towed.is_some() {
            if let Some(rope) = rope {
                rope.set_transform(&self.transform);
                if !rope.running() {
                    rope.start(false);
                }
            }
        } else {
            let rope = rope.expect("no-tow refresh requires the connection effect (§5.1.5)");
            if rope.running() {
                rope.stop(false);
            }
        }
    }
}

impl Fleet {
    /// §5.1 and inherited §§5.1.4–5.1.5 in owner order. Publications are
    /// explicit output commands; the slot-100 model bridge is still WC11.
    #[allow(clippy::too_many_arguments)] // Independent foreign-service groups and the two frame times.
    pub fn animation_tick_core<T: GroundingTerrain>(
        &mut self,
        id: BoatId,
        time: AnimationTimes,
        motion: &mut MotionServices<'_, T>,
        animation: &mut AnimationServices<'_, '_>,
        crashes: &mut dyn CrashEffects,
        manager: &mut Option<&mut dyn DockManager>,
    ) -> Result<bool> {
        if !self.animation_budget(id, time.scale, time.animation, manager)? {
            return Ok(true);
        }
        let boat = self.boat_mut(id)?;
        if boat.speed != 0.0 {
            boat.events.push(Event::ActiveSound(true));
        }
        // Reborrow, preserving the caller's effect reference for later frames.
        match animation.rope.as_mut() {
            Some(rope) => boat.refresh_rope(Some(&mut **rope)),
            None => boat.refresh_rope(None),
        }
        boat.advance_speed_tween(time.animation);
        animation.subsidiary.visibility_tick(time.animation, animation.models);
        animation
            .subsidiary
            .animation_tick(time.animation, animation.service, animation.models, boat);
        if boat.record.base.automata_flags & 2 != 0
            && !self.runtime_state_tick(id, time.animation, motion, crashes, manager)?
            && self.boat(id)?.record.base.automata_flags & 4 == 0
        {
            self.boat_mut(id)?
                .generic_seek(time.movement, time.scale, |p, n, mask| {
                    motion.spatial.adjust_segment(p, n, mask);
                });
        }
        let boat = self.boat_mut(id)?;
        boat.events.push(Event::Notify(0, 0));
        boat.update_backwards_time(time.movement);
        boat.update_wakes(time.movement, time.cell_size);
        if !boat.still_time_allows_animation() {
            // §5.1.3 is the manager caller's removal request, separate from
            // an owner PushState/remove event.
            boat.events.push(Event::Remove(false));
            return Ok(false);
        }
        if self.lifetime_tick(id, time.movement, manager)? {
            let rectangle = motion.queries.cell_rectangle(id);
            if self.movement_possible(id, rectangle, motion.terrain, manager)? {
                let dt = self.boat(id)?.clamped_frame_time(time.movement);
                self.ai_motion_core(id, dt, motion.terrain, |c| motion.spatial.centre(c), manager)?;
                self.boat_mut(id)?.movement_position_new(dt, motion.terrain);
                self.detect_collision(id, motion.ai_collisions, motion.terrain, motion.queries, manager)?;
                self.dock_requests(id, manager)?;
                self.clear_dock_requests(id, manager)?;
            } else if self.boat(id)?.on_land(motion.terrain) {
                self.push_state(id, State::new(state::IMMEDIATE_REMOVE), manager)?;
            }
        }
        let boat = self.boat_mut(id)?;
        boat.events.push(Event::EffectUpdate);
        boat.events.push(Event::ModelUpdate);
        animation.subsidiary.lighting_tick(
            boat,
            animation.processed_model,
            animation.heading_threshold,
            animation.distance_squared,
            animation
                .lighting
                .as_mut()
                .map(|lighting| &mut **lighting as &mut dyn Lighting),
        );
        Ok(true)
    }

    /// Full §6.1/§6.2.3 drive order. A false base return suppresses only the
    /// base channel/translation/cell work; the owner continues and returns true.
    pub fn player_drive_tick<T: Terrain>(
        &mut self,
        id: BoatId,
        dt: f32,
        motion: &mut MotionServices<'_, T>,
        drive: &mut DriveServices<'_>,
        manager: &mut Option<&mut dyn DockManager>,
    ) -> Result<bool> {
        if !drive.removal_predicate {
            drive.command.advance_channels(dt);
            self.player_commands(id, dt, drive.command, drive.effects, manager)?;
            drive.cell = motion.terrain.cell_at(self.boat(id)?.position);
        }
        self.release_tow_ahead(id, manager)?;
        self.boat_mut(id)?
            .coast(dt, drive.command.throttle, drive.command.reverse);
        let rectangle = motion.queries.cell_rectangle(id);
        if self.movement_possible(id, rectangle, motion.terrain, manager)? {
            let time = self.boat(id)?.clamped_frame_time(dt);
            self.boat_mut(id)?.movement_position_new(time, motion.terrain);
            self.detect_collision(id, motion.ai_collisions, motion.terrain, motion.queries, manager)?;
        }
        Ok(true)
    }

    /// Full far/final docking branch. The final helper remains useful by
    /// itself; the owner no longer needs a provider of a far-approach result.
    pub fn docking_tick_core<T: Terrain>(
        &mut self,
        id: BoatId,
        dt: f32,
        motion: &mut MotionServices<'_, T>,
        manager: &mut Option<&mut dyn DockManager>,
    ) -> Result<bool> {
        let boat = self.boat(id)?;
        let station = boat.other_station().ok_or(ContractGap::MissingStationWc7)?;
        let distance = math::length(math::sub(station.position, boat.inherited_position));
        if distance <= boat.drive.0[37] {
            return self.docking_tick(id, dt, motion.terrain, manager);
        }
        if boat.on_land(motion.terrain) {
            self.push_state(id, State::new(state::IMMEDIATE_REMOVE), manager)?;
        }
        let boat = self.boat_mut(id)?;
        if boat.inherited_heading[1] != 0.0 {
            boat.inherited_heading = math::normalize(math::flat(boat.inherited_heading));
        }
        let time = boat.clamped_frame_time(dt);
        self.ai_motion_core(id, time, motion.terrain, |c| motion.spatial.centre(c), manager)?;
        self.boat_mut(id)?.movement_position_new(time, motion.terrain);
        self.detect_collision(id, motion.ai_collisions, motion.terrain, motion.queries, manager)?;
        Ok(true)
    }

    /// Watercraft-owned cases in the inherited state dispatch (§4). A false
    /// result requests generic seek; a finished owner movement pops its state.
    pub fn runtime_state_tick<T: GroundingTerrain>(
        &mut self,
        id: BoatId,
        dt: f32,
        motion: &mut MotionServices<'_, T>,
        crashes: &mut dyn CrashEffects,
        manager: &mut Option<&mut dyn DockManager>,
    ) -> Result<bool> {
        let active = match self.boat(id)?.top_state() {
            Some(state::RUN_AGROUND | state::SINK) => {
                let rectangle = motion.queries.cell_rectangle(id);
                self.crash_state_tick(id, dt, rectangle, motion.terrain, crashes, manager)?
            }
            Some(state::IDLE) => {
                self.boat_mut(id)?.idle_elapsed += dt;
                if self.boat(id)?.idle_elapsed > 60_000.0 {
                    self.push_state(id, State::new(state::SLOW_REMOVE), manager)?;
                    let rectangle = motion.queries.cell_rectangle(id);
                    self.crash_state_tick(id, dt, rectangle, motion.terrain, crashes, manager)?;
                }
                return Ok(true);
            }
            Some(state::DOCK) => self.docking_tick_core(id, dt, motion, manager)?,
            Some(state::TOWED) => self.following_tick(id, dt, motion.terrain)?,
            Some(state::UNDOCK) => {
                let rectangle = motion.queries.cell_rectangle(id);
                self.undocking_tick(id, dt, rectangle, motion.terrain, manager)?
            }
            Some(state::QUEUE) => {
                self.dock_requests(id, manager)?;
                return Ok(true);
            }
            Some(state::IMMEDIATE_REMOVE | state::SLOW_REMOVE) => {
                let immediate = self.boat(id)?.top_state() == Some(state::IMMEDIATE_REMOVE);
                self.boat_mut(id)?.events.push(Event::Remove(immediate));
                return Ok(true);
            }
            _ => return Ok(false),
        };
        if !active {
            self.pop_state(id, manager)?;
        }
        Ok(true)
    }
}

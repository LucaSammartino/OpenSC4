// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::automata::StateExtra;

impl Pedestrian {
    pub fn set_state(
        &mut self,
        kind: u32,
        args: SetStateArgs,
        world: &mut impl PedestrianWorld,
    ) -> crate::vehicles::Result<bool> {
        if !self.base_mut().set_state(kind, args) {
            return Ok(false);
        }
        self.enter_state(false, world)?;
        Ok(true)
    }

    pub fn push_state(
        &mut self,
        kind: u32,
        args: SetStateArgs,
        world: &mut impl PedestrianWorld,
    ) -> crate::vehicles::Result<bool> {
        if !self.base_mut().push_state(kind, args) {
            return Ok(false);
        }
        self.enter_state(false, world)?;
        Ok(true)
    }

    pub fn pop_state(&mut self, world: &mut impl PedestrianWorld) -> crate::vehicles::Result<bool> {
        let old = self.base().states.last().map(|s| s.state_type);
        let result = self.base_mut().pop_state();
        if result {
            if let Some(kind) = old {
                world.effects(EffectAction::ExitState(kind));
            }
            self.enter_state(false, world)?;
        }
        Ok(result)
    }

    /// `inherited_handled` is the inherited hook's return, not a host owner call.
    pub fn enter_state(
        &mut self,
        inherited_handled: bool,
        world: &mut impl PedestrianWorld,
    ) -> crate::vehicles::Result<()> {
        let kind = self.base().states.last().map(|s| s.state_type);
        if let Some(kind) = kind {
            world.effects(EffectAction::EnterState(kind));
        }
        if inherited_handled {
            return Ok(());
        }
        let (on, off, repath) = match kind {
            None => (0, 0x6000, false),
            Some(1) => (0x26000, 0, false),
            Some(2 | 3) => (0x2000, 0x4000, false),
            Some(4 | 9) => (0x2002, 0x4000, false),
            Some(6) => (2, 0x2000, true),
            Some(7) => (0, 0, false),
            Some(8) => (2 | 0x2000 | 0x10000 | 0x1000000 | 0x2000000 | 0x10000000, 0x4000, false),
            Some(_) => (0, 0x6000, true),
        };
        self.base_mut().automata_flags = (self.base().automata_flags | on) & !off;
        if matches!(kind, Some(8 | 9)) {
            world.play_animation(RIOT_ANIMATION);
        }
        if kind == Some(8) {
            self.riot_leader_tick(0.0, 1.0, true, world);
        }
        if repath {
            self.repath(world)?;
        }
        Ok(())
    }

    pub(crate) fn repath(&mut self, world: &mut impl PedestrianWorld) -> crate::vehicles::Result<bool> {
        let target = self.base().vectors[1];
        self.path_to_box(
            Bounds {
                min: target,
                max: target,
            },
            world,
        )
    }

    fn path_to_box(&mut self, target: Bounds, world: &mut impl PedestrianWorld) -> crate::vehicles::Result<bool> {
        let Some(route) = world.find_path_to_box(self.base().vectors[0], target) else {
            return Ok(false);
        };
        self.traffic_owner().set_route_info(&route, 1.0);
        self.traffic_owner().begin_route(world)?;
        Ok(true)
    }

    pub fn state_tick(
        &mut self,
        dt_ms: f32,
        movement: Movement,
        world: &mut impl PedestrianWorld,
    ) -> crate::vehicles::Result<bool> {
        if self.base().states.is_empty() {
            return Ok(false);
        }
        if self.inherited_tick(dt_ms, movement, world)? {
            return Ok(true);
        }
        let kind = self.base().states.last().map(|s| s.state_type);
        match kind {
            Some(2) => {
                self.bee_line_tick(dt_ms, movement.speed_scale, world);
            }
            Some(3) => {
                self.crowd_tick(dt_ms, movement.speed_scale, world);
            }
            Some(4) => {
                self.queue_tick(dt_ms, movement.speed_scale, world)?;
            }
            Some(6) => {
                self.strike_tick(world)?;
            }
            Some(8) => {
                self.riot_leader_tick(dt_ms, movement.speed_scale, false, world);
            }
            Some(9) => {
                self.riot_follower_tick(dt_ms, movement.speed_scale, world);
            }
            Some(12) => {
                self.ignite_tick(world)?;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub fn bee_line_tick(&mut self, dt_ms: f32, modifier: f32, world: &mut impl PedestrianWorld) -> bool {
        let direction = self.base().v2;
        if length(direction) != 0.0 {
            self.move_forward_with_collision(direction, dt_ms, modifier, world);
        }
        VehicleWorld::refresh_behavior(world);
        true
    }

    pub fn crowd_tick(&mut self, dt_ms: f32, modifier: f32, world: &mut impl PedestrianWorld) {
        if let Some(push) = self.runtime.pending_push.take() {
            let position = world.ground_position(add(self.base().vectors[0], push), 0xC49);
            self.move_to(position, self.base().vectors[2], world);
        }
        let controller = self
            .base()
            .states
            .last()
            .map(|s| s.base.controller.clone())
            .unwrap_or(RefValue::Null);
        if let Some(target) = world.controller_attractor(&controller) {
            self.move_forward_with_collision(sub(target, self.base().vectors[0]), dt_ms, modifier, world);
        }
        // The summary gives no reaction timing/distribution: literal cycling at
        // renderer completion, uniformly over the foreign animation candidates.
        if self.runtime.reaction_kind != 0 && (!self.runtime.reaction_playing || world.animation_finished()) {
            let animations = world.reaction_animations(self.runtime.reaction_kind);
            if !animations.is_empty() {
                let index = world.automata_range(0.0, animations.len() as f64) as usize;
                world.play_animation(animations[index.min(animations.len() - 1)]);
                self.runtime.reaction_playing = true;
            }
        }
    }

    pub fn queue_tick(
        &mut self,
        dt_ms: f32,
        modifier: f32,
        world: &mut impl PedestrianWorld,
    ) -> crate::vehicles::Result<()> {
        let controller = self
            .base()
            .states
            .last()
            .map(|s| s.base.controller.clone())
            .unwrap_or(RefValue::Null);
        if matches!(controller, RefValue::Null) {
            self.pop_state(world)?;
            return Ok(());
        }
        if let Some(point) = world.queue_step(&controller, self.base().vectors[0]) {
            self.base_mut().vectors[1] = point;
        }
        self.traffic_owner().straight_tick(dt_ms, modifier, world);
        if self.base().automata_flags & 0x20 != 0 {
            if let Some(point) = world.queue_head(&controller) {
                let position = self.base().vectors[0];
                self.move_to(position, unit(sub(point, position)), world);
            }
            world.play_idle();
        }
        VehicleWorld::refresh_behavior(world);
        Ok(())
    }

    pub fn strike_tick(&mut self, world: &mut impl PedestrianWorld) -> crate::vehicles::Result<()> {
        let controller = self
            .base()
            .states
            .last()
            .map(|s| s.base.controller.clone())
            .unwrap_or(RefValue::Null);
        if let Some(mut bounds) = world.controller_target(&controller) {
            // Literal one-cell inset (city cells are 16 world units).
            for axis in [0, 2] {
                bounds.min[axis] += 16.0;
                bounds.max[axis] -= 16.0;
            }
            let p = self.base().vectors[0];
            let inside =
                p[0] >= bounds.min[0] && p[0] <= bounds.max[0] && p[2] >= bounds.min[2] && p[2] <= bounds.max[2];
            if inside || !self.path_to_box(bounds, world)? {
                world.return_to_source();
            }
        }
        Ok(())
    }

    pub fn riot_leader_tick(&mut self, dt_ms: f32, modifier: f32, start: bool, world: &mut impl PedestrianWorld) {
        let Some(cell) = world.road_cell(self.base().vectors[0]) else {
            self.traffic_owner().remove(world);
            return;
        };
        let previous = self.base().states.last().and_then(|s| match s.extra {
            StateExtra::RiotLeader { a, b, c } => Some(([a, b], c)),
            _ => None,
        });
        let mut heading = previous.map(|(_, h)| h).unwrap_or(0);
        if start || previous.map(|(old, _)| old != cell).unwrap_or(true) {
            let headings: Vec<u8> = (0..4).filter(|h| world.walkable_heading(cell, *h)).collect();
            if headings.is_empty() {
                self.traffic_owner().remove(world);
                return;
            }
            // Literal random choice among walkable headings; PN3 tracks missing weighting.
            let index = world.automata_range(0.0, headings.len() as f64) as usize;
            heading = headings[index.min(headings.len() - 1)];
            if let Some(state) = self.base_mut().states.last_mut() {
                state.extra = StateExtra::RiotLeader {
                    a: cell[0],
                    b: cell[1],
                    c: heading,
                };
            }
        }
        let direction =
            [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [-1.0, 0.0, 0.0], [0.0, 0.0, -1.0]][usize::from(heading % 4)];
        self.move_forward_with_collision(direction, dt_ms, modifier, world);
    }

    pub fn riot_follower_tick(&mut self, dt_ms: f32, modifier: f32, world: &mut impl PedestrianWorld) {
        if matches!(self.base().leader, RefValue::Null) {
            self.base_mut().leader = world.acquire_leader(IID_AUTOMATON);
        }
        let leader = self.base().leader.clone();
        let direction = world
            .leader_position(&leader)
            .map(|p| sub(p, self.base().vectors[0]))
            .filter(|v| length(*v) > 10.0);
        let direction = direction.unwrap_or_else(|| {
            // Literal random milling; no turn interval is supplied by the summary.
            let angle = world.automata_range(0.0, std::f64::consts::TAU) as f32;
            [angle.cos(), 0.0, angle.sin()]
        });
        self.move_forward_with_collision(direction, dt_ms, modifier, world);
    }

    pub fn ignite_tick(&mut self, world: &mut impl PedestrianWorld) -> crate::vehicles::Result<()> {
        if let Some(state) = self.base().states.iter().rev().find(|s| s.state_type == 12) {
            world.ignite_target(&state.base.controller, 2);
        }
        self.set_state(1, SetStateArgs::default(), world)?;
        Ok(())
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
//! Model geometry and owner collision decisions from watercraft §5.7.
use super::{math, state, BoatId, Cell, DockManager, Event, Fleet, Result, Terrain, Transform, Watercraft};
use crate::automata::{State, Vec3};

/// Foreign occupant, terrain-normal and surface-water queries only.
pub trait CollisionQueries {
    fn cell_rectangle(&mut self, boat: BoatId) -> [Cell; 4];
    fn occupied(&mut self, cell: Cell) -> bool;
    fn is_wet(&mut self, cell: Cell) -> bool;
    fn terrain_normal(&mut self, position: Vec3) -> Vec3;
}

#[derive(Clone, Debug)]
pub struct ModelGeometry {
    pub bounds: [Vec3; 2],
    pub transform: Transform,
    pub queried_interface: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Footprint {
    pub valid: bool,
    pub center: [f32; 2],
    pub half_extents: [f32; 2],
    pub axis: [f32; 2],
    /// The two additional scratch floats cleared by the builder.
    pub scratch: [f32; 2],
}

impl Footprint {
    /// Missing geometry clears six floats, retaining validity and axis (§5.7.3).
    pub fn build(&mut self, model: Option<&ModelGeometry>, offset: Vec3, multiplier: f32) -> &mut Self {
        self.center = [0.0; 2];
        self.half_extents = [0.0; 2];
        self.scratch = [0.0; 2];
        let Some(model) = model else {
            return self;
        };
        let margin = if model.queried_interface {
            [2.0, 1.0]
        } else {
            [0.5, 0.5]
        };
        let d = math::sub(model.bounds[1], model.bounds[0]);
        self.center = [
            model.transform.translation[0] + offset[0],
            model.transform.translation[2] + offset[2],
        ];
        self.half_extents = [
            (d[0] - margin[0]) * 0.5 * multiplier,
            (d[2] - margin[1]) * 0.5 * multiplier,
        ];
        // +4 is row0/column0, +0xC is row0/column2.
        let a = [model.transform.basis[0][0], -model.transform.basis[2][0]];
        let inverse = 1.0 / (a[0] * a[0] + a[1] * a[1]).sqrt();
        self.axis = [a[0] * inverse, a[1] * inverse];
        self.valid = true;
        self
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        if !self.valid || !other.valid {
            return false;
        }
        let e = self.axis;
        let f = [-e[1], e[0]];
        let g = other.axis;
        let h = [-g[1], g[0]];
        let delta = [other.center[0] - self.center[0], other.center[1] - self.center[1]];
        let dot = |a: [f32; 2], b: [f32; 2]| a[0] * b[0] + a[1] * b[1];
        for n in [e, f, g, h] {
            let radius = self.half_extents[0] * dot(e, n).abs()
                + self.half_extents[1] * dot(f, n).abs()
                + other.half_extents[0] * dot(g, n).abs()
                + other.half_extents[1] * dot(h, n).abs();
            if dot(delta, n).abs() > radius {
                return false;
            }
        }
        true
    }
}

impl super::Group {
    pub fn contains(&self, tag: u32) -> bool {
        self.id == tag || self.parent.as_deref().is_some_and(|parent| parent.contains(tag))
    }
    pub fn size_class(&self) -> u32 {
        if self.contains(0x4414) {
            3
        } else if self.contains(0x4413) {
            2
        } else {
            1
        }
    }
}

impl Watercraft {
    /// §5.7.4 all occupant queries precede all navigability queries.
    pub fn inanimate_collision(
        &mut self,
        rectangle: [i32; 4],
        mut occupied: impl FnMut(Cell) -> bool,
        mut navigable: impl FnMut(Cell) -> bool,
    ) -> bool {
        let [x0, z0, x1, z1] = rectangle;
        let corners = [
            Cell { x: x0, z: z0 },
            Cell { x: x0, z: z1 },
            Cell { x: x1, z: z0 },
            Cell { x: x1, z: z1 },
        ];
        let hit = corners.iter().position(|c| occupied(*c)).or_else(|| {
            if self.docking_in_progress {
                None
            } else {
                corners.iter().position(|c| !navigable(*c))
            }
        });
        let Some(corner) = hit else {
            return false;
        };
        let [x, _, z] = self.inherited_heading;
        let column = usize::from(x.abs() <= z.abs()) * 4 + usize::from(x <= 0.0) * 2 + usize::from(z >= 0.0);
        const TABLE: [[[u8; 2]; 8]; 4] = [
            [[1, 0], [1, 0], [0, 1], [0, 1], [0, 0], [1, 1], [0, 0], [1, 1]],
            [[1, 1], [1, 1], [0, 0], [0, 0], [1, 0], [0, 1], [1, 0], [0, 1]],
            [[0, 0], [0, 0], [1, 1], [1, 1], [0, 1], [1, 0], [0, 1], [1, 0]],
            [[0, 1], [0, 1], [1, 0], [1, 0], [1, 1], [0, 0], [1, 1], [0, 0]],
        ];
        self.crash_direction = TABLE[corner][column];
        true
    }

    pub fn bump_rollback(&mut self) {
        self.position = self.previous_position;
        self.transform.translation = self.position;
        self.transform.changed(1);
        self.publish_transform();
    }
}

impl Fleet {
    /// Full ordered detector. A boat bump continues into the inanimate scan;
    /// outside-city removal and either crash return before later queries.
    pub fn detect_collision(
        &mut self,
        id: BoatId,
        ai_collisions_enabled: bool,
        terrain: &dyn Terrain,
        queries: &mut dyn CollisionQueries,
        manager: &mut Option<&mut dyn DockManager>,
    ) -> Result<bool> {
        let rectangle = queries.cell_rectangle(id);
        if [0, 2, 1, 3].into_iter().any(|i| !terrain.contains(rectangle[i])) {
            self.push_state(id, State::new(state::IMMEDIATE_REMOVE), manager)?;
            return Ok(true);
        }
        let a = self.boat(id)?;
        if a.top_state() == Some(state::PLAYER_DRIVE) || ai_collisions_enabled {
            if let Some(other) = self.find_overlapping_boat(id, [0.0; 3], 1.0)? {
                let crashes = self.boat(id)?.boat_crash_threshold_exceeded(self.boat(other)?.speed);
                let other_rectangle = if crashes {
                    [Cell { x: 0, z: 0 }; 4]
                } else {
                    queries.cell_rectangle(other)
                };
                if self.respond_to_boat_collision(id, other, other_rectangle, terrain, manager)? {
                    return Ok(true);
                }
            }
        }
        let rectangle = queries.cell_rectangle(id);
        let bounds = [rectangle[0].x, rectangle[0].z, rectangle[3].x, rectangle[3].z];
        let hit =
            self.boat_mut(id)?
                .inanimate_collision(bounds, |c| queries.occupied(c), |c| terrain.navigable(c, 0, None));
        if !hit {
            return Ok(false);
        }
        if !self.boat(id)?.shore_crash_threshold_exceeded() {
            self.boat_mut(id)?.bump_rollback();
            return Ok(false);
        }
        if self.boat(id)?.top_state() == Some(state::PLAYER_DRIVE) {
            self.boat_mut(id)?.events.push(Event::Aems(7));
        }
        let mut dry = [false; 4];
        // The case helper short-circuits at the first dry corner.
        for (i, cell) in queries.cell_rectangle(id).into_iter().enumerate() {
            dry[i] = !queries.is_wet(cell);
            if dry[i] {
                break;
            }
        }
        let normal = if dry.iter().any(|c| *c) {
            queries.terrain_normal(self.boat(id)?.position)
        } else {
            [0.0, 1.0, 0.0]
        };
        let case = self.boat(id)?.determine_crash_case(dry, normal)?;
        self.set_state_internal(id, case, manager)?;
        Ok(true)
    }

    /// Manager-list order; model-less scratch validity is supplied explicitly.
    pub fn find_overlapping_boat(&self, id: BoatId, offset: Vec3, multiplier: f32) -> Result<Option<BoatId>> {
        let a = self.boat(id)?;
        let mut footprint = a.footprint_scratch.clone();
        footprint.build(a.model_geometry.as_ref(), offset, multiplier);
        for &other in &self.order {
            if other == id {
                continue;
            }
            let b = self.boat(other)?;
            let max = Watercraft::maximum_extent(a.occupant_bounds).max(Watercraft::maximum_extent(b.occupant_bounds));
            if math::length(math::sub(a.inherited_position, b.inherited_position)) >= max {
                continue;
            }
            let mut candidate = b.footprint_scratch.clone();
            candidate.build(b.model_geometry.as_ref(), [0.0; 3], 1.0);
            if footprint.overlaps(&candidate) {
                return Ok(Some(other));
            }
        }
        Ok(None)
    }

    /// Admitted boat response (§5.7.0). False means a bump, then shore work.
    pub fn respond_to_boat_collision(
        &mut self,
        id: BoatId,
        other: BoatId,
        other_rectangle: [Cell; 4],
        terrain: &dyn Terrain,
        manager: &mut Option<&mut dyn DockManager>,
    ) -> Result<bool> {
        let a = self.boat(id)?;
        let b = self.boat(other)?;
        if a.boat_crash_threshold_exceeded(b.speed) {
            let player_a = a.top_state() == Some(state::PLAYER_DRIVE);
            let player_b = b.top_state() == Some(state::PLAYER_DRIVE);
            let size_a = a.group.as_deref().map_or(1, super::Group::size_class);
            let size_b = b.group.as_deref().map_or(1, super::Group::size_class);
            if player_a {
                self.boat_mut(id)?.events.push(Event::Aems(7));
            }
            for (victim, loses) in [(id, size_a <= size_b), (other, size_b <= size_a)] {
                if !loses {
                    continue;
                }
                if player_a || player_b {
                    let boat = self.boat_mut(victim)?;
                    if boat.top_state() == Some(state::PLAYER_DRIVE) {
                        boat.sunk_by_collision = true;
                    }
                    boat.events.push(Event::Notify(0xb, 0));
                    let mut next = State::new(state::SINK);
                    next.base.countdown_ms = boat.drive.0[21];
                    next.base.is_final = true;
                    self.push_state(victim, next, manager)?;
                } else {
                    self.push_state(victim, State::new(state::SLOW_REMOVE), manager)?;
                }
            }
            return Ok(true);
        }
        self.boat_mut(id)?.bump_rollback();
        let speed = self.boat(id)?.speed;
        if self.movement_possible(other, other_rectangle, terrain, manager)? {
            self.boat_mut(other)?.speed = speed;
        }
        let boat = self.boat_mut(id)?;
        boat.speed = boat.drive.0[29] * speed;
        Ok(false)
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
use std::collections::VecDeque;

use crate::automata::{AutomatonBase, SetStateArgs, TrafficRecord, Vec3};
use crate::stream::{InStream, OutStream, RefValue};

use super::{add, direction_toward, length, scale, sub, Result, Unsupported};

pub const REMOVE: u32 = 1;
pub const REACHED: u32 = 4;
pub const COLLISION: u32 = 8;
pub const BLOCKED: u32 = 0x10;
pub const STOPPED: u32 = 0x20;
pub const SPECIAL_ROUTE: u32 = 0x40;
pub const STRAIGHT: u32 = 0x2000;

pub type RouteStep = [u8; 3];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectAction {
    EnterState(u32),
    ExitState(u32),
    Update,
    StopAll,
}

/// Foreign network/occupant/renderer operations, rather than owner callbacks.
/// `advance_path` supplies network geometry only; the owner decides flags,
/// speed matching, destination anchoring, movement and lifetime.
pub trait TrafficWorld {
    fn network_at(&mut self, step: RouteStep) -> Option<RefValue>;
    fn placement(&mut self, network: &RefValue, step: RouteStep) -> Option<(Vec3, Vec3)>;
    fn look_ahead(&mut self, step: RouteStep) -> Option<Vec3>;
    fn advance_path(&mut self, position: Vec3, step: f32, vehicle: bool) -> Option<PathAdvance>;
    fn intersection(&mut self, network: &RefValue, position: Vec3) -> (u32, f32);
    fn collision_allows(&mut self, from: Vec3, to: Vec3, flags: u32) -> bool;
    fn move_models(&mut self, position: Vec3, orientation: Vec3);
    fn effects(&mut self, action: EffectAction);
    fn event(&mut self, id: u32, value: u32);
    fn removed(&mut self);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathAdvance {
    pub position: Vec3,
    pub facing: Vec3,
    pub remaining: f32,
    pub at_end: bool,
    pub stopped: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Movement {
    pub speed_scale: f32,
    pub max_speed: f32,
    pub leader_speed: Option<f32>,
}

/// Candidate eligibility is supplied by the occupant manager and slot 73.
#[derive(Clone, Debug, PartialEq)]
pub struct LeaderCandidate {
    pub id: u64,
    pub position: Vec3,
    pub removed: bool,
    pub may_follow: bool,
    pub straight: bool,
    pub lane_bits: u32,
    /// `FindLeader` additionally filters candidates behind the automaton.
    pub ahead: bool,
}

/// vehicles.md §2, conformance 4. Height never affects the distance.
pub fn find_leader_in_tile(position: Vec3, lanes: u32, candidates: &[LeaderCandidate]) -> Option<u64> {
    candidates
        .iter()
        .filter(|c| !c.removed && c.may_follow && (c.straight || c.lane_bits == lanes))
        .min_by(|a, b| horizontal_distance(a.position, position).total_cmp(&horizontal_distance(b.position, position)))
        .map(|c| c.id)
}

fn horizontal_distance(a: Vec3, b: Vec3) -> f32 {
    (a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)
}

/// Route tiles must be supplied in route order. With <2 entries use current
/// tile, then the next tile; otherwise stop at the first route tile with a leader.
pub fn find_leader(position: Vec3, lanes: u32, tiles: &[Vec<LeaderCandidate>]) -> Option<u64> {
    tiles.iter().find_map(|tile| {
        let ahead: Vec<_> = tile.iter().filter(|c| c.ahead).cloned().collect();
        find_leader_in_tile(position, lanes, &ahead)
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollisionContext {
    pub occupant_flags: u32,
    pub leader_gone: bool,
    pub too_close: bool,
    pub speed_modifier: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrafficRuntime {
    pub look_ahead: VecDeque<Vec3>,
    pub has_tile_position: bool,
    pub has_path: bool,
    pub initialized: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrafficAutomaton {
    pub record: TrafficRecord,
    pub runtime: TrafficRuntime,
}

/// Small shared facade also used over a VehicleRecord, without a second copy
/// of the traffic state or record.
pub struct TrafficOwner<'a> {
    pub record: &'a mut TrafficRecord,
    pub runtime: &'a mut TrafficRuntime,
}

impl TrafficAutomaton {
    pub fn owner(&mut self) -> TrafficOwner<'_> {
        TrafficOwner {
            record: &mut self.record,
            runtime: &mut self.runtime,
        }
    }
    pub fn read(&mut self, input: &mut dyn InStream) -> bool {
        let Some(record) = TrafficRecord::read(input) else {
            return false;
        };
        if input.error() != 0 {
            return false;
        }
        self.record = record;
        self.runtime = TrafficRuntime::default();
        true
    }
    pub fn write(&self, output: &mut dyn OutStream) -> bool {
        self.record.write(output)
    }
    pub fn init(&mut self) -> Result<()> {
        // [spec gap VH4] Constructor/Init flags are not enumerated.
        Err(Unsupported::InitializationVh4)
    }
    pub fn best_neighbor_connection(&self) -> Result<RouteStep> {
        Err(Unsupported::NeighborRankingVh2)
    }

    pub fn animation_tick(
        &mut self,
        dt_ms: f32,
        movement: Movement,
        moves: bool,
        held: bool,
        world: &mut impl TrafficWorld,
    ) -> Result<()> {
        advance_tweens(&mut self.record.base, dt_ms);
        world.effects(EffectAction::Update);
        let kind = self.record.base.states.last().map(|s| s.state_type).unwrap_or(0);
        let handled = if !moves {
            true
        } else {
            match kind {
                1 => {
                    self.owner().remove(world);
                    true
                }
                10 => self.record.base.automata_flags & REMOVE == 0,
                13 => self.owner().blocked_wait_tick(dt_ms, movement.speed_scale, world),
                30 => {
                    if tween_value(&self.record.base.f3[..4]) > 0.0 {
                        self.owner().path_tick(dt_ms, movement, false, world)?;
                    }
                    true
                }
                7 | 11 | 15 | 16 | 18 | 19 | 20 => return Err(Unsupported::BaseStateVh9),
                _ => false,
            }
        };
        if !handled {
            if self.record.base.automata_flags & REACHED == 0 {
                if !matches!(self.record.network, RefValue::Null)
                    && self.runtime.has_tile_position
                    && self.runtime.has_path
                    && self.record.base.automata_flags & STRAIGHT == 0
                {
                    self.owner().path_tick(dt_ms, movement, false, world)?;
                } else {
                    self.owner().straight_tick(dt_ms, movement.speed_scale, world);
                }
            } else if !held {
                self.owner().remove(world);
            }
        }
        world.event(0, 0);
        Ok(())
    }
}

pub fn tween_value(t: &[f32]) -> f32 {
    if t[2] <= 0.0 {
        t[1]
    } else {
        t[0] + (t[3] / t[2]).clamp(0.0, 1.0) * (t[1] - t[0])
    }
}

pub fn tween_speed(base: &mut AutomatonBase, target: f32, seconds: f32) {
    let start = tween_value(&base.f3[..4]);
    base.f3[..4].copy_from_slice(&[start, target, seconds, 0.0]);
}

pub fn return_to_default_speed(base: &mut AutomatonBase, modifier: f32) {
    tween_speed(base, base.f1[1], 0.3 / modifier);
}

pub fn advance_tweens(base: &mut AutomatonBase, dt_ms: f32) {
    for tween in base.f3.chunks_mut(4) {
        tween[3] += dt_ms / 1000.0;
        if tween[3] >= tween[2] {
            tween[0] = tween[1];
        }
    }
}

impl TrafficOwner<'_> {
    pub fn set_route_info(&mut self, route: &[RouteStep], speed_modifier: f32) {
        self.record.route = route.to_vec();
        return_to_default_speed(&mut self.record.base, speed_modifier);
    }

    pub fn begin_route(&mut self, world: &mut impl TrafficWorld) -> Result<()> {
        self.runtime.look_ahead.clear();
        if self.record.route.is_empty() {
            return Err(Unsupported::RouteGeometryVh7);
        }
        let first = self.record.route.remove(0);
        let single = self.record.route.is_empty();
        let Some(network) = world.network_at(first) else {
            // [literal VH7] No placement without an occupant; failure flags
            // and rollback are not specified, so report the missing contract.
            return Err(Unsupported::RouteGeometryVh7);
        };
        let Some((position, facing)) = world.placement(&network, first) else {
            return Err(Unsupported::RouteGeometryVh7);
        };
        self.record.network = network;
        self.runtime.has_tile_position = true;
        self.record.base.vectors[0] = position;
        self.record.base.vectors[2] = facing;
        world.move_models(position, facing);
        if single {
            self.record.base.automata_flags |= REACHED;
        }
        Ok(())
    }

    /// Caller supplies slot 175's requested count (VH2); entries stop at first failure.
    pub fn fill_look_ahead(&mut self, requested: usize, world: &mut impl TrafficWorld) {
        for step in self.record.route.iter().skip(self.runtime.look_ahead.len()) {
            if self.runtime.look_ahead.len() >= requested {
                break;
            }
            let Some(point) = world.look_ahead(*step) else {
                break;
            };
            self.runtime.look_ahead.push_back(point);
        }
    }

    pub fn remove(&mut self, world: &mut impl TrafficWorld) {
        if self.record.base.automata_flags & REMOVE == 0 {
            self.record.base.automata_flags |= REMOVE;
            world.removed();
        }
    }

    pub fn shutdown(&mut self, world: &mut impl TrafficWorld) {
        world.effects(EffectAction::StopAll);
        self.runtime.look_ahead.clear();
        self.record.network = RefValue::Null;
        self.record.automaton = RefValue::Null;
        self.record.base.leader = RefValue::Null;
        self.runtime.initialized = false;
        self.runtime.has_tile_position = false;
        self.runtime.has_path = false;
    }

    /// vehicles.md §2. Refresh is returned to the caller so the foreign
    /// occupant query can be made without borrowing this owner's state.
    pub fn update_collision(&mut self, context: CollisionContext, world: &mut impl TrafficWorld) -> bool {
        let flags = self.record.base.automata_flags;
        let refresh = flags & COLLISION != 0 && (context.leader_gone || flags & BLOCKED != 0);
        if flags & 0x800 != 0 || (flags & 0x200000 != 0 && context.occupant_flags & 4 == 0) {
            self.record.base.automata_flags &= !BLOCKED;
            if context.leader_gone {
                return_to_default_speed(&mut self.record.base, context.speed_modifier);
            }
            return refresh;
        }
        if self.record.base.states.last().map(|s| s.state_type) == Some(13) {
            return refresh;
        }
        let (taken, delay) = world.intersection(&self.record.network, self.record.base.vectors[0]);
        if taken != 0 {
            self.record.base.automata_flags |= BLOCKED;
            if flags & STOPPED != 0 && taken == 1 {
                self.record.base.push_state(
                    13,
                    SetStateArgs {
                        timeout_ms: delay * 1000.0,
                        ..SetStateArgs::default()
                    },
                );
                world.effects(EffectAction::EnterState(13));
            }
            tween_speed(&mut self.record.base, 0.0, 1.5 / context.speed_modifier);
        } else if flags & BLOCKED == 0 || !context.too_close {
            self.record.base.automata_flags &= !BLOCKED;
            return_to_default_speed(&mut self.record.base, context.speed_modifier);
        }
        refresh
    }

    pub fn blocked_wait_tick(&mut self, dt_ms: f32, modifier: f32, world: &mut impl TrafficWorld) -> bool {
        let Some(state) = self.record.base.states.last_mut() else {
            return false;
        };
        if state.base.countdown_ms > 0.0 {
            if self.record.base.automata_flags & STOPPED == 0 {
                return false;
            }
            let modifier = if self.record.base.automata_flags & 0x80 != 0 {
                modifier.clamp(0.0, 1.0)
            } else {
                modifier
            };
            state.base.countdown_ms -= modifier * dt_ms;
            if state.base.countdown_ms > 0.0 {
                return true;
            }
        }
        return_to_default_speed(&mut self.record.base, modifier);
        self.record.base.automata_flags &= !(STOPPED | BLOCKED);
        if world.intersection(&self.record.network, self.record.base.vectors[0]).0 == 0 {
            self.record.base.pop_state();
            world.effects(EffectAction::ExitState(13));
        }
        false
    }

    pub fn path_tick(
        &mut self,
        dt_ms: f32,
        movement: Movement,
        vehicle: bool,
        world: &mut impl TrafficWorld,
    ) -> Result<Option<f32>> {
        let step = tween_value(&self.record.base.f3[..4]) * movement.speed_scale * dt_ms;
        if step < -movement.max_speed && self.record.base.automata_flags & SPECIAL_ROUTE == 0 {
            self.remove(world);
            return Ok(None);
        }
        let old = self.record.base.vectors[0];
        let Some(next) = world.advance_path(old, step, vehicle) else {
            return Err(Unsupported::RouteGeometryVh7);
        };
        if next.remaining < 16.0 {
            if let Some(speed) = movement.leader_speed {
                tween_speed(&mut self.record.base, speed, 0.5);
            }
        }
        self.record.base.automata_flags = (self.record.base.automata_flags & !(REACHED | STOPPED))
            | if next.at_end { REACHED } else { 0 }
            | if next.stopped { STOPPED } else { 0 };
        self.record.base.vectors[0] = next.position;
        self.record.base.vectors[2] = next.facing;
        self.record.base.vectors[1] = if vehicle {
            sub(next.position, scale(next.facing, self.record.base.f1[0] * 0.5))
        } else {
            next.position
        };
        world.move_models(next.position, next.facing);
        Ok(if vehicle {
            Some(length(sub(next.position, old)) / dt_ms)
        } else {
            None
        })
    }

    pub fn straight_tick(&mut self, dt_ms: f32, speed_scale: f32, world: &mut impl TrafficWorld) {
        self.record.base.vectors[1][1] = 0.0;
        let from = self.record.base.vectors[0];
        let destination = self.record.base.vectors[1];
        let step = tween_value(&self.record.base.f3[..4]) * speed_scale * dt_ms;
        if length(sub(destination, from)) <= step {
            self.record.base.automata_flags |= REACHED;
            return; // No final-frame snap (automata §6.5).
        }
        let facing = direction_toward(from, destination);
        let next = add(from, scale(facing, step));
        if !world.collision_allows(from, next, self.record.base.automata_flags) {
            self.remove(world);
        } else {
            self.record.base.vectors[0] = next;
            self.record.base.vectors[2] = facing;
            world.move_models(next, facing);
        }
    }
}

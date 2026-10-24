// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::automata::Vec3;
use crate::stream::RefValue;

#[derive(Clone, Debug)]
pub struct LookAheadNode {
    pub network: RefValue,
    pub path: u32,
    pub terminal: bool,
}
impl PartialEq for LookAheadNode {
    fn eq(&self, other: &Self) -> bool {
        crate::automata::refs_equal(&self.network, &other.network)
            && self.path == other.path
            && self.terminal == other.terminal
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathRequest {
    pub start: [i32; 4],
    pub end: [i32; 4],
    pub travel_mask: u32,
    pub starting_mask: u32,
    pub density: u32,
    pub flags: u32,
    pub destination_filter: u32,
}

/// Foreign network/pathfinder services only. Acceptance, route consumption,
/// fallback timing, ranking and tie breaking belong to this core module.
pub trait RoutingWorld {
    fn city_size(&mut self) -> Option<(i32, i32)>;
    fn connections_available(&mut self) -> bool;
    fn finder_available(&mut self) -> bool;
    fn position_cell(&mut self, position: Vec3) -> [i32; 2];
    fn connections(&mut self, network_type: u32) -> Vec<[i32; 2]>;
    fn find_path(&mut self, request: PathRequest) -> (u32, usize);
    fn reset_finder(&mut self);
    fn network_cell(&mut self, cell: [i32; 2]) -> Option<RefValue>;
    fn next_tile(&mut self, previous: &LookAheadNode, mode: u32, selector: u8) -> Option<RefValue>;
    fn ordered_exits(&mut self, network: &RefValue, entry: u8, travel_type: u32) -> ([u8; 4], usize);
    fn accepts(&mut self, network: &RefValue, selector: u8, entry: u8, exit: u8) -> bool;
    fn network_list(&mut self, cell: [u8; 2]) -> Vec<RefValue>;
    fn network_flag_two(&mut self, network: &RefValue) -> bool;
    /// Slot 9: caller supplies the path-information service's generated word.
    /// Its native return status is deliberately not used by the owner.
    fn generate_path(&mut self, network: &RefValue, selector: u8, entry: u8, exit: u8) -> u32;
}

impl TrafficAutomaton {
    pub fn best_neighbor_connection(&self, world: &mut impl RoutingWorld) -> RefValue {
        let Some((width, height)) = world.city_size() else {
            return RefValue::Null;
        };
        if width <= 0 || height <= 0 || !world.connections_available() || !world.finder_available() {
            return RefValue::Null;
        }
        let [x, z] = world.position_cell(self.record.base.vectors[0]);
        let mut winner = None;
        let mut best_count = usize::MAX;
        for network_type in [3, 12, 8, 2, 10] {
            for [cx, cz] in world.connections(network_type) {
                let end = [cx.clamp(0, width - 1), cz.clamp(0, height - 1)];
                let (status, count) = world.find_path(PathRequest {
                    start: [x, z, x, z],
                    end: [end[0], end[1], end[0], end[1]],
                    travel_mask: 2,
                    starting_mask: 2,
                    density: 0,
                    flags: 1,
                    destination_filter: 3,
                });
                world.reset_finder();
                if status == 1 && (winner.is_none() || count < best_count) {
                    best_count = count;
                    winner = Some(end);
                }
            }
        }
        winner
            .and_then(|cell| world.network_cell(cell))
            .unwrap_or(RefValue::Null)
    }
}

impl TrafficOwner<'_> {
    pub fn slow_down(&mut self, modifier: f32) {
        let duration = if modifier > 0.0 { (1.0 / modifier) * 1.5 } else { 0.1 };
        tween_speed(&mut self.record.base, 0.0, duration);
    }

    pub fn expand_look_ahead(
        &mut self,
        previous: &LookAheadNode,
        output: &mut LookAheadNode,
        world: &mut impl RoutingWorld,
    ) -> bool {
        let selector = ((previous.path >> 24) & 15) as u8;
        let entry = ((previous.path as u8).wrapping_add(2)) % 4;
        let pack = |exit: u8| u32::from(selector) * 0x01000000 + u32::from(entry) * 0x100 + u32::from(exit);
        if self.record.base.automata_flags & SPECIAL_ROUTE != 0 {
            if self.record.route.is_empty() {
                return false;
            }
            let [x, z, _] = self.record.route.remove(0);
            if let Some(next) = self.record.route.first() {
                let exit = if next[1] < z {
                    1
                } else if next[1] > z {
                    3
                } else if next[0] < x {
                    0
                } else {
                    2
                };
                for network in world.network_list([x, z]) {
                    if !network.is_null() && world.accepts(&network, selector, entry, exit) {
                        output.network = network;
                        output.path = pack(exit);
                        output.terminal = false;
                        return true;
                    }
                }
                return false;
            }
            output.terminal = true;
            let Some(network) = world.next_tile(previous, 1, selector).filter(|n| !n.is_null()) else {
                return false;
            };
            output.network = network;
            for offset in 0..4 {
                let exit = (entry + 2 + offset) % 4;
                if world.accepts(&output.network, selector, entry, exit) {
                    output.path = pack(exit);
                    return true;
                }
            }
            return false;
        }
        let Some(network) = world.next_tile(previous, 0, selector).filter(|n| !n.is_null()) else {
            return false;
        };
        output.network = network;
        let (exits, eligible) = world.ordered_exits(&output.network, entry, self.runtime.travel_type);
        let fallback_allowed = self.runtime.fallback_byte == 0
            && ![7, 8].contains(&self.runtime.travel_type)
            && self.record.base.automata_flags & 0x200000 == 0
            && (self.record.network.is_null() || !world.network_flag_two(&self.record.network));
        for (index, exit) in exits.into_iter().enumerate() {
            if self.runtime.exclude_previous_entry && exit == (previous.path >> 8) as u8 {
                continue;
            }
            if world.accepts(&output.network, selector, entry, exit) {
                output.path = pack(exit);
                return true;
            }
            if index < eligible && fallback_allowed {
                output.path = world.generate_path(&output.network, selector, entry, 0xFF);
                output.terminal = true;
                return true;
            }
        }
        false
    }

    pub fn fill_look_ahead(&mut self, requested: usize, world: &mut impl RoutingWorld) -> bool {
        if self.runtime.look_ahead.is_empty() {
            self.runtime.look_ahead.push_back(LookAheadNode {
                network: self.record.network.clone(),
                path: self.runtime.path_word,
                terminal: false,
            });
        }
        while self.runtime.look_ahead.len().saturating_sub(1) < requested {
            let previous = self.runtime.look_ahead.back().expect("seeded").clone();
            if previous.terminal || previous.network.is_null() {
                return false;
            }
            let mut output = LookAheadNode {
                network: RefValue::Null,
                path: 0,
                terminal: false,
            };
            if !self.expand_look_ahead(&previous, &mut output, world) {
                self.runtime.look_ahead.back_mut().expect("seeded").terminal = true;
                return false;
            }
            self.runtime.look_ahead.push_back(output);
        }
        true
    }
}

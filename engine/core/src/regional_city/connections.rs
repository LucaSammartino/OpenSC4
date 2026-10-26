// SPDX-License-Identifier: GPL-3.0-or-later
use super::{ConnectionHandle, DealHandle, RegionalCity, CONNECTIONS_READY};
use crate::neighbors::{neighbouring_region_cell, side_of, NeighborConnection};
use crate::stream::{InStream, OutStream};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingChange {
    pub conn_type: u32,
    pub x: i32,
    pub z: i32,
    pub add: u8,
}
impl PendingChange {
    pub fn read(inp: &mut dyn InStream) -> Option<Self> {
        Some(Self {
            conn_type: inp.get_u32()?,
            x: inp.get_i32()?,
            z: inp.get_i32()?,
            add: inp.get_u8()?,
        })
    }
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u32(self.conn_type) && out.put_i32(self.x) && out.put_i32(self.z) && out.put_u8(self.add)
    }
    pub fn side(&self, count_x: i32) -> usize {
        if self.x < 0 {
            0
        } else if self.z < 0 {
            1
        } else if self.x < count_x {
            3
        } else {
            2
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreparedChange {
    pub change: PendingChange,
    pub pair: bool,
    pub direction: bool,
}

/// Calls on city services. Network occupant handles stay host-owned. The core
/// owns the algorithm and byte updates, while the host supplies interface calls.
/// RR3: the semantic demolition/segment calls follow the literal inferred spec;
/// undecoded native argument meanings are deliberately not invented here.
pub trait ConnectionHost<C = std::rc::Rc<NeighborConnection>> {
    fn drive_side(&self) -> bool;
    fn other_city_established(&mut self, connection: &C) -> bool;
    /// Adjacent cities including unestablished ones, in the service's order.
    fn adjacent_connections(&mut self) -> Vec<IncomingCity<C>>;
    fn points_to_this_city(&mut self, connection: &C) -> bool;
    fn network_exists(&mut self, network: u32) -> bool;
    fn find_connection(&mut self, change: PendingChange) -> bool;
    fn create_connection(&mut self, change: PendingChange, direction: bool);
    fn delete_connection(&mut self, change: PendingChange);
    fn traffic_present(&mut self, cell: (i32, i32)) -> bool;
    fn power_present(&mut self, cell: (i32, i32)) -> bool;
    fn demolish(&mut self, cells: &[(i32, i32)], layer: u32);
    fn dezone(&mut self, cells: &[(i32, i32)]);
    fn place_segment(&mut self, network: u32, first: (i32, i32), last: (i32, i32));
    fn pole_present(&mut self, cell: (i32, i32)) -> bool;
    fn occupants(&mut self, network: u32, cell: (i32, i32)) -> Vec<u64>;
    fn connection_word(&mut self, occupant: u64) -> u32;
    fn set_connection_word(&mut self, occupant: u64, word: u32);
    /// Queue a new standard message at normal priority with all data/void/object
    /// fields zero (foundations §11.6); release the created reference afterwards.
    fn post(&mut self, message: u32);
}

pub struct IncomingCity<C = std::rc::Rc<NeighborConnection>> {
    pub position: (i32, i32),
    pub size: (i32, i32),
    pub connections: Vec<C>,
}

impl<C: ConnectionHandle, D: DealHandle> RegionalCity<C, D> {
    /// Converts via world connection cells; edge ring coordinates are retained.
    pub fn change_symmetric_connection(
        &mut self,
        c: &C,
        source_position: (i32, i32),
        source_size: (i32, i32),
        add: u8,
    ) {
        let (x, z) = c.cell();
        let side = side_of(x, z, source_size.0 * 64, source_size.1 * 64);
        let region = neighbouring_region_cell(side, x, z, source_position, source_size);
        use crate::neighbors::Side;
        let lateral_x = x % 64 + (region.0 - self.position.0) * 64;
        let lateral_z = z % 64 + (region.1 - self.position.1) * 64;
        let (x, z) = match side {
            Side::West => (self.size.0 * 64, lateral_z),
            Side::East => (-1, lateral_z),
            Side::North => (lateral_x, self.size.1 * 64),
            Side::South => (lateral_x, -1),
            _ => (0, 0),
        };
        self.pending.push(PendingChange {
            conn_type: c.conn_type(),
            x,
            z,
            add,
        });
    }

    pub fn mark_connections_for_removal(&mut self, host: &mut impl ConnectionHost<C>) {
        for c in &self.connections {
            if host.other_city_established(c) {
                let (x, z) = c.cell();
                self.pending.push(PendingChange {
                    conn_type: c.conn_type(),
                    x,
                    z,
                    add: 0,
                });
            }
        }
    }
    pub fn treat_incoming_as_new(&mut self, host: &mut impl ConnectionHost<C>) {
        for city in host.adjacent_connections() {
            for c in city.connections {
                if host.points_to_this_city(&c) {
                    self.change_symmetric_connection(&c, city.position, city.size, 1);
                }
            }
        }
    }

    pub fn prepare_changes(&self, drive_side: bool) -> Vec<PreparedChange> {
        let mut seen = BTreeSet::new();
        let mut entries: Vec<_> = self
            .pending
            .iter()
            .rev()
            .copied()
            .filter(|p| seen.insert((p.conn_type, p.x, p.z)))
            .collect();
        entries.reverse();
        let (cx, cz) = self.cell_counts();
        entries.sort_unstable_by_key(|p| (p.side(cx), p.x, p.z));
        let mut second = false;
        entries
            .into_iter()
            .map(|p| {
                let wide = p.add != 0 && matches!(p.conn_type, 3 | 8 | 12);
                let pair = !wide || second;
                let direction = wide && (pair ^ (p.x >= 0 && p.z < cz) ^ !drive_side);
                if wide {
                    second = !second;
                }
                PreparedChange {
                    change: p,
                    pair,
                    direction,
                }
            })
            .collect()
    }

    pub fn reconcile_connections(&mut self, established: bool, forced: bool, host: &mut impl ConnectionHost<C>) {
        if !established || forced || self.moved {
            self.pending.clear();
            if self.moved {
                self.mark_connections_for_removal(host);
            }
            self.treat_incoming_as_new(host);
        }
        self.create_symmetric_connections(host);
        self.moved = false;
    }

    pub fn create_symmetric_connections(&mut self, host: &mut impl ConnectionHost<C>) {
        for p in self.prepare_changes(host.drive_side()) {
            let c = p.change;
            let Some(network) = network_type(c.conn_type) else {
                host.create_connection(c, p.direction);
                continue;
            };
            if !host.network_exists(network) {
                continue;
            }
            let exists = host.find_connection(c);
            let side = c.side(self.cell_counts().0);
            let (dx, dz) = INWARD[side];
            let inner = (c.x + dx, c.z + dz);
            let last = (inner.0 + 2 * dx, inner.1 + 2 * dz);
            if c.add != 0 {
                if exists {
                    continue;
                }
                if !p.pair {
                    host.create_connection(c, p.direction);
                    continue;
                }
                let strip = [inner, (inner.0 + dx, inner.1 + dz), last];
                let cells: Vec<_> = strip
                    .into_iter()
                    .filter(|&cell| {
                        if c.conn_type == 4 {
                            !host.power_present(cell)
                        } else {
                            !host.traffic_present(cell)
                        }
                    })
                    .collect();
                let layer = match c.conn_type {
                    5 => 3,
                    6 => 2,
                    _ => 1,
                };
                host.demolish(&cells, layer);
                if c.conn_type == 4 {
                    host.dezone(&cells);
                }
                host.create_connection(c, p.direction);
                host.place_segment(network, inner, last);
                let found = if c.conn_type == 4 {
                    host.pole_present(inner)
                } else {
                    !host.occupants(network, inner).is_empty()
                };
                if !found {
                    host.delete_connection(c);
                }
                update_edges(host, network, inner, side, 2);
                host.place_segment(network, inner, last);
            } else if exists {
                host.delete_connection(c);
                update_edges(host, network, inner, side, 0);
                host.place_segment(network, inner, last);
            }
        }
        self.pending.clear();
        host.post(CONNECTIONS_READY);
    }
}
const INWARD: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
fn update_edges<C>(host: &mut impl ConnectionHost<C>, network: u32, inner: (i32, i32), side: usize, value: u32) {
    for o in host.occupants(network, inner) {
        let word = host.connection_word(o);
        host.set_connection_word(o, byte(word, side, value));
        if let Some(link) = (0..4).find(|&s| (word >> (8 * s)) & 0xFF == 4) {
            let (dx, dz) = INWARD[link];
            for other in host.occupants(network, (inner.0 - dx, inner.1 - dz)) {
                let w = host.connection_word(other);
                host.set_connection_word(other, byte(w, link, value));
            }
        }
    }
}
fn byte(word: u32, side: usize, value: u32) -> u32 {
    (word & !(0xFF << (8 * side))) | (value << (8 * side))
}
pub fn network_type(t: u32) -> Option<u32> {
    match t {
        1 => Some(1),
        2 => Some(0),
        3 => Some(2),
        4 => Some(5),
        5 => Some(4),
        6 => Some(7),
        7 => Some(8),
        8 => Some(6),
        10 => Some(11),
        11 => Some(9),
        12 => Some(12),
        _ => None,
    }
}

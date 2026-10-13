// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The neighbour connection (§4): a connection cell one cell outside the
//! city, its type, the region cell on the other side, two flag bits and a
//! property holder.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::{connection_inner_cell, neighbouring_region_cell, side_of, CLSID_NEIGHBOR_CONNECTION};
use crate::property::PropertyHolder;
use crate::stream::{InStream, OutStream, SaveObject};

/// The record version (§4.3).
pub const CONNECTION_RECORD_VERSION: u16 = 1;

const VISITED: u8 = 0x01;
const INACTIVE: u8 = 0x02;

/// `cISC4NeighborConnection` (§4).
#[derive(Debug)]
pub struct NeighborConnection {
    conn_type: Cell<u32>,
    cell: Cell<(i32, i32)>,
    other_region_cell: Cell<(i32, i32)>,
    /// Bit 0 visitedThisCycle, bit 1 inactive (§4.1).
    flags: Cell<u8>,
    props: RefCell<PropertyHolder>,
    /// Not saved (§4.1); the traffic spec owns the sync itself.
    needs_traffic_sync: Cell<bool>,
    token: Cell<u64>,
}

impl Default for NeighborConnection {
    fn default() -> NeighborConnection {
        NeighborConnection {
            conn_type: Cell::new(0),
            cell: Cell::new((0, 0)),
            other_region_cell: Cell::new((0, 0)),
            flags: Cell::new(0),
            props: RefCell::new(PropertyHolder::new()),
            needs_traffic_sync: Cell::new(true),
            token: Cell::new(0),
        }
    }
}

impl NeighborConnection {
    /// A connection in its initial state (§4.1).
    pub fn new() -> Rc<NeighborConnection> {
        Rc::new(NeighborConnection::default())
    }

    /// A connection with type and cell set (test and load helper).
    pub fn with(conn_type: u32, x: i32, z: i32) -> Rc<NeighborConnection> {
        let c = NeighborConnection::new();
        c.set_type(conn_type);
        c.set_cell(x, z);
        c
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    /// `GetType()`.
    pub fn conn_type(&self) -> u32 {
        self.conn_type.get()
    }

    pub fn set_type(&self, t: u32) {
        self.conn_type.set(t);
    }

    /// `GetConnectionCell`.
    pub fn cell(&self) -> (i32, i32) {
        self.cell.get()
    }

    pub fn set_cell(&self, x: i32, z: i32) {
        self.cell.set((x, z));
    }

    /// `GetRegionCellOnOtherSideOfConnection`.
    pub fn other_region_cell(&self) -> (i32, i32) {
        self.other_region_cell.get()
    }

    pub fn set_other_region_cell(&self, x: i32, z: i32) {
        self.other_region_cell.set((x, z));
    }

    /// `GetConnectionInnerCell` (§4.2) for a city of `counts` cells.
    pub fn inner_cell(&self, counts: (i32, i32)) -> Option<(i32, i32)> {
        let (x, z) = self.cell();
        connection_inner_cell(x, z, counts.0, counts.1)
    }

    /// `Repair(R)` (§4.2): recompute the other-side region cell from the cell
    /// with the regional city at `pos` of size `size` (§2.2).
    pub fn repair(&self, counts: (i32, i32), pos: (i32, i32), size: (i32, i32)) {
        let (x, z) = self.cell();
        let side = side_of(x, z, counts.0, counts.1);
        let (rx, rz) = neighbouring_region_cell(side, x, z, pos, size);
        self.set_other_region_cell(rx, rz);
    }

    pub fn visited_this_cycle(&self) -> bool {
        self.flags.get() & VISITED != 0
    }

    pub fn set_visited_this_cycle(&self, on: bool) {
        let f = self.flags.get();
        self.flags.set(if on { f | VISITED } else { f & !VISITED });
    }

    /// `GetIsActive`: bit 1 clear (§4.2).
    pub fn is_active(&self) -> bool {
        self.flags.get() & INACTIVE == 0
    }

    /// `SetIsActive(true)` clears bit 1, `(false)` sets it.
    pub fn set_active(&self, on: bool) {
        let f = self.flags.get();
        self.flags.set(if on { f & !INACTIVE } else { f | INACTIVE });
    }

    /// The saved flags byte.
    pub fn flags(&self) -> u8 {
        self.flags.get()
    }

    pub fn needs_traffic_sync(&self) -> bool {
        self.needs_traffic_sync.get()
    }

    pub fn set_needs_traffic_sync(&self, on: bool) {
        self.needs_traffic_sync.set(on);
    }

    /// `AsISCPropertyHolder()`.
    pub fn properties(&self) -> PropertyHolder {
        self.props.borrow().clone()
    }

    pub fn set_properties(&self, h: PropertyHolder) {
        *self.props.borrow_mut() = h;
    }

    /// §4.3.
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u16(CONNECTION_RECORD_VERSION);
        out.put_u32(self.conn_type());
        let (x, z) = self.cell();
        out.put_i32(x);
        out.put_i32(z);
        let (rx, rz) = self.other_region_cell();
        out.put_i32(rx);
        out.put_i32(rz);
        out.put_u8(self.flags.get());
        let props = self.props.borrow().clone();
        props.write(out) && out.error() == 0
    }

    /// §4.3: fails unless `version == 1`.
    pub fn read(&self, inp: &mut dyn InStream) -> bool {
        if inp.get_u16() != Some(CONNECTION_RECORD_VERSION) {
            return false;
        }
        let t = inp.get_u32().unwrap_or(0);
        let x = inp.get_i32().unwrap_or(0);
        let z = inp.get_i32().unwrap_or(0);
        let rx = inp.get_i32().unwrap_or(0);
        let rz = inp.get_i32().unwrap_or(0);
        let flags = inp.get_u8().unwrap_or(0);
        let mut props = PropertyHolder::new();
        let ok = props.read(inp);
        self.conn_type.set(t);
        self.cell.set((x, z));
        self.other_region_cell.set((rx, rz));
        self.flags.set(flags);
        *self.props.borrow_mut() = props;
        ok && inp.error() == 0
    }
}

impl SaveObject for NeighborConnection {
    fn class_id(&self) -> u32 {
        CLSID_NEIGHBOR_CONNECTION
    }

    fn write(&self, out: &mut dyn OutStream) -> bool {
        NeighborConnection::write(self, out)
    }

    fn read(&self, inp: &mut dyn InStream) -> bool {
        NeighborConnection::read(self, inp)
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn std::any::Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
//! The city owns cloned handles, never a copy of a live neighbour object.
//! Host handles implement Clone/Drop with their interface's AddRef/Release.
use crate::neighbors::{NeighborConnection, NeighborDeal};
use std::rc::Rc;

pub trait ConnectionHandle: Clone {
    fn conn_type(&self) -> u32;
    fn cell(&self) -> (i32, i32);
    fn set_cell(&self, x: i32, z: i32);
    fn repair(&self, counts: (i32, i32), position: (i32, i32), size: (i32, i32));
}

impl ConnectionHandle for Rc<NeighborConnection> {
    fn conn_type(&self) -> u32 {
        self.as_ref().conn_type()
    }
    fn cell(&self) -> (i32, i32) {
        self.as_ref().cell()
    }
    fn set_cell(&self, x: i32, z: i32) {
        self.as_ref().set_cell(x, z);
    }
    fn repair(&self, counts: (i32, i32), position: (i32, i32), size: (i32, i32)) {
        self.as_ref().repair(counts, position, size);
    }
}

pub trait DealHandle: Clone {
    fn city_side(&self) -> (i32, i32);
    fn other_side(&self) -> (i32, i32);
    fn set_city_side(&self, x: i32, z: i32);
    fn set_other_side(&self, x: i32, z: i32);
    fn deal_type(&self) -> u32;
    fn amount(&self) -> i32;
    fn flags(&self) -> u32;
    fn set_flags(&self, flags: u32, on: bool) -> bool;
}

impl DealHandle for Rc<NeighborDeal> {
    fn city_side(&self) -> (i32, i32) {
        self.as_ref().city_side()
    }
    fn other_side(&self) -> (i32, i32) {
        self.as_ref().other_side()
    }
    fn set_city_side(&self, x: i32, z: i32) {
        self.as_ref().set_city_side(x, z);
    }
    fn set_other_side(&self, x: i32, z: i32) {
        self.as_ref().set_other_side(x, z);
    }
    fn deal_type(&self) -> u32 {
        self.as_ref().deal_type()
    }
    fn amount(&self) -> i32 {
        self.as_ref().amount()
    }
    fn flags(&self) -> u32 {
        self.as_ref().flags()
    }
    fn set_flags(&self, flags: u32, on: bool) -> bool {
        NeighborDeal::set_flags(self, flags, on)
    }
}

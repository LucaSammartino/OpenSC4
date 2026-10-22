// SPDX-License-Identifier: GPL-3.0-or-later
//! Typed external primitives. Tool/connection algorithms live in the portable core.

use super::{CurveParams, Point, PoleData, Reference};

pub trait Terrain {
    fn cell_width(&mut self) -> f32;
    fn special_height(&mut self, cell: (i32, i32)) -> bool;
    fn height(&mut self, x: f32, z: f32) -> f32;
    fn corners(&mut self, cell: (i32, i32)) -> [f32; 4];
    fn intersect(&mut self, origin: Point, direction: Point, arg0: u32, arg1: u32) -> Option<Point>;
}

/// Reference tokens have no Drop/AddRef behaviour: structural copies are raw pointer copies (§6.5).
pub trait World: Terrain {
    fn add_ref(&mut self, reference: Reference);
    fn release(&mut self, reference: Reference);
    fn query_cell(&mut self, cell: (i32, i32)) -> Vec<Reference>;
    fn class(&mut self, occupant: Reference) -> u32;
    fn query_interface(&mut self, occupant: Reference, iid: u32) -> Option<Reference>;
    fn occupant_position(&mut self, occupant: Reference) -> Point;
    fn world_to_cell(&mut self, position: Point) -> (i32, i32);
    fn zone(&mut self, cell: (i32, i32)) -> i32;
    fn blocked(&mut self, cell: (i32, i32), arg: u32) -> bool;
    fn wire_endpoints(&mut self, wire: Reference) -> [Option<Reference>; 2];
    fn wire_cell(&mut self, wire: Reference) -> (i32, i32);
    fn pole_data(&self, pole: Reference) -> &PoleData;
    fn pole_data_mut(&mut self, pole: Reference) -> &mut PoleData;
    fn create_pole(&mut self, cell: (i32, i32), mask: u8, position: Point) -> Reference;
    fn create_wire(&mut self, cell: (i32, i32), endpoints: [Reference; 2], pivot: bool, position: Point, real: bool) -> Reference;
    fn insert_pole(&mut self, pole: Reference, arg: u32);
    fn has_occupant_manager(&self) -> bool;
    fn remove_pole(&mut self, pole: Reference, arg0: bool, arg1: bool);
    fn highlight_pole(&mut self, pole: Reference, mode: u32, arg: u32);
    fn reset_pole_preview(&mut self, pole: Reference, arg: u32);
    fn refresh_pole(&mut self, pole: Reference);
    /// Genuine external pole-occupant primitives, not tool algorithm callbacks.
    fn remove_connection(&mut self, pole: Reference, other: Reference, arg: u32);
    fn connection_cells(&mut self, pole: Reference, other: Reference) -> Vec<(i32, i32)>;
    fn clear_connections(&mut self, pole: Reference, arg: bool);
    fn draw_line(&mut self, run: &mut super::ToolRun, anchors: [(i32, i32); 2], snap: u32) -> Vec<(usize, usize)>;
    fn select(&mut self, cell: (i32, i32), color: u32, args: [u32; 3]);
    fn extra_geometry(&mut self, cell: (i32, i32)) -> bool;
    fn notify_pole(&mut self, pole: Reference, message: u32, arg: u32);
    fn curve_params(&self) -> CurveParams;
    fn base_cleanup(&mut self);
}

pub const POLE_IID: u32 = 0xA8C8_6142;
pub const WIRE_IID: u32 = 0xC98F_3B24;

pub fn classify(w: &mut impl World, cell: (i32, i32)) -> u32 {
    if w.zone(cell) != 0 && w.zone(cell) != 15 && w.zone((cell.1, cell.1)) != 14 {
        return 1;
    }
    let mut result = 0;
    for occupant in w.query_cell(cell) {
        match w.class(occupant) {
            0x2781_28A0 => result = 2,
            0xC772_BF98 => {
                if let Some(qi) = w.query_interface(occupant, 0x4A10_AFC2) {
                    w.release(qi);
                } else {
                    result = 3;
                }
            }
            _ => {}
        }
        w.release(occupant);
    }
    if result == 0 && w.blocked(cell, 0) {
        result = 2;
    }
    result
}

pub fn probe(w: &mut impl World, cell: (i32, i32), pole: bool) -> Option<Reference> {
    let mut selected = None;
    for occupant in w.query_cell(cell) {
        if let Some(qi) = w.query_interface(occupant, if pole { POLE_IID } else { WIRE_IID }) {
            let matches = if pole {
                let position = w.occupant_position(occupant);
                w.world_to_cell(position) == cell
            } else {
                true
            };
            if matches {
                if let Some(old) = selected.replace(qi) {
                    w.release(old);
                }
            } else {
                w.release(qi);
            }
        }
        w.release(occupant);
    }
    selected
}

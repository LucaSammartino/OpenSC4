// SPDX-License-Identifier: GPL-3.0-or-later
//! Read-only pole inventory selection. This does not instantiate pole models,
//! wire occupants or curves. Nonempty occupant queries are explicitly refused.

use std::ffi::c_void;

use fc4_core::network_tools::power_line::*;
use fc4_core::network_tools::ToolRun;

use crate::network_geometry::{fc4_netgeom_cell, fc4_netgeom_step};
use crate::{guard, slice_arg};

#[repr(C)]
pub struct fc4_powerline_selection_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    // All callbacks are mandatory. cell returns false for unavailable inputs
    // or any existing occupant: reuse/repair is outside this bounded adapter.
    pub cell: Option<unsafe extern "C" fn(*mut c_void, i32, i32, *mut i32, *mut bool) -> bool>,
    pub width: Option<unsafe extern "C" fn(*mut c_void) -> f32>,
    pub height: Option<unsafe extern "C" fn(*mut c_void, f32, f32) -> f32>,
    pub special: Option<unsafe extern "C" fn(*mut c_void, i32, i32) -> bool>,
    pub corners: Option<unsafe extern "C" fn(*mut c_void, i32, i32, *mut f32)>,
    pub ray: Option<unsafe extern "C" fn(*mut c_void, *const f32, *const f32, *mut f32) -> bool>,
}

#[repr(C)]
#[derive(Default)]
pub struct fc4_powerline_selection {
    pub valid: u32,
    pub failure: u32,
    pub poles: u32,
    pub wires: u32,
    pub cost: i32,
}

#[repr(C)]
pub struct fc4_powerline_selected_pole {
    pub x: i32,
    pub z: i32,
    pub mask: u32,
}

struct SelectionWorld<'a> {
    host: &'a fc4_powerline_selection_host,
    complete: bool,
    dummy: PoleData,
}

impl Terrain for SelectionWorld<'_> {
    fn cell_width(&mut self) -> f32 {
        unsafe { self.host.width.unwrap()(self.host.user) }
    }
    fn special_height(&mut self, c: (i32, i32)) -> bool {
        unsafe { self.host.special.unwrap()(self.host.user, c.0, c.1) }
    }
    fn height(&mut self, x: f32, z: f32) -> f32 {
        unsafe { self.host.height.unwrap()(self.host.user, x, z) }
    }
    fn corners(&mut self, c: (i32, i32)) -> [f32; 4] {
        let mut out = [0.0; 4];
        unsafe { self.host.corners.unwrap()(self.host.user, c.0, c.1, out.as_mut_ptr()) };
        out
    }
    fn intersect(&mut self, a: Point, b: Point, _: u32, _: u32) -> Option<Point> {
        let mut out = [0.0; 3];
        unsafe { self.host.ray.unwrap()(self.host.user, a.as_ptr(), b.as_ptr(), out.as_mut_ptr()) }.then_some(out)
    }
}

// These operations belong to placement/repair, which this adapter never calls.
// Mark an unexpected call incomplete instead of treating it as successful.
macro_rules! unsupported {
    ($($name:ident($($arg:ident: $ty:ty),*) $(-> $ret:ty)? $body:block)*) => {$(
        fn $name(&mut self, $($arg: $ty),*) $(-> $ret)? { self.complete = false; $body }
    )*};
}

impl World for SelectionWorld<'_> {
    fn query_cell(&mut self, c: (i32, i32)) -> Vec<Reference> {
        let (mut zone, mut blocked) = (0, false);
        self.complete &= unsafe { self.host.cell.unwrap()(self.host.user, c.0, c.1, &mut zone, &mut blocked) };
        Vec::new()
    }
    fn zone(&mut self, c: (i32, i32)) -> i32 {
        let (mut zone, mut blocked) = (0, false);
        self.complete &= unsafe { self.host.cell.unwrap()(self.host.user, c.0, c.1, &mut zone, &mut blocked) };
        zone
    }
    fn blocked(&mut self, c: (i32, i32), _: u32) -> bool {
        let (mut zone, mut blocked) = (0, false);
        self.complete &= unsafe { self.host.cell.unwrap()(self.host.user, c.0, c.1, &mut zone, &mut blocked) };
        blocked
    }
    fn pole_data(&self, _: Reference) -> &PoleData {
        &self.dummy
    }
    fn pole_data_mut(&mut self, _: Reference) -> &mut PoleData {
        self.complete = false;
        &mut self.dummy
    }
    fn has_occupant_manager(&self) -> bool {
        false
    }
    fn curve_params(&self) -> CurveParams {
        CurveParams::default()
    }
    unsupported! {
        add_ref(_r: Reference) {}
        release(_r: Reference) {}
        class(_r: Reference) -> u32 { 0 }
        query_interface(_r: Reference, _iid: u32) -> Option<Reference> { None }
        occupant_position(_r: Reference) -> Point { [0.0; 3] }
        world_to_cell(_p: Point) -> (i32, i32) { (0, 0) }
        wire_endpoints(_r: Reference) -> [Option<Reference>; 2] { [None; 2] }
        wire_cell(_r: Reference) -> (i32, i32) { (0, 0) }
        create_pole(_c: (i32, i32), _mask: u8, _p: Point) -> Reference { Reference(0) }
        allocate_wire() -> Reference { Reference(0) }
        init_wire(_r: Reference) {}
        set_wire_mode(_r: Reference, _real: bool) {}
        bind_wire(_r: Reference, _c: (i32, i32), _e: [Reference; 2], _pivot: bool) {}
        set_wire_position(_r: Reference, _p: Point) {}
        insert_pole(_r: Reference, _arg: u32) {}
        remove_pole(_r: Reference, _a: bool, _b: bool) {}
        highlight_pole(_r: Reference, _m: u32, _a: u32) {}
        reset_pole_preview(_r: Reference, _a: u32) {}
        refresh_pole(_r: Reference) {}
        remove_connection(_r: Reference, _other: Reference, _a: u32) {}
        connection_cells(_r: Reference, _other: Reference) -> Vec<(i32, i32)> { Vec::new() }
        connection_cells_at(_r: Reference, _i: usize) -> Vec<(i32, i32)> { Vec::new() }
        clear_connections(_r: Reference, _a: bool) {}
        draw_line(_r: &mut ToolRun, _a: [(i32, i32); 2], _s: u32) -> Vec<(usize, usize)> { Vec::new() }
        select(_c: (i32, i32), _color: u32, _a: [u32; 3]) {}
        extra_geometry(_c: (i32, i32)) -> bool { false }
        notify_pole(_r: Reference, _message: u32, _arg: u32) {}
        base_cleanup() {}
    }
}

/// NT31 selection/count/retained-price scope only. All arrays remain caller owned.
/// Returns false for incomplete observations, invalid ranges or insufficient output.
#[no_mangle]
pub unsafe extern "C" fn fc4_powerline_select(
    host: *const fc4_powerline_selection_host,
    cells: *const fc4_netgeom_cell,
    cell_count: u32,
    steps: *const fc4_netgeom_step,
    step_count: u32,
    prefix: u32,
    spacing: u32,
    clearance: f32,
    pole_cost: u32,
    wire_cost: f32,
    poles: *mut fc4_powerline_selected_pole,
    capacity: u32,
    out: *mut fc4_powerline_selection,
) -> bool {
    guard(false, || {
        let (Some(h), Some(cs), Some(ss), Some(out)) = (
            host.as_ref(),
            slice_arg(cells, cell_count as usize),
            slice_arg(steps, step_count as usize),
            out.as_mut(),
        ) else {
            return false;
        };
        *out = fc4_powerline_selection::default();
        if h.struct_size as usize != std::mem::size_of_val(h)
            || h.cell.is_none()
            || h.width.is_none()
            || h.height.is_none()
            || h.special.is_none()
            || h.corners.is_none()
            || h.ray.is_none()
            || cs.is_empty()
            || ss.is_empty()
            || prefix > step_count
            || ss
                .iter()
                .any(|s| s.first > s.last || s.last >= cell_count || s.kind != 0)
        {
            return false;
        }
        let mut run = ToolRun {
            type_id: 5,
            start: (cs[0].x, cs[0].z),
            end: (cs[cs.len() - 1].x, cs[cs.len() - 1].z),
            cells: cs.iter().map(|c| (c.x, c.z)).collect(),
            valid_count: step_count,
            dragged_count: step_count,
            straight_count: prefix,
            ..ToolRun::default()
        };
        let mut tool = PowerLineTool {
            anchors: vec![run.start, run.end],
            steps: ss.iter().map(|s| (s.first as usize, s.last as usize)).collect(),
            params: PowerLineParams {
                v_3a4: spacing,
                v_3a8: clearance,
                cost_per_pole: pole_cost as i32,
                cost_per_wire: wire_cost,
            },
            ..PowerLineTool::default()
        };
        let mut world = SelectionWorld {
            host: h,
            complete: true,
            dummy: PoleData {
                cell: (0, 0),
                position: [0.0; 3],
                divisors: [1.0; 2],
                attachments: [[[0.0; 3]; 4]; 4],
                connections: Vec::new(),
                dirty: false,
            },
        };
        determine_pole_positions(&mut run, &mut tool, &mut world, &mut out.failure);
        if !world.complete || tool.poles.len() > capacity as usize || (!tool.poles.is_empty() && poles.is_null()) {
            return false;
        }
        for (i, p) in tool.poles.iter().enumerate() {
            poles.add(i).write(fc4_powerline_selected_pole {
                x: p.x,
                z: p.z,
                mask: u32::from(p.mask),
            });
        }
        out.valid = run.valid_count;
        out.poles = tool.counted_poles;
        out.wires = tool.wire_cells;
        out.cost = tool.cost_of_solution();
        true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn cell(u: *mut c_void, _: i32, _: i32, zone: *mut i32, blocked: *mut bool) -> bool {
        zone.write(0);
        blocked.write(false);
        u.is_null()
    }
    unsafe extern "C" fn width(_: *mut c_void) -> f32 {
        16.0
    }
    unsafe extern "C" fn height(_: *mut c_void, _: f32, _: f32) -> f32 {
        100.0
    }
    unsafe extern "C" fn special(_: *mut c_void, _: i32, _: i32) -> bool {
        false
    }
    unsafe extern "C" fn corners(_: *mut c_void, _: i32, _: i32, out: *mut f32) {
        for i in 0..4 {
            out.add(i).write(100.0);
        }
    }
    unsafe extern "C" fn ray(_: *mut c_void, _: *const f32, _: *const f32, _: *mut f32) -> bool {
        false
    }

    #[test]
    fn nt31_inventory_uses_explicit_inputs_and_refuses_incomplete_hosts() {
        let mut host = fc4_powerline_selection_host {
            struct_size: std::mem::size_of::<fc4_powerline_selection_host>() as u32,
            user: std::ptr::null_mut(),
            cell: Some(cell),
            width: Some(width),
            height: Some(height),
            special: Some(special),
            corners: Some(corners),
            ray: Some(ray),
        };
        let cells: Vec<_> = (4..=16).map(|x| fc4_netgeom_cell { x, z: 32 }).collect();
        let mut steps: Vec<_> = (0..13)
            .map(|i| fc4_netgeom_step {
                first: i,
                last: i,
                kind: 0,
            })
            .collect();
        let mut poles: Vec<_> = (0..13)
            .map(|_| fc4_powerline_selected_pole { x: 0, z: 0, mask: 0 })
            .collect();
        let mut out = fc4_powerline_selection::default();
        let invoke = |h: &fc4_powerline_selection_host,
                      s: &[fc4_netgeom_step],
                      p: &mut [fc4_powerline_selected_pole],
                      o: &mut fc4_powerline_selection,
                      spacing| unsafe {
            fc4_powerline_select(
                h,
                cells.as_ptr(),
                13,
                s.as_ptr(),
                13,
                13,
                spacing,
                7.0,
                15,
                2.0,
                p.as_mut_ptr(),
                p.len() as u32,
                o,
            )
        };
        assert!(invoke(&host, &steps, &mut poles, &mut out, 10));
        assert_eq!(
            (out.valid, out.failure, out.poles, out.wires, out.cost),
            (13, 0, 3, 10, 65)
        );
        assert_eq!(
            poles[..3].iter().map(|p| (p.x, p.z)).collect::<Vec<_>>(),
            vec![(4, 32), (14, 32), (16, 32)]
        );
        assert!(poles[..3].iter().all(|p| p.mask != 0));
        assert!(invoke(&host, &steps, &mut poles, &mut out, 20));
        assert_eq!((out.poles, out.wires, out.cost), (2, 11, 52));
        host.ray = None;
        assert!(!invoke(&host, &steps, &mut poles, &mut out, 10));
        host.ray = Some(ray);
        host.user = std::ptr::NonNull::<u8>::dangling().as_ptr().cast();
        assert!(!invoke(&host, &steps, &mut poles, &mut out, 10));
        host.user = std::ptr::null_mut();
        steps[0].last = 13;
        assert!(!invoke(&host, &steps, &mut poles, &mut out, 10));
    }
}

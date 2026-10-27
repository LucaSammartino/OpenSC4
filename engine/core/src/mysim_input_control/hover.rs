// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

/// These are raw public-interface reads. The unresolved names in MST1/MST2
/// are never translated into guessed semantic methods by a default adapter.
pub trait HoverServices: BuildingServices {
    fn pick_occupant(&mut self, x: i32, z: i32) -> Option<(Handle, u32)>;
    fn building_interface(&mut self, occupant: Handle) -> Option<Handle>;
    fn building_slot9(&mut self, _building: Handle) -> Result<f32> {
        // SPEC-GAP: open MST1. A host may supply the published raw slot result.
        Err(Unsupported::BuildingSlotsMst1)
    }
    fn building_slot11(&mut self, _building: Handle) -> Result<u32> {
        Err(Unsupported::BuildingSlotsMst1)
    }
    fn lot_slot41(&mut self, _lot: Handle) -> Result<u32> {
        // SPEC-GAP: open MST2; do not substitute a semantic wealth getter.
        Err(Unsupported::LotSlotsMst2)
    }
    fn lot_slot33(&mut self, _lot: Handle) -> Result<u32> {
        Err(Unsupported::LotSlotsMst2)
    }
    fn lot_slot35(&mut self, _lot: Handle) -> Result<u32> {
        Err(Unsupported::LotSlotsMst2)
    }
    fn lot_slot63(&mut self, _lot: Handle, _wealth: u32) -> Result<i32> {
        Err(Unsupported::LotSlotsMst2)
    }
    fn my_sim_lives_here(&mut self, agent: Handle, building: Handle) -> Option<Handle>;
    fn test_residence(&mut self, sim: SimView, building: Handle, a: bool, b: bool) -> bool;
    fn change_residence(&mut self, sim: SimView, building: Handle) -> bool;
    fn change_job(&mut self, sim: SimView, building: Handle) -> bool;
    fn set_residence(&mut self, sim: SimView, building: Handle);
    fn lot_by_occupant(&mut self, building: Handle) -> Option<Handle>;
    fn lot_by_position(&mut self, position: [f32; 3]) -> Option<Handle>;
    fn building_position(&mut self, building: Handle) -> [f32; 3];
    fn building_name(&mut self, building: Handle) -> String;
    /// None when the OccupantGroups property fails QI/type 0x8006.
    fn occupant_groups(&mut self, building: Handle) -> Option<Vec<u32>>;
    /// Created reference, with its window registered in the shared base tree.
    fn create_callout(&mut self, windows: &mut Windows) -> Option<Window>;
    fn callout_font(&mut self, callout: Window, slot: u32, font: u32);
    fn callout_icon(&mut self, callout: Window, icon: u32);
    fn callout_skin(&mut self, callout: Window, key: Key, padding: (i32, i32));
    fn callout_offset(&mut self, callout: Window, offset: Rect);
    fn callout_meter(&mut self, callout: Window, key: Key, count: u32, fill: f32);
    fn callout_title(&mut self, callout: Window, title: &str);
    fn callout_body(&mut self, callout: Window, body: &str);
    fn callout_point(&mut self, callout: Window, point: (i32, i32));
    fn position_to_cell(&mut self, position: [f32; 3]) -> (i32, i32);
    fn cell_eq(&mut self, cell: (i32, i32), wealth: u32) -> f32;
    fn cell_hq(&mut self, cell: (i32, i32), wealth: u32) -> f32;
    fn string(&mut self, table: u32, instance: u32) -> String;
    fn job_title(&mut self, sim: SimView, building: Handle) -> String;
}

pub fn eq_text(value: f32) -> u32 {
    band(value, [0xea5b_8c93, 0xea5b_8c94, 0xea5b_8c95])
}
pub fn hq_text(value: f32) -> u32 {
    band(value, [0xea5b_8c90, 0xea5b_8c91, 0xea5b_8c92])
}
fn band(value: f32, ids: [u32; 3]) -> u32 {
    if value < 50.0 {
        ids[0]
    } else if value < 100.0 {
        ids[1]
    } else {
        ids[2]
    }
}
pub fn wealth_fill(wealth: u32) -> Result<f32> {
    match wealth {
        1 => Ok(0.33),
        2 => Ok(0.66),
        3 => Ok(1.0),
        // SPEC-GAP: native stale-local bits have no portable value to invent.
        _ => Err(Unsupported::StaleMeterFill),
    }
}
pub fn job_skin(groups: Option<&[u32]>) -> u32 {
    let Some(groups) = groups else {
        return 0x1441_6200;
    };
    for (group, icon) in [
        (0x1000, 0x08),
        (0x1001, 0x02),
        (0x1002, 0x06),
        (0x1003, 0x09),
        (0x1004, 0x0a),
        (0x1004, 0x0a),
        (0x1501, 0x07),
        (0x1500, 0x07),
        (0x1502, 0x03),
        (0x1503, 0x01),
        (0x1504, 0x01),
        (0x1505, 0x01),
        (0x1506, 0x01),
        (0x1507, 0x01),
        (0x150a, 0x0c),
        (0x150b, 0x0c),
        (0x1006, 0x04),
    ] {
        if groups.contains(&group) {
            return 0x1441_6200 + icon;
        }
    }
    0x1441_6200
}

struct Candidate {
    building: Handle,
    name: String,
    wealth: Option<u32>,
}

impl BuildingTool {
    fn candidate(&self, x: i32, z: i32, host: &mut impl HoverServices) -> Result<Option<Candidate>> {
        let Some((occupant, 0x2781_28a0)) = host.pick_occupant(x, z) else {
            return Ok(None);
        };
        let Some(building) = host.building_interface(occupant) else {
            return Ok(None);
        };
        let Some(agent) = self.agent else {
            return Ok(None);
        };
        let sim = self.sim.ok_or(Unsupported::Precondition("initialized Sim"))?;
        if self.kind != BuildingKind::ChangeJob {
            if host.my_sim_lives_here(agent, building).is_some() {
                return Ok(None);
            }
            if host
                .building_slot9(building)?
                .partial_cmp(&1.0)
                .map_or(true, |order| order.is_lt())
                || host.building_slot11(building)? != 1
            {
                return Ok(None);
            }
            if !host.test_residence(sim, building, true, self.kind == BuildingKind::ChangeHouse) {
                return Ok(None);
            }
            let lot = host.lot_by_occupant(building).ok_or(Unsupported::MissingLot)?;
            let wealth = host.lot_slot41(lot)?;
            let name = host.building_name(building);
            if host.lot_slot33(lot)? != 3 {
                return Ok(None);
            }
            Ok(Some(Candidate {
                building,
                name,
                wealth: Some(wealth),
            }))
        } else {
            let name = host.building_name(building);
            if host
                .building_slot9(building)?
                .partial_cmp(&1.0)
                .map_or(true, |order| order.is_lt())
                || host.building_slot11(building)? == 1
            {
                return Ok(None);
            }
            let position = host.building_position(building);
            let lot = host.lot_by_position(position).ok_or(Unsupported::MissingLot)?;
            let wealth = host.wealth(sim);
            if host.lot_slot63(lot, wealth)? <= 0 || host.lot_slot35(lot)? >= 4 || host.lot_slot33(lot)? != 3 {
                return Ok(None);
            }
            Ok(Some(Candidate {
                building,
                name,
                wealth: None,
            }))
        }
    }
    fn badge(&self, allowed: bool, x: i32, z: i32, windows: &mut Windows, host: &mut impl HoverServices) {
        let (show, hide) = if allowed {
            (self.allowed, self.denied)
        } else {
            (self.denied, self.allowed)
        };
        if let Some(hide) = hide {
            windows.hide(host, hide);
        }
        if let Some(show) = show {
            windows.move_to(
                host,
                show,
                x.wrapping_sub(self.badge_half_width),
                z.wrapping_sub(self.badge_height),
            );
            windows.show(host, show);
        }
    }
    fn ensure_callout(&mut self, windows: &mut Windows, host: &mut impl HoverServices) -> Result<Window> {
        if let Some(callout) = self.callout {
            return Ok(callout);
        }
        let callout = host
            .create_callout(windows)
            .ok_or(Unsupported::Precondition("callout factory"))?;
        self.callout = Some(callout);
        host.callout_font(callout, 11, 0xa85f_1a83);
        host.callout_font(callout, 10, 0x0a52_f5c6);
        host.callout_icon(callout, 0);
        if self.kind != BuildingKind::ChangeJob {
            host.callout_skin(callout, [0, 0x46a0_06b0, 0x1441_6208], (12, 10));
        }
        if let Some(main) = host.main_window() {
            windows.child_add(host, main, callout);
        }
        host.callout_offset(callout, Rect(-self.badge_half_width, -1000, self.badge_half_width, 28));
        Ok(callout)
    }
    pub fn on_mouse_move(
        &mut self,
        x: i32,
        z: i32,
        windows: &mut Windows,
        host: &mut impl HoverServices,
    ) -> Result<bool> {
        let Some(candidate) = self.candidate(x, z, host)? else {
            self.badge(false, x, z, windows, host);
            if let Some(callout) = self.callout {
                windows.hide(host, callout);
            }
            self.restore_highlight(host);
            return Ok(true);
        };
        self.badge(true, x, z, windows, host);
        self.restore_highlight(host);
        host.add_ref(candidate.building);
        let level = host.highlight(candidate.building);
        self.highlighted = Some((candidate.building, level));
        host.set_highlight(candidate.building, 1, false);
        let callout = self.ensure_callout(windows, host)?;
        if let Some(wealth) = candidate.wealth {
            host.callout_meter(callout, [0, 0x46a0_06b0, 0x1441_621f], 3, wealth_fill(wealth)?);
            host.callout_title(callout, &candidate.name);
            let position = host.building_position(candidate.building);
            let cell = host.position_to_cell(position);
            let eq = host.cell_eq(cell, wealth);
            let hq = host.cell_hq(cell, wealth);
            let eq = host.string(0xea23_1e96, eq_text(eq));
            let hq = host.string(0xea23_1e96, hq_text(hq));
            host.callout_body(callout, &format!("{eq}\n{hq}"));
        } else {
            let groups = host.occupant_groups(candidate.building);
            host.callout_skin(callout, [0, 0x46a0_06b0, job_skin(groups.as_deref())], (12, 10));
            host.callout_title(callout, &candidate.name);
            let sim = self.sim.ok_or(Unsupported::Precondition("initialized Sim"))?;
            let title = host.job_title(sim, candidate.building);
            host.callout_body(callout, &title);
        }
        host.callout_point(callout, (x.wrapping_add(1), z));
        windows.show(host, callout);
        Ok(true)
    }
    pub fn on_mouse_down_l(&mut self, host: &mut impl HoverServices) -> Result<bool> {
        if !host.is_current_control() {
            return Ok(false);
        }
        if let Some((building, _)) = self.highlighted {
            let sim = self.sim.ok_or(Unsupported::Precondition("initialized Sim"))?;
            let end = match self.kind {
                BuildingKind::ChangeHouse => {
                    host.test_residence(sim, building, true, false) && host.change_residence(sim, building)
                }
                BuildingKind::ChangeJob => {
                    host.change_job(sim, building);
                    true
                }
                BuildingKind::MoveIn => {
                    host.set_residence(sim, building);
                    true
                }
            };
            if end {
                host.end_input();
            }
        }
        Ok(true)
    }
}

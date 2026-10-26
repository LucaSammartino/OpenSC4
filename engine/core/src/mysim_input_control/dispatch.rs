// SPDX-License-Identifier: GPL-3.0-or-later
//! cSC4MySimInputControlDispatch and its pylon filter, mysim-tools §5.
use super::*;

#[cfg(test)]
mod tests;

pub const MESSAGES: [u32; 6] = [
    0x09c8_b8ea,
    0xe946_71eb,
    0xe946_71ec,
    0xe946_71ed,
    0x8946_71f6,
    0xaa25_827c,
];
pub const NORMAL_CURSOR: u32 = 0x2bac_4970;
pub const DRAG_CURSOR: u32 = 0x13b1_38d0;
pub const CURSOR_TEXT: u32 = 0x49d0_2c4d;
pub const PYLON_MESSAGE: u32 = 0x8bbb_d22a;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Control {
    pub identity: Handle,
    pub id: u32,
}

pub trait DispatchServices: InputServices {
    fn set_control_id(&mut self, id: u32);
    fn subscribe(&mut self, message: u32, subscribe: bool);
    fn clear_cursor_text(&mut self, key: u32);
    fn remove_all_controls(&mut self, argument: bool);
    fn key_is_passed(&mut self, key: u32) -> bool;
    /// Created timer and filter references. Filter callbacks use PylonFilter.
    fn create_timer(&mut self, units: u32) -> Handle;
    fn timer_units(&mut self, timer: Handle, units: u32);
    fn timer_running(&mut self, timer: Handle) -> bool;
    fn timer_elapsed(&mut self, timer: Handle) -> u32;
    fn timer_restart(&mut self, timer: Handle);
    fn timer_reset(&mut self, timer: Handle);
    fn destroy_timer(&mut self, timer: Handle);
    fn create_filter(&mut self) -> Handle;
    fn sim_active(&mut self, sim: Handle) -> bool;
    fn sim_pylon(&mut self, sim: Handle) -> Option<Handle>;
    fn create_sim_pylon(&mut self, sim: Handle) -> Option<Handle>;
    fn dispatch_tool_end(&mut self, sim: Handle);
    fn lock_pylon(&mut self, sim: Handle, locked: bool);
    fn test_location(&mut self, sim: Handle, x: f32, z: f32) -> bool;
    fn pylon_interface(&mut self, occupant: Handle) -> Option<Handle>;
    fn occupant_type(&mut self, occupant: Handle) -> u32;
    fn pylon_slot7(&mut self, _pylon: Handle) -> Result<u32> {
        // SPEC-GAP: open MST3; use raw slot values or report Unsupported.
        Err(Unsupported::PylonSlotsMst3)
    }
    fn pylon_slot20_sim(&mut self, _pylon: Handle) -> Result<Option<Handle>> {
        Err(Unsupported::PylonSlotsMst3)
    }
    fn drag_state(&mut self, pylon: Handle) -> bool;
    fn set_drag_state(&mut self, pylon: Handle, dragging: bool);
    fn dispatch_state(&mut self, pylon: Handle) -> u32;
    fn set_dispatch_state(&mut self, pylon: Handle, state: u32);
    fn position(&mut self, pylon: Handle) -> [f32; 3];
    fn set_position(&mut self, pylon: Handle, position: [f32; 3]);
    fn set_ui_state(&mut self, pylon: Handle, state: u32);
    /// Renderer slot 65 applies this core callback while selecting its hit.
    /// The returned occupant and all QI/owner lookups are borrowed identities.
    fn renderer_pick(
        &mut self,
        x: i32,
        z: i32,
        filter: impl FnMut(&mut Self, Handle) -> Result<bool>,
    ) -> Result<Option<Handle>>;
    fn pick_terrain(&mut self, x: i32, z: i32) -> Option<[f32; 3]>;
    fn altitude(&mut self, x: f32, z: f32) -> f32;
    fn world_to_screen(&mut self, position: [f32; 3]) -> [f32; 2];
    /// SC4PostMessage2(id, selected, 0, 0, 0, sim's interface).
    fn post_selection(&mut self, id: u32, selected: bool, sim: Handle);
    fn play_result_sound(&mut self, result: u32);
}

#[derive(Debug, Default)]
pub struct PylonFilter;
impl PylonFilter {
    pub fn includes(occupant: Handle, host: &mut impl DispatchServices) -> Result<bool> {
        if host.occupant_type(occupant) != 0xa9b6_d7de {
            return Ok(false);
        }
        let Some(pylon) = host.pylon_interface(occupant) else {
            return Ok(false);
        };
        if host.pylon_slot7(pylon)? == 0 || host.drag_state(pylon) {
            return Ok(false);
        }
        Ok(host.pylon_slot20_sim(pylon)?.is_some())
    }
}

#[derive(Debug)]
pub struct Dispatch {
    identity: Handle,
    initialized: bool,
    active: bool,
    last_location_test: bool,
    drag_offset: [i32; 2],
    timer: Option<Handle>,
    normal_cursor: u32,
    hovered: Option<Handle>,
    filter: Option<Handle>,
    sim: Option<Handle>,
    saved_position: [f32; 3],
    saved_state: u32,
    /// The documented one-reference Shutdown/destructor quirk, not new holds.
    leaked_hover_references: u32,
}

impl Dispatch {
    pub fn new(identity: Handle, sim: Option<Handle>, host: &mut impl DispatchServices) -> Self {
        if let Some(sim) = sim {
            host.add_ref(sim);
        }
        Self {
            identity,
            initialized: false,
            active: false,
            last_location_test: false,
            drag_offset: [0; 2],
            timer: Some(host.create_timer(4)),
            normal_cursor: NORMAL_CURSOR,
            hovered: None,
            filter: None,
            sim,
            saved_position: [0.0; 3],
            saved_state: 0,
            leaked_hover_references: 0,
        }
    }
    pub fn active(&self) -> bool {
        self.active
    }
    pub fn sim(&self) -> Option<Handle> {
        self.sim
    }
    pub fn hovered(&self) -> Option<Handle> {
        self.hovered
    }
    pub fn drag_offset(&self) -> [i32; 2] {
        self.drag_offset
    }
    pub fn last_location_test(&self) -> bool {
        self.last_location_test
    }
    pub fn leaked_hover_references(&self) -> u32 {
        self.leaked_hover_references
    }
    pub fn init(&mut self, host: &mut impl DispatchServices) -> Result<bool> {
        if !self.initialized {
            let timer = self.timer.ok_or(Unsupported::Precondition("live dispatch timer"))?;
            host.set_control_id(0xebab_04a9);
            self.normal_cursor = NORMAL_CURSOR;
            host.set_cursor(self.normal_cursor);
            host.timer_units(timer, 4);
            host.base_init(); // §5.2 returns true independently of the base byte.
            for message in MESSAGES {
                host.subscribe(message, true);
            }
            self.active = true;
            self.filter = Some(host.create_filter());
            self.initialized = true;
        }
        Ok(true)
    }
    fn restore_drag(&self, host: &mut impl DispatchServices) {
        if let Some(pylon) = self.sim.and_then(|sim| host.sim_pylon(sim)) {
            if host.drag_state(pylon) {
                host.set_position(pylon, self.saved_position);
                host.set_dispatch_state(pylon, self.saved_state);
            }
        }
    }
    pub fn shutdown(&mut self, host: &mut impl DispatchServices) -> bool {
        if self.initialized {
            host.clear_cursor_text(CURSOR_TEXT);
            for message in MESSAGES {
                host.subscribe(message, false);
            }
            if let Some(pylon) = self.hovered.take() {
                host.set_ui_state(pylon, 1);
                host.release(pylon); // §5.2 quirk: one of the two retained refs leaks.
                self.leaked_hover_references += 1;
            }
            self.restore_drag(host);
            if let Some(sim) = self.sim {
                let pylon = host.sim_pylon(sim);
                if let Some(pylon) = pylon {
                    host.add_ref(pylon);
                }
                host.dispatch_tool_end(sim);
                if let Some(pylon) = pylon {
                    host.set_drag_state(pylon, false);
                    host.release(pylon);
                }
            }
            if let Some(filter) = self.filter.take() {
                host.release(filter);
            }
            host.base_shutdown();
            self.initialized = false;
        }
        true
    }
    pub fn destroy(&mut self, host: &mut impl DispatchServices) {
        if let Some(sim) = self.sim.take() {
            host.release(sim);
        }
        if let Some(filter) = self.filter.take() {
            host.release(filter);
        }
        if let Some(pylon) = self.hovered.take() {
            host.release(pylon);
            self.leaked_hover_references += 1;
        }
        if let Some(timer) = self.timer.take() {
            host.destroy_timer(timer);
        }
        host.base_destroy();
        self.initialized = false;
    }
    pub fn set_hover_pylon(&mut self, pylon: Option<Handle>, host: &mut impl DispatchServices) {
        if self.hovered == pylon {
            return;
        }
        if let Some(old) = self.hovered.take() {
            host.set_ui_state(old, 1);
            host.release(old);
            host.release(old);
        }
        if let Some(pylon) = pylon {
            host.add_ref(pylon);
            host.add_ref(pylon);
            self.hovered = Some(pylon);
            host.set_ui_state(pylon, 3);
        }
    }
    pub fn set_new_sim(&mut self, sim: Option<Handle>, host: &mut impl DispatchServices) -> bool {
        if self.sim != sim {
            if let Some(old) = self.sim {
                host.dispatch_tool_end(old);
            }
            if let Some(sim) = sim {
                host.add_ref(sim);
            }
            let old = self.sim;
            self.sim = sim;
            if let Some(old) = old {
                host.release(old);
            }
        }
        true
    }
    /// Successful picks transfer one retained reference to the caller.
    pub fn pick_pylon(&self, x: i32, z: i32, host: &mut impl DispatchServices) -> Result<Option<Handle>> {
        if self.filter.is_none() {
            return Err(Unsupported::Precondition("initialized pylon filter"));
        }
        let Some(occupant) = host.renderer_pick(x, z, |host, object| PylonFilter::includes(object, host))? else {
            return Ok(None);
        };
        let Some(pylon) = host.pylon_interface(occupant) else {
            return Ok(None);
        };
        if host.pylon_slot20_sim(pylon)?.is_none() {
            return Ok(None);
        }
        host.add_ref(pylon);
        Ok(Some(pylon))
    }
    pub fn calculate_drag_offset(
        &mut self,
        pylon: Handle,
        x: i32,
        z: i32,
        host: &mut impl DispatchServices,
    ) -> Result<()> {
        let position = host.position(pylon);
        let screen = host.world_to_screen(position);
        let offset = [screen[0] - x as f32, screen[1] - z as f32];
        if offset
            .iter()
            .any(|v| !v.is_finite() || *v < i32::MIN as f32 || *v >= 2147483648.0)
        {
            // SPEC-GAP: no portable policy for non-finite/overflowing truncation.
            return Err(Unsupported::ScreenCoordinate);
        }
        self.drag_offset = [offset[0].trunc() as i32, offset[1].trunc() as i32];
        Ok(())
    }
    fn click_is_double(&self, host: &mut impl DispatchServices) -> Result<bool> {
        let timer = self.timer.ok_or(Unsupported::Precondition("live dispatch timer"))?;
        if host.timer_running(timer) && host.timer_elapsed(timer) < 500 {
            host.timer_reset(timer);
            Ok(true)
        } else {
            host.timer_restart(timer);
            Ok(false)
        }
    }
    fn begin_picked(
        &mut self,
        pylon: Handle,
        dbl: bool,
        x: i32,
        z: i32,
        host: &mut impl DispatchServices,
    ) -> Result<bool> {
        let sim = host.pylon_slot20_sim(pylon)?.ok_or(Unsupported::PylonSlotsMst3)?;
        self.set_new_sim(Some(sim), host);
        host.lock_pylon(sim, dbl);
        host.set_drag_state(pylon, !dbl);
        host.post_selection(PYLON_MESSAGE, dbl, sim);
        if !dbl {
            self.calculate_drag_offset(pylon, x, z, host)?;
            self.saved_position = host.position(pylon);
            self.saved_state = host.dispatch_state(pylon);
        }
        Ok(true)
    }
    pub fn on_mouse_down_l(&mut self, x: i32, z: i32, host: &mut impl DispatchServices) -> Result<bool> {
        if host.view_window().is_none() {
            host.play_result_sound(0);
            return Ok(false);
        }
        let held = self.sim.and_then(|sim| host.sim_pylon(sim));
        if let Some(held) = held {
            host.add_ref(held);
        }
        let mut picked = None;
        let result = (|| {
            let dbl = self.click_is_double(host)?;
            if let Some(pylon) = held {
                if host.drag_state(pylon) {
                    let sim = self.sim.ok_or(Unsupported::Precondition("Sim"))?;
                    if dbl {
                        host.post_selection(PYLON_MESSAGE, true, sim);
                    } else if let Some(position) =
                        host.pick_terrain(x.wrapping_add(self.drag_offset[0]), z.wrapping_add(self.drag_offset[1]))
                    {
                        host.set_position(pylon, position); // No location test/altitude in §5.5 step 4.
                    }
                    host.set_drag_state(pylon, false);
                    host.lock_pylon(sim, true);
                    return Ok(true);
                }
            }
            picked = self.pick_pylon(x, z, host)?;
            if let Some(pylon) = picked {
                return self.begin_picked(pylon, dbl, x, z, host);
            }
            if held.is_some() {
                return Ok(false);
            }
            let sim = self.sim.ok_or(Unsupported::Precondition("Sim"))?;
            if let Some(pylon) = host.create_sim_pylon(sim) {
                host.add_ref(pylon);
                if let Some(mut position) = host.pick_terrain(x, z) {
                    if host.test_location(sim, position[0], position[2]) {
                        position[1] = host.altitude(position[0], position[2]);
                        host.set_position(pylon, position);
                        host.lock_pylon(sim, true);
                    }
                }
                host.release(pylon);
            }
            Ok(true)
        })();
        if let Some(picked) = picked {
            host.release(picked);
        }
        if let Some(held) = held {
            host.release(held);
        }
        host.play_result_sound(0);
        result
    }
    pub fn on_mouse_move(&mut self, x: i32, z: i32, host: &mut impl DispatchServices) -> Result<bool> {
        if host.view_window().is_none() {
            return Ok(true);
        }
        let Some(sim) = self.sim else {
            return Ok(true);
        };
        let Some(pylon) = host.sim_pylon(sim) else {
            return Ok(true);
        };
        if host.drag_state(pylon) {
            host.set_cursor(DRAG_CURSOR);
            let timer = self.timer.ok_or(Unsupported::Precondition("live dispatch timer"))?;
            if host.timer_running(timer) && host.timer_elapsed(timer) < 500 {
                return Ok(true);
            }
            host.timer_reset(timer);
            if let Some(mut position) =
                host.pick_terrain(x.wrapping_add(self.drag_offset[0]), z.wrapping_add(self.drag_offset[1]))
            {
                position[1] = host.altitude(position[0], position[2]);
                self.last_location_test = host.test_location(sim, position[0], position[2]);
                if self.last_location_test {
                    host.set_position(pylon, position);
                    host.set_dispatch_state(pylon, 3);
                } else {
                    host.set_dispatch_state(pylon, 1);
                }
            }
        } else if let Some(picked) = self.pick_pylon(x, z, host)? {
            self.set_hover_pylon(Some(picked), host);
            host.release(picked);
            host.set_cursor(DRAG_CURSOR);
        } else {
            host.set_cursor(self.normal_cursor);
            self.set_hover_pylon(None, host);
        }
        Ok(true)
    }
    pub fn do_message(
        &mut self,
        id: u32,
        old: Option<Control>,
        new: Option<Control>,
        occupant: Option<Handle>,
        host: &mut impl DispatchServices,
    ) -> bool {
        match id {
            0x09c8_b8ea => {
                let was_this = old.is_some_and(|c| c.identity == self.identity);
                let is_this = new.is_some_and(|c| c.identity == self.identity);
                if was_this && !is_this && new.is_none_or(|c| c.id != 0x48e2_a1f8) {
                    self.active = false;
                    host.clear_cursor_text(CURSOR_TEXT);
                    self.set_hover_pylon(None, host);
                    self.restore_drag(host);
                } else if is_this && !was_this {
                    self.active = true;
                }
                true
            }
            0x99ef_1143 => {
                if occupant.is_some() && occupant == self.hovered {
                    host.release(self.hovered.take().expect("non-null matching occupant"));
                    self.leaked_hover_references += 1;
                }
                true
            }
            0xe946_71eb..=0xe946_71ed | 0x8946_71f6 => true,
            0xaa25_827c => {
                if self.sim.is_some_and(|sim| !host.sim_active(sim)) {
                    host.remove_all_controls(false);
                }
                false
            }
            _ => false,
        }
    }
    pub fn on_key_down(&self, key: u32, host: &mut impl DispatchServices) -> bool {
        host.key_is_passed(key);
        false
    }
}

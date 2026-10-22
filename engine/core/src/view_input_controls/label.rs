// SPDX-License-Identifier: GPL-3.0-or-later
use super::{
    project, sound, Key, MouseState, Pixel, PropFilter, PropId, PropWorld, Text, Unsupported, Vec3, LABEL_TYPE,
    PICK_UP, PUT_DOWN,
};
use crate::view_utilities::ViewUtilities;

pub type Edges = [u8; 4]; // W,N,E,S, bytes of an edge word.
#[derive(Clone, Copy, Debug, Default)]
pub struct NetworkTile {
    pub flags: u32,
    pub road: Edges,
    pub rail: Edges,
    pub one_way: Edges,
    pub street: Edges,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelPlacement {
    pub position: Vec3,
    pub orientation: u8,
    pub color: (f32, f32, f32),
    pub height: f32,
}

pub trait LabelWorld: PropWorld + LabelDialog {
    fn view_rotation(&self) -> u32;
    fn position_to_cell(&self, p: Vec3) -> (i32, i32);
    fn cell_center(&self, cell: (i32, i32)) -> Vec3;
    fn cell_has_water(&self, cell: (i32, i32)) -> bool;
    fn network_tile(&self, cell: (i32, i32), mask: u32) -> Option<NetworkTile>;
    fn avenue_piece_edges(&self, p: Vec3, mask: u32) -> Option<Edges>;
    fn set_label_color(&mut self, prop: PropId, color: (f32, f32, f32));
    fn set_label_height(&mut self, prop: PropId, height: f32);
    fn set_label_orientation(&mut self, prop: PropId, orientation: u8);
}

/// Ordered road rules, not an output supplied by a native owner.
pub fn road_center_and_orientation(e: Edges) -> (u8, (f32, f32)) {
    let k = e.map(|x| x & 7);
    let c = k.map(|x| x != 0);
    let d = k.map(|x| x == 1 || x == 3);
    let n = c.iter().filter(|&&v| v).count();
    if n == 1 {
        return (if c[1] || c[3] { 0 } else { 1 }, (0.0, 0.0));
    }
    if n == 2 || n == 3 {
        if c[0] && c[2] && !d[2] {
            return (1, (0.0, 0.0));
        }
        if c[1] && c[3] && !d[1] {
            return (0, (0.0, 0.0));
        }
        if c[0] && c[1] && d[1] {
            return (2, (-4.0, -4.0));
        }
        if c[2] && c[3] && d[3] {
            return (2, (4.0, 4.0));
        }
        if c[0] && c[3] && d[3] {
            return (3, (-4.0, 4.0));
        }
        if c[1] && c[2] && d[2] {
            return (3, (4.0, -4.0));
        }
    }
    (1, (0.0, 0.0))
}

pub fn avenue_center_and_orientation(e: Edges) -> (u8, (f32, f32)) {
    let k = e.map(|x| x & 3);
    let connected = k.map(|x| x != 0);
    let diagonal = k.map(|x| x == 1 || x == 3);
    let n = connected.iter().filter(|&&v| v).count();
    let d = diagonal.iter().filter(|&&v| v).count();
    // "Orthogonal" means a connected non-diagonal side (kind 2), not a
    // disconnected side. VIC3 asks about unlisted edge encodings.
    let orthogonal = k.map(|x| x == 2);
    if d == n {
        if n == 4 {
            return (if k[0] == 1 { 2 } else { 3 }, (0.0, 0.0));
        }
        if k[0] == 1 {
            return (2, (-8.0, -8.0));
        }
        if k[0] == 3 {
            return (3, (-8.0, 8.0));
        }
        if k[2] == 1 {
            return (2, (8.0, 8.0));
        }
        if k[2] == 3 {
            return (3, (8.0, -8.0));
        }
        return (1, (0.0, 0.0));
    }
    if d == 2 && n == 4 {
        if orthogonal[0] {
            return if orthogonal[3] {
                (2, (-8.0, 8.0))
            } else {
                (3, (-8.0, -8.0))
            };
        }
        if orthogonal[2] {
            return if orthogonal[3] {
                (3, (8.0, 8.0))
            } else {
                (2, (8.0, -8.0))
            };
        }
        return (1, (0.0, 0.0));
    }
    let inner = e.map(|x| x & 4 != 0);
    let ox = (i32::from(inner[2]) - i32::from(inner[0])) as f32 * 8.0;
    let oz = (i32::from(inner[3]) - i32::from(inner[1])) as f32 * 8.0;
    (if orthogonal[0] || orthogonal[2] { 1 } else { 0 }, (ox, oz))
}

pub fn label_placement(host: &impl LabelWorld, p: Vec3, orientation: u8) -> Result<LabelPlacement, Unsupported> {
    let cell = host.position_to_cell(p);
    if host.cell_has_water(cell) {
        return Ok(LabelPlacement {
            position: p,
            orientation,
            color: (0.5, 0.5, 1.0),
            height: 2.0,
        });
    }
    let Some(tile) = host.network_tile(cell, 0x44B) else {
        return Ok(LabelPlacement {
            position: p,
            orientation,
            color: (0.5, 1.0, 0.5),
            height: 2.0,
        });
    };
    let avenue = tile.flags & 0x40 == 0x40;
    let height = if avenue {
        2.0
    } else if tile.flags & 0x403 != 0 {
        0.8
    } else {
        0.64
    };
    let snapped = if avenue {
        host.avenue_piece_edges(p, 0x40).map(avenue_center_and_orientation)
    } else {
        let t = host.network_tile(cell, 0x40B).ok_or(Unsupported::PrimitiveData)?;
        let edges = if t.flags & 2 != 0 {
            t.rail
        } else if t.flags & 0x400 != 0 {
            t.one_way
        } else if t.flags & 1 != 0 {
            t.road
        } else {
            t.street
        };
        Some(road_center_and_orientation(edges))
    };
    let (position, orientation) = if let Some((orientation, (ox, oz))) = snapped {
        let center = host.cell_center(cell);
        (Vec3::new(center.x + ox, p.y, center.z + oz), orientation)
    } else {
        (p, 1)
    };
    Ok(LabelPlacement {
        position,
        orientation,
        color: (1.0, 0.902, 0.0),
        height,
    })
}

pub const LABEL_OK: u32 = 0x8A8D_FD4E;
pub const LABEL_CANCEL: u32 = 0x8A8D_FD4A;
pub const LABEL_CLOSE: u32 = 0x6BA1_8178;
pub const LABEL_FIELD: u32 = 0xAA41_49BF;
pub const LABEL_FIELD_INTERFACE: u32 = 0x231A_1C57;
pub const LABEL_DIALOG: u32 = 0x8A8D_FCF5;
pub const LABEL_SCRIPT: (u32, u32) = (0x96A0_06B0, 0x6B70_4690);

#[derive(Clone, Copy, Debug, Default)]
pub struct DialogMessage {
    pub kind: u32,
    pub message: u32,
    pub widget: u32,
    pub key: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterAction {
    EndModal { kind: u32, result: u32 },
    Focus(u32),
    None,
}
pub fn label_message_filter(m: DialogMessage) -> FilterAction {
    if m.kind == 3 && m.message == 0x2872_59F6 && [LABEL_CLOSE, LABEL_CANCEL, LABEL_OK].contains(&m.widget) {
        FilterAction::EndModal {
            kind: 0xF,
            result: m.widget,
        }
    } else if m.kind == 0x11 {
        FilterAction::Focus(LABEL_FIELD)
    } else if m.kind == 0x1A && m.key == 0x0D {
        FilterAction::EndModal {
            kind: 0xF,
            result: LABEL_OK,
        }
    } else {
        FilterAction::None
    }
}
/// Field text/select operations use LABEL_FIELD_INTERFACE; these are primitive
/// GUI operations, not the label owner's dialog acceptance algorithm.
pub trait LabelDialog {
    fn load_dialog(&mut self, script: (u32, u32), window: u32) -> Option<u64>;
    fn set_field_text(&mut self, window: u64, field: u32, text: &Text);
    fn select_all_text(&mut self, window: u64, field: u32);
    fn field_text(&self, window: u64, field: u32) -> Text;
    fn run_modal(&mut self, window: u64, filter: fn(DialogMessage) -> FilterAction) -> u32;
    fn focus_city_view(&mut self);
    fn shutdown_dialog(&mut self, window: u64);
}

#[derive(Debug)]
pub struct LabelTool {
    state: MouseState,
    selected: Option<PropId>,
    orientation: u8,
    press_offset: Vec3,
}
impl Default for LabelTool {
    fn default() -> Self {
        Self {
            state: MouseState::Idle,
            selected: None,
            orientation: 1,
            press_offset: Vec3::ZERO,
        }
    }
}
impl LabelTool {
    pub const ID: u32 = 0x8B3C_E416;
    pub const CURSOR: u32 = 0xEB72_FEF4;
    pub fn init(&self, host: &mut impl LabelWorld) {
        host.show_signs_and_labels(true);
        super::set_cursor(host, Self::CURSOR);
    }
    pub fn state(&self) -> MouseState {
        self.state
    }
    pub fn selected(&self) -> Option<PropId> {
        self.selected
    }
    pub fn orientation(&self) -> u8 {
        self.orientation
    }
    pub fn press_offset(&self) -> Vec3 {
        self.press_offset
    }
    pub fn decal_at(&self, host: &mut impl LabelWorld, p: Vec3) -> Option<PropId> {
        host.first_prop_at(p, PropFilter::Label)
    }
    pub fn select(&mut self, host: &mut impl LabelWorld, label: Option<PropId>) {
        if self.selected == label {
            return;
        }
        if let Some(new) = label {
            host.set_alpha(new, 0.7);
        }
        if let Some(old) = self.selected {
            host.set_alpha(old, 0.5);
        }
        self.selected = label;
    }
    pub fn update_text_decal(&self, host: &mut impl LabelWorld, p: Vec3, label: PropId) -> Result<(), Unsupported> {
        let place = label_placement(host, p, self.orientation)?;
        host.set_label_color(label, place.color);
        host.set_label_height(label, place.height);
        host.set_position(label, place.position);
        host.set_label_orientation(label, place.orientation);
        Ok(())
    }
    pub fn edit_label(&self, host: &mut impl LabelWorld, label: Option<PropId>) -> Result<bool, Unsupported> {
        let Some(label) = label else {
            return Ok(true);
        };
        let win = host
            .load_dialog(LABEL_SCRIPT, LABEL_DIALOG)
            .ok_or(Unsupported::ResourceUnavailable)?;
        host.set_field_text(win, LABEL_FIELD, &host.text(label));
        host.select_all_text(win, LABEL_FIELD);
        let result = host.run_modal(win, label_message_filter);
        let success = if result == LABEL_OK {
            let text = host.field_text(win, LABEL_FIELD);
            host.set_text(label, &text);
            !text.is_empty()
        } else {
            result != LABEL_CANCEL
        };
        host.focus_city_view();
        host.shutdown_dialog(win);
        Ok(success)
    }
    pub fn mouse_down(&mut self, host: &mut impl LabelWorld, at: Pixel) -> Result<bool, Unsupported> {
        if !host.renderer_available() {
            return Ok(true);
        }
        let Some(p) = host.pick_terrain(at) else {
            return Ok(false);
        };
        if let Some(label) = self.decal_at(host, p) {
            self.state = MouseState::Pressed;
            self.select(host, Some(label));
            self.press_offset = host.position(label) - p;
            sound(host, PICK_UP);
        } else {
            let label = host.add_city_prop(LABEL_TYPE).ok_or(Unsupported::ResourceUnavailable)?;
            match self.edit_label(host, Some(label)) {
                Ok(true) => {
                    self.orientation = if host.view_rotation() == 0 || host.view_rotation() == 2 {
                        1
                    } else {
                        0
                    };
                    if let Err(e) = self.update_text_decal(host, p, label) {
                        host.remove_prop(label);
                        return Err(e);
                    }
                    self.select(host, Some(label));
                }
                Ok(false) => host.remove_prop(label),
                Err(e) => {
                    host.remove_prop(label);
                    return Err(e);
                }
            }
        }
        Ok(true)
    }
    pub fn mouse_move(&mut self, host: &mut impl LabelWorld, at: Pixel) -> Result<bool, Unsupported> {
        if self.state == MouseState::Pressed {
            if let Some(prop) = self.selected {
                self.state = MouseState::Dragging;
                host.set_dynamic(prop, true);
            }
        }
        if self.state == MouseState::Dragging {
            if let (Some(prop), Some(p)) = (self.selected, host.pick_terrain(at)) {
                self.update_text_decal(host, p, prop)?;
            }
        } else if self.state == MouseState::Idle {
            let label = host.pick_terrain(at).and_then(|p| self.decal_at(host, p));
            self.select(host, label);
        }
        Ok(true)
    }
    pub fn mouse_up(&mut self, host: &mut impl LabelWorld) -> Result<bool, Unsupported> {
        let mut result = Ok(true);
        if self.state == MouseState::Pressed {
            match self.edit_label(host, self.selected) {
                Ok(false) => {
                    if let Some(prop) = self.selected.take() {
                        host.set_alpha(prop, 0.5);
                        host.remove_prop(prop);
                    }
                }
                Err(e) => result = Err(e),
                Ok(true) => {}
            }
        } else if self.state == MouseState::Dragging {
            if let Some(prop) = self.selected {
                host.set_dynamic(prop, false);
            }
            sound(host, PUT_DOWN);
        }
        self.state = MouseState::Idle;
        result
    }
    pub fn key_down(&mut self, host: &mut impl LabelWorld, key: Key) -> Result<bool, Unsupported> {
        if let Some(prop) = self.selected {
            if key == Key::Delete && self.state == MouseState::Idle {
                host.remove_prop(prop);
                self.selected = None;
                return Ok(true);
            }
            if key == Key::Home || key == Key::End {
                self.orientation =
                    if key == Key::Home { [3, 2, 0, 1] } else { [2, 3, 1, 0] }[self.orientation as usize];
                self.update_text_decal(host, host.position(prop), prop)?;
            }
        }
        Ok(false)
    }
    pub fn shutdown(&mut self, host: &mut impl LabelWorld) {
        if let Some(prop) = self.selected.take() {
            host.set_alpha(prop, 0.5);
        }
    }
}

/// Shared screen projection for the sign's saved press offset, not the label's
/// world-space offset (which is deliberately unused when dragging).
pub(crate) fn screen_offset(utilities: &ViewUtilities, p: Vec3, mouse: Pixel) -> Vec3 {
    let p = project(utilities, p);
    Vec3::new(p.x - mouse.x as f32, p.y - mouse.y as f32, 0.0)
}

// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub const MODEL_KEY: [u32; 3] = [0x5ad0_e817, 0xbadb_57f1, 0x8bb9_0000];
pub const ICON_INSTANCES: [u32; 7] = [
    0x1441_61a1,
    0x1441_61a2,
    0x1441_61a3,
    0x4bb0_ecf3,
    0x2bb1_2d1f,
    0x4bb0_ecf3,
    0x1441_61a2,
];
pub const VIEW_CHANGED: u32 = 0x8aca_a95f;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    pub flags: u8,
    pub rgba: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub rotation: [[f32; 3]; 3],
    pub translation: Vec3,
    pub flags: [u8; 2],
}
impl Default for Transform {
    fn default() -> Self {
        Self {
            rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            translation: [0.0; 3],
            flags: [1; 2],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ModelOp {
    BindOccupant(Token),
    Mode(u32),
    ZoomRotation(u32, u32),
    ProcessorModel { processor: Token, base: Token },
    Material(Material),
    Rate(f32),
    DestroyProcessor(Token),
    Light { processor: Token, flags: u32 },
    Transform(Transform),
    Fade(bool, f32),
    Advance(f32),
    Frame(u32),
    Draw(u32),
}

/// Host preserves unmentioned material/transform bytes. Returned model and
/// automaton interfaces are created/acquired references, released by core.
pub trait Services: DisplayServices {
    /// Load (7AB50E44,1ABE787D,instance), service 1AC0E11A / FAC0E219.
    fn icon_handle(&mut self, instance: u32) -> Option<Icon>;
    /// SPEC-GAP: Init packing values are absent; supply established adapter
    /// states in member order +14,+1C,+24, or report OpenRenderStates (DVV30-1).
    fn render_states(&mut self) -> Option<[Token; 3]>;
    fn subscribe(&mut self, id: u32, on: bool);
    fn occupant_type(&mut self, occupant: Token) -> u32;
    fn automaton(&mut self, _occupant: Token) -> Option<Token> {
        None
    }
    fn automaton_flag(&mut self, _automaton: Token, _flag: u32) -> bool {
        false
    }
    fn set_automaton_flag(&mut self, automaton: Token, flag: u32, on: bool);
    fn load_model(&mut self, _key: [u32; 3]) -> Option<Token> {
        None
    }
    fn model_op(&mut self, model: Token, op: ModelOp);
    fn allocate_processor(&mut self) -> Option<Token> {
        None
    }
    fn model_base(&mut self, model: Token) -> Token;
    fn model_bounds(&mut self, model: Token) -> [Vec3; 2];
    fn model_material(&mut self, model: Token) -> Material;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    pub token: Token,
    pub id: u32,
    pub area: Rect,
    pub visible: bool,
}
pub trait WindowServices {
    fn main_window(&mut self) -> Option<Window>;
    /// Children in slot-32 enumeration order, with filter ID 22BA0121.
    fn child_windows(&mut self, parent: Token) -> Vec<Window>;
    /// Slot 113(E6E998FE,out): bool and optional created reference to release.
    fn window_exclusion(&mut self, child: Token) -> (bool, Option<Token>);
    fn release_window_object(&mut self, object: Token);
}

impl DispatchVehicleView {
    pub fn rebuild_windows(&mut self, host: &mut impl WindowServices) {
        let Some(main) = host.main_window() else {
            return;
        };
        self.windows.clear();
        self.windows.push(Obstacle {
            rectangle: main.area,
            category: 0xff,
        });
        for child in host.child_windows(main.token) {
            let narrower =
                child.area.right.wrapping_sub(child.area.left) < main.area.right.wrapping_sub(main.area.left);
            let shorter = child.area.bottom.wrapping_sub(child.area.top) < main.area.bottom.wrapping_sub(main.area.top);
            if child.token == 0
                || !child.visible
                || !(narrower || shorter)
                || [0xaa32_bce6, 0xea8c_ad14, 0x4c30_e4fa].contains(&child.id)
            {
                continue;
            }
            let (exclude, object) = host.window_exclusion(child.token);
            if let Some(object) = object {
                host.release_window_object(object);
            }
            if !exclude {
                self.windows.push(Obstacle {
                    rectangle: child.area,
                    category: 0xff,
                });
            }
        }
    }
}

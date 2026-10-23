//! Pylon-owned state, records and placement (network-occupants §§6.1–6.3).
//! NO36 explicitly leaves texture/model composition and drawing unsettled.

use super::{identity::OCCUPANT_TYPE_PYLON, xform::Xform};
use crate::automata::OccupantBase;
use crate::stream::{InStream, OutStream, RefValue};

pub const PYLON_RECORD_VERSION: u16 = 5;
pub const PYLON_MESSAGES: [u32; 3] = [0xA6B7_9602, 0x06CD_B65B, 0x26D3_1EC1];
pub const PYLON_TICK: u32 = 0x80;
pub const ANIMATION_TICK_MANAGER_CLASS: u32 = 0x86F9_FB4D;
pub const ANIMATION_TICK_MANAGER_IID: u32 = 0x86F9_FB4C;

/// Positional PackRenderState arguments; the twelfth argument is binary32.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderStateArgs(pub [i32; 11], pub f32, pub [i32; 2]);
pub const PYLON_RENDER_STATES: [RenderStateArgs; 2] = [
    RenderStateArgs([1, 1, 1, 1, 1, 1, 1, 3, 4, 5, 4], 0.0, [1, -1]),
    RenderStateArgs([1, 0, 0, 1, 1, 1, 1, -1, 4, 5, 4], 0.0, [1, -1]),
];

/// Equality of opaque interface identities, without acquiring a reference.
pub(super) fn same_ref(a: &RefValue, b: &RefValue) -> bool {
    match (a, b) {
        (RefValue::Null, RefValue::Null) => true,
        (RefValue::Host { token: a, .. }, RefValue::Host { token: b, .. }) => a == b,
        (RefValue::Object(a), RefValue::Object(b)) => std::rc::Rc::ptr_eq(a, b),
        _ => false,
    }
}

/// Foreign reference primitives. Stream-returned refs already have ownership.
pub trait References {
    fn add_ref(&mut self, value: &RefValue);
    fn release(&mut self, value: &RefValue);
}

pub(super) fn clear_ref(value: &mut RefValue, refs: &mut impl References) {
    let old = std::mem::replace(value, RefValue::Null);
    if !matches!(old, RefValue::Null) {
        refs.release(&old);
    }
}

/// Semantic live fields: exactly one transform and one source reference.
#[derive(Clone, Debug)]
pub struct PylonRecord {
    pub base: OccupantBase,
    pub dirty: bool,
    pub states: [i32; 3],
    pub blink: bool,
    pub call_number: u16,
    pub source: RefValue,
    pub transform: Xform,
    pub drag: u8,
    pub bitmap: i32,
    pub text: Vec<u8>,
    pub owner: RefValue,
}

impl PylonRecord {
    /// The constructor values of the three enums are not published (NO44).
    pub fn new(states: [i32; 3]) -> Self {
        Self {
            base: OccupantBase::new(OCCUPANT_TYPE_PYLON),
            dirty: true,
            states,
            blink: false,
            call_number: 0,
            source: RefValue::Null,
            transform: Xform::default(),
            drag: 0,
            bitmap: 0,
            text: Vec::new(),
            owner: RefValue::Null,
        }
    }
}

impl PylonRecord {
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u16(PYLON_RECORD_VERSION);
        if self.base.write(out) {
            out.put_bool(self.dirty);
            for state in self.states {
                out.put_u8(state as u8);
            }
            out.put_bool(self.blink);
            out.put_u16(self.call_number);
            out.put_ref(&self.source);
            self.transform.write(out);
            out.put_bool(self.drag != 0);
            out.put_u32(self.bitmap as u32);
            out.put_str(&self.text);
            out.put_ref(&self.owner);
        }
        out.error() == 0
    }

    /// No rollback or early exits inside the reached body (§6.3).
    pub fn read_into(&mut self, inp: &mut dyn InStream, refs: &mut impl References) -> bool {
        let Some(version) = inp.get_u16() else {
            return false;
        };
        if version < 3 {
            return false;
        }
        if let Some(base) = OccupantBase::read(inp) {
            self.base = base;
            if let Some(v) = inp.get_bool() {
                self.dirty = v;
            }
            for state in &mut self.states {
                if let Some(v) = inp.get_u8() {
                    *state = i32::from(v as i8);
                }
            }
            if let Some(v) = inp.get_bool() {
                self.blink = v;
            }
            if let Some(v) = inp.get_u16() {
                self.call_number = v;
            }
            clear_ref(&mut self.source, refs);
            if let Some(v) = inp.get_ref_as(0x26BB_767E) {
                self.source = v;
            }
            if version == 3 {
                self.transform.legacy_load_into(inp);
            } else {
                self.transform.read_into(inp);
            }
            if let Some(v) = inp.get_bool() {
                self.drag = u8::from(v);
            }
            if version >= 5 {
                if let Some(v) = inp.get_u32() {
                    self.bitmap = v as i32;
                }
                if let Some(v) = inp.get_str() {
                    self.text = v;
                }
                clear_ref(&mut self.owner, refs);
                if let Some(v) = inp.get_ref_as(1) {
                    self.owner = v;
                }
            }
        }
        inp.error() == 0
    }
}

/// All calls are foreign base/service primitives, never owner-result callbacks.
pub trait PylonHost: References {
    fn occupant_init(&mut self, base: &mut OccupantBase);
    fn occupant_shutdown(&mut self, base: &mut OccupantBase);
    fn message(&mut self, id: u32, subscribe: bool);
    fn animation(&mut self, class: u32, iid: u32, tick: u32, subscribe: bool);
    fn renderer_exists(&self) -> bool;
    fn renderer_probe(&mut self);
    fn pack(&mut self, args: RenderStateArgs) -> u64;
    fn city_manager(&mut self) -> bool;
    fn manager_insert(&mut self) -> bool;
    fn manager_remove(&mut self);
    fn prepare_instance(&mut self, argument: u32) -> bool;
    /// Adapter supplies the adjusted 3D-instance view (§6.2), not primary.
    fn renderer_insert(&mut self) -> bool;
    fn renderer_remove(&mut self);
    fn base_visibility(&mut self, visible: bool, argument: u32, flags: &mut u8);
    fn release_texture(&mut self, id: u32);
    fn texture_manager_exists(&self) -> bool;
}

#[derive(Clone, Debug, Default)]
pub struct PylonGlobals {
    pub field_b23b78: bool,
    pub blink_accumulator: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PylonUnsupported {
    ModelTextureGeometryDrawBoundsTransformMessagesNO36,
}

#[derive(Clone, Debug)]
pub struct Pylon {
    pub record: PylonRecord,
    pub initialized: bool,
    pub placement_flags: u8,
    pub auxiliary: [[f32; 6]; 2],
    pub packed_states: [u64; 2],
    pub texture_binding: RefValue,
    pub texture: u32,
    pub mysim_texture: RefValue,
}

// RefValue has no Default: native construction initializes held pointers null.
impl Pylon {
    pub fn new(states: [i32; 3]) -> Self {
        Self {
            record: PylonRecord::new(states),
            initialized: false,
            placement_flags: 0,
            auxiliary: [[1.0, 1.0, 1.0, 0.0, 0.0, 0.0]; 2],
            packed_states: [0; 2],
            texture_binding: RefValue::Null,
            texture: 0,
            mysim_texture: RefValue::Null,
        }
    }

    pub fn set_ui_state(&mut self, value: i32) {
        self.set_state(0, value);
    }
    pub fn set_dispatch_state(&mut self, value: i32) {
        self.set_state(1, value);
    }
    pub fn set_dispatch_type(&mut self, value: i32) {
        self.set_state(2, value);
    }
    pub fn dispatch_type(&self) -> i32 {
        self.record.states[2]
    }
    fn set_state(&mut self, index: usize, value: i32) {
        if self.record.states[index] != value {
            self.record.states[index] = value;
            self.record.dirty = true;
            self.record.blink = false;
        }
    }
    pub fn set_bitmap_info(&mut self, bitmap: i32, text: &[u8]) {
        self.record.bitmap = bitmap;
        self.record.text = text.to_vec();
        self.record.dirty = true;
    }
    pub fn set_call_number(&mut self, value: u16) {
        self.record.call_number = value;
        self.record.dirty = true;
    }
    pub fn get_call_number(&self) -> u16 {
        self.record.call_number
    }
    pub fn set_drag_state(&mut self, value: u8) {
        self.record.drag = value;
    }
    pub fn get_drag_state(&self) -> u8 {
        self.record.drag
    }
    pub fn source_occupant(&self) -> &RefValue {
        &self.record.source
    }
    fn replace_ref(slot: &mut RefValue, value: RefValue, refs: &mut impl References) {
        if same_ref(slot, &value) {
            return;
        }
        if !matches!(value, RefValue::Null) {
            refs.add_ref(&value);
        }
        let old = std::mem::replace(slot, value);
        if !matches!(old, RefValue::Null) {
            refs.release(&old);
        }
    }
    pub fn set_source_occupant(&mut self, value: RefValue, refs: &mut impl References) {
        Self::replace_ref(&mut self.record.source, value, refs);
    }
    pub fn set_owner(&mut self, value: RefValue, refs: &mut impl References) {
        Self::replace_ref(&mut self.record.owner, value, refs);
    }
    pub fn animation_tick(&mut self, interval: u32, globals: &mut PylonGlobals) {
        if !matches!(self.record.states[1], 1 | 4) {
            return;
        }
        globals.blink_accumulator = globals.blink_accumulator.wrapping_add(interval);
        if globals.blink_accumulator >= 500 {
            globals.blink_accumulator = 0;
            self.record.blink = !self.record.blink;
            self.record.dirty = true;
        }
    }
    pub fn init(&mut self, globals: &mut PylonGlobals, host: &mut impl PylonHost) -> bool {
        if self.initialized {
            return true;
        }
        host.occupant_init(&mut self.record.base);
        self.initialized = true;
        globals.field_b23b78 = true;
        for id in PYLON_MESSAGES {
            host.message(id, true);
        }
        host.animation(
            ANIMATION_TICK_MANAGER_CLASS,
            ANIMATION_TICK_MANAGER_IID,
            PYLON_TICK,
            true,
        );
        if host.renderer_exists() {
            host.renderer_probe();
            for (state, args) in self.packed_states.iter_mut().zip(PYLON_RENDER_STATES) {
                *state = host.pack(args);
            }
        }
        true
    }
    pub fn place_in_manager(&mut self, insert: bool, host: &mut impl PylonHost) -> bool {
        let manager = host.city_manager();
        if (self.placement_flags & 1 != 0) == insert {
            return true;
        }
        if !manager {
            return false;
        }
        if insert {
            if !host.manager_insert() {
                return false;
            }
            self.placement_flags |= 1;
        } else {
            if self.placement_flags & 2 != 0 {
                if host.renderer_exists() {
                    host.renderer_remove();
                }
                self.placement_flags &= !2;
            }
            host.manager_remove();
            self.placement_flags &= !1;
        }
        true
    }
    pub fn place_in_renderer(&mut self, insert: bool, host: &mut impl PylonHost) -> bool {
        host.city_manager();
        if (self.placement_flags & 2 != 0) == insert {
            return true;
        }
        if !host.renderer_exists() {
            return false;
        }
        if insert {
            self.place_in_manager(true, host);
            if self.placement_flags & 1 == 0 || !host.prepare_instance(4) || !host.renderer_insert() {
                return false;
            }
            self.placement_flags |= 2;
        } else {
            host.renderer_remove();
            self.placement_flags &= !2;
        }
        true
    }
    pub fn shutdown(&mut self, host: &mut impl PylonHost) -> bool {
        if !self.initialized {
            return true;
        }
        host.occupant_shutdown(&mut self.record.base);
        self.initialized = false;
        if self.placement_flags & 1 != 0 {
            self.place_in_manager(false, host);
        }
        for id in PYLON_MESSAGES {
            host.message(id, false);
        }
        host.animation(
            ANIMATION_TICK_MANAGER_CLASS,
            ANIMATION_TICK_MANAGER_IID,
            PYLON_TICK,
            false,
        );
        clear_ref(&mut self.mysim_texture, host);
        if !matches!(self.texture_binding, RefValue::Null) {
            clear_ref(&mut self.texture_binding, host);
        } else if self.texture != 0 && host.texture_manager_exists() {
            host.release_texture(self.texture);
        }
        self.texture = 0;
        clear_ref(&mut self.record.source, host);
        clear_ref(&mut self.record.owner, host);
        true
    }
    pub fn get_position(&self, out: Option<&mut [f32; 3]>) {
        if let Some(out) = out {
            *out = self.record.transform.translation;
        }
    }
    pub fn rebuild_model(&mut self) -> Result<(), PylonUnsupported> {
        Err(PylonUnsupported::ModelTextureGeometryDrawBoundsTransformMessagesNO36)
    }
    pub fn set_position(
        &mut self,
        position: Option<[f32; 3]>,
        host: &mut impl PylonHost,
    ) -> Result<(), PylonUnsupported> {
        let Some(position) = position else {
            return Ok(());
        };
        self.place_in_manager(false, host);
        self.place_in_renderer(false, host);
        self.record.transform.translation = position;
        self.record.transform.counter = self.record.transform.counter.wrapping_add(1);
        self.record.transform.mask |= 1;
        let result = self.rebuild_model();
        self.place_in_manager(true, host);
        self.place_in_renderer(true, host);
        result
    }
    pub fn set_visibility(&mut self, visible: bool, argument: u32, host: &mut impl PylonHost) {
        host.base_visibility(visible, argument, &mut self.placement_flags);
        if self.placement_flags & 1 != 0 {
            self.place_in_renderer(visible, host);
        }
    }
}

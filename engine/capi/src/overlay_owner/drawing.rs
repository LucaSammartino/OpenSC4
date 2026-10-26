// SPDX-License-Identifier: GPL-3.0-or-later
//! Private draw transport. These are FC4 callback records, never native context layouts.
use super::{native::ResourceMap, Owner};
use fc4_core::terrain_overlays::{
    CellRect, DrawCommand, DrawingProducer, FrameOptions, RectangleRoute, ShadowLighting, ShadowParameters,
    TextureBinding, TextureSource, VisibleRows,
};
use std::sync::Arc;

#[repr(C)]
struct Command {
    kind: u32,
    a: u32,
    b: u32,
    c: u32,
    data: [f32; 16],
}
#[repr(C)]
#[derive(Default)]
struct Rows {
    begin: i32,
    end: i32,
    count: u32,
    starts: *const i32,
    ends: *const i32,
}
#[repr(C)]
pub struct Frame {
    context: usize,
    flags: u32,
    reset_model: Option<unsafe extern "C" fn(usize)>,
    model_matrix: Option<unsafe extern "C" fn(usize, *mut f32)>,
    value: Option<unsafe extern "C" fn(usize, u32) -> u32>,
    vertex_extension: Option<unsafe extern "C" fn(usize, u32) -> u32>,
    rows: Option<unsafe extern "C" fn(usize, *mut Rows)>,
    command: Option<unsafe extern "C" fn(usize, *const Command)>,
    texture: Option<unsafe extern "C" fn(usize, u32, usize, usize)>,
    rectangle: Option<unsafe extern "C" fn(usize, u32, *const i32)>,
    water: Option<unsafe extern "C" fn(usize, *mut i32) -> i32>,
    shadow_parameters: Option<unsafe extern "C" fn(usize, *mut f32)>,
    strength: Option<unsafe extern "C" fn(usize, *const f32) -> f32>,
}
impl Frame {
    fn complete(&self) -> bool {
        self.reset_model.is_some()
            && self.model_matrix.is_some()
            && self.value.is_some()
            && self.vertex_extension.is_some()
            && self.rows.is_some()
            && self.command.is_some()
            && self.texture.is_some()
            && self.rectangle.is_some()
            && self.water.is_some()
            && self.shadow_parameters.is_some()
            && self.strength.is_some()
    }
    fn value(&self, index: u32) -> u32 {
        unsafe { self.value.unwrap()(self.context, index) }
    }
}
struct Drawing<'a> {
    frame: &'a Frame,
    identities: &'a ResourceMap,
    failed: bool,
}
fn words(rect: CellRect) -> [i32; 4] {
    [rect.min[0], rect.min[1], rect.max[0], rect.max[1]]
}
impl Drawing<'_> {
    fn resource<T: ?Sized>(&mut self, resource: Option<&Arc<T>>, kind: u32) -> Option<usize> {
        let Some(resource) = resource else { return Some(0) };
        let token = Arc::as_ptr(resource) as *const () as usize;
        let found = self.identities.lock().ok().and_then(|map| map.get(&token).copied());
        if let Some((object, _)) = found.filter(|(_, actual)| *actual == kind) {
            Some(object)
        } else {
            // Dimension-only portable bindings do not identify a foreign texture.
            self.failed = true;
            None
        }
    }
}
impl DrawingProducer for Drawing<'_> {
    fn reset_model(&mut self) {
        unsafe { self.frame.reset_model.unwrap()(self.frame.context) };
    }
    fn model_matrix(&self) -> [f32; 16] {
        let mut matrix = [0.0; 16];
        unsafe { self.frame.model_matrix.unwrap()(self.frame.context, matrix.as_mut_ptr()) };
        matrix
    }
    fn depth_offset(&self) -> u32 {
        self.frame.value(0)
    }
    fn acquire_vertex_extension(&mut self) -> bool {
        unsafe { self.frame.vertex_extension.unwrap()(self.frame.context, 1) != 0 }
    }
    fn release_vertex_extension(&mut self) {
        unsafe { self.frame.vertex_extension.unwrap()(self.frame.context, 0) };
    }
    fn draw_rows(&mut self) -> VisibleRows {
        let mut rows = Rows::default();
        unsafe { self.frame.rows.unwrap()(self.frame.context, &mut rows) };
        if rows.count == 0 {
            return VisibleRows {
                begin: rows.begin,
                end: rows.end,
                ..VisibleRows::default()
            };
        }
        if rows.starts.is_null() || rows.ends.is_null() || rows.count as usize > isize::MAX as usize / 4 {
            self.failed = true;
            return VisibleRows::default();
        }
        VisibleRows {
            begin: rows.begin,
            end: rows.end,
            starts: unsafe { std::slice::from_raw_parts(rows.starts, rows.count as usize).to_vec() },
            ends: unsafe { std::slice::from_raw_parts(rows.ends, rows.count as usize).to_vec() },
        }
    }
    fn command(&mut self, command: DrawCommand) {
        let mut out = Command {
            kind: 0,
            a: 0,
            b: 0,
            c: 0,
            data: [0.0; 16],
        };
        match command {
            DrawCommand::DepthOffset(value) => out.a = value,
            DrawCommand::DrawState(value) => {
                out.kind = 1;
                out.a = u32::from(value);
            }
            DrawCommand::Transparency(value) => {
                out.kind = 2;
                out.data[0] = value;
            }
            DrawCommand::Shade { rgb, self_lit } => {
                out.kind = 3;
                out.a = u32::from(self_lit);
                out.data[..3].copy_from_slice(&rgb);
            }
            DrawCommand::VertexColors(a, b) => {
                out.kind = 4;
                out.a = u32::from(a);
                out.b = u32::from(b);
            }
            DrawCommand::ResetTransparency => out.kind = 5,
            DrawCommand::ResetShade => out.kind = 6,
            DrawCommand::Transform { stage, matrix } => {
                out.kind = 7;
                out.a = u32::from(stage);
                out.data = matrix;
            }
            DrawCommand::ClearTransform(stage) => {
                out.kind = 8;
                out.a = u32::from(stage);
            }
            DrawCommand::Filtering { stage, min, mip } => {
                out.kind = 9;
                out.a = u32::from(stage);
                out.b = u32::from(min);
                out.c = u32::from(mip);
            }
            DrawCommand::Wrap { stage, modes } => {
                out.kind = 10;
                out.a = u32::from(stage);
                out.b = u32::from(modes[0]);
                out.c = u32::from(modes[1]);
            }
            DrawCommand::ShadowCombiner { stage, descriptor } => {
                out.kind = 11;
                out.a = u32::from(stage);
                out.b = u32::from(descriptor);
            }
            DrawCommand::TextureEnabled { stage, enabled } => {
                out.kind = 12;
                out.a = u32::from(stage);
                out.b = u32::from(enabled);
            }
            DrawCommand::Environment { rgb, fourth } => {
                out.kind = 13;
                out.a = u32::from(fourth.is_some());
                out.data[..3].copy_from_slice(&rgb);
                out.data[3] = fourth.unwrap_or(0.0);
            }
            DrawCommand::AlphaTest(reference) => {
                out.kind = 14;
                out.a = u32::from(reference.is_some());
                out.data[0] = reference.unwrap_or(0.0);
            }
            DrawCommand::DeviceFlag7(on) => {
                out.kind = 15;
                out.a = u32::from(on);
            }
        }
        unsafe { self.frame.command.unwrap()(self.frame.context, &out) };
    }
    fn bind_texture(
        &mut self,
        stage: u8,
        binding: Option<&Arc<dyn TextureBinding>>,
        source: Option<&Arc<dyn TextureSource>>,
    ) {
        let binding = self.resource(binding, 0);
        let source = self.resource(source, 1);
        if let (Some(binding), Some(source)) = (binding, source) {
            unsafe { self.frame.texture.unwrap()(self.frame.context, u32::from(stage), binding, source) };
        }
    }
    fn draw_rectangle(&mut self, route: RectangleRoute, vertices: CellRect) {
        let route = match route {
            RectangleRoute::LandStatic => 0,
            RectangleRoute::LandDynamic => 1,
            RectangleRoute::LandStaticExtension => 2,
            RectangleRoute::LandDynamicExtension => 3,
            RectangleRoute::WaterStatic => 4,
            RectangleRoute::WaterDynamic => 5,
            RectangleRoute::ShadowLandStatic => 6,
            RectangleRoute::ShadowLandDynamic => 7,
        };
        if !self.failed {
            unsafe { self.frame.rectangle.unwrap()(self.frame.context, route, words(vertices).as_ptr()) };
        }
    }
    fn water_slot53(&mut self, temporary_cells: &mut CellRect) -> i32 {
        let mut cells = words(*temporary_cells);
        let result = unsafe { self.frame.water.unwrap()(self.frame.context, cells.as_mut_ptr()) };
        *temporary_cells = CellRect {
            min: [cells[0], cells[1]],
            max: [cells[2], cells[3]],
        };
        result
    }
    fn device_flag7(&self) -> bool {
        self.frame.value(1) != 0
    }
    fn location_strength_enabled(&self) -> bool {
        self.frame.value(2) != 0
    }
}
struct Lighting<'a>(&'a Frame);
impl ShadowLighting for Lighting<'_> {
    fn parameters_slot14(&mut self) -> ShadowParameters {
        let mut values = [0.0; 5];
        unsafe { self.0.shadow_parameters.unwrap()(self.0.context, values.as_mut_ptr()) };
        ShadowParameters {
            q: values[0],
            s: values[1],
            rgb: [values[2], values[3], values[4]],
        }
    }
    fn strength_slot15(&mut self, world_xz: [f32; 2]) -> f32 {
        unsafe { self.0.strength.unwrap()(self.0.context, world_xz.as_ptr()) }
    }
}

#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_native_draw(owner: *mut Owner, frame: *const Frame) -> u32 {
    let (Some(owner), Some(frame)) = (owner.as_ref(), frame.as_ref()) else {
        return 0;
    };
    let Ok(mut core) = owner.0.lock() else { return 0 };
    let options = FrameOptions {
        snapshot: frame.flags & 1 != 0,
        decals: frame.flags & 2 != 0,
        shadows: frame.flags & 4 != 0,
        use_second_stage: frame.flags & 8 != 0,
    };
    // The early dynamic-snapshot return reads no producer input or callback.
    if options.snapshot && core.draw_frame_exact(options).is_ok() {
        return 1;
    }
    if !core.is_initialized() || !frame.complete() {
        return 0;
    }
    let mut drawing = Drawing {
        frame,
        identities: &owner.2,
        failed: false,
    };
    let result = core.draw_frame(options, &mut drawing, &mut Lighting(frame));
    u32::from(result.is_ok() && !drawing.failed)
}

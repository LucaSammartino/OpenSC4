// SPDX-License-Identifier: GPL-3.0-or-later
//! Owner draw orchestration. Terrain geometry and lighting are foreign producers.
//! Matrix arithmetic uses portable f64 intermediates with binary32 stores.
//! OV7 remains open for exact x87 inverse/cofactor and all-entry dot ordering.

use super::{flag, CellRect, ContractGap, Error, FrameOptions, OverlayManager, TextureBinding, TextureSource};
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct VisibleRows {
    pub begin: i32,
    pub end: i32,
    pub starts: Vec<i32>,
    pub ends: Vec<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RectangleRoute {
    LandStatic,
    LandDynamic,
    LandStaticExtension,
    LandDynamicExtension,
    WaterStatic,
    WaterDynamic,
    ShadowLandStatic,
    ShadowLandDynamic,
}

/// Semantic context operations; the adapter owns packed device descriptors.
#[derive(Clone, Debug, PartialEq)]
pub enum DrawCommand {
    DepthOffset(u32),
    DrawState(u8),
    Transparency(f32),
    Shade {
        rgb: [f32; 3],
        self_lit: bool,
    },
    VertexColors(bool, bool),
    ResetTransparency,
    ResetShade,
    Transform {
        stage: u8,
        matrix: [f32; 16],
    },
    ClearTransform(u8),
    Filtering {
        stage: u8,
        min: u8,
        mip: u8,
    },
    Wrap {
        stage: u8,
        modes: [u8; 2],
    },
    /// Full-shadow descriptors +0x38, +0x44, +0x50, respectively.
    ShadowCombiner {
        stage: u8,
        descriptor: u8,
    },
    TextureEnabled {
        stage: u8,
        enabled: bool,
    },
    Environment {
        rgb: [f32; 3],
        fourth: Option<f32>,
    },
    /// Some(reference): comparison 6, greater-or-equal. None: disabled.
    AlphaTest(Option<f32>),
    DeviceFlag7(bool),
}

#[derive(Clone, Copy, Debug)]
pub struct ShadowParameters {
    pub q: f32,
    pub s: f32,
    pub rgb: [f32; 3],
}

/// Lighting slots 14/15; their interpolation/terrain algorithms are OV8.
pub trait ShadowLighting {
    fn parameters_slot14(&mut self) -> ShadowParameters;
    fn strength_slot15(&mut self, world_xz: [f32; 2]) -> f32;
}

/// Raw foreign inputs and terrain-view rectangle consumers, never preselected
/// IDs, completed transforms, counter deltas, or computed owner bounds.
pub trait DrawingProducer {
    fn reset_model(&mut self);
    fn model_matrix(&self) -> [f32; 16];
    fn depth_offset(&self) -> u32;
    /// Acquires a reference if present; paired exactly once on common exit.
    fn acquire_vertex_extension(&mut self) -> bool;
    fn release_vertex_extension(&mut self);
    fn draw_rows(&mut self) -> VisibleRows;
    fn command(&mut self, command: DrawCommand);
    fn bind_texture(
        &mut self,
        stage: u8,
        binding: Option<&Arc<dyn TextureBinding>>,
        source: Option<&Arc<dyn TextureSource>>,
    );
    fn draw_rectangle(&mut self, route: RectangleRoute, vertices: CellRect);
    fn water_slot53(&mut self, temporary_cells: &mut CellRect) -> i32;
    fn device_flag7(&self) -> bool;
    fn location_strength_enabled(&self) -> bool;
}

impl OverlayManager {
    /// Executes the settled owner protocol, with portable numerical evaluation.
    /// This is not a claim of bit-exact OV7 rendering or of foreign geometry.
    pub fn draw_frame(
        &mut self,
        options: FrameOptions,
        drawing: &mut dyn DrawingProducer,
        lighting: &mut dyn ShadowLighting,
    ) -> Result<(), Error> {
        self.require_init()?;
        if self.dynamic && options.snapshot {
            return Ok(());
        }
        drawing.reset_model();
        let camera = inverse(drawing.model_matrix());
        let saved_depth = drawing.depth_offset();
        let extension = drawing.acquire_vertex_extension();
        let result = self.draw_selected(options, drawing, lighting, camera, extension);
        drawing.command(DrawCommand::ClearTransform(0));
        drawing.command(DrawCommand::DepthOffset(saved_depth));
        if extension {
            drawing.release_vertex_extension();
        }
        result
    }

    /// Explicitly unavailable exact arithmetic, rather than treating a library
    /// inverse as a proved native implementation (spec §4.7, OV7).
    pub fn draw_frame_exact(&self, options: FrameOptions) -> Result<(), Error> {
        self.require_init()?;
        if self.dynamic && options.snapshot {
            Ok(())
        } else {
            Err(Error::Gap(ContractGap::WorldTextureTransformOv7))
        }
    }

    fn draw_selected(
        &mut self,
        options: FrameOptions,
        drawing: &mut dyn DrawingProducer,
        lighting: &mut dyn ShadowLighting,
        camera: [f32; 16],
        extension: bool,
    ) -> Result<(), Error> {
        let ids = if !self.dynamic && self.selection_byte != 0 {
            let rows = drawing.draw_rows();
            self.collect_visible_rows(rows.begin, rows.end, &rows.starts, &rows.ends)?
        } else {
            self.overlays.iter().map(|(id, _)| id).collect()
        };
        if ids.is_empty() {
            return Ok(());
        }
        if options.decals {
            self.decals_drawn = 0;
            drawing.command(DrawCommand::DepthOffset(2));
            for &id in &ids {
                let Some(o) = self.overlays.get(id).filter(|o| o.decal_visible(options.snapshot)) else {
                    continue;
                };
                drawing.command(DrawCommand::Transparency(o.alpha));
                let self_lit = o.flags & flag::SELF_LIT != 0;
                drawing.command(DrawCommand::Shade { rgb: o.color, self_lit });
                drawing.command(DrawCommand::VertexColors(!self_lit, !self_lit));
                drawing.command(DrawCommand::DrawState(o.draw_state));
                drawing.bind_texture(0, o.binding.as_ref(), o.source.as_ref());
                // §4.2 names the filter semantics; native device enum mapping is adapter-owned.
                drawing.command(DrawCommand::Filtering {
                    stage: 0,
                    min: 1,
                    mip: 1,
                });
                let modes = if o.flags & flag::REPEAT != 0 {
                    [3, 3]
                } else if o.flags & flag::TEXTURE_MATRIX != 0 {
                    [3, 2]
                } else {
                    [2, 2]
                };
                drawing.command(DrawCommand::Wrap { stage: 0, modes });
                drawing.command(DrawCommand::Transform {
                    stage: 0,
                    matrix: compose(camera, o.primary),
                });
                self.submit(drawing, o.cells, extension, false);
                self.decals_drawn = self.decals_drawn.wrapping_add(1);
            }
            drawing.command(DrawCommand::ResetTransparency);
            drawing.command(DrawCommand::ResetShade);
            drawing.command(DrawCommand::VertexColors(true, true));
        }
        if options.shadows {
            let full = self.shadow_quality > 2 && options.use_second_stage;
            let debug = self.shadow_debug == 4;
            let old_flag = drawing.device_flag7();
            self.shadows_drawn = 0;
            drawing.command(DrawCommand::DepthOffset(3));
            if !full {
                drawing.command(DrawCommand::DeviceFlag7(true));
            }
            let p = lighting.parameters_slot14();
            drawing.command(DrawCommand::Environment {
                rgb: p.rgb,
                fourth: None,
            });
            drawing.command(DrawCommand::AlphaTest(if p.s > 0.0 {
                Some((f64::from(p.q) * f64::from(p.s)) as f32)
            } else {
                None
            }));
            if full {
                drawing.command(DrawCommand::Filtering {
                    stage: 0,
                    min: 1,
                    mip: 1,
                });
                drawing.command(DrawCommand::ShadowCombiner {
                    stage: 0,
                    descriptor: 0x38,
                });
                if let Some(binding) = &self.shadow_binding {
                    let stage = if debug { 0 } else { 1 };
                    drawing.command(DrawCommand::TextureEnabled { stage, enabled: true });
                    drawing.bind_texture(stage, Some(binding), None);
                    drawing.command(DrawCommand::Filtering { stage, min: 0, mip: 0 });
                    drawing.command(DrawCommand::Wrap { stage, modes: [2, 2] });
                    drawing.command(DrawCommand::ShadowCombiner {
                        stage,
                        descriptor: if debug { 0x50 } else { 0x44 },
                    });
                }
            }
            let result = self.draw_shadows(&ids, drawing, lighting, camera, extension, full, debug, p);
            if full {
                drawing.command(DrawCommand::TextureEnabled {
                    stage: 1,
                    enabled: false,
                });
            } else if !old_flag {
                drawing.command(DrawCommand::DeviceFlag7(false));
            }
            result?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_shadows(
        &mut self,
        ids: &[u32],
        drawing: &mut dyn DrawingProducer,
        lighting: &mut dyn ShadowLighting,
        camera: [f32; 16],
        extension: bool,
        full: bool,
        debug: bool,
        p: ShadowParameters,
    ) -> Result<(), Error> {
        for &id in ids {
            let Some(o) = self.overlays.get(id).filter(|o| o.flags & 9 == 9) else {
                continue;
            };
            if !full || !debug {
                drawing.bind_texture(0, o.binding.as_ref(), None);
                drawing.command(DrawCommand::Wrap {
                    stage: 0,
                    modes: if o.flags & flag::REPEAT != 0 { [3, 3] } else { [2, 2] },
                });
                drawing.command(DrawCommand::Transform {
                    stage: 0,
                    matrix: compose(camera, o.primary),
                });
            }
            if full {
                let secondary = o.secondary.ok_or(Error::Gap(ContractGap::WorldTextureTransformOv7))?;
                drawing.command(DrawCommand::Transform {
                    stage: if debug { 0 } else { 1 },
                    matrix: compose(camera, secondary),
                });
                if !debug && drawing.location_strength_enabled() {
                    let width = self
                        .resources
                        .terrain
                        .as_ref()
                        .ok_or(Error::NotInitialized)?
                        .cell_width();
                    let midpoint = [0, 1].map(|axis| {
                        ((f64::from(width) * 0.5) * f64::from(o.cells.min[axis].wrapping_add(o.cells.max[axis]))) as f32
                    });
                    let q = if self.water {
                        p.q
                    } else {
                        (f64::from(lighting.strength_slot15(midpoint)) * f64::from(p.q)) as f32
                    };
                    if f64::from(q) < 0.05 {
                        continue;
                    }
                    if p.s > 0.0 {
                        drawing.command(DrawCommand::AlphaTest(Some((f64::from(q) * f64::from(p.s)) as f32)));
                    }
                    drawing.command(DrawCommand::Environment {
                        rgb: p.rgb,
                        fourth: Some(q),
                    });
                }
            }
            let shadow = if full && self.water {
                let mut temporary = o.cells;
                let r = drawing.water_slot53(&mut temporary);
                let product = o.cells.max[0]
                    .wrapping_sub(o.cells.min[0])
                    .wrapping_add(1)
                    .wrapping_mul(o.cells.max[1].wrapping_sub(o.cells.min[1]).wrapping_add(1));
                r < product
            } else {
                false
            };
            self.submit(drawing, o.cells, extension, shadow);
            self.shadows_drawn = self.shadows_drawn.wrapping_add(1);
        }
        Ok(())
    }

    fn submit(&self, drawing: &mut dyn DrawingProducer, cells: CellRect, extension: bool, shadow: bool) {
        let route = if shadow {
            if self.dynamic {
                RectangleRoute::ShadowLandDynamic
            } else {
                RectangleRoute::ShadowLandStatic
            }
        } else if self.water {
            if self.dynamic {
                RectangleRoute::WaterDynamic
            } else {
                RectangleRoute::WaterStatic
            }
        } else {
            match (self.dynamic, extension) {
                (false, false) => RectangleRoute::LandStatic,
                (true, false) => RectangleRoute::LandDynamic,
                (false, true) => RectangleRoute::LandStaticExtension,
                (true, true) => RectangleRoute::LandDynamicExtension,
            }
        };
        drawing.draw_rectangle(
            route,
            CellRect {
                min: cells.min,
                max: cells.max.map(|v| v.wrapping_add(1)),
            },
        );
    }
}

/// C×M, binary32 results. Specified row-3 U/V orders are retained; remaining
/// all-entry order is explicitly OV7, evaluated left-to-right here.
pub(super) fn compose(c: [f32; 16], m: [f32; 16]) -> [f32; 16] {
    std::array::from_fn(|i| {
        let (row, col) = (i / 4, i % 4);
        let order = match (row, col) {
            (3, 0) => [2, 0, 1, 3],
            (3, 1) => [3, 1, 2, 0],
            _ => [0, 1, 2, 3],
        };
        let terms = order.map(|k| f64::from(c[row * 4 + k]) * f64::from(m[k * 4 + col]));
        (((terms[0] + terms[1]) + terms[2]) + terms[3]) as f32
    })
}

/// Cofactor inverse with reciprocal stored binary32. No singular guard or
/// identity fallback; exact native cofactor/spill ordering remains OV7.
fn inverse(m: [f32; 16]) -> [f32; 16] {
    let cofactor = |row: usize, col: usize| {
        let mut minor = [0.0; 9];
        let mut n = 0;
        for r in 0..4 {
            for c in 0..4 {
                if r != row && c != col {
                    minor[n] = f64::from(m[r * 4 + c]);
                    n += 1;
                }
            }
        }
        let [a, b, c, d, e, f, g, h, i] = minor;
        let value = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
        if (row + col) % 2 == 0 {
            value
        } else {
            -value
        }
    };
    let cof: [f64; 16] = std::array::from_fn(|i| cofactor(i / 4, i % 4));
    let det = (0..4).fold(0.0, |sum, i| sum + f64::from(m[i]) * cof[i]);
    let reciprocal = (1.0 / det) as f32;
    std::array::from_fn(|i| (cof[(i % 4) * 4 + i / 4] * f64::from(reciprocal)) as f32)
}

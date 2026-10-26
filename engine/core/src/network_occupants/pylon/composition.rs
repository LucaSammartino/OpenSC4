//! NO36: owner-side composition and billboard work, network §6.2.3–6.2.4.
use super::*;
use crate::network_occupants::xform::IDENTITY_MATRIX;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BillboardVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BitmapInfo {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub left: u32,
    pub top: u32,
}

/// Acquisitions return owned references; locks expose bytes, never a precomposed image.
pub trait BitmapHost: References {
    fn load_bitmap(&mut self, key: [u32; 3], format: u32, immediate: bool) -> RefValue;
    fn portrait_service(&mut self, class: u32, iid: u32) -> RefValue;
    fn portrait(&mut self, service: &RefValue, name: &[u8], argument: u32) -> RefValue;
    fn bitmap_info(&mut self, bitmap: &RefValue) -> BitmapInfo;
    fn lock_bitmap(&mut self, bitmap: &RefValue, flags: u32) -> bool;
    fn unlock_bitmap(&mut self, bitmap: &RefValue, flags: u32);
    fn pixel(&mut self, bitmap: &RefValue, byte_offset: u32) -> u32;
    fn put_pixel(&mut self, bitmap: &RefValue, byte_offset: u32, pixel: u32);
    /// SetupTexture(bitmap, manager, &id, 1,0,0,0).
    fn setup_texture(&mut self, bitmap: &RefValue, id: &mut u32) -> bool;
    fn font(&mut self, resource: u32) -> RefValue;
    fn font_setting(&mut self, font: &RefValue) -> u32;
    fn set_font_setting(&mut self, font: &RefValue, setting: u32);
    fn measure_text(&mut self, font: &RefValue, slot: u32, text: &[u8]) -> i32;
    fn acquire_bitmap(&mut self) -> RefValue;
    fn create_bitmap(&mut self, bitmap: &RefValue, side: u32, descriptor: [u32; 2]) -> bool;
    fn clear_bitmap(&mut self, bitmap: &RefValue, value: u32, argument: u32);
    /// Font +CC(bitmap,0,0,text,length,FF000000,08000000,0,0).
    fn draw_text(&mut self, font: &RefValue, bitmap: &RefValue, text: &[u8], colours: [u32; 2]);
}

pub trait PylonModelHost: PylonTextureHost {
    fn viewport_to_world(&mut self, distance: f32) -> f64;
    fn pitch(&mut self) -> f64;
    /// View +54 and +5C, before negation.
    fn facing(&mut self) -> [f64; 2];
    fn renderer_refresh_remove(&mut self);
    fn renderer_refresh_insert(&mut self, argument: u32);
    fn manager_changed(&mut self, argument: u32);
}

pub trait PylonDrawHost: PylonModelHost {
    fn install_transform(&mut self, transform: Xform);
    fn clear_texture_transform(&mut self, stage: u32);
    fn transparency(&mut self, value: Option<f32>);
    fn render_state(&mut self, state: u64);
    fn draw_quad(&mut self, texture: u32, vertices: [BillboardVertex; 4], primitive: u32, format: u32);
}

impl Pylon {
    pub fn compose_mysim(&mut self, host: &mut impl PylonTextureHost) -> Result<bool, PylonUnsupported> {
        let state = self.record.states[1];
        if !(0..=6).contains(&state) {
            return Err(PylonUnsupported::TextureStateOutsideTable);
        }
        let id = if state == 1 { 0x2BB0_6F3F } else { 0x2BB0_75B4 };
        let background = host.load_bitmap([0x856D_DBAC, 0xAB7E_5421, id], 9, true);
        if matches!(background, RefValue::Null) {
            return Ok(false);
        }
        let mut service = RefValue::Null;
        let portrait = if self.record.bitmap != 0 {
            host.load_bitmap([0x856D_DBAC, 0x46A0_06B0, self.record.bitmap as u32], 9, true)
        } else {
            service = host.portrait_service(0xA2FF_B5D3, 0x02FF_BB60);
            if matches!(service, RefValue::Null) {
                host.release(&background);
                return Ok(false);
            }
            host.portrait(&service, &self.record.text, 8)
        };
        if !matches!(portrait, RefValue::Null) {
            let src = host.bitmap_info(&portrait);
            let dst = host.bitmap_info(&background);
            let d = 52u32.wrapping_sub(src.width) >> 1;
            for n in [52, 104, 156] {
                let source_locked = host.lock_bitmap(&portrait, 0x8040);
                let destination_locked = host.lock_bitmap(&background, 0x8040);
                // SPEC-GAP: failed bitmap locks have no pixel contract; skip that copy.
                if source_locked && destination_locked {
                    for y in 0..src.height {
                        for x in 0..src.width {
                            let offset = (src.top.wrapping_add(d / 2).wrapping_add(y))
                                .wrapping_mul(dst.stride)
                                .wrapping_add(src.left.wrapping_add(d).wrapping_add(n).wrapping_add(x).wrapping_mul(4));
                            let source =
                                host.pixel(&portrait, y.wrapping_mul(src.stride).wrapping_add(x.wrapping_mul(4)));
                            if host.pixel(&background, offset) & 0xC000_0000 == 0 && source & 0x00FF_FFFF != 0x00FF_00FF
                            {
                                host.put_pixel(&background, offset, source);
                            }
                        }
                    }
                }
                if destination_locked {
                    host.unlock_bitmap(&background, 0x8040);
                }
                if source_locked {
                    host.unlock_bitmap(&portrait, 0x8040);
                }
            }
            host.release(&portrait);
        }
        let result = host.setup_texture(&background, &mut self.texture);
        if !matches!(service, RefValue::Null) {
            host.release(&service);
        }
        host.release(&background);
        Ok(result)
    }

    pub fn rebuild_label(&mut self, _width: f32, height: f32, host: &mut impl PylonModelHost) {
        let font = host.font(0x6896_3C4C);
        // SPEC-GAP: no font-acquisition failure behavior is specified; preserve the label.
        if matches!(font, RefValue::Null) {
            return;
        }
        let setting = host.font_setting(&font);
        host.set_font_setting(&font, 3);
        let text = self.record.call_number.to_string().into_bytes();
        let a = host.measure_text(&font, 0xC0, &text);
        let b = host.measure_text(&font, 0x94, &text);
        let side = if a <= 32 && b <= 32 {
            32
        } else {
            (a.max(b) as u32).next_power_of_two()
        };
        if self.label_texture != 0 {
            host.release_texture(self.label_texture);
        }
        self.label_texture = 0;
        let bitmap = host.acquire_bitmap();
        if !matches!(bitmap, RefValue::Null) {
            if host.create_bitmap(&bitmap, side, [9, 32]) {
                if host.lock_bitmap(&bitmap, 0x8001) {
                    host.clear_bitmap(&bitmap, 0, 0);
                    host.unlock_bitmap(&bitmap, 0x8001);
                }
                host.draw_text(&font, &bitmap, &text, [0xFF00_0000, 0x0800_0000]);
                if host.texture_manager_exists() {
                    host.setup_texture(&bitmap, &mut self.label_texture);
                }
            }
            host.release(&bitmap);
        }
        let cosine = host.pitch().cos();
        let r = (f64::from(host.viewport_to_world(14.0) as f32) / cosine) as f32;
        let j = host.viewport_to_world(9.0) as f32;
        let q = host.viewport_to_world(a as f32) * 0.5;
        let y = f64::from(height) - f64::from(r) - f64::from(j) / cosine;
        let u = 1.0 / side as f32;
        let right = (f64::from(a as f32) * f64::from(u)) as f32;
        let bottom = (f64::from(b) * f64::from(u)) as f32;
        self.label_vertices = [
            BillboardVertex {
                position: [-q as f32, y as f32, f32::from_bits(0x3C23_D70A)],
                uv: [u * 0.0, u * 0.0],
            },
            BillboardVertex {
                position: [-q as f32, (y - f64::from(r)) as f32, f32::from_bits(0x3C23_D70A)],
                uv: [u * 0.0, bottom],
            },
            BillboardVertex {
                position: [q as f32, (y - f64::from(r)) as f32, f32::from_bits(0x3C23_D70A)],
                uv: [right, bottom],
            },
            BillboardVertex {
                position: [q as f32, y as f32, f32::from_bits(0x3C23_D70A)],
                uv: [right, u * 0.0],
            },
        ];
        host.set_font_setting(&font, setting);
        host.release(&font);
    }

    pub fn rebuild_model(&mut self, host: &mut impl PylonModelHost) -> Result<(), PylonUnsupported> {
        let width = host.viewport_to_world(44.0) as f32;
        let a = width * 0.5;
        let height = (host.viewport_to_world(150.0) / host.pitch().cos()) as f32;
        let texture_result = self.rebuild_current_texture(host);
        let k = if self.record.blink {
            3
        } else if self.record.states[0] == 0 {
            1
        } else {
            self.record.states[0]
        };
        let (s, b) = if self.dispatch_type() == 4 {
            (0.203125f64, f32::from_bits(0x3F31_45D1))
        } else {
            (0.171875, 0.5859375)
        };
        let (u0, u1) = ((f64::from(k) * s) as f32, ((f64::from(k) + 1.0) * s) as f32);
        self.main_vertices = [
            BillboardVertex {
                position: [-a, height, 0.0],
                uv: [u0, 0.0],
            },
            BillboardVertex {
                position: [-a, 0.0, 0.0],
                uv: [u0, b],
            },
            BillboardVertex {
                position: [a, 0.0, 0.0],
                uv: [u1, b],
            },
            BillboardVertex {
                position: [a, height, 0.0],
                uv: [u1, 0.0],
            },
        ];
        let transform = &mut self.record.transform;
        if transform.scale.to_bits() != 1.0f32.to_bits() {
            transform.scale = 1.0;
            transform.mask &= !2;
            transform.counter = transform.counter.wrapping_add(1);
        }
        if transform.mask & 4 != 0 {
            transform.matrix = IDENTITY_MATRIX;
            transform.mask &= !4;
            transform.counter = transform.counter.wrapping_add(1);
        }
        let [view_x, view_z] = host.facing();
        let angle = (-view_x).atan2(-view_z);
        let c = f64::from(angle.cos() as f32);
        let t = angle.sin();
        for i in 0..3 {
            let x = f64::from(transform.matrix[i * 3]);
            let z = f64::from(transform.matrix[i * 3 + 2]);
            transform.matrix[i * 3] = if i == 0 {
                (x * c - z * t) as f32
            } else {
                (x * c) as f32 - (z * t) as f32
            };
            transform.matrix[i * 3 + 2] = if i == 2 {
                (x * t + f64::from((z * c) as f32)) as f32
            } else {
                (x * t) as f32 + (z * c) as f32
            };
        }
        transform.mask |= 4;
        transform.counter = transform.counter.wrapping_add(1);
        self.auxiliary[0] = [-a, 0.0, 0.0, a, height, 0.0];
        let m = transform.matrix.map(f64::from);
        let a = f64::from(a);
        let h = f64::from(height);
        let scale = f64::from(transform.scale);
        let p = transform.translation.map(f64::from);
        let first = [
            (-a * m[0] + (m[2] + m[1]) * 0.0) * scale + p[0],
            f64::from((-a * m[3] + (m[5] + m[4]) * 0.0) as f32) * scale + p[1],
            f64::from((f64::from((-a * m[6] + (m[8] + m[7]) * 0.0) as f32) * scale) as f32) + p[2],
        ];
        let first = [f64::from(first[0] as f32), f64::from(first[1] as f32), first[2]];
        let second = [
            (f64::from((((h * m[1] + a * m[0]) + m[2] * 0.0) * scale) as f32) + p[0]) as f32,
            ((a * m[3] + (m[5] * 0.0 + h * m[4])) * scale + p[1]) as f32,
            (f64::from((a * m[6] + (m[8] * 0.0 + h * m[7])) as f32) * scale + p[2]) as f32,
        ]
        .map(f64::from);
        self.auxiliary[1] = std::array::from_fn(|i| {
            if i < 3 {
                first[i].min(second[i]) as f32
            } else {
                first[i - 3].max(second[i - 3]) as f32
            }
        });
        if self.record.call_number != 0 {
            self.rebuild_label(width, height, host);
        }
        if self.placement_flags & 2 != 0 {
            host.renderer_refresh_remove();
            host.renderer_refresh_insert(3);
            if self.placement_flags & 1 != 0 {
                host.manager_changed(0);
            }
        } else {
            if self.placement_flags & 1 != 0 {
                host.manager_changed(0);
            } else {
                self.place_in_manager(true, host);
            }
            if self.placement_flags & 1 != 0 {
                self.place_in_renderer(true, host);
            }
        }
        self.record.dirty = false;
        texture_result.map(|_| ())
    }

    pub fn draw(
        &mut self,
        pass: u32,
        self_lit: bool,
        context_gate: bool,
        host: &mut impl PylonDrawHost,
    ) -> Result<u32, PylonUnsupported> {
        if self_lit || context_gate {
            return Ok(1);
        }
        let label =
            !matches!(self.record.states[1], 1 | 4) && self.record.call_number != 0 && self.dispatch_type() != 4;
        if pass > 1 {
            return Ok(1);
        }
        if pass == 0 && self.record.dirty {
            self.rebuild_model(host)?;
        }
        host.install_transform(self.record.transform);
        host.clear_texture_transform(0);
        host.transparency((pass == 0).then_some(f32::from_bits(0x3ECC_CCCD)));
        host.render_state(self.packed_states[usize::from(pass == 0)]);
        host.draw_quad(self.texture, self.main_vertices, 6, 2);
        if pass == 0 {
            host.transparency(None);
        } else if label {
            host.draw_quad(self.label_texture, self.label_vertices, 6, 2);
        }
        Ok(if pass == 0 { 2 } else { 1 })
    }

    pub fn do_message(
        &mut self,
        id: u32,
        animation_millis: u32,
        filter_accepts: bool,
        globals: &mut PylonGlobals,
        host: &mut impl PylonModelHost,
    ) -> Result<bool, PylonUnsupported> {
        match id {
            0x86AD_10EE => self.animation_tick(animation_millis / 1000, globals),
            0x26D3_1EC1 => {
                self.rebuild_model(host)?;
            }
            0xA6B7_9602 => self.record.dirty = true,
            0x06CD_B65B if filter_accepts != (self.placement_flags & 2 != 0) => {
                if filter_accepts {
                    self.placement_flags |= 2;
                    host.renderer_insert();
                } else {
                    self.placement_flags &= !2;
                    host.renderer_remove();
                }
            }
            _ => {}
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests;

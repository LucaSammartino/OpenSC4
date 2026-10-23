// SPDX-License-Identifier: GPL-3.0-or-later
//! Qualified Â§Â§4.1/5/6 algorithms, not a complete owner or native entry ABI.
//! Arithmetic is deliberately restricted to exact stages (AS7); no x87 mode,
//! hardware trig, factory defaults or native fault behavior is emulated.
use super::numeric::X;
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    Arithmetic,
    Holder,
    Selection,
    Packing,
    Texture,
    MissingCache,
}

/// Already decoded/scaled stored metrics, not a selected host frame oracle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StoredMetric {
    pub page: u32,
    pub width: f32,
    pub height: f32,
    pub offset: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MetresPerPixel {
    Utilities(f32),
    /// Foreign GetViewVolume endpoints AFTER its binary32 stores.
    ViewVolume {
        left: f32,
        right: f32,
        context_width: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewInputs {
    pub matrix: [[f32; 3]; 3],
    pub mpp: MetresPerPixel,
    pub magnification: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CachedQuad {
    pub corners: [Vec3; 4],
    pub bounds: Bounds,
}

// Every nonzero operand/result is restricted to [2^-12,2^12] in magnitude.
// Therefore two f32 operands' sum/difference/product are exact in f64 (at
// most 48 significant bits). f64 is a proof/checking carrier here, NOT a
// replacement for native extended arithmetic. Reject any inexact f32 stage.
// This intentionally rejects many valid native inputs; AS7 remains partial.
fn finite_operand(x: f32) -> Result<f32, Unsupported> {
    if x.is_finite() {
        Ok(x)
    } else {
        Err(Unsupported::Arithmetic)
    }
}
fn plus(a: f32, b: f32) -> Result<f32, Unsupported> {
    finite_operand(X::f(finite_operand(a)?).add(X::f(finite_operand(b)?)).store())
}
fn times(a: f32, b: f32) -> Result<f32, Unsupported> {
    finite_operand(X::f(finite_operand(a)?).mul(X::f(finite_operand(b)?)).store())
}

/// Â§5.1's owned dependency/sign/order, exact-stage subset only. Requiring
/// exactness even for unstored native transients makes PC/RC immaterial here.
/// Larger/inexact/tiny/overflow/nonfinite regions return Unsupported, rather
/// than promising native bits from blanket f32/f64 arithmetic.
pub fn update_view_exact(metric: StoredMetric, view: ViewInputs) -> Result<CachedQuad, Unsupported> {
    for value in view.matrix.into_iter().flatten().chain([
        metric.width,
        metric.height,
        metric.offset[0],
        metric.offset[1],
        view.magnification,
    ]) {
        finite_operand(value)?;
    }
    let m = match view.mpp {
        MetresPerPixel::Utilities(m) => finite_operand(m)?,
        MetresPerPixel::ViewVolume {
            left,
            right,
            context_width,
        } => {
            if context_width == 0 {
                return Err(Unsupported::Arithmetic);
            }
            finite_operand(
                X::f(right)
                    .sub(X::f(left))
                    .div(X::integer(i64::from(context_width)))
                    .store(),
            )?
        }
    };
    let r = view.matrix.map(|row| X::f(row[0]));
    let b = view.matrix.map(|row| X::f(row[2]));
    let uy = view.matrix[1][1];
    if uy == 0.0 {
        return Err(Unsupported::Arithmetic);
    }
    let [ox, oy] = metric.offset.map(X::f);
    let d = X::f(metric.height).sub(oy).b();
    let g = X::f(view.magnification);
    let t = X::f(m).div(X::f(uy)).mul(g).b();
    let h = [
        r[0].mul(X::f(m)).b().mul(g).b(),
        r[1].mul(X::f(m)).mul(g).b(),
        r[2].mul(X::f(m)).mul(g).b(),
    ];
    let displacement = if d.ge(X::f(0.)) {
        if view.matrix[1][2] == 0. {
            return Err(Unsupported::Arithmetic);
        }
        let a = t.mul(d).div(b[1]);
        [b[0].mul(a), b[1].mul(a).b(), b[2].mul(a).b()]
    } else {
        [X::f(0.); 3]
    };
    // Literal B boundaries named in Â§5.1; remaining numeric qualification AS7.
    let w = X::f(metric.width).sub(ox);
    let upper_y = displacement[1].add(t.mul(oy));
    let lower_y = displacement[1].sub(t.mul(d)).b();
    let left_x = displacement[0].sub(h[0].mul(ox)).b().store();
    let left_z = displacement[2].sub(h[2].mul(ox).b()).b().store();
    let right_x = displacement[0].add(h[0].mul(w).b()).store();
    let right_z = displacement[2].add(h[2].mul(w).b()).store();
    let corners = [
        [left_x, upper_y.sub(h[1].mul(ox).b()).store(), left_z],
        [left_x, lower_y.sub(h[1].mul(ox).b()).store(), left_z],
        [right_x, lower_y.add(h[1].mul(w).b()).store(), right_z],
        [right_x, upper_y.add(h[1].mul(w).b()).store(), right_z],
    ];
    for x in corners.into_iter().flatten() {
        finite_operand(x)?;
    }
    // Literal reset/origin/v0/v3/reflected-M, not all four corners.
    let mut bounds = Bounds {
        min: [0.0; 3],
        max: [0.0; 3],
    };
    bounds.widen(&[corners[0], corners[3]]);
    let m = bounds.max;
    bounds.widen(&[[-m[0], m[1], -m[2]]]);
    Ok(CachedQuad { corners, bounds })
}

/// Rust-owned interleaved data, no borrowed game layout or serialized schema.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawVertex {
    pub position: Vec3,
    pub colour: u32,
    pub uv: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteCache {
    pub metric: StoredMetric,
    pub local: [Vec3; 4],
    pub vertices: [DrawVertex; 4],
    pub view_dirty: bool,
    pub counter: u8,
}

/// Raw borrowed resource consumer fields (Â§4.1), not file/factory parsing.
#[derive(Clone, Debug)]
pub struct RawSpriteHolder {
    pub divisors: [u32; 3],
    pub angle_limits: [f32; 4],
    pub entries: Vec<[u8; 8]>,
    pub scale: f32,
}

/// Â§4.1 valid-packing subset. Malformed records are not repaired into an atlas.
pub fn decode_entry_exact(entry: [u8; 8], frame: u8, scale: f32) -> Result<(StoredMetric, [[f32; 2]; 4]), Unsupported> {
    let side = 1_u32 << (entry[1] & 31);
    let width = i16::from(entry[4] as i8).unsigned_abs() as u32;
    let height = i16::from(entry[5] as i8).unsigned_abs() as u32;
    let mut x = u32::from(entry[2]);
    let mut y = u32::from(entry[3]);
    if width == 0 || height == 0 || width > side || height > side || x > side - width || y > side - height {
        return Err(Unsupported::Packing);
    }
    let mut page = u32::from(entry[0]);
    if frame != 0 {
        let mut delta = width * u32::from(frame);
        if delta + x > side - width {
            delta -= ((side - x) / width) * width;
            x = 0;
            y += height;
            if y > side - height {
                y = 0;
                page += 1;
            }
            let row_span = (side / width) * width;
            while delta > side - width {
                delta -= row_span;
                y += height;
                if y > side - height {
                    y = 0;
                    page += 1;
                }
            }
        }
        x += delta;
    }
    // Integer values <=2^20 and power-of-two division are exact binary32.
    let mut u = [x as f32 / side as f32, (x + width) as f32 / side as f32];
    let mut v = [y as f32 / side as f32, (y + height) as f32 / side as f32];
    if (entry[4] as i8) < 0 {
        u.swap(0, 1);
    }
    if (entry[5] as i8) < 0 {
        v.swap(0, 1);
    }
    // Exact-stage restriction for the native scale comparison/multiplication.
    finite_operand(scale)?;
    let scale = if X::f(scale).sub(X::f(1.)).abs().ge(X::f(f32::from_bits(0x38d1b717))) {
        scale
    } else {
        1.0
    };
    let metric = StoredMetric {
        page,
        width: times(width as f32, scale)?,
        height: times(height as f32, scale)?,
        offset: [
            times(f32::from(entry[6] as i8), scale)?,
            times(f32::from(entry[7] as i8), scale)?,
        ],
    };
    Ok((metric, [[u[0], v[0]], [u[0], v[1]], [u[1], v[1]], [u[1], v[0]]]))
}

impl RawSpriteHolder {
    /// Â§4.1 stored-addition conversion (inherited RC), not a Rust cast/round.
    pub fn convert_bin_nearest_even(value: f32) -> u32 {
        // Conditional R2-D1 primitive, explicit nearest-even RC. f64 exactly
        // holds this addition for finite small f32 inputs; final conversion is
        // the required binary32 store. Not used as a mode-free owner oracle.
        ((f64::from(value) + 12_582_912.) as f32)
            .to_bits()
            .wrapping_add(0xb4c00000)
    }
    pub fn select_angles(&self, a: f32, b: f32, rotation: i32) -> Result<usize, Unsupported> {
        if !a.is_finite() || !b.is_finite() {
            return Err(Unsupported::Arithmetic);
        }
        let h = self.derive_metrics()?;
        if h.p == 0 || h.divisor == 0 {
            return Err(Unsupported::Selection);
        }
        let convert = |v: X| {
            let stored = v.add(X::f(12_582_912.)).store();
            if stored.is_finite() {
                Ok(stored.to_bits().wrapping_add(0xb4c00000))
            } else {
                Err(Unsupported::Arithmetic)
            }
        };
        let reciprocal = X::f(f32::from_bits(0x3e22f983));
        let p = convert(
            X::f(a)
                .add(X::f(f32::from_bits(0x40490fdb)))
                .sub(X::f(h.bases[0]))
                .mul(X::integer(i64::from(h.p)))
                .mul(reciprocal),
        )? % h.p;
        let mut yaw = X::f(b).sub(X::integer(i64::from(rotation)).mul(X::f(f32::from_bits(0x3fc90fdb))));
        if !yaw.ge(X::f(0.)) {
            yaw = yaw.add(X::f(f32::from_bits(0x40c90fdb)));
        }
        if !yaw.valid() {
            return Err(Unsupported::Arithmetic);
        }
        let mut y = convert(X::integer(i64::from(h.divisor)).mul(reciprocal).mul(yaw))?;
        if y >= h.divisor {
            y %= h.divisor;
        }
        let index = p.wrapping_mul(h.r).wrapping_add(y.wrapping_mul(h.product));
        // Address arithmetic is 32-bit, including the eight-byte stride.
        Ok((index.wrapping_mul(8) / 8) as usize)
    }

    fn select_exact(&self, back: Vec3, rotation: u8, frame: u8) -> Result<(StoredMetric, [[f32; 2]; 4]), Unsupported> {
        // AS6: the native asin/atan2 argument/return/store qualification is
        // open. The exact zero-angle domain needs no substitute host trig.
        if back != [0.0, 0.0, 1.0] {
            return Err(Unsupported::Selection);
        }
        let index = self.select_angles(0., 0., i32::from(rotation))?;
        let entry = *self.entries.get(index).ok_or(Unsupported::Selection)?;
        decode_entry_exact(entry, frame, self.scale)
    }
}

#[derive(Clone, Copy)]
pub struct DrawResources<'a> {
    pub holders: [Option<&'a RawSpriteHolder>; 5],
    pub textures: &'a [TextureHandle],
}

#[derive(Clone, Copy, Debug)]
pub struct DrawInputs {
    pub highlight_pass: bool,
    pub zoom: i32,
    pub rotation: i32,
    pub view: ViewInputs,
    pub rgb: u32,
    pub packed_state: u32,
}

/// Individual foreign context calls only. Returns are intentionally ignored.
/// The shared Rust snapshot is an explicit source-level shade argument, not
/// a declaration of a native shared-instance pointer/layout.
pub trait SpriteDrawContext {
    fn set_model_transform(&mut self) -> bool;
    fn set_model_shade(&mut self, shared: &InstanceState, rgb: u32) -> bool;
    fn set_self_lit_shade(&mut self, rgb: u32) -> bool;
    fn set_render_state(&mut self, state: u32) -> bool;
    fn set_transparency(&mut self, alpha: f32) -> bool;
    fn set_alpha_func(&mut self, function: u32, reference: f32) -> bool;
    fn set_texture(&mut self, texture: TextureHandle, stage: u32) -> bool;
    fn draw_prims(&mut self, a: u32, b: u32, count: u32, stride: u32, vertices: &[DrawVertex; 4]) -> bool;
    fn reset_shade(&mut self) -> bool;
    fn reset_transparency(&mut self) -> bool;
}

impl<O> AnimatedSprite<O> {
    /// Explicit current-state suboperation input. Not native constructor/load.
    pub fn install_cache(&mut self, cache: SpriteCache) {
        self.cache = Some(cache);
    }
    pub fn cache(&self) -> Option<&SpriteCache> {
        self.cache.as_ref()
    }
    pub fn mark_view_dirty(&mut self) {
        if let Some(cache) = &mut self.cache {
            cache.view_dirty = true;
        }
    }

    /// Stored metric suboperation only: no orientation/record selection occurs.
    /// Leaves dirty flags unchanged, as UpdateView does.
    pub fn update_stored_view_exact(&mut self, view: ViewInputs) -> Result<(), Unsupported> {
        let cache = self.cache.as_mut().ok_or(Unsupported::MissingCache)?;
        let quad = update_view_exact(cache.metric, view)?;
        cache.local = quad.corners;
        self.state.bounds = quad.bounds;
        Ok(())
    }

    /// Qualified Draw with explicit checked Unsupported boundaries. Native
    /// invalid-pointer/holder faults and fault-time partial writes are not
    /// modeled. Successful paths own selection, cache decisions and protocol.
    pub fn draw(
        &mut self,
        input: DrawInputs,
        resources: DrawResources<'_>,
        context: &mut impl SpriteDrawContext,
    ) -> Result<u32, Unsupported> {
        if input.highlight_pass || input.zoom < 2 {
            return Ok(1);
        }
        let zoom = input.zoom as u8;
        if !(2..=4).contains(&zoom) {
            return Err(Unsupported::Holder);
        }
        let holder = resources.holders[usize::from(zoom)].ok_or(Unsupported::Holder)?;
        let mut cache = self.cache.ok_or(Unsupported::MissingCache)?;
        cache.view_dirty |=
            input.zoom != i32::from(self.state.zoom) || input.rotation != i32::from(self.state.rotation);
        let mut bounds = self.state.bounds;
        if cache.view_dirty || self.frame_dirty {
            let matrix = self.state.transform.rotation;
            let back = [matrix[2], matrix[5], matrix[8]];
            // The owner's published frame byte, not a sequence-start addition.
            let (metric, uv) = holder.select_exact(back, input.rotation as u8, self.state.playback.frame as u8)?;
            cache.metric = metric;
            for (vertex, uv) in cache.vertices.iter_mut().zip(uv) {
                vertex.uv = uv;
            }
            if cache.view_dirty {
                let quad = update_view_exact(metric, input.view)?;
                cache.local = quad.corners;
                bounds = quad.bounds;
            }
        }
        if cache.view_dirty || cache.counter != self.state.transform.change {
            for (vertex, local) in cache.vertices.iter_mut().zip(cache.local) {
                vertex.position = [
                    plus(local[0], self.state.transform.translation[0])?,
                    plus(local[1], self.state.transform.translation[1])?,
                    plus(local[2], self.state.transform.translation[2])?,
                ];
            }
            cache.counter = self.state.transform.change;
            cache.view_dirty = false;
        }
        let texture = *resources
            .textures
            .get(cache.metric.page as usize)
            .ok_or(Unsupported::Texture)?;
        let alpha_reference = if self.state.flags & FLAG_FADE != 0 {
            Some(times(self.state.alpha, 0.5)?)
        } else {
            None
        };
        self.state.zoom = zoom;
        self.state.rotation = input.rotation as u8;
        self.state.bounds = bounds;
        self.frame_dirty = false;
        self.cache = Some(cache);
        context.set_model_transform();
        if self.state.flags & FLAG_SELF_LIT != 0 {
            context.set_self_lit_shade(input.rgb);
        } else {
            context.set_model_shade(&self.state, input.rgb);
        }
        context.set_render_state(input.packed_state);
        if let Some(reference) = alpha_reference {
            context.set_transparency(self.state.alpha);
            context.set_alpha_func(4, reference);
        }
        context.set_texture(texture, 0);
        context.draw_prims(2, 10, 4, 24, &cache.vertices);
        context.reset_shade();
        context.reset_transparency();
        Ok(1)
    }

    pub fn pick_cached(&self, picker: &mut impl TrianglePicker) -> Result<bool, Unsupported> {
        let vertices = self.cache.as_ref().ok_or(Unsupported::MissingCache)?.vertices;
        Ok(pick_quad(&vertices.map(|v| v.position), picker))
    }
}

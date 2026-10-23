// SPDX-License-Identifier: GPL-3.0-or-later
//! Qualified §§4.1/5/6 algorithms, not a complete owner or native entry ABI.
//! Arithmetic is deliberately restricted to exact stages (AS7); no x87 mode,
//! hardware trig, factory defaults or native fault behavior is emulated.
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
    if x == 0.0 || (x.is_finite() && (1.0 / 4096.0..=4096.0).contains(&x.abs())) {
        Ok(x)
    } else {
        Err(Unsupported::Arithmetic)
    }
}
fn exact_store(x: f64) -> Result<f32, Unsupported> {
    let f = x as f32;
    if f64::from(f) != x {
        return Err(Unsupported::Arithmetic);
    }
    finite_operand(f)
}
fn plus(a: f32, b: f32) -> Result<f32, Unsupported> {
    exact_store(f64::from(finite_operand(a)?) + f64::from(finite_operand(b)?))
}
fn minus(a: f32, b: f32) -> Result<f32, Unsupported> {
    exact_store(f64::from(finite_operand(a)?) - f64::from(finite_operand(b)?))
}
fn times(a: f32, b: f32) -> Result<f32, Unsupported> {
    exact_store(f64::from(finite_operand(a)?) * f64::from(finite_operand(b)?))
}
fn divide(a: f32, b: f32) -> Result<f32, Unsupported> {
    finite_operand(a)?;
    finite_operand(b)?;
    if b == 0.0 {
        return Err(Unsupported::Arithmetic);
    }
    let q = finite_operand((f64::from(a) / f64::from(b)) as f32)?;
    // Exact f32 operands' product is representable in f64: rational proof,
    // rather than trusting the rounded f64 division's apparent equality.
    if f64::from(q) * f64::from(b) != f64::from(a) {
        return Err(Unsupported::Arithmetic);
    }
    Ok(q)
}

/// §5.1's owned dependency/sign/order, exact-stage subset only. Requiring
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
            if context_width == 0 || context_width > 4096 {
                return Err(Unsupported::Arithmetic);
            }
            divide(minus(right, left)?, context_width as f32)?
        }
    };
    let r = view.matrix.map(|row| row[0]);
    let b = view.matrix.map(|row| row[2]);
    let uy = view.matrix[1][1];
    let [ox, oy] = metric.offset;
    let d = minus(metric.height, oy)?; // native binary32 store
    let t = times(divide(m, uy)?, view.magnification)?; // native binary32 store
    let h = [
        times(times(r[0], m)?, view.magnification)?,
        times(times(r[1], m)?, view.magnification)?,
        times(times(r[2], m)?, view.magnification)?,
    ];
    let displacement = if d >= 0.0 {
        let a = divide(times(t, d)?, b[1])?;
        [times(b[0], a)?, times(b[1], a)?, times(b[2], a)?]
    } else {
        [0.0; 3]
    };
    let w = minus(metric.width, ox)?;
    let upper_y = plus(displacement[1], times(t, oy)?)?;
    let lower_y = minus(displacement[1], times(t, d)?)?; // native stored baseline
    let left_x = minus(displacement[0], times(h[0], ox)?)?;
    let left_z = minus(displacement[2], times(h[2], ox)?)?;
    let right_x = plus(displacement[0], times(h[0], w)?)?;
    let right_z = plus(displacement[2], times(h[2], w)?)?;
    let corners = [
        [left_x, minus(upper_y, times(h[1], ox)?)?, left_z],
        [left_x, minus(lower_y, times(h[1], ox)?)?, left_z],
        [right_x, plus(lower_y, times(h[1], w)?)?, right_z],
        [right_x, plus(upper_y, times(h[1], w)?)?, right_z],
    ];
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

/// Raw borrowed resource consumer fields (§4.1), not file/factory parsing.
#[derive(Debug)]
pub struct RawSpriteHolder {
    pub divisors: [u32; 3],
    pub angle_limits: [f32; 4],
    pub entries: Vec<[u8; 8]>,
    pub scale: f32,
}

/// §4.1 valid-packing subset. Malformed records are not repaired into an atlas.
pub fn decode_entry_exact(entry: [u8; 8], frame: u8, scale: f32) -> Result<(StoredMetric, [[f32; 2]; 4]), Unsupported> {
    let side = 1_u32 << (entry[1] & 31);
    let width = i16::from(entry[4] as i8).unsigned_abs() as u32;
    let height = i16::from(entry[5] as i8).unsigned_abs() as u32;
    let mut x = u32::from(entry[2]);
    let mut y = u32::from(entry[3]);
    if side > (1 << 20)
        || width == 0
        || height == 0
        || width > side
        || height > side
        || x > side - width
        || y > side - height
    {
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
    let scale = if minus(scale, 1.0)?.abs() >= f32::from_bits(0x38d1b717) {
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
    fn select_exact(&self, back: Vec3, rotation: u8, frame: u8) -> Result<(StoredMetric, [[f32; 2]; 4]), Unsupported> {
        // Deliberately narrow, provable no-trig domain: asin(0)=atan2(0,1)=0,
        // base=stored pi, P=R=divisor0=1, low rotation=0 => B(0)=0.
        // Do not use the old portable direction_index (wrong rounding/layout).
        if back != [0.0, 0.0, 1.0]
            || rotation != 0
            || self.divisors != [1; 3]
            || self.angle_limits != [f32::from_bits(0x40490fdb), f32::from_bits(0x40490fdb), 0.0, 0.0]
        {
            return Err(Unsupported::Selection);
        }
        let entry = *self.entries.first().ok_or(Unsupported::Selection)?;
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

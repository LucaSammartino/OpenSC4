// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

/// Editor Â§9 raw offsets: +10/+14 are minima, +18/+1C maxima.
/// The existing shared record field names describe the opposite convention;
/// do not change the shared decoder or normalize editor geometry (question LE6).
pub fn bounds(o: &LotConfigObject) -> [f32; 4] {
    [o.max_x, o.max_z, o.min_x, o.min_z]
}
fn set_bounds(o: &mut LotConfigObject, b: [f32; 4]) {
    [o.max_x, o.max_z, o.min_x, o.min_z] = b;
}
fn translate(o: &mut LotConfigObject, axis: usize, delta: f64) {
    let mut b = bounds(o);
    o.offset[axis * 2] = (f64::from(o.offset[axis * 2]) + delta) as f32;
    b[axis] = (f64::from(b[axis]) + delta) as f32;
    b[axis + 2] = (f64::from(b[axis + 2]) + delta) as f32;
    set_bounds(o, b);
}
/// Â§9.4 comparisons explicitly permit unordered operands, independently.
pub fn contained(o: &LotConfigObject, size: [u8; 2]) -> bool {
    let [l, t, r, b] = bounds(o);
    !(r > f32::from(size[0]) * 16.0 || l < 0.0 || b > f32::from(size[1]) * 16.0 || t < 0.0)
}
/// Â§9.2/9.6: unordered individual tests skip their translation.
pub fn move_within(o: &mut LotConfigObject, size: [u8; 2], partial: bool) {
    let eps = f64::from(f32::from_bits(0x3D4CCCCD));
    for (axis, upper) in [(0, false), (1, false), (0, true), (1, true)] {
        let b = bounds(o);
        let value = if partial {
            b[axis + if upper { 0 } else { 2 }]
        } else {
            o.offset[axis * 2]
        };
        let target = if upper {
            f64::from(size[axis]) * 16.0 - eps
        } else {
            eps
        };
        let value = f64::from(value);
        if if upper {
            value > target
        } else {
            value < target
        } {
            translate(o, axis, target - value);
        }
    }
    if o.offset[0] < 0.0 {
        o.offset[0] = 0.0;
    }
}
pub fn pixel_units(pixel: [i32; 2], scale: f32, rect_origin: [f32; 2]) -> [f32; 2] {
    let scaled = [
        (f64::from(pixel[0]) * f64::from(scale)) as f32,
        (f64::from(pixel[1]) * f64::from(scale)) as f32,
    ];
    [scaled[0] - rect_origin[0], scaled[1] - rect_origin[1]]
}

/// Exact signed-int × binary32 × 1/16, truncated through the published signed
/// 64-bit intermediate. The 55-bit product fits x87 precision but may not fit
/// binary64, so computing this in f64 can change the low byte near large values.
fn fence_coordinate(value: i32, scale: f32) -> Result<u8> {
    let bits = scale.to_bits();
    let exp = (bits >> 23) & 255;
    if exp == 255 {
        return Err(Unsupported::NumericConversion);
    }
    let significand = if exp == 0 {
        bits & 0x7FFFFF
    } else {
        (bits & 0x7FFFFF) | 0x800000
    };
    let product = u128::from(value.unsigned_abs()) * u128::from(significand);
    if product == 0 {
        return Ok(0);
    }
    let shift = if exp == 0 {
        -149 - 4
    } else {
        exp as i32 - 127 - 23 - 4
    };
    let magnitude = if shift >= 0 {
        product
            .checked_shl(shift as u32)
            .filter(|_| shift < 64)
            .ok_or(Unsupported::NumericConversion)?
    } else {
        product.checked_shr((-shift) as u32).unwrap_or(0)
    };
    let negative = (value < 0) ^ (bits >> 31 != 0);
    if magnitude > (1u128 << 63) || (!negative && magnitude == 1u128 << 63) {
        return Err(Unsupported::NumericConversion);
    }
    let byte = magnitude as u8;
    Ok(if negative { byte.wrapping_neg() } else { byte })
}
impl Editor {
    /// Established finite movement subset; LE3 conversion/hit helpers remain unsupported.
    pub fn move_key(
        &mut self,
        id: RecordId,
        key: u32,
        modifiers: u32,
        space: bool,
        h: &mut impl Host,
    ) -> Result<()> {
        let size = self.config()?.size;
        let o = self.record_mut(id).ok_or(Unsupported::MissingRecord)?;
        if o.offset
            .iter()
            .chain(bounds(o).iter())
            .any(|v| !v.is_finite())
        {
            return Err(Unsupported::NumericConversion);
        }
        let mut step = if modifiers & 2 != 0 { 0.5f32 } else { 0.25 };
        if modifiers & 1 != 0 {
            step *= 4.0;
        }
        if modifiers & 4 != 0 {
            step *= 8.0;
        }
        match key {
            0x25 => translate(o, 0, -f64::from(step)),
            0x27 => translate(o, 0, f64::from(step)),
            0x26 | 0x28 if space => {
                if o.kind == 1 {
                    o.offset[1] += if key == 0x26 { step } else { -step };
                }
            }
            0x26 => translate(o, 1, -f64::from(step)),
            0x28 => translate(o, 1, f64::from(step)),
            _ => (),
        }
        let partial = o.kind != 4;
        move_within(o, size, partial);
        self.edited(h);
        Ok(())
    }
    pub fn move_position(
        &mut self,
        id: RecordId,
        position: [f32; 2],
        h: &mut impl Host,
    ) -> Result<()> {
        let size = self.config()?.size;
        let o = self.record_mut(id).ok_or(Unsupported::MissingRecord)?;
        if position
            .iter()
            .chain(o.offset.iter())
            .chain(bounds(o).iter())
            .any(|v| !v.is_finite())
        {
            return Err(Unsupported::NumericConversion);
        }
        let dx = f64::from(position[0]) - f64::from(o.offset[0]);
        let dz = f64::from(position[1]) - f64::from(o.offset[2]);
        translate(o, 0, dx);
        translate(o, 1, dz);
        let partial = o.kind != 4;
        move_within(o, size, partial);
        self.edited(h);
        Ok(())
    }
    pub fn cleanup(&mut self, h: &mut impl Host) -> Result<usize> {
        let c = self.config_mut()?;
        let n = c.objects.len() + c.fences.len();
        c.objects
            .retain(|r| !matches!(r.data.kind, 0 | 1 | 4) || contained(&r.data, c.size));
        c.fences.retain(|f| {
            i16::from(f[2] as i8) <= i16::from(c.size[0])
                && i16::from(f[3] as i8) <= i16::from(c.size[1])
        });
        let removed = n - c.objects.len() - c.fences.len();
        if removed != 0 {
            self.set_dirty(1, h);
        }
        Ok(removed)
    }
    pub fn duplicate(&mut self, id: RecordId, h: &mut impl Host) -> Result<Option<RecordId>> {
        let mut o = self.record(id).ok_or(Unsupported::MissingRecord)?.clone();
        if !matches!(o.kind, 0 | 1 | 2 | 4) {
            return Ok(None);
        }
        // Sequential stores, not four independently computed candidate copies.
        translate(&mut o, 0, 4.0);
        translate(&mut o, 1, 4.0);
        if !contained(&o, self.config()?.size) {
            translate(&mut o, 0, -8.0);
            translate(&mut o, 1, -8.0);
            if !contained(&o, self.config()?.size) {
                translate(&mut o, 0, 4.0);
                if !contained(&o, self.config()?.size) {
                    translate(&mut o, 0, -8.0);
                    translate(&mut o, 1, 8.0);
                }
            }
        }
        let id = self.insert(o, true);
        self.set_selection(Some(id), h);
        self.edited(h);
        Ok(Some(id))
    }
    pub fn delete(&mut self, id: RecordId, confirm: bool, h: &mut impl Host) -> Result<bool> {
        if confirm {
            // The type-name helper is foreign/open; do not synthesize a name.
            return Err(Unsupported::Dependency("LE5 deletion name helper"));
        }
        self.config_mut()?.objects.retain(|r| r.id != id);
        if self.selected == Some(id) {
            self.selected = None;
        }
        self.draw(h);
        Ok(true)
    }
    pub fn rotate(&mut self, id: RecordId, h: &mut impl Host) -> Result<()> {
        let o = self.record_mut(id).ok_or(Unsupported::MissingRecord)?;
        if o.offset
            .iter()
            .chain(bounds(o).iter())
            .any(|v| !v.is_finite())
        {
            return Err(Unsupported::NumericConversion);
        }
        o.orientation = o.orientation.wrapping_add(1) & 3;
        let [l, t, r, b] = bounds(o);
        let hz = (f64::from(t) - f64::from(b)) * 0.5;
        let hx = ((f64::from(l) - f64::from(r)) * 0.5) as f32;
        let [x, _, z] = o.offset;
        set_bounds(
            o,
            [
                (f64::from(x) + hz) as f32,
                z + hx,
                (f64::from(x) - hz) as f32,
                z - hx,
            ],
        );
        self.edited(h);
        Ok(())
    }
    pub fn mirror(&mut self, id: RecordId, h: &mut impl Host) -> Result<()> {
        let o = self.record_mut(id).ok_or(Unsupported::MissingRecord)?;
        if o.kind == 2 {
            o.mirror = u8::from(o.mirror == 0);
            self.edited(h);
        }
        Ok(())
    }
    pub fn foundation(&self) -> u32 {
        self.current
            .as_ref()
            .and_then(|c| c.objects.iter().find(|r| r.data.kind == 3))
            .map_or(0, |r| r.data.foundation)
    }
    pub fn add_foundation(&mut self, id: u32, enable: bool) -> Result<()> {
        let c = self.config_mut()?;
        if let Some(i) = c.objects.iter().position(|r| r.data.kind == 3) {
            if enable && c.objects[i].data.foundation == id {
                return Ok(());
            }
            c.objects.remove(i);
        }
        if enable {
            let rid = self.insert(
                LotConfigObject {
                    kind: 3,
                    foundation: id,
                    ..Default::default()
                },
                false,
            );
            let r = self
                .config_mut()?
                .objects
                .iter_mut()
                .find(|r| r.id == rid)
                .unwrap();
            r.creation_known = false; // +24 has no published value, not a zero guarantee.
        }
        Ok(())
    }
    pub fn toggle_fence(&mut self, fence: [u8; 4], h: &mut impl Host) -> Result<()> {
        let fences = &mut self.config_mut()?.fences;
        if let Some(i) = fences.iter().position(|f| *f == fence) {
            fences.remove(i);
        } else {
            fences.push(fence);
        }
        self.draw(h);
        Ok(())
    }
    pub fn toggle_fence_pixels(&mut self, x: i32, y: i32, h: &mut impl Host) -> Result<bool> {
        let size = self.config()?.size;
        let inv = 1.0f32 / self.scale;
        let trunc = |v: f64| -> Result<i64> {
            if !v.is_finite() || v < i64::MIN as f64 || v >= 9223372036854775808.0 {
                Err(Unsupported::NumericConversion)
            } else {
                Ok(v.trunc() as i64)
            }
        };
        let mut fence = None;
        for (axis, p, other) in [(0, x, y), (1, y, x)] {
            for i in 0..=u16::from(size[axis]) {
                let line = trunc(f64::from(i * 16) * f64::from(inv))?;
                if (i128::from(p) - i128::from(line)).unsigned_abs() <= 3 {
                    let t = fence_coordinate(other, self.scale)?;
                    fence = Some(if axis == 0 {
                        [i as u8, t, i as u8, t.wrapping_add(1)]
                    } else {
                        [t, i as u8, t.wrapping_add(1), i as u8]
                    });
                    break;
                }
            }
            if fence.is_some() {
                break;
            }
        }
        if let Some(f) = fence {
            self.toggle_fence(f, h)?;
            self.set_dirty(1, h);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hit {
    Object(RecordId, usize),
    Candidates(Vec<RecordId>),
    Cancelled,
    UnsupportedFallback,
}
fn inside(p: [i32; 2], r: [i32; 4]) -> bool {
    p[0] >= r[0] && p[1] >= r[1] && p[0] <= r[2] && p[1] <= r[3]
}
impl Editor {
    /// Priority kernel only. Integer rectangle production remains LE3/LE5;
    /// caller-supplied rectangles are explicit test/input facts, not a host hit result.
    pub fn hit_candidates(
        &self,
        point: [i32; 2],
        flags: u32,
        selected_rect: Option<[i32; 4]>,
        rectangles: &[(RecordId, [i32; 4])],
        popup: Option<i32>,
    ) -> Hit {
        if flags & 2 == 0 {
            if let (Some(id), Some(rect)) = (self.selected, selected_rect) {
                if inside(point, rect) {
                    return Hit::Object(id, self.selected_index + 100);
                }
            }
        }
        let Some(c) = &self.current else {
            return Hit::UnsupportedFallback;
        };
        let candidates: Vec<_> = c
            .objects
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                rectangles
                    .iter()
                    .any(|(id, rect)| *id == r.id && inside(point, *rect))
            })
            .collect();
        if candidates.is_empty() {
            return Hit::UnsupportedFallback;
        }
        if flags & 2 == 0 || candidates.len() == 1 {
            return Hit::Object(candidates[0].1.id, candidates[0].0 + 100);
        }
        match popup {
            None => Hit::Candidates(candidates.iter().map(|(_, r)| r.id).collect()),
            Some(row) if row >= 0 && (row as usize) < candidates.len() => {
                let (i, r) = candidates[row as usize];
                Hit::Object(r.id, i + 100)
            }
            _ => Hit::Cancelled,
        }
    }
}

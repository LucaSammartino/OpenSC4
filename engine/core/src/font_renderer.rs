// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable small renderer (font-renderer.md sections 2-6), not a native object or ABI.
//!
//! The host supplies primitive target queries and byte windows; this module owns clipping,
//! addressing and pixel arithmetic. Windows start at the published source/pixel/colour bases.
//! Reads and stores are little-endian and recorded separately, including same-value stores.
//!
//! FR2 safety extension: missing records/target, inadequate addressed backing, nonpositive
//! pitches, negative target dimensions and overflowing geometry/address arithmetic are refused.
//! Refusal is not native fault behavior, source clipping, a default value or rollback: earlier
//! records/pixels and callbacks can already have effects. No producer guarantees are inferred.
//! FR1's two ignored-argument slots are deliberately withheld. FR3's larger holder is foreign.

/// Signed destination/clip coordinates. Reversed rectangles are not normalized.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Typed view of one section 3 record, not a native-layout claim.
#[derive(Clone, Copy, Debug)]
pub struct Record<'a> {
    pub destination: Rect,
    pub source: &'a [u8],
    pub source_pitch: u32,
    /// Copied metadata only: these do not bound the raster extent.
    pub source_width: i32,
    pub source_height: i32,
    pub character: u32,
}

/// Primitive foreign target operations. `format`'s raw result is ignored, while
/// lock/unlock return the target's AL byte verbatim. No retained target or lock state.
pub trait Target {
    fn width(&mut self) -> i32;
    fn height(&mut self) -> i32;
    fn depth(&mut self) -> u32;
    fn format(&mut self, words: &mut [u32; 2]) -> u8;
    fn pitch(&mut self) -> u32;
    fn pixels(&mut self) -> &mut [u8];
    fn lock(&mut self, flags: u32) -> u8;
    fn unlock(&mut self, flags: u32) -> u8;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Memory {
    Source,
    Colour,
    Target,
}

/// Access ledger of completed queries/accesses (PixelBase precedes window retrieval).
/// These events expose the portable model; they do not assert a native tracing ABI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Record {
        index: usize,
        width: i32,
        height: i32,
        character: u32,
    },
    Width(i32),
    Height(i32),
    Depth(u32),
    Format([u32; 2]),
    Pitch(u32),
    PixelBase,
    Read {
        memory: Memory,
        offset: usize,
        bytes: usize,
        value: u32,
    },
    Write {
        offset: usize,
        bytes: usize,
        value: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Records,
    Target,
    Geometry,
    Backing {
        memory: Memory,
        offset: usize,
        bytes: usize,
    },
}

/// Font and mode arguments have no effect and therefore are not part of this portable API.
pub fn lock(target: &mut dyn Target) -> u8 {
    target.lock(0x8080)
}

pub fn unlock(target: &mut dyn Target) -> u8 {
    target.unlock(0x8080)
}

/// Batch count is a dword, including values whose signed interpretation is negative.
/// Count zero never inspects records, target or colours. Control records never need a target.
pub fn draw(
    mut target: Option<&mut dyn Target>,
    records: &[Record<'_>],
    count: u32,
    mode: u32,
    colours: &[u8],
    clip: Option<Rect>,
    events: &mut Vec<Event>,
) -> Result<bool, Refusal> {
    for index in 0..count {
        let item = records.get(index as usize).ok_or(Refusal::Records)?;
        events.push(Event::Record {
            index: index as usize,
            width: item.source_width,
            height: item.source_height,
            character: item.character,
        });
        if item.character < 0x20 {
            continue;
        }
        let target = target.as_deref_mut().ok_or(Refusal::Target)?;
        raster(target, item, mode, colours, clip, events)?;
    }
    Ok(true)
}

fn clipped(mut r: Rect, clip: Option<Rect>, w: i32, h: i32) -> Result<(Rect, u32, u32), Refusal> {
    if w < 0 || h < 0 {
        return Err(Refusal::Geometry);
    }
    let mut sx = 0i32;
    let mut sy = 0i32;
    if let Some(c) = clip {
        if c.left > r.left {
            sx = c.left.checked_sub(r.left).ok_or(Refusal::Geometry)?;
            r.left = c.left;
        }
        if c.top > r.top {
            sy = c.top.checked_sub(r.top).ok_or(Refusal::Geometry)?;
            r.top = c.top;
        }
        r.right = r.right.min(c.right);
        r.bottom = r.bottom.min(c.bottom);
    }
    if r.left < 0 {
        sx = sx.checked_sub(r.left).ok_or(Refusal::Geometry)?;
        r.left = 0;
    }
    if r.top < 0 {
        sy = sy.checked_sub(r.top).ok_or(Refusal::Geometry)?;
        r.top = 0;
    }
    r.right = r.right.min(w);
    r.bottom = r.bottom.min(h);
    Ok((r, sx as u32, sy as u32))
}

fn raster(
    target: &mut dyn Target,
    item: &Record<'_>,
    mode: u32,
    colours: &[u8],
    clip: Option<Rect>,
    events: &mut Vec<Event>,
) -> Result<(), Refusal> {
    let width = target.width();
    events.push(Event::Width(width));
    let height = target.height();
    events.push(Event::Height(height));
    let depth = target.depth();
    events.push(Event::Depth(depth));
    let mut format = [3, 16];
    if mode == 3 {
        let _ = target.format(&mut format);
        events.push(Event::Format(format));
    }
    let (r, sx, sy) = clipped(item.destination, clip, width, height)?;
    if r.right <= r.left || r.bottom <= r.top || !matches!(depth, 16 | 24 | 32) {
        return Ok(());
    }
    let foreground = read(colours, Memory::Colour, 0, 4, events)?;
    let pitch = target.pitch();
    events.push(Event::Pitch(pitch));
    events.push(Event::PixelBase);
    let pixels = target.pixels();
    let bytes = if depth == 16 { 2 } else { 4 };
    // FR2: ordinary positive usable pitches only. Do not reinterpret invalid signed pitches.
    let stride = (pitch / bytes) * bytes;
    if stride == 0 || pitch > i32::MAX as u32 || item.source_pitch == 0 || item.source_pitch > i32::MAX as u32 {
        return Err(Refusal::Geometry);
    }
    for y in r.top..r.bottom {
        for x in r.left..r.right {
            let source_x = sx.checked_add((x - r.left) as u32).ok_or(Refusal::Geometry)?;
            let source_y = sy.checked_add((y - r.top) as u32).ok_or(Refusal::Geometry)?;
            let source_offset = address(source_y, item.source_pitch, source_x, 1)?;
            let a = read(item.source, Memory::Source, source_offset, 1, events)?;
            let target_offset = address(y as u32, stride, x as u32, bytes)?;
            let value = match mode {
                1 if a > 127 => Some(foreground),
                1 => None,
                2 if a != 0 => Some(read(colours, Memory::Colour, (a * 4) as usize, 4, events)?),
                2 => None,
                _ if depth != 16 => {
                    let d = read(pixels, Memory::Target, target_offset, 4, events)?;
                    Some(blend_dword(d, foreground, a))
                }
                _ if a != 0 && matches!(format[0], 4..=7) => {
                    let d = read(pixels, Memory::Target, target_offset, 2, events)?;
                    Some(blend_word(d, foreground, a, format[0]))
                }
                _ => None,
            };
            if let Some(value) = value {
                store(pixels, target_offset, bytes as usize, value, events)?;
            }
        }
    }
    Ok(())
}

fn address(row: u32, pitch: u32, column: u32, bytes: u32) -> Result<usize, Refusal> {
    row.checked_mul(pitch)
        .and_then(|v| column.checked_mul(bytes).and_then(|x| v.checked_add(x)))
        .and_then(|v| v.checked_add(bytes - 1).map(|_| v as usize))
        .ok_or(Refusal::Geometry)
}

fn read(window: &[u8], memory: Memory, offset: usize, bytes: usize, events: &mut Vec<Event>) -> Result<u32, Refusal> {
    let end = offset.checked_add(bytes).ok_or(Refusal::Geometry)?;
    let slice = window
        .get(offset..end)
        .ok_or(Refusal::Backing { memory, offset, bytes })?;
    let mut value = 0;
    for (i, byte) in slice.iter().enumerate() {
        value |= u32::from(*byte) << (i * 8);
    }
    events.push(Event::Read {
        memory,
        offset,
        bytes,
        value,
    });
    Ok(value)
}

fn store(window: &mut [u8], offset: usize, bytes: usize, value: u32, events: &mut Vec<Event>) -> Result<(), Refusal> {
    let end = offset.checked_add(bytes).ok_or(Refusal::Geometry)?;
    let slice = window.get_mut(offset..end).ok_or(Refusal::Backing {
        memory: Memory::Target,
        offset,
        bytes,
    })?;
    slice.copy_from_slice(&value.to_le_bytes()[..bytes]);
    let value = if bytes == 2 { value & 0xffff } else { value };
    events.push(Event::Write { offset, bytes, value });
    Ok(())
}

// Section 5's dyadic inputs are exact; integer division is the same nonnegative truncation.
// Immutable initialization is a portable synchronization extension, not the native once flag.
static BLEND: [u8; 32768] = make_table();

const fn make_table() -> [u8; 32768] {
    let mut table = [0; 32768];
    let mut f = 0;
    while f < 32 {
        let mut d = 0;
        while d < 32 {
            let mut q = 0;
            while q < 32 {
                table[(f * 32 + d) * 32 + q] = ((d * (32 - q) + f * q) / 32) as u8;
                q += 1;
            }
            d += 1;
        }
        f += 1;
    }
    table
}

fn blend_word(d: u32, f: u32, a: u32, format: u32) -> u32 {
    let (shifts, mask, forced) = match format {
        7 => ([8, 4, 0], 15, 0xf000),
        5 => ([11, 6, 0], 31, 0),
        _ => ([10, 5, 0], 31, 0),
    };
    let mut out = forced;
    for shift in shifts {
        let index = ((((f >> shift) & mask) * 32 + ((d >> shift) & mask)) * 32 + (a >> 3)) as usize;
        out |= u32::from(BLEND[index]) << shift;
    }
    out
}

fn blend_dword(d: u32, f: u32, a: u32) -> u32 {
    const M: u32 = 0x00ff00ff;
    let h = ((d >> 8) & M)
        .wrapping_mul(256 - a)
        .wrapping_add(((f >> 8) & M).wrapping_mul(a));
    let j = (d & M)
        .wrapping_mul(256 - a)
        .wrapping_add((f & M).wrapping_mul(a))
        .wrapping_add(0x00800080);
    (((j >> 8) ^ h.wrapping_sub(0x007fff80)) & M) ^ h.wrapping_add(0x00800080)
}

/// Safe ownership model only: Freed is an effect for a caller, never a raw deallocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lifetime {
    count: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dead;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Release {
    Retained(u32),
    Freed,
}

impl Default for Lifetime {
    fn default() -> Self {
        Self::new()
    }
}

impl Lifetime {
    pub fn new() -> Self {
        Self::from_count(0)
    }

    /// Explicit initial bits for modeling signed Release and wrapping AddRef.
    pub fn from_count(count: u32) -> Self {
        Self { count: Some(count) }
    }

    pub fn count(&self) -> Option<u32> {
        self.count
    }

    pub fn add_ref(&mut self) -> Result<u32, Dead> {
        let next = self.count.ok_or(Dead)?.wrapping_add(1);
        self.count = Some(next);
        Ok(next)
    }

    /// `identity` is an opaque caller token, not a pointer or native QI implementation.
    pub fn query(&mut self, iid: u32, out: &mut u64, identity: u64) -> Result<bool, Dead> {
        self.count.ok_or(Dead)?;
        if !matches!(iid, 1 | 0xa8bc2a75) {
            return Ok(false);
        }
        *out = identity;
        self.add_ref()?;
        Ok(true)
    }

    pub fn release(&mut self) -> Result<Release, Dead> {
        let count = self.count.ok_or(Dead)?;
        if (count as i32) > 1 {
            self.count = Some(count - 1);
            Ok(Release::Retained(count - 1))
        } else {
            self.count = None;
            Ok(Release::Freed)
        }
    }
}

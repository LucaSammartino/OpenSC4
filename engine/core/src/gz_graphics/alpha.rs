// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

fn edges(r: Rect) -> [i32; 4] { [r.0,r.1,r.2,r.3] }
fn rect(a: [i32; 4]) -> Rect { Rect(a[0],a[1],a[2],a[3]) }

/// Shared edge ordering, including early buffer failure and late optional failure.
fn clip_edges(bounds: Rect, dest: &mut Rect, extra: Option<Rect>, mut changed: impl FnMut(usize,i32)) -> bool {
    let mut d = edges(*dest);
    let mut apply = |index: usize, bound: i32| {
        let old = d[index];
        d[index] = if index < 2 { old.max(bound) } else { old.min(bound) };
        changed(index,d[index].wrapping_sub(old));
        *dest = rect(d);
        d[0] < d[2] && d[1] < d[3]
    };
    for (index,bound) in [(2,bounds.2),(0,0),(3,bounds.3),(1,0)] {
        if !apply(index,bound) { return false; }
    }
    if let Some(r) = extra {
        for (index,bound) in [(2,r.2),(0,r.0),(3,r.3),(1,r.1)] { apply(index,bound); }
    }
    dest.0 < dest.2 && dest.1 < dest.3
}

pub fn generic_buffer_clip(area: Rect, source: &mut Rect, dest: &mut Rect, extra: Option<Rect>) -> bool {
    let mut s = edges(*source);
    let result = clip_edges(area,dest,extra,|index,delta| { s[index] = s[index].wrapping_add(delta); });
    *source = rect(s);
    result
}

pub fn generic_buffer_clip_stretch(width: i32, height: i32, source: &mut Rect, dest: &mut Rect, extra: Option<Rect>) -> bool {
    let old = *dest;
    let sx = (source.width() as f64 / old.width() as f64) as f32;
    let sy = (source.height() as f64 / old.height() as f64) as f32;
    if !clip_edges(Rect(0,0,width,height),dest,extra,|_,_| {}) { return false; }
    if *dest == old { return true; }
    if (sx == 0.5 && dest.width() == 1) || (sy == 0.5 && dest.height() == 1) { return false; }
    let left = source.0.wrapping_add((dest.0.wrapping_sub(old.0) as f64 * sx as f64) as i32);
    let top = source.1.wrapping_add((dest.1.wrapping_sub(old.1) as f64 * sy as f64) as i32);
    let right = (left as f64 + dest.width() as f64 * sx as f64) as i32;
    let bottom = (top as f64 + dest.height() as f64 * sy as f64) as i32;
    *source = Rect(left,top,right,bottom);
    if source.0 == source.2 {
        if source.0 == 0 { source.2 = source.2.wrapping_add(1); } else { source.0 = source.0.wrapping_sub(1); }
    }
    if source.1 == source.3 {
        if source.1 == 0 { source.3 = source.3.wrapping_add(1); } else { source.1 = source.1.wrapping_sub(1); }
    }
    true
}

/// Ordinary AlphaBlt's local copies; pixel kernels are still delegated (GZG5).
pub fn clipped_alpha_rectangles(area: Rect, source: Rect, point: [i32; 2], extra: Option<Rect>) -> Option<(Rect,Rect)> {
    let mut src = source;
    let mut dest = Rect(point[0],point[1],point[0].wrapping_add(source.width()),point[1].wrapping_add(source.height()));
    generic_buffer_clip(area,&mut src,&mut dest,extra).then_some((src,dest))
}

/// Foreign resource operations, not an alternate BMP decoder. The fixed offsets
/// and type-2 initialization are imposed by the loader, not by this adapter.
pub trait AlphaFileIo {
    fn create_buffer(&mut self) -> Result<Arc<dyn Buffer>>;
    fn open(&mut self, path: &str) -> bool;
    fn read_at(&mut self, offset: usize, destination: &mut [u8]) -> usize;
    fn initialize(&mut self, buffer: &Arc<dyn Buffer>, width: u32, height: u32, kind: u32, depth: u32) -> bool;
    fn close(&mut self);
    /// Record the native free attempt without performing an invalid deallocation.
    fn free_source(&mut self, _offset: usize) {}
}

pub(super) fn load_surface(path: &str, levels: Option<u32>, io: &mut dyn AlphaFileIo) -> Result<Option<Arc<dyn Buffer>>> {
    let buffer = io.create_buffer()?;
    if !io.open(path) {
        // §6.3.2: creation precedes open, and this reference is leaked on failure.
        std::mem::forget(buffer);
        return Ok(None);
    }
    let result = (|| {
        let mut header = [0_u8; 0x428];
        if io.read_at(14,&mut header) != header.len() { return Err(Error::InvalidInput); }
        let width = u32::from_le_bytes(header[4..8].try_into().unwrap());
        let height = u32::from_le_bytes(header[8..12].try_into().unwrap());
        // Short reads, bad dimensions and zero divisors are unchecked native
        // inputs. Safe errors here assert no native malformed-file outcome.
        if width > i32::MAX as u32 || height > i32::MAX as u32 { return Err(Error::InvalidInput); }
        let stride = (width as usize).checked_add(3).ok_or(Error::InvalidInput)? & !3;
        let length = stride.checked_mul(height as usize).ok_or(Error::InvalidInput)?;
        let mut source = Vec::new();
        source.try_reserve_exact(length).map_err(|_| Error::InvalidInput)?;
        source.resize(length,0);
        if io.read_at(14+0x428,&mut source) != length { return Err(Error::InvalidInput); }
        if let Some(levels) = levels {
            if !source.is_empty() {
                let divisor = 256_u32.checked_div(levels).filter(|&v| v != 0).ok_or(Error::InvalidInput)?;
                for byte in &mut source { *byte = (u32::from(*byte)/divisor) as u8; }
            }
        }
        let _ = io.initialize(&buffer,width,height,2,8);
        if !buffer.lock(0x8040) {
            io.free_source(length);
            return Err(Error::Foreign);
        }
        let record = buffer.record();
        let copy = buffer.pixels(record,&mut |dest,origin| {
            for y in 0..height as usize {
                let offset = origin as i64 + y as i64 * i64::from(record.stride);
                let start = usize::try_from(offset).map_err(|_| Error::InvalidInput)?;
                let end = start.checked_add(width as usize).ok_or(Error::InvalidInput)?;
                let row = dest.get_mut(start..end).ok_or(Error::InvalidInput)?;
                let input = (height as usize-1-y)*stride;
                row.copy_from_slice(&source[input..input+width as usize]);
            }
            Ok(())
        });
        buffer.unlock(0x8040);
        io.free_source(0);
        copy
    })();
    io.close();
    match result {
        Ok(()) => Ok(Some(buffer)),
        Err(Error::Foreign) => {
            // Lock failure attempts one-past-end free and retains the reference.
            // A safe adapter reports failure rather than promising native return.
            std::mem::forget(buffer);
            Err(Error::Foreign)
        }
        Err(e) => Err(e),
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
//! Named texture metadata, surface lifetime, upload addressing (§9).
use super::backend::*;

#[derive(Clone, Debug)]
pub struct Texture {
    pub used: bool, pub width: u32, pub height: u32, pub levels: u32,
    pub priority: f32, pub format: u32, pub flags: u32, pub parameters: [u32; 4], pub surface: u32,
}
impl Default for Texture {
    fn default() -> Self {
        Self { used: true, width: 0, height: 0, levels: 0, priority: 0.5,
            format: 4, flags: 0, parameters: [0, 0, 3, 3], surface: 0 }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Format {
    /// alpha, red, green, blue masks, flags, FourCC, bits, converter key.
    pub words: [u32; 8],
}
#[derive(Default, Debug)]
pub struct TextureManager {
    pub records: Vec<Texture>, pub freed: Vec<u32>, pub dd: u32, pub device: u32,
    pub formats: Option<[Format; 8]>, pub converters: Option<[[bool; 8]; 15]>,
}
#[derive(Clone, Copy, Debug)]
pub struct Upload {
    pub name: u32, pub level: u32, pub x: u32, pub y: u32, pub width: u32, pub height: u32,
    pub format: u32, pub kind: u32, pub row_length: u32, pub pixels: u32,
}
pub fn source_format(format: u32, kind: u32) -> Option<(usize, u32)> {
    Some(match (kind, format) {
        (0 | 1, 3) => (0, 32), (0 | 1, 1) => (1, 32), (0 | 1, 2) => (2, 24), (0 | 1, 0) => (3, 24),
        (13, 3) => (4, 16), (13, 1) => (5, 16), (10, 3) => (6, 16), (10, 1) => (7, 16),
        (14, 2) => (8, 16), (14, 0) => (9, 16), (0 | 1, 7 | 8) => (10, 4),
        (0 | 1, 9) => (12, 8), (0 | 1, 10) => (14, 8), _ => return None,
    })
}
impl TextureManager {
    pub fn get(&self, name: u32) -> Option<&Texture> { self.records.get(name as usize).filter(|r| r.used) }
    pub fn get_mut(&mut self, name: u32) -> Option<&mut Texture> { self.records.get_mut(name as usize).filter(|r| r.used) }
    pub fn valid(&self, name: u32) -> bool { self.get(name).is_some() }
    pub fn alpha(&self, name: u32) -> bool { self.get(name).is_some_and(|r| r.format != 0 && r.format != 1) }
    pub fn surface(&self, name: u32) -> u32 { self.get(name).map_or(0, |r| r.surface) }
    pub fn new_name(&mut self) -> u32 {
        if let Some(name) = self.freed.pop() {
            let r = &mut self.records[name as usize]; r.used = true; r.width = 0; r.height = 0; name
        } else { let name = self.records.len() as u32; self.records.push(Texture::default()); name }
    }
    pub fn free(&mut self, name: u32, host: &mut impl Device) -> bool {
        if let Some(r) = self.get_mut(name) { release(host, &mut r.surface); r.used = false; self.freed.push(name); true } else { false }
    }
    pub fn priority(&mut self, name: u32, value: f32) -> bool {
        if let Some(r) = self.get_mut(name) { r.priority = value; if value == 0.95f32 { r.flags |= 1; } true } else { false }
    }
    pub fn init(&mut self, dd: u32, device: u32, host: &mut impl Device) -> Result<()> {
        if dd == 0 || device == 0 || self.dd != 0 { return Ok(()); }
        acquire(host, dd); acquire(host, device); self.dd = dd; self.device = device;
        // RD6 explicitly leaves ranking and availability open. Supply resolved tables
        // at the boundary, not an algorithm based on guessed native formats.
        if self.formats.is_none() || self.converters.is_none() {
            let mut words = [0; 8 * 8 + 15 * 8];
            host.unresolved("RD6", &[device], &mut words)?;
            self.formats = Some(std::array::from_fn(|i| Format { words: std::array::from_fn(|j| words[i * 8 + j]) }));
            self.converters = Some(std::array::from_fn(|i| std::array::from_fn(|j| words[64 + i * 8 + j] != 0)));
        }
        let name = self.new_name();
        self.create(name, 0, 4, 1, 1, 1, host)?;
        // The known white pixel is an immediate value, not a foreign address.
        let mut white = [0];
        host.unresolved("RD6", &[0xffff_ffff, self.surface(name), 0, 4, 1, 1], &mut white)?;
        Ok(())
    }
    pub fn lose(&mut self, host: &mut impl Device) {
        for r in &mut self.records { release(host, &mut r.surface); }
        release(host, &mut self.device);
    }
    pub fn init_device(&mut self, device: u32, host: &mut impl Device) -> Result<()> {
        acquire(host, device); release(host, &mut self.device); self.device = device;
        for i in 0..self.records.len() {
            let r = self.records[i].clone();
            if r.used && r.width != 0 { self.create(i as u32, r.flags, r.format, r.width, r.height, r.levels, host)?; }
        }
        Ok(())
    }
    pub fn shutdown(&mut self, host: &mut impl Device) {
        self.free(0, host);
        for r in &mut self.records { release(host, &mut r.surface); }
        self.records.clear(); self.freed.clear(); release(host, &mut self.device); release(host, &mut self.dd);
    }
    #[allow(clippy::too_many_arguments)]
    pub fn create(&mut self, name: u32, flags: u32, format: u32, width: u32, height: u32, levels: u32,
        host: &mut impl Device) -> Result<bool> {
        if !(1..=4096).contains(&width) || !(1..=4096).contains(&height) { return Ok(false); }
        let device = self.device; let dd = self.dd;
        let chosen = self.formats.and_then(|f| f.get(format as usize).copied());
        let r = match self.get_mut(name) { Some(r) => r, None => return Ok(false) };
        r.width = width; r.height = height; r.levels = levels; r.flags = flags; r.format = format;
        release(host, &mut r.surface);
        if device == 0 { return Ok(true); }
        let chosen = chosen.ok_or(Error::Unsupported("RD6"))?;
        let caps2 = if flags & 1 != 0 { 0x20004 } else if flags & 2 != 0 { 0x20080 } else { 0x20000 };
        let mut args = vec![0x7c, 0x21007, width, height, levels.max(1), 0x401008, caps2];
        args.extend(chosen.words);
        let mut out = [0]; let hr = host.dx(dd, Call::CreateSurface, &args, &[], &mut out);
        if failed(hr) { return Ok(false); } r.surface = out[0]; Ok(true)
    }
    pub fn load(&self, u: Upload, host: &mut impl Device) -> Result<bool> {
        let r = match self.get(u.name) { Some(r) if r.surface != 0 && (u.level == 0 || u.level < r.levels) => r, _ => return Ok(false) };
        let (source, bits) = match source_format(u.format, u.kind) { Some(v) => v, None => return Ok(false) };
        let available = self.converters.ok_or(Error::Unsupported("RD6"))?;
        if r.format >= 8 || !available[source][r.format as usize] { return Ok(false); }
        let mut surface = r.surface; let mut owned = false;
        for _ in 0..u.level {
            let mut out = [0]; let hr = host.dx(surface, Call::AttachedSurface, &[0x400000], &[], &mut out);
            if owned { release(host, &mut surface); }
            if failed(hr) { return Ok(false); } surface = out[0]; owned = true;
        }
        let result = self.load_surface(surface, u, source as u32, bits, host);
        if owned { release(host, &mut surface); } result
    }
    fn load_surface(&self, surface: u32, u: Upload, source: u32, bits: u32, host: &mut impl Device) -> Result<bool> {
        let mut out = [0; 5]; // mapped address, pitch, bits, linear-size flag, linear byte size
        loop {
            let hr = host.dx(surface, Call::LockSurface, &[0x2821], &[], &mut out);
            if hr == LOST && !failed(dx(host, surface, Call::RestoreSurface, &[])) { continue; }
            if failed(hr) { return Ok(false); } break;
        }
        let row = if u.row_length == 0 { u.width } else { u.row_length };
        let pitch = bits.wrapping_mul(row) / 8;
        let result = if out[3] != 0 && (u.x != 0 || u.y != 0) { Ok(()) } else {
            let destination = if out[3] != 0 { out[0] } else { out[0].wrapping_add(u.y.wrapping_mul(out[1])).wrapping_add(u.x.wrapping_mul(out[2] / 8)) };
            host.unresolved("RD6", &[source, self.get(u.name).unwrap().format, u.pixels, pitch,
                destination, out[1], u.width, u.height, out[3], out[4]], &mut [])
        };
        dx(host, surface, Call::UnlockSurface, &[]); result.map(|()| true)
    }
}

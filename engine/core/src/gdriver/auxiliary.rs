// SPDX-License-Identifier: GPL-3.0-or-later
//! Auxiliary identities: lighting/materials and buffer regions (§§12.4–12.5).
use super::{backend::*, D3DLight, Driver};

#[derive(Clone, Debug)]
pub struct Region {
    pub identity: u32,
    pub kind: u32,
    pub surface: u32,
    pub reserved: [u32; 2],
    pub width: u32,
    pub height: u32,
}
impl Default for Region {
    fn default() -> Self {
        Self { identity: u32::MAX, kind: 0, surface: 0, reserved: [0; 2], width: 0, height: 0 }
    }
}

impl D3DLight {
    pub fn bytes(&self) -> Vec<u8> {
        let mut bytes = self.kind.to_le_bytes().to_vec();
        // D3DLIGHT7 memory order is diffuse, specular, ambient.
        for i in [1, 2, 0] { bytes.extend(floats(&self.colors[i])); }
        bytes.extend(floats(&self.position));
        bytes.extend(floats(&self.direction));
        bytes.extend(floats(&[self.range, self.falloff]));
        bytes.extend(floats(&self.attenuation));
        bytes.extend(floats(&[self.theta, self.phi]));
        bytes
    }
}

impl Driver {
    fn auxiliary_hr(&mut self, hr: u32) {
        if hr != 0 { self.error = u32::from(hr != LOST); }
    }
    fn auxiliary_submit(&mut self, host: &mut impl Device) {
        let hr = self.primitives.flush(host);
        self.auxiliary_hr(hr);
    }
    pub fn auxiliary_constant(&self) -> bool { true }
    pub fn lighting_noop(&self, _a: u32, _b: u32) {}
    pub fn lighting_enable(&mut self, value: u32, host: &mut impl Device) {
        let value = value as u8;
        if self.lighting_enabled == value { return; }
        self.lighting_enabled = value;
        self.auxiliary_submit(host);
        let hr = dx(host, self.device, Call::RenderState, &[137, value as u32]);
        self.auxiliary_hr(hr);
    }
    pub fn light_enable(&mut self, index: u32, value: u32, host: &mut impl Device) {
        self.auxiliary_submit(host);
        let hr = dx(host, self.device, Call::LightEnable, &[index, value & 255]);
        self.auxiliary_hr(hr);
    }
    pub fn global_ambient(&mut self, rgb: [f32; 3], host: &mut impl Device) {
        let packed = 0xff000000 | quantize(rgb[0]) << 16 | quantize(rgb[1]) << 8 | quantize(rgb[2]);
        self.auxiliary_submit(host);
        let hr = dx(host, self.device, Call::RenderState, &[139, packed]);
        self.auxiliary_hr(hr);
    }
    fn light_record(&mut self, index: u32) -> &mut D3DLight {
        self.lights.resize_with(self.lights.len().max(index as usize + 1), D3DLight::default);
        &mut self.lights[index as usize]
    }
    fn send_light(&mut self, index: u32, host: &mut impl Device) {
        let bytes = self.lights[index as usize].bytes();
        self.auxiliary_submit(host);
        let hr = host.dx(self.device, Call::Light, &[index], &bytes, &mut []);
        self.auxiliary_hr(hr);
    }
    pub fn light_colors(&mut self, index: u32, colors: [Option<[f32; 4]>; 3], host: &mut impl Device) {
        let light = self.light_record(index);
        for (out, value) in light.colors.iter_mut().zip(colors) {
            if let Some(value) = value { *out = value; }
        }
        self.send_light(index, host);
    }
    pub fn light_property(&mut self, index: u32, property: u32, value: Option<[f32; 4]>, host: &mut impl Device) -> Result<()> {
        let light = self.light_record(index);
        if property < 3 { light.colors[property as usize] = value.ok_or(Error::InvalidInput)?; }
        self.send_light(index, host);
        Ok(())
    }
    fn send_material(&mut self, host: &mut impl Device) {
        self.auxiliary_submit(host);
        let hr = host.dx(self.device, Call::Material, &[], &floats(&self.material), &mut []);
        self.auxiliary_hr(hr);
    }
    pub fn material_colors(&mut self, colors: [Option<[f32; 4]>; 4], power: f32, host: &mut impl Device) {
        for (i, value) in colors.into_iter().enumerate() {
            if let Some(value) = value {
                let offset = [4, 0, 8, 12][i];
                self.material[offset..offset + 4].copy_from_slice(&value);
            }
        }
        if power >= 0. { self.material[16] = power; }
        self.send_material(host);
    }
    pub fn material_property(&mut self, property: u32, values: Option<&[f32]>, host: &mut impl Device) -> Result<()> {
        if property <= 4 {
            let offset = [4, 0, 8, 12, 16][property as usize];
            let count = if property == 4 { 1 } else { 4 };
            let values = values.and_then(|v| v.get(..count)).ok_or(Error::InvalidInput)?;
            self.material[offset..offset + count].copy_from_slice(values);
        }
        self.send_material(host);
        Ok(())
    }
    pub fn copy_color_buffer(&self) -> Result<()> { Err(Error::Unsupported("RD7")) }
    fn region_live(&self, kind: u32) -> u32 {
        self.auxiliary_surfaces.get(kind as usize).copied().unwrap_or(0)
    }
    pub fn new_region(&mut self, kind: u32, host: &mut impl Device) -> u32 {
        if self.lost { return 0; }
        let i = self.regions.iter().position(|r| r.identity == u32::MAX).unwrap_or(self.regions.len());
        if i == self.regions.len() { self.regions.push(Region::default()); }
        let live = self.region_live(kind);
        if live == 0 { return 0; }
        let mut format = [0; 8];
        if failed(host.dx(live, Call::PixelFormat, &[], &[], &mut format)) { return 0; }
        let mut desc = [0; 3]; // width, height, caps
        if failed(host.dx(live, Call::SurfaceDescriptor, &[], &[], &mut desc)) || self.context.dd == 0 { return 0; }
        let caps = if kind == 0 { 0x40 } else { 0x20000 } | if desc[2] & 0x4000 != 0 { 0x4000 } else { 0x800 };
        let mut args = vec![0x7c, 0x1007, desc[0], desc[1], caps];
        args.extend(format);
        let mut out = [0];
        let mut hr = host.dx(self.context.dd, Call::CreateSurface, &args, &[], &mut out);
        if hr == 0x8876017c {
            dx(host, self.device, Call::EvictManagedTextures, &[]);
            hr = host.dx(self.context.dd, Call::CreateSurface, &args, &[], &mut out);
        }
        if failed(hr) { self.regions[i].surface = 0; return 0; }
        self.regions[i] = Region { identity: i as u32, kind, surface: out[0], reserved: [0; 2], width: self.current.words[1], height: self.current.words[2] };
        i as u32 + 1
    }
    pub fn delete_region(&mut self, handle: u32, host: &mut impl Device) -> bool {
        let i = handle.wrapping_sub(1);
        match self.regions.get_mut(i as usize) {
            Some(r) if r.identity == i => { release(host, &mut r.surface); *r = Region::default(); true }
            _ => false,
        }
    }
    pub fn is_region(&self, handle: u32) -> Result<bool> {
        let i = handle.wrapping_sub(1) as usize;
        if i == self.regions.len() { return Err(Error::InvalidInput); } // native off-by-one read
        Ok(self.regions.get(i).is_some_and(|r| r.identity == i as u32))
    }
    /// Transfer arguments: handle, x, y, width, height, otherX, otherY.
    pub fn transfer_region(&mut self, save: bool, a: [u32; 7], host: &mut impl Device) -> Result<bool> {
        let [handle, x, y, w, h, ox, oy] = a;
        let right = x.wrapping_add(w);
        let bottom = y.wrapping_add(h);
        if self.lost || (x as i32) < 0 || (y as i32) < 0 || right > self.current.words[1] || bottom > self.current.words[2] { return Ok(false); }
        // Transfer callers must supply a readable live handle; do not invent occupancy checks.
        let r = self.regions.get(handle.wrapping_sub(1) as usize).ok_or(Error::InvalidInput)?.clone();
        if !save && ((right as i32) > r.width as i32 || (bottom as i32) > r.height as i32) { return Ok(false); }
        self.auxiliary_submit(host);
        let live = self.region_live(r.kind);
        if r.surface == 0 || live == 0 || (!save && self.auxiliary_surfaces[1] == 0) { return Ok(false); }
        let region_rect = [x, y, right, bottom];
        let live_rect = [ox, oy, ox.wrapping_add(w), oy.wrapping_add(h)];
        let (target, source, dest_rect, src_rect) = if save { (r.surface, live, region_rect, live_rect) } else { (live, r.surface, live_rect, region_rect) };
        let mut args = vec![source];
        args.extend(dest_rect);
        args.extend(src_rect);
        args.push(0x01000200);
        let mut retained = true;
        loop {
            args[9] = 0x01000200;
            let mut hr = dx(host, target, Call::Blt, &args);
            if failed(hr) { args[9] = 0x01000000; hr = dx(host, target, Call::Blt, &args); }
            if !failed(hr) { return Ok(retained); }
            if hr != LOST { return Ok(false); }
            if !save { dx(host, r.surface, Call::SurfaceLost, &[]); return Ok(false); }
            let restored = dx(host, r.surface, Call::RestoreSurface, &[]);
            if failed(dx(host, self.context.identity, Call::RestoreContext, &[])) { return Ok(false); }
            dx(host, self.primitive_identity, Call::PrimitiveRecovery, &[]);
            if restored == 0 && !(x == 0 && y == 0 && w == r.width && h == r.height) {
                if r.kind < 2 {
                    host.dx(r.surface, Call::Blt, &[0, 0, if r.kind == 0 { 0x01000400 } else { 0x03000000 }], &[0; 100], &mut []);
                }
                retained = false;
            }
        }
    }
}

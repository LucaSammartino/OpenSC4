// SPDX-License-Identifier: GPL-3.0-or-later
//! Context selection, resource ownership and presentation (§14.2–14.3).
use super::backend::*;
use crate::gdrivers_director::Allocation;
#[derive(Debug, Default)]
pub struct Context {
    pub identity: u32,
    pub library: u32,
    pub dd: u32,
    pub d3d: u32,
    pub device: u32,
    pub primary: u32,
    pub target: u32,
    pub depth: u32,
    pub clipper: u32,
    pub window: u32,
    pub flipping: bool,
    pub active: bool,
    pub selected: DeviceDescriptor,
    pub request: Option<Request>,
}
/// GUID words in little-endian memory order, capability dword and render-bit-depth sentinel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeviceDescriptor {
    pub guid: [u32; 4],
    pub caps: u32,
    pub render_bits: u32,
}
pub const PREFERRED: [u32; 4] = [0x881949a1, 0x11d0d6f3, 0xa000ab89, 0x294105c9];
pub const FALLBACK: [u32; 4] = [0xa4665c60, 0x11cf2673, 0xaa001aa3, 0x5633b900];
pub const DIRECT: [u32; 4] = [0x50936643, 0x11d113e9, 0xa000aa89, 0x294105c9];
pub fn select_device(selector: u32, devices: &[DeviceDescriptor]) -> Result<DeviceDescriptor> {
    if selector == (-3i32) as u32 {
        return Ok(DeviceDescriptor {
            guid: DIRECT,
            caps: 0,
            render_bits: 0x10,
        });
    }
    if selector != u32::MAX && selector != (-4i32) as u32 {
        // §14.2 explicitly leaves the native stack selection undefined.
        return Err(Error::InvalidInput);
    }
    let mut selected = DeviceDescriptor::default();
    let score = |caps: u32| u32::from(caps & 0x10000 != 0) + u32::from(caps & 0x80000 != 0);
    for d in devices {
        let replace = if selector == u32::MAX {
            selected.render_bits == 0 || score(d.caps) > score(selected.caps)
        } else {
            d.guid == PREFERRED || d.guid == FALLBACK && (selected.render_bits == 0 || selected.guid != PREFERRED)
        };
        if replace {
            selected = *d;
        }
    }
    Ok(selected)
}
#[derive(Clone, Copy, Debug)]
pub struct Request {
    pub selector: u32,
    pub fullscreen: bool,
    pub flip: u8,
    pub window: u32,
    pub width: u32,
    pub height: u32,
    pub back_buffers: u32,
}
impl Context {
    pub fn create(r: Request, host: &mut impl Device) -> Result<(Self, u32)> {
        let mut c = Self {
            identity: host.alloc(0x48, Allocation::Heap),
            window: r.window,
            ..Self::default()
        };
        if c.identity == 0 {
            return Ok((c, OUT_OF_MEMORY));
        }
        let result = c.initialize(r, host);
        match result {
            Ok(hr) if !failed(hr) => Ok((c, hr)),
            Ok(hr) => {
                c.destroy(host);
                Ok((c, hr))
            }
            Err(e) => {
                c.destroy(host);
                Err(e)
            }
        }
    }
    fn initialize(&mut self, r: Request, host: &mut impl Device) -> Result<u32> {
        let mut out = [0];
        let mut hr = host.dx(0, Call::LoadLibrary, &[], b"DDRAW.DLL", &mut out);
        self.library = out[0];
        if failed(hr) || self.library == 0 {
            return Ok(FAIL);
        }
        hr = host.dx(self.library, Call::ResolveCreate, &[], b"DirectDrawCreateEx", &mut out);
        if failed(hr) || out[0] == 0 {
            return Ok(FAIL);
        }
        let entry = out[0];
        out[0] = 0;
        hr = host.dx(entry, Call::CreateDirectDraw, &[], &[], &mut out);
        if failed(hr) {
            return Ok(hr);
        }
        self.dd = out[0];
        hr = host.dx(self.dd, Call::GetCaps, &[], &[], &mut [0; 1]);
        if failed(hr) {
            return Ok(hr);
        }
        hr = host.dx(self.dd, Call::QueryDirect3D, &[], &[], &mut out);
        if failed(hr) {
            return Ok(hr);
        }
        self.d3d = out[0];
        self.rebuild(r, host)
    }
    pub fn restore(&mut self, host: &mut impl Device) -> Result<u32> {
        let hr = dx(host, self.dd, Call::RestoreAll, &[]);
        if failed(hr) {
            return Ok(hr);
        }
        match self.request {
            Some(r) => self.rebuild(r, host),
            None => Ok(0),
        }
    }
    pub fn rebuild(&mut self, r: Request, host: &mut impl Device) -> Result<u32> {
        self.active = false;
        self.flipping = false;
        self.request = Some(r);
        for token in [
            &mut self.device,
            &mut self.depth,
            &mut self.target,
            &mut self.primary,
            &mut self.clipper,
        ] {
            release(host, token);
        }
        let mut out = [0];
        let mut hr = dx(
            host,
            self.dd,
            Call::CooperativeFlags,
            &[r.window, if r.fullscreen { 0x11 } else { 8 }],
        );
        if failed(hr) {
            return Ok(hr);
        }
        if r.fullscreen {
            hr = dx(host, self.dd, Call::DisplayMode, &[r.width, r.height, 16]);
            if failed(hr) {
                return Ok(hr);
            }
        }
        let mut devices = Vec::new();
        if matches!(r.selector, u32::MAX | 0xffff_fffc) {
            let mut count = [0];
            hr = host.dx(self.d3d, Call::EnumerateDevices, &[], &[], &mut count);
            if failed(hr) {
                return Ok(hr);
            }
            for i in 0..count[0] {
                let mut words = [0; 6];
                hr = host.dx(self.d3d, Call::EnumerateDevices, &[i], &[], &mut words);
                if failed(hr) {
                    return Ok(hr);
                }
                devices.push(DeviceDescriptor {
                    guid: words[..4].try_into().unwrap(),
                    caps: words[4],
                    render_bits: words[5],
                });
            }
        }
        self.selected = select_device(r.selector, &devices)?;
        if r.fullscreen && r.flip != 0 {
            let mut n = r.back_buffers;
            loop {
                hr = host.dx(
                    self.dd,
                    Call::CreateSurface,
                    &[if n > 0 { 0x21 } else { 1 }, if n > 0 { 0x2218 } else { 0x2200 }, n],
                    &[],
                    &mut out,
                );
                if !failed(hr) {
                    self.primary = out[0];
                    self.flipping = n > 0;
                    break;
                }
                if n == 0 {
                    return Ok(hr);
                }
                n -= 1;
            }
            if self.flipping {
                hr = host.dx(self.primary, Call::AttachedSurface, &[4], &[], &mut out);
                if failed(hr) {
                    return Ok(hr);
                }
                self.target = out[0];
                if self.target == 0 {
                    return Ok(FAIL);
                }
            }
            for flag in [1, 4, 8] {
                dx(host, self.primary, Call::GetColorKey, &[flag]);
            }
        } else {
            hr = host.dx(self.dd, Call::CreateSurface, &[1, 0x200], &[], &mut out);
            if failed(hr) {
                return Ok(hr);
            }
            self.primary = out[0];
            hr = host.dx(
                self.dd,
                Call::CreateSurface,
                &[7, 0x2040, r.width, r.height],
                &[],
                &mut out,
            );
            if failed(hr) {
                return Ok(hr);
            }
            self.target = out[0];
            if !r.fullscreen {
                hr = host.dx(self.dd, Call::CreateClipper, &[], &[], &mut out);
                if failed(hr) {
                    return Ok(hr);
                }
                self.clipper = out[0];
                dx(host, self.clipper, Call::ClipperWindow, &[r.window]);
                dx(host, self.primary, Call::AttachClipper, &[self.clipper]);
            }
        }
        let mut format = [0; 8];
        // This backend operation cancels enumeration after the first callback.
        hr = host.dx(
            self.d3d,
            Call::EnumerateDepthFormats,
            &self.selected.guid,
            &[],
            &mut format,
        );
        if failed(hr) {
            return Ok(hr);
        }
        if format[0] == 0 {
            return Ok(FAIL);
        }
        let mut args = vec![
            0x1007,
            r.width,
            r.height,
            if self.selected.caps & 0x80000 != 0 {
                0x20000
            } else {
                0x20800
            },
        ];
        args.extend(format);
        out[0] = 0;
        hr = host.dx(self.dd, Call::CreateSurface, &args, &[], &mut out);
        if failed(hr) {
            return Ok(hr);
        }
        self.depth = out[0];
        hr = dx(host, self.target, Call::AttachDepth, &[self.depth]);
        if failed(hr) {
            release(host, &mut self.depth);
            return Ok(hr);
        }
        let mut display = [0];
        hr = host.dx(self.dd, Call::CurrentDisplayMode, &[], &[], &mut display);
        if failed(hr) {
            return Ok(hr);
        }
        if display[0] <= 8 {
            return Ok(0x88760078);
        }
        let mut args = self.selected.guid.to_vec();
        args.push(self.target);
        out[0] = 0;
        hr = host.dx(self.d3d, Call::CreateDevice, &args, &[], &mut out);
        if failed(hr) {
            return Ok(hr);
        }
        self.device = out[0];
        hr = dx(
            host,
            self.device,
            Call::Viewport,
            &[0, 0, r.width, r.height, 0, 1f32.to_bits()],
        );
        if failed(hr) {
            return Ok(hr);
        }
        self.active = true;
        Ok(0)
    }
    pub fn present(&self, flags: u32, host: &mut impl Device) -> u32 {
        if !self.active {
            return flags;
        }
        if self.flipping {
            return dx(host, self.primary, Call::Flip, &[0, flags]);
        }
        // Count-zero primary fallback has no Flip and no separate render target.
        if self.target == 0 {
            return flags;
        }
        let mut rect = [0; 4];
        host.dx(self.window, Call::ClientRect, &[], &[], &mut rect);
        if rect[0] == rect[2] || rect[1] == rect[3] {
            return LOST;
        }
        for pair in rect.chunks_mut(2) {
            let args = [pair[0], pair[1]];
            host.dx(self.window, Call::ClientToScreen, &args, &[], pair);
        }
        let args = [self.target, rect[0], rect[1], rect[2], rect[3], 0, 0x01000200];
        let hr = dx(host, self.primary, Call::Blt, &args);
        if failed(hr) {
            let mut retry = args;
            retry[6] = 0x01000000;
            dx(host, self.primary, Call::Blt, &retry)
        } else {
            hr
        }
    }
    pub fn destroy(&mut self, host: &mut impl Device) {
        self.active = false;
        for token in [
            &mut self.device,
            &mut self.depth,
            &mut self.target,
            &mut self.primary,
            &mut self.clipper,
            &mut self.d3d,
            &mut self.dd,
        ] {
            release(host, token);
        }
        if self.library != 0 {
            dx(host, self.library, Call::UnloadLibrary, &[]);
            self.library = 0;
        }
        if self.identity != 0 {
            host.free(self.identity, Allocation::Heap);
            self.identity = 0;
        }
    }
}

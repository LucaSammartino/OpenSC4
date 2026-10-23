// SPDX-License-Identifier: GPL-3.0-or-later
//! The specified context prefix and presentation (§14.2–14.3). Device/depth
//! selection itself is the explicit RD8 boundary, not a guessed Windows adapter.
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
        hr = dx(
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
                hr = dx(host, self.clipper, Call::ClipperWindow, &[r.window]);
                if failed(hr) {
                    return Ok(hr);
                }
                hr = dx(host, self.primary, Call::AttachClipper, &[self.clipper]);
                if failed(hr) {
                    return Ok(hr);
                }
            }
        }
        let mut selected = [0; 2];
        host.unresolved("RD8", &[self.d3d, self.target, r.selector], &mut selected)?;
        self.device = selected[0];
        self.depth = selected[1];
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

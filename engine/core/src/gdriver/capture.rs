// SPDX-License-Identifier: GPL-3.0-or-later
//! Owner surface references (§14.2) and CopyColorBuffer (§12.6).
use super::*;

impl Driver {
    pub(super) fn bind_owner_surfaces(&mut self, host: &mut impl Device) {
        for (slot, token) in [
            (&mut self.auxiliary_surfaces[1], self.context.depth),
            (&mut self.primary_surface, self.context.primary),
        ] {
            if *slot != token {
                acquire(host, token);
                release(host, slot);
                *slot = token;
            }
        }
        release(host, &mut self.auxiliary_surfaces[0]);
        host.dx(
            self.device,
            Call::GetRenderTarget,
            &[],
            &[],
            &mut self.auxiliary_surfaces[..1],
        );
        // +234 is deliberately not populated; it is not another depth alias.
    }

    pub fn copy_color_buffer(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        destination: u32,
        host: &mut impl Device,
    ) -> Result<u32> {
        if self.lost {
            return Ok(0);
        }
        // §12.6 tests nonzero, rather than the usual negative-HRESULT test.
        let hr = self.primitives.flush(host);
        if hr != 0 {
            self.error = u32::from(hr != LOST);
        }
        let surface = self.auxiliary_surfaces[0];
        let mut desc = [0; 10];
        if self.color_probe.is_none() {
            let hr = lock_surface(surface, &mut desc, host);
            let success = !failed(hr);
            if success {
                dx(host, surface, Call::UnlockSurface, &[]);
            }
            self.color_probe = Some(success);
        }
        if self.color_probe != Some(true) {
            return Ok(0);
        }
        if failed(lock_surface(surface, &mut desc, host)) {
            return Ok(destination);
        }
        // All portable normal/fault exits after this lock unlock exactly once.
        let result = self.copy_locked_color(x, y, width, height, destination, desc, host);
        dx(host, surface, Call::UnlockSurface, &[]);
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn copy_locked_color(
        &self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        mut dest: u32,
        desc: [u32; 10],
        host: &mut impl Device,
    ) -> Result<u32> {
        if dest == 0 {
            let mut out = [0];
            host.dx(0, Call::CreateColorBuffer, &[], &[], &mut out);
            dest = out[0];
        }
        if dest == 0 {
            return Ok(0);
        }
        let mut info = [0; 3];
        host.dx(dest, Call::ColorBufferInfo, &[], &[], &mut info);
        let (working_width, working_height) = if info[0] == 0 {
            if failed(dx(host, dest, Call::InitColorBuffer, &[width as u32, height as u32, 9])) {
                return Ok(dest);
            }
            (width, height)
        } else {
            (width.max(info[1] as i32), height.max(info[2] as i32))
        };
        let left = x.max(0);
        let top = y.max(0);
        let right = x.wrapping_add(width).min(desc[2] as i32);
        let bottom = y.wrapping_add(height).min(desc[3] as i32);
        if left != x
            || top != y
            || right != x.wrapping_add(working_width)
            || bottom.wrapping_add(y).wrapping_add(working_height) != 0
        {
            fill_buffer(dest, 0xff000000, host);
        }
        if desc[4] & 0x40 == 0 || !matches!(desc[5], 16 | 24 | 32) {
            fill_buffer(dest, 0xff00ffff, host);
            return Ok(dest);
        }
        let mut out = [0; 2];
        if failed(host.dx(dest, Call::LockColorBuffer, &[0x8080], &[], &mut out)) {
            return Ok(dest);
        }
        let result = copy_pixels(left, top, right, bottom, desc, out, host);
        dx(host, dest, Call::UnlockColorBuffer, &[0x8080]);
        result.map(|()| dest)
    }
}

fn lock_surface(surface: u32, desc: &mut [u32; 10], host: &mut impl Device) -> u32 {
    loop {
        let hr = host.dx(surface, Call::LockColorSurface, &[1], &[], desc);
        if hr != 0x8876021c {
            return hr;
        }
    }
}
fn fill_buffer(dest: u32, color: u32, host: &mut impl Device) {
    if !failed(dx(host, dest, Call::LockColorBuffer, &[0x8001])) {
        dx(host, dest, Call::FillColorBuffer, &[color, 0]);
        dx(host, dest, Call::UnlockColorBuffer, &[0x8001]);
    }
}
fn color16(v: u32, masks: &[u32]) -> Option<u32> {
    let (r, g, b, a, gbits) = match masks {
        [0xf800, 0x07e0, 0x001f, _] => ((v >> 11) & 31, (v >> 5) & 63, v & 31, 255, 6),
        [0x7c00, 0x03e0, 0x001f, alpha @ (0 | 0x8000)] => (
            (v >> 10) & 31,
            (v >> 5) & 31,
            v & 31,
            if *alpha == 0 || v & 0x8000 != 0 { 255 } else { 0 },
            5,
        ),
        [0xf800, 0x07c0, 0x003e, 1] => (
            (v >> 11) & 31,
            (v >> 6) & 31,
            (v >> 1) & 31,
            if v & 1 != 0 { 255 } else { 0 },
            5,
        ),
        _ => return None,
    };
    Some(a << 24 | (255 * r / 31) << 16 | (255 * g / ((1 << gbits) - 1)) << 8 | (255 * b / 31))
}
fn copy_pixels(
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    desc: [u32; 10],
    out: [u32; 2],
    host: &mut impl Device,
) -> Result<()> {
    if desc[5] == 16 && color16(0, &desc[6..]).is_none() {
        return Ok(());
    }
    // §12.6 gives no safe empty/wrapped rectangle promise. Refuse such inputs
    // instead of claiming the native unsigned pointer loop is an empty copy.
    if right <= left || bottom <= top || out[1] == 0 || out[1] as i32 <= 0 {
        return Err(Error::InvalidInput);
    }
    let size = desc[5] / 8;
    for row in 0..(bottom - top) as u32 {
        for col in 0..(right - left) as u32 {
            let mut bytes = [0; 4];
            host.read(
                desc[0]
                    .wrapping_add((top as u32 + row).wrapping_mul(desc[1]))
                    .wrapping_add((left as u32 + col).wrapping_mul(size)),
                &mut bytes[..size as usize],
            )?;
            let pixel = match size {
                2 => color16(u16::from_le_bytes(bytes[..2].try_into().unwrap()) as u32, &desc[6..]).unwrap(),
                3 => u32::from_le_bytes([255, bytes[0], bytes[1], bytes[2]]),
                _ => u32::from_le_bytes(bytes),
            };
            host.write(
                out[0].wrapping_add(row.wrapping_mul(out[1])).wrapping_add(col * 4),
                &pixel.to_le_bytes(),
            )?;
        }
    }
    Ok(())
}

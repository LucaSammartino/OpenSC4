// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use base::Rect;

pub const NETWORK_MESSAGES: [u32; 2] = [0x99ef_1142, 0x99ef_1143];

impl MiniMap {
    pub fn set_area(&mut self, windows: &mut Windows, host: &mut impl Host, area: Rect) -> Result<()> {
        windows.set_area(host, self.window, area);
        let side = area.width().min(area.height());
        // SPEC-GAP: nonpositive window extents have no specified power of two; keep n=0.
        self.n = if side > 0 { 1usize << (31 - (side as u32).leading_zeros()) } else { 0 };
        if self.map.pixels.is_some() {
            self.on_city_init(host)?;
        }
        Ok(())
    }

    pub fn init_tz_map(&mut self, host: &mut impl Host) -> Result<()> {
        let [mut cx, cz] = host.city_cells();
        if cx == 0 || cz == 0 {
            return Ok(());
        }
        if self.n == 0 {
            return Err(Unsupported::Precondition("positive minimap size"));
        }
        self.map.init(self.n, self.n)?;
        self.shift = 0;
        while cx > self.n {
            cx >>= 1;
            self.shift += 1;
        }
        while cx < self.n {
            cx <<= 1;
            self.shift -= 1;
        }
        self.invalidate_tz_map();
        Ok(())
    }

    pub fn on_city_init(&mut self, host: &mut impl Host) -> Result<()> {
        if self.attached {
            return Ok(());
        }
        self.attached = true;
        self.init_tz_map(host)?;
        self.cells = host.city_cells();
        let words = self.cells[0].checked_mul(self.cells[1])
            .and_then(|v| v.checked_mul(2)).and_then(|v| v.checked_add(31))
            .ok_or(Unsupported::Precondition("network vector size"))? >> 5;
        self.networks.try_reserve(words.saturating_sub(self.networks.len()))
            .map_err(|_| Unsupported::Precondition("network allocation"))?;
        self.networks.resize(words, 0);
        host.animation_target(self.window, true);
        self.hook_occupant_messages(host);
        Ok(())
    }

    pub fn on_city_shutdown(&mut self, host: &mut impl Host) {
        if self.attached {
            self.unhook_occupant_messages(host);
            host.animation_target(self.window, false);
            self.map.free();
            self.attached = false;
            self.cells = [0; 2];
        }
    }

    pub fn invalidate_tz_map(&mut self) {
        self.dirty = [u32::MAX; 16];
        self.map_dirty = true;
        self.surface_dirty = true;
    }

    pub fn hook_occupant_messages(&mut self, host: &mut impl Host) {
        if !self.hooked && self.attached {
            if self.flags & 2 != 0 {
                for id in NETWORK_MESSAGES { host.subscribe(self.window, id, true); }
            }
            self.hooked = true;
        }
    }

    pub fn unhook_occupant_messages(&mut self, host: &mut impl Host) {
        if self.hooked {
            if self.flags & 2 != 0 {
                for id in NETWORK_MESSAGES { host.subscribe(self.window, id, false); }
            }
            self.hooked = false;
        }
    }
}

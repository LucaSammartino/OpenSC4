// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use base::Rect;

pub trait Network: widget_controls::Foreign {
    /// Occupants slot 32. Returns an owned result; the foreign service applies
    /// the supplied core filter to its candidates, without a native owner call.
    fn find_occupant(&mut self, x: usize, z: usize, filter: &SurfaceNetworkFilter) -> Option<Handle>;
    /// QI 0xA821EF94, an acquired interface when present.
    fn network_interface(&mut self, occupant: Handle) -> Option<Handle>;
    fn network_type_mask(&mut self, interface: Handle) -> u32;
    fn occupant_cell(&mut self, interface: Handle) -> [i32; 2];
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SurfaceNetworkFilter;
impl SurfaceNetworkFilter {
    pub fn query_interface(&self, iid: u32) -> bool { matches!(iid, 1 | 0x46a8_0026) }
    pub fn accepts(&self, host: &mut impl Network, occupant: Handle) -> bool {
        let Some(interface) = host.network_interface(occupant) else { return false; };
        let accepted = host.network_type_mask(interface) & 0x1f4f != 0;
        host.release(interface);
        accepted
    }
}

impl MiniMap {
    pub fn update_networks(&mut self, host: &mut impl Host) -> Result<()> {
        let filter = SurfaceNetworkFilter;
        for z in 0..self.cells[1] { for x in 0..self.cells[0] {
            let b = 2 * (self.cells[0] * z + x);
            let word = self.networks.get_mut(b >> 5).ok_or(Unsupported::Precondition("network cache size"))?;
            let checked = 1u32 << (b & 31);
            let present = checked << 1;
            if *word & checked == 0 {
                *word &= !present;
                if let Some(occupant) = host.find_occupant(x, z, &filter) {
                    *word |= present;
                    host.release(occupant);
                }
                *word |= checked;
            }
            if *word & present != 0 {
                let (row, col) = if self.shift >= 0 { (x >> self.shift, z >> self.shift) }
                    else { (x << -self.shift, z << -self.shift) };
                let pixels = self.map.pixels.as_mut().ok_or(Unsupported::Precondition("network map pixels"))?;
                let offset = row.checked_mul(self.n).and_then(|v| v.checked_add(col));
                let pixel = offset.filter(|_| col < self.n).and_then(|i| pixels.get_mut(i))
                    .ok_or(Unsupported::Precondition("network pixel outside map"))?;
                *pixel = 0xffcf_cfc5;
            }
        } }
        Ok(())
    }

    pub fn network_changed(&mut self, host: &mut impl Host, occupant: Handle, tag: u32, removed: bool) -> Result<()> {
        if tag != 0xc772_bf98 { return Ok(()); }
        let Some(interface) = host.network_interface(occupant) else { return Ok(()); };
        let [x, z] = host.occupant_cell(interface);
        host.release(interface);
        if x < 0 || z < 0 || x as usize >= self.cells[0] || z as usize >= self.cells[1] { return Ok(()); }
        let b = 2 * (self.cells[0] * z as usize + x as usize);
        let word = self.networks.get_mut(b >> 5).ok_or(Unsupported::Precondition("network change cache size"))?;
        *word &= !(1 << (b & 31));
        if removed { self.invalidate_tz_map_rect(Rect(x, z, x, z))?; }
        else { self.surface_dirty = true; }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::tests::fixture;

    #[test]
    fn filter_iids_and_surface_mask() {
        let (_, mut h, _) = fixture();
        let f = SurfaceNetworkFilter;
        assert!(f.query_interface(1) && f.query_interface(0x46a80026));
        assert!(!f.query_interface(INTERFACE_ID));
        assert!(!f.accepts(&mut h, 100));
        h.network_interface_present = true;
        h.network_mask = 0x1f4f;
        assert!(f.accepts(&mut h, 100));
        h.network_mask = 0x80000000;
        assert!(!f.accepts(&mut h, 100));
        assert_eq!(h.events, ["release:200", "release:200"]);
    }

    #[test]
    fn checked_pairs_stale_reuse_upscale_single_pixel_and_changes() {
        let (mut w, mut h, mut m) = fixture();
        h.cells = [16, 16];
        h.network_cell = Some([1, 2]);
        m.set_area(&mut w, &mut h, Rect(0, 0, 32, 32)).unwrap();
        m.on_city_init(&mut h).unwrap();
        m.update_networks(&mut h).unwrap();
        assert_eq!(h.finds, 256);
        assert_eq!(m.map.pixels().unwrap()[2 * 32 + 4], 0xffcfcfc5);
        assert_eq!(m.map.pixels().unwrap()[2 * 32 + 5], 0);
        assert_eq!(m.map.pixels().unwrap()[3 * 32 + 4], 0);
        m.on_city_shutdown(&mut h);
        h.network_cell = None;
        m.on_city_init(&mut h).unwrap();
        m.update_networks(&mut h).unwrap();
        assert_eq!(h.finds, 256); // stale checked/present pair survived
        assert_eq!(m.map.pixels().unwrap()[2 * 32 + 4], 0xffcfcfc5);
        h.network_interface_present = true;
        h.occupant_cell = [1, 2];
        m.dirty.fill(0);
        m.map_dirty = false;
        m.surface_dirty = false;
        m.network_changed(&mut h, 100, 0, false).unwrap();
        assert!(!m.surface_dirty);
        m.network_changed(&mut h, 100, 0xc772bf98, false).unwrap();
        assert!(m.surface_dirty && !m.map_dirty);
        m.update_networks(&mut h).unwrap();
        assert_eq!(h.finds, 257);
        m.network_changed(&mut h, 100, 0xc772bf98, true).unwrap();
        assert_eq!(m.dirty[0], 1);
        assert!(m.map_dirty);
    }
}

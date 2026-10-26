// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub const TUNING_KEY: [u32; 3] = [0x6534_284a, 0x690f_693f, 0x8a35_6f2c];

/// §4.2: unsigned 8-bit fixed-point interpolation, including the alpha lane.
pub fn interpolate_ramp(out: &mut [u32], ramp: &[(u32, u32)]) -> Result<()> {
    if ramp.is_empty() || ramp.windows(2).any(|w| w[0].0 >= w[1].0) {
        return Err(Unsupported::Precondition("nonempty rising ramp keys"));
    }
    let mut j = 0;
    for (i, pixel) in out.iter_mut().enumerate() {
        let i = i as u64;
        while j + 1 < ramp.len() && u64::from(ramp[j + 1].0) <= i { j += 1; }
        if i < u64::from(ramp[0].0) || j + 1 == ramp.len() {
            *pixel = ramp[j].1;
            continue;
        }
        let (key, a) = ramp[j];
        let (next, b) = ramp[j + 1];
        let t = ((i - u64::from(key)) << 8) / u64::from(next - key);
        *pixel = 0;
        for shift in [0, 8, 16, 24] {
            let a = i64::from((a >> shift) & 255);
            let b = i64::from((b >> shift) & 255);
            let byte = (a * 256 + (b - a) * t as i64 + 128) >> 8;
            *pixel |= (byte as u32) << shift;
        }
    }
    Ok(())
}

impl MiniMap {
    pub fn load_tunable_values(&mut self, host: &mut impl Host) -> Result<()> {
        if let Some(ramp) = host.tuning_ramp(TUNING_KEY, 0xea35_73af)? {
            interpolate_ramp(&mut self.terrain_palette, &ramp)?;
            for color in &mut self.terrain_palette { *color &= 0x00ff_ffff; }
        }
        if let Some(ramp) = host.tuning_ramp(TUNING_KEY, 0x6b73_3233)? {
            interpolate_ramp(&mut self.water_palette, &ramp)?;
        }
        self.invalidate_tz_map();
        Ok(())
    }

    pub fn set_default_zone_color(&mut self, color: u32) {
        self.zone_colors.fill(color | 0xff00_0000);
        self.zone_colors[0] = 0;
    }

    pub fn set_default_zone_color_to_trans(&mut self) { self.zone_colors.fill(0); }

    pub fn set_zone_color(&mut self, index: usize, color: u32) {
        if let Some(slot) = self.zone_colors.get_mut(index) {
            *slot = if color == 0 { 0 } else { color | 0xff00_0000 };
        }
    }

    pub fn set_overlay(&mut self, host: &mut impl Host, overlay: Option<Handle>) {
        widget_controls::replace_ref(&mut self.overlay, overlay, host);
        self.surface_dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::tests::fixture;

    #[test]
    fn mm_c6_shipped_ramp_and_endpoints() {
        let ramp = [(0, 0xff536fd7), (58, 0xff4b83e7), (62, 0xffbbb198),
            (64, 0xffcdbb8d), (68, 0xff65963d), (72, 0xff70aa8e),
            (88, 0xff9a8a62), (255, 0xffffffff)];
        let mut palette = [0; 256];
        interpolate_ramp(&mut palette, &ramp).unwrap();
        assert_eq!(palette[60], 0xff839ac0); // MM-C6, t=128
        assert_eq!(palette[0], 0xff536fd7);
        assert_eq!(palette[255], 0xffffffff);
        let mut p = [0; 6];
        interpolate_ramp(&mut p, &[(2, 0), (4, 0xff00ff00)]).unwrap();
        assert_eq!(p, [0, 0, 0, 0x80008000, 0xff00ff00, 0xff00ff00]);
        assert!(interpolate_ramp(&mut p, &[]).is_err());
        assert!(interpolate_ramp(&mut p, &[(2, 1), (2, 2)]).is_err());
    }

    #[test]
    fn missing_tuning_keeps_palettes_but_dirties_map() {
        let (_, mut host, mut map) = fixture();
        map.terrain_palette[1] = 9;
        map.water_palette[1] = 0xabcdef01;
        map.load_tunable_values(&mut host).unwrap();
        assert_eq!(map.terrain_palette[1], 9);
        assert_eq!(map.water_palette[1], 0xabcdef01);
        assert!(map.map_dirty && map.surface_dirty);
    }
}

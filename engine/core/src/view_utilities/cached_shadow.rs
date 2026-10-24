// SPDX-License-Identifier: GPL-3.0-or-later
//! Read-only cached-input adapter for VU16 R-class arithmetic comparisons.
//! This does not validate camera/frustum construction: those outputs are inputs
//! to this explicitly scoped comparison, never expected projection outputs.
use super::*;

impl ViewUtilities {
    /// Internal observer transport, with the native word offsets in spec §1.
    /// No pointers in the snapshot are dereferenced or retained.
    #[doc(hidden)]
    pub fn from_native_cached_words(words: &[u32]) -> Option<Self> {
        if words.len() != 0x470 / 4 {
            return None;
        }
        let f = |offset: usize| f32::from_bits(words[offset / 4]);
        let v = |offset| Vec3::new(f(offset), f(offset + 4), f(offset + 8));
        let volume = ViewVolume {
            left: f(0x108),
            right: f(0x10c),
            bottom: f(0x110),
            top: f(0x114),
            near: f(0x118),
            far: f(0x11c),
        };
        let mut out = Self {
            valid: words[0] & 0xff != 0,
            city: (f(0x20), f(0x24)),
            camera: CameraState {
                position: v(0x3c),
                look_at: v(0x48),
                zoom: words[0x0c / 4],
                rotation: words[0x10 / 4] as i32,
                viewport_width: words[0x28 / 4] as i32,
                viewport_height: words[0x2c / 4] as i32,
                near_clip_distance: f(0x158),
                far_clip_distance: f(0x15c),
                metres_per_pixel: f(0x150),
                pixels_per_metre: f(0x154),
                volume,
                ..CameraState::default()
            },
            sin_negative_yaw: f(0x448),
            cos_negative_yaw: f(0x44c),
            sin_pitch: f(0x450),
            cos_pitch: f(0x454),
            scales: v(0x458),
            inverse_scales: v(0x464),
            ..Self::default()
        };
        out.view.volume = volume;
        out.view.bounds = Aabb {
            min: v(0x78),
            max: v(0x84),
        };
        for (i, corner) in out.view.corners.iter_mut().enumerate() {
            *corner = v(0xa8 + i * 12);
        }
        for (i, plane) in out.view.planes.iter_mut().enumerate() {
            *plane = Plane {
                normal: v(0x160 + i * 16),
                offset: f(0x16c + i * 16),
            };
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_adapter_uses_inputs_without_recomputing_trig_or_scales() {
        let mut words = [0; 0x470 / 4];
        words[0x44c / 4] = 1.0_f32.to_bits();
        words[0x454 / 4] = 1.0_f32.to_bits();
        words[0x3c / 4] = 8.0_f32.to_bits();
        words[0x458 / 4] = 4.0_f32.to_bits();
        let view = ViewUtilities::from_native_cached_words(&words).unwrap();
        assert_eq!(
            view.project_point_to_view_coordinates(Vec3::new(10.0, 2.0, 3.0)),
            Vec3::new(2.0, 2.0, 3.0)
        );
        assert_eq!(
            view.project_point_from_view_coordinates_to_viewport_coordinates(Vec3::new(2.0, 0.0, 0.0))
                .x,
            8.0
        );
        assert!(ViewUtilities::from_native_cached_words(&words[..10]).is_none());
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
//! Read-only cached-input adapter for VU16 R-class arithmetic comparisons.
//! This does not validate camera/frustum construction: those outputs are inputs
//! to this explicitly scoped comparison, never expected projection outputs.
use super::*;

impl TriangleIterator<'_> {
    /// Native non-deleting destructor transport (§12/§13.3). The iterator
    /// borrows its buffers; only its vtable changes when flags bit 0 is clear.
    /// A native-heap deletion cannot be represented by borrowed word storage.
    #[doc(hidden)]
    pub fn native_non_deleting_destructor_words(words: &mut [u32], flags: u32) -> Option<()> {
        if words.len() != 0x2c / 4 || flags & 1 != 0 {
            return None;
        }
        words[0] = 0x00ab_d92c;
        Some(())
    }
}

impl ViewUtilities {
    /// Constructor transport only (§1). No camera-derived field is seeded
    /// from a native constructor result.
    #[doc(hidden)]
    pub fn native_constructor_words(initial: &[u32]) -> Option<[u32; 0x470 / 4]> {
        if initial.len() != 0x470 / 4 {
            return None;
        }
        let view = Self::new();
        let mut words = [0; 0x470 / 4];
        for (offset, value) in [0x3f4, 0x3f8, 0x410].into_iter().zip(view.unused_constructor_fields) {
            words[offset / 4] = value.to_bits();
        }
        // §1/§6 flags occupy bytes, not whole words. Preserve their adjacent
        // padding from the supplied canary. gen27-r2b found exactly these three
        // words (0, 138, 273) unchanged outside the documented flag bytes.
        words[0] = initial[0] & !0xff;
        words[0x228 / 4] = initial[0x228 / 4] & !0xff;
        words[0x444 / 4] = initial[0x444 / 4] & !0xffff;
        Some(words)
    }

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
            renderer_present: words[1] != 0,
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
        if words[0x444 / 4] & 0xff != 0 {
            let _ = out.visible_area.set(Some(VisibleArea {
                inner: Aabb {
                    min: v(0x414),
                    max: v(0x420),
                },
                outer: Aabb {
                    min: v(0x42c),
                    max: v(0x438),
                },
            }));
        }
        out.view.volume = volume;
        out.view.bounds = Aabb {
            min: v(0x78),
            max: v(0x84),
        };
        for (i, corner) in out.view.corners.iter_mut().enumerate() {
            *corner = v(0xa8 + i * 12);
        }
        let lines = |base| {
            std::array::from_fn(|i| {
                let at = base + i * 16;
                GroundLine {
                    origin: Vec3::new(f(at), 0.0, f(at + 4)),
                    direction: Vec3::new(f(at + 8), 0.0, f(at + 12)),
                }
            })
        };
        out.view.ground_lines = lines(0x208);
        out.draw.ground_lines = lines(0x3bc);
        for (i, plane) in out.view.planes.iter_mut().enumerate() {
            *plane = Plane {
                normal: v(0x160 + i * 16),
                offset: f(0x16c + i * 16),
            };
        }
        out.view.city_clipped_bounds = Some(Aabb {
            min: v(0x90),
            max: v(0x9c),
        });
        out.view.ground_points = std::array::from_fn(|i| v(0x120 + i * 12));
        out.draw_narrowed = words[0x228 / 4] & 0xff != 0;
        out.draw_rect = PixelRect::new(
            words[0x22c / 4] as i32,
            words[0x230 / 4] as i32,
            words[0x234 / 4] as i32,
            words[0x238 / 4] as i32,
        );
        out.draw.bounds = Aabb {
            min: v(0x23c),
            max: v(0x248),
        };
        out.draw.city_clipped_bounds = Some(Aabb {
            min: v(0x254),
            max: v(0x260),
        });
        out.draw.corners = std::array::from_fn(|i| v(0x26c + i * 12));
        out.draw.ground_points = std::array::from_fn(|i| v(0x2e4 + i * 12));
        out.draw.volume = ViewVolume {
            left: f(0x2cc),
            right: f(0x2d0),
            bottom: f(0x2d4),
            top: f(0x2d8),
            near: f(0x2dc),
            far: f(0x2e0),
        };
        out.draw.planes = std::array::from_fn(|i| Plane {
            normal: v(0x314 + i * 16),
            offset: f(0x320 + i * 16),
        });
        out.view.reduced_planes = std::array::from_fn(|i| v(0x1c0 + i * 12));
        out.draw.reduced_planes = std::array::from_fn(|i| v(0x374 + i * 12));
        Some(out)
    }

    /// A private provider explicitly supplies camera presence; no game pointer
    /// is inferred or dereferenced by the transport.
    #[doc(hidden)]
    pub fn native_camera_presence(&mut self, present: bool) {
        self.camera_present = present;
    }

    /// Serialize the whole-viewport draw branch, including cache writes.
    #[doc(hidden)]
    pub fn native_whole_draw_words(&self, words: &mut [u32]) -> Option<()> {
        if words.len() != 0x470 / 4 || self.draw_narrowed {
            return None;
        }
        words[0x228 / 4] &= !0xff;
        for (i, value) in [
            self.draw_rect.top_left.x,
            self.draw_rect.top_left.y,
            self.draw_rect.bottom_right.x,
            self.draw_rect.bottom_right.y,
        ]
        .into_iter()
        .enumerate()
        {
            words[0x22c / 4 + i] = value as u32;
        }
        let put = |words: &mut [u32], at: usize, p: Vec3| {
            words[at / 4..at / 4 + 3].copy_from_slice(&[p.x.to_bits(), p.y.to_bits(), p.z.to_bits()]);
        };
        put(words, 0x23c, self.draw.bounds.min);
        put(words, 0x248, self.draw.bounds.max);
        let bounds = self.draw.city_clipped_bounds?;
        put(words, 0x254, bounds.min);
        put(words, 0x260, bounds.max);
        for (i, p) in self.draw.corners.into_iter().enumerate() {
            put(words, 0x26c + i * 12, p);
        }
        let volume = self.draw.volume;
        for (i, f) in [
            volume.left,
            volume.right,
            volume.bottom,
            volume.top,
            volume.near,
            volume.far,
        ]
        .into_iter()
        .enumerate()
        {
            words[0x2cc / 4 + i] = f.to_bits();
        }
        for (i, plane) in self.draw.planes.into_iter().enumerate() {
            put(words, 0x314 + i * 16, plane.normal);
            words[0x320 / 4 + i * 4] = plane.offset.to_bits();
        }
        for (i, p) in self.draw.reduced_planes.into_iter().enumerate() {
            put(words, 0x374 + i * 12, p);
        }
        for (i, line) in self.draw.ground_lines.into_iter().enumerate() {
            words[0x3bc / 4 + i * 4..0x3bc / 4 + i * 4 + 4].copy_from_slice(&[
                line.origin.x.to_bits(),
                line.origin.z.to_bits(),
                line.direction.x.to_bits(),
                line.direction.z.to_bits(),
            ]);
        }
        if let Some(Some(points)) = self.viewport_ground.get() {
            words[0x444 / 4] = (words[0x444 / 4] & !0xffff) | 0x0100;
            for (i, p) in points.iter().enumerate() {
                for (j, f) in [p.x, p.y, p.z].into_iter().enumerate() {
                    words[0x120 / 4 + i * 3 + j] = f.to_bits();
                    words[0x2e4 / 4 + i * 3 + j] = f.to_bits();
                }
            }
        }
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_deleting_destructor_preserves_borrowed_fields_and_rejects_heap_deletion() {
        let initial = [0xcccc_cccc; 11];
        for flags in [0, 2, 0xffff_fffe] {
            let mut words = initial;
            assert_eq!(
                TriangleIterator::native_non_deleting_destructor_words(&mut words, flags),
                Some(())
            );
            assert_eq!(words[0], 0x00ab_d92c);
            assert_eq!(words[1..], initial[1..]);
        }
        for flags in [1, 3, u32::MAX] {
            let mut words = initial;
            assert_eq!(
                TriangleIterator::native_non_deleting_destructor_words(&mut words, flags),
                None
            );
            assert_eq!(words, initial);
        }
        let mut short = [0xcccc_cccc; 10];
        assert_eq!(
            TriangleIterator::native_non_deleting_destructor_words(&mut short, 0),
            None
        );
        assert_eq!(short, [0xcccc_cccc; 10]);
    }

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

    #[test]
    fn cached_box_predicate_uses_the_published_ground_lines() {
        let mut words = [0; 0x470 / 4];
        // A at x=-1 points toward -z; B at x=1 points toward -z.
        // Inside corners satisfy A<0 and B>0, with permissive vertical planes.
        for (corner, x, z) in [(3, -1.0_f32, 0.0_f32), (7, -1.0, -1.0), (0, 1.0, 0.0), (4, 1.0, -1.0)] {
            words[(0xa8 + corner * 12) / 4] = x.to_bits();
            words[(0xb0 + corner * 12) / 4] = z.to_bits();
        }
        words[0x18c / 4] = 1.0_f32.to_bits();
        words[0x19c / 4] = 1.0_f32.to_bits();
        words[0x208 / 4] = (-1.0_f32).to_bits();
        words[0x214 / 4] = (-1.0_f32).to_bits();
        words[0x218 / 4] = 1.0_f32.to_bits();
        words[0x224 / 4] = (-1.0_f32).to_bits();
        let view = ViewUtilities::from_native_cached_words(&words).unwrap();
        assert!(view.is_bounding_box_in_view_frustum(Aabb {
            min: Vec3::new(-0.5, -100.0, -100.0),
            max: Vec3::new(0.5, 100.0, 100.0),
        }));
        assert!(!view.is_bounding_box_in_view_frustum(Aabb {
            min: Vec3::new(2.0, 0.0, 0.0),
            max: Vec3::new(3.0, 1.0, 1.0),
        }));
    }
}

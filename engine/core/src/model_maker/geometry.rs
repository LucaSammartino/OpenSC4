// SPDX-License-Identifier: GPL-3.0-or-later
use super::{Handle, ModelMaker, Rect, ALL_ROTATIONS, ALL_ZOOMS};

/// §7: intentionally different from floor/ceil at exact odd integers.
pub fn fast_floor(value: f32) -> i32 {
    let shifted = value - 0.5;
    let nearest = crate::x87::rhe(shifted) as f32;
    (if nearest + 1.0 == shifted {
        nearest + 1.0
    } else {
        nearest
    }) as i32
}

pub fn fast_ceil(value: f32) -> i32 {
    let shifted = value + 0.5;
    let nearest = crate::x87::rhe(shifted) as f32;
    (if nearest - 1.0 == shifted {
        nearest - 1.0
    } else {
        nearest
    }) as i32
}

pub fn lowest_zoom(mask: u32) -> u32 {
    mask.trailing_zeros().min(5)
}
pub fn lowest_rotation(mask: u32) -> u32 {
    mask.trailing_zeros().min(4)
}

impl Rect {
    pub fn empty(self) -> bool {
        self.x0 > self.x1 || self.z0 > self.z1
    }

    pub fn intersection(self, other: Self) -> (bool, Self) {
        if self.x1 >= other.x0 && self.x0 <= other.x1 && self.z1 >= other.z0 && self.z0 <= other.z1 {
            (
                true,
                Self::new(
                    self.x0.max(other.x0),
                    self.z0.max(other.z0),
                    self.x1.min(other.x1),
                    self.z1.min(other.z1),
                ),
            )
        } else {
            (false, Self::default())
        }
    }

    pub fn grow(&mut self, radius: i32) {
        self.x0 = self.x0.wrapping_sub(radius);
        self.z0 = self.z0.wrapping_sub(radius);
        self.x1 = self.x1.wrapping_add(radius);
        self.z1 = self.z1.wrapping_add(radius);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LoadView {
    pub zoom: usize,
    pub visible_metres: [f32; 4],
    pub centre_metres: [f32; 2],
    pub global_offset: f32,
}

/// Services used by §7.2; a missing window or renderer gives None.
pub trait LoadRectHost {
    fn view(&mut self) -> Option<LoadView>;
    fn fetch_camera_control(&mut self);
    fn expand_by_background_loader(&mut self, rect: &mut Rect, zoom: usize);
}

#[derive(Clone, Copy, Debug)]
pub struct HolderMasks {
    pub count: usize,
    pub zooms: u32,
    pub rotations: u32,
}

/// Lists return acquired occupants in manager order. Holder getters borrow.
pub trait UnloadHost {
    fn system_memory_mib(&mut self) -> Option<u32>;
    /// Apply the maker's §6.1 type filter. Some(rect) uses standard city cells;
    /// None enumerates the whole city through the manager-cell query.
    fn occupants(&mut self, city_cells: Option<Rect>) -> Vec<Handle>;
    fn holder(&mut self, occupant: Handle) -> Option<HolderMasks>;
    fn unload(&mut self, occupant: Handle);
    fn release_occupant(&mut self, occupant: Handle);
    fn flush_resources(&mut self);
}

impl ModelMaker {
    /// m is the rounded MiB result of GetSystemMemory, not raw bytes.
    pub fn determine_load_radii(&mut self, memory_mib: Option<u32>) {
        if let Some(m) = memory_mib {
            for (zoom, minimum) in [8, 8, 6, 6, 6].into_iter().enumerate() {
                self.load_radii[zoom] = fast_floor(m as f32 * (0.5 / (1u32 << zoom) as f32)).max(minimum);
            }
        }
    }

    pub fn update_load_rect(&mut self, host: &mut impl LoadRectHost) {
        let Some(view) = host.view() else { return };
        // SPEC-GAP: invalid zoom inputs have no specified result; leave the rect untouched.
        if view.zoom >= 5 {
            return;
        }
        let [x0, z0, x1, z1] = view.visible_metres;
        self.load_rect = Rect::new(
            fast_floor(x0 / 16.0),
            fast_floor(z0 / 16.0),
            fast_ceil(x1 / 16.0),
            fast_ceil(z1 / 16.0),
        );
        host.fetch_camera_control();
        self.focus = view
            .centre_metres
            .map(|v| fast_floor((2.0 * view.global_offset + v) / 16.0));
        if self.background_enabled && self.port(super::Port::BackgroundLoader).is_some() {
            host.expand_by_background_loader(&mut self.load_rect, view.zoom);
        } else {
            self.load_rect.grow(self.load_radii[view.zoom]);
        }
        self.load_rect.x0 = self.load_rect.x0.max(0);
        self.load_rect.z0 = self.load_rect.z0.max(0);
        self.load_rect.x1 = self.load_rect.x1.min(self.saved.cells[0] - 1);
        self.load_rect.z1 = self.load_rect.z1.min(self.saved.cells[1] - 1);
    }

    pub fn exclusion_rects(&self, load: Rect) -> [Rect; 4] {
        let [x, z] = self.saved.cells;
        [
            Rect::new(0, 0, x - 1, load.z0 - 1),
            Rect::new(0, load.z1 + 1, x - 1, z - 1),
            Rect::new(0, load.z0, load.x0 - 1, load.z1),
            Rect::new(load.x1 + 1, load.z0, x - 1, load.z1),
        ]
    }

    pub fn unload_unnecessary_models(&mut self, host: &mut impl UnloadHost) {
        if !self.unload_pending {
            return;
        }
        self.unload_pending = false;
        if host.system_memory_mib().is_some_and(|m| m > 32) {
            return;
        }
        for rect in self.exclusion_rects(self.load_rect) {
            if rect.empty() {
                continue;
            }
            for o in host.occupants(Some(rect)) {
                if host
                    .holder(o)
                    .is_some_and(|h| h.count != 0 && (h.zooms != ALL_ZOOMS || h.rotations != ALL_ROTATIONS))
                {
                    host.unload(o);
                }
                host.release_occupant(o);
            }
        }
        host.flush_resources();
    }

    pub fn remove_models_inappropriate_for_rotation(&mut self, rotation: u32, host: &mut impl UnloadHost) {
        // SPEC-GAP: rotations outside 0..3 are rejected rather than shifted.
        if rotation >= 4 {
            return;
        }
        for o in host.occupants(None) {
            if host
                .holder(o)
                .is_some_and(|h| h.count != 0 && h.rotations & (1 << rotation) == 0)
            {
                host.unload(o);
            }
            host.release_occupant(o);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn m13_memory_radii_and_m14_odd_integer_rounding() {
        let mut m = ModelMaker::default();
        m.determine_load_radii(None);
        assert_eq!(m.load_radii, [256; 5]);
        for (memory, expected) in [
            (512, [256, 128, 64, 32, 16]),
            (8, [8, 8, 6, 6, 6]),
            (4096, [2048, 1024, 512, 256, 128]),
        ] {
            m.determine_load_radii(Some(memory));
            assert_eq!(m.load_radii, expected);
        }
        for (x, floor, ceil) in [(2.5, 2, 3), (3.0, 2, 4), (4.0, 4, 4), (-3.0, -4, -2)] {
            assert_eq!(fast_floor(x), floor);
            assert_eq!(fast_ceil(x), ceil);
        }
        assert_eq!((lowest_zoom(0), lowest_zoom(0x80), lowest_zoom(0x14)), (5, 5, 2));
        assert_eq!(
            (lowest_rotation(0), lowest_rotation(0x10), lowest_rotation(8)),
            (4, 4, 3)
        );
    }

    struct View;
    impl LoadRectHost for View {
        fn view(&mut self) -> Option<LoadView> {
            Some(LoadView {
                zoom: 2,
                visible_metres: [100.0, 40.0, 300.0, 200.0],
                centre_metres: [160.0, 160.0],
                global_offset: 0.0,
            })
        }
        fn fetch_camera_control(&mut self) {}
        fn expand_by_background_loader(&mut self, _: &mut Rect, _: usize) {
            panic!("loader off");
        }
    }
    #[test]
    fn m15_load_rect_and_m16_city_exclusions() {
        let mut m = ModelMaker::default();
        m.saved.cells = [256; 2];
        m.background_enabled = false;
        m.determine_load_radii(Some(512));
        m.update_load_rect(&mut View);
        assert_eq!(m.load_rect, Rect::new(0, 0, 83, 77));
        assert_eq!(m.focus, [10, 10]);
        m.saved.cells = [64; 2];
        assert_eq!(
            m.exclusion_rects(Rect::new(10, 20, 30, 40)),
            [
                Rect::new(0, 0, 63, 19),
                Rect::new(0, 41, 63, 63),
                Rect::new(0, 20, 9, 40),
                Rect::new(31, 20, 63, 40)
            ]
        );
        assert_eq!(
            Rect::new(0, 0, 2, 2).intersection(Rect::new(2, 2, 3, 3)),
            (true, Rect::new(2, 2, 2, 2))
        );
        assert_eq!(
            Rect::new(0, 0, 2, 2).intersection(Rect::new(3, 3, 4, 4)),
            (false, Rect::default())
        );
    }

    #[derive(Default)]
    struct Unloader {
        memory: Option<u32>,
        lists: Vec<Option<Rect>>,
        unloaded: Vec<Handle>,
        released: Vec<Handle>,
        flush: usize,
    }
    impl UnloadHost for Unloader {
        fn system_memory_mib(&mut self) -> Option<u32> {
            self.memory
        }
        fn occupants(&mut self, r: Option<Rect>) -> Vec<Handle> {
            self.lists.push(r);
            vec![1, 2, 3]
        }
        fn holder(&mut self, o: Handle) -> Option<HolderMasks> {
            Some(HolderMasks {
                count: usize::from(o != 3),
                zooms: if o == 1 { ALL_ZOOMS } else { 1 },
                rotations: if o == 1 { ALL_ROTATIONS } else { 1 },
            })
        }
        fn unload(&mut self, o: Handle) {
            self.unloaded.push(o);
        }
        fn release_occupant(&mut self, o: Handle) {
            self.released.push(o);
        }
        fn flush_resources(&mut self) {
            self.flush += 1;
        }
    }
    #[test]
    fn low_memory_unload_gate_full_masks_and_rotation() {
        let mut m = ModelMaker::default();
        m.saved.cells = [64; 2];
        m.load_rect = Rect::new(10, 20, 30, 40);
        let mut h = Unloader {
            memory: Some(512),
            ..Default::default()
        };
        m.unload_pending = true;
        m.unload_unnecessary_models(&mut h);
        assert!(!m.unload_pending);
        assert!(h.lists.is_empty());
        h.memory = Some(32);
        m.unload_pending = true;
        m.unload_unnecessary_models(&mut h);
        assert_eq!(h.unloaded, [2; 4]);
        assert_eq!(h.released.len(), 12);
        assert_eq!(h.flush, 1);
        h.unloaded.clear();
        m.remove_models_inappropriate_for_rotation(1, &mut h);
        assert_eq!(h.unloaded, [2]);
        assert_eq!(h.lists.last(), Some(&None));
    }
}

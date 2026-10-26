// SPDX-License-Identifier: GPL-3.0-or-later
//! Terrain hiding, docs/specs/model-maker.md §13.
use super::{Handle, ModelMaker, Port, Rect};

#[derive(Clone, Debug)]
pub struct TerrainOccupant {
    pub type_id: u32,
    pub cells: Rect,
    /// None when the occupant does not answer the network interface.
    pub network_flags: Option<u32>,
    /// The two used outputs from the optional 0x4A10AFC2 interface.
    pub intersection_cover: Option<[bool; 2]>,
    pub foundation_kind: Option<u32>,
}

/// Interface-valued getters acquire and release their temporary interfaces in
/// the adapter. The occupant list itself returns acquired occupant references.
pub trait TerrainHost {
    fn occupant_cells(&mut self, occupant: Handle) -> Rect;
    fn foundation_mode(&mut self, occupant: Handle) -> Option<u32>;
    fn flags(&mut self, cell: [i32; 2]) -> u32;
    fn set_flags(&mut self, cell: [i32; 2], flags: u32);
    fn occupants(&mut self, rect: Rect, view_filter: Option<Handle>) -> Vec<Handle>;
    fn occupant(&mut self, occupant: Handle) -> TerrainOccupant;
    fn lot_flags(&mut self, first_cell: [i32; 2]) -> Option<u32>;
    /// i/j are relative to the query rectangle, not the base-texture occupant.
    fn base_texture_cover(&mut self, occupant: Handle, i: i32, j: i32) -> bool;
    fn release_occupant(&mut self, occupant: Handle);
    fn refresh_terrain(&mut self, end_exclusive_rect: Rect);
}

impl ModelMaker {
    pub fn hide_occupant_terrain(
        &self,
        occupant: Handle,
        type_id: u32,
        flag: bool,
        host: &mut impl TerrainHost,
    ) -> bool {
        let cells = host.occupant_cells(occupant);
        if cells.empty() {
            return false;
        }
        let (intersects, mut rect) = cells.intersection(self.saved.city);
        if !intersects || rect.empty() {
            return false;
        }
        let mode = type_id == 0x68FD_0C69 && flag && host.foundation_mode(occupant) == Some(2);
        if !self.hide_terrain_cells(rect, mode, host) {
            return false;
        }
        rect.x1 = rect.x1.wrapping_add(1);
        rect.z1 = rect.z1.wrapping_add(1);
        host.refresh_terrain(rect);
        true
    }

    pub fn hide_terrain_cells(&self, rect: Rect, mode: bool, host: &mut impl TerrainHost) -> bool {
        if rect.empty() {
            return false;
        }
        let height = (i64::from(rect.z1) - i64::from(rect.z0) + 1) as usize;
        let width = (i64::from(rect.x1) - i64::from(rect.x0) + 1) as usize;
        let mut grid = vec![0u32; width * height];
        let index = |x: i32, z: i32| {
            (i64::from(x) - i64::from(rect.x0)) as usize * height + (i64::from(z) - i64::from(rect.z0)) as usize
        };
        for x in rect.x0..=rect.x1 {
            for z in rect.z0..=rect.z1 {
                let cell = [x, z];
                let mut f = host.flags(cell);
                if mode {
                    let new_flags = (f & !4) | 3;
                    if f != new_flags {
                        host.set_flags(cell, new_flags);
                    }
                    f = new_flags;
                }
                grid[index(x, z)] = f & !7;
            }
        }
        for o in host.occupants(rect, self.port(Port::ViewFilter)) {
            let occupant = host.occupant(o);
            let mut mask = u32::MAX;
            let mut bits = 0;
            let (intersects, overlap) = rect.intersection(occupant.cells);
            match occupant.type_id {
                0xC772_BF98 => {
                    if let Some(flags) = occupant.network_flags {
                        if flags & 0x4000 == 0 {
                            mask = !8;
                        }
                        if flags & 0x100_0000 != 0 {
                            bits = 8;
                        }
                        if flags & 0x800_0000 == 0 {
                            if flags & 0x200_0000 == 0 {
                                bits |= 1;
                            }
                            if flags & 0x400_0000 == 0 {
                                bits |= 2;
                            }
                        }
                    }
                    if let Some([a, c]) = occupant.intersection_cover {
                        if a {
                            bits |= 1;
                        }
                        if c {
                            bits |= 2;
                        }
                    }
                }
                0x68FD_0C69 if occupant.foundation_kind == Some(2) => {
                    if host
                        .lot_flags([occupant.cells.x0, occupant.cells.z0])
                        .is_some_and(|f| f & 1 != 0 && f & 6 != 0)
                    {
                        bits = 3;
                    }
                }
                0x497F_6D9D if intersects && !overlap.empty() => {
                    for x in overlap.x0..=overlap.x1 {
                        for z in overlap.z0..=overlap.z1 {
                            if host.base_texture_cover(o, x - rect.x0, z - rect.z0) {
                                grid[index(x, z)] |= 4;
                            }
                        }
                    }
                }
                0x2A18_3167 | 0x8A13_918A => bits = 3,
                _ => {}
            }
            if intersects && !overlap.empty() && (bits != 0 || mask != u32::MAX) {
                for x in overlap.x0..=overlap.x1 {
                    for z in overlap.z0..=overlap.z1 {
                        let f = &mut grid[index(x, z)];
                        *f = (*f & mask) | bits;
                    }
                }
            }
            host.release_occupant(o);
        }
        let mut changed = false;
        for x in rect.x0..=rect.x1 {
            for z in rect.z0..=rect.z1 {
                let flags = grid[index(x, z)];
                if host.flags([x, z]) != flags {
                    host.set_flags([x, z], flags);
                    changed = true;
                }
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct Terrain {
        flags: BTreeMap<[i32; 2], u32>,
        entries: Vec<TerrainOccupant>,
        released: Vec<Handle>,
        writes: Vec<([i32; 2], u32)>,
        refreshed: Vec<Rect>,
        cover_queries: Vec<[i32; 2]>,
    }
    impl TerrainHost for Terrain {
        fn occupant_cells(&mut self, _: Handle) -> Rect {
            Rect::new(11, 20, 12, 21)
        }
        fn foundation_mode(&mut self, _: Handle) -> Option<u32> {
            Some(2)
        }
        fn flags(&mut self, c: [i32; 2]) -> u32 {
            self.flags.get(&c).copied().unwrap_or(0)
        }
        fn set_flags(&mut self, c: [i32; 2], f: u32) {
            self.flags.insert(c, f);
            self.writes.push((c, f));
        }
        fn occupants(&mut self, _: Rect, _: Option<Handle>) -> Vec<Handle> {
            (0..self.entries.len() as u64).collect()
        }
        fn occupant(&mut self, o: Handle) -> TerrainOccupant {
            self.entries[o as usize].clone()
        }
        fn lot_flags(&mut self, _: [i32; 2]) -> Option<u32> {
            Some(3)
        }
        fn base_texture_cover(&mut self, _: Handle, i: i32, j: i32) -> bool {
            self.cover_queries.push([i, j]);
            i == 1 && j == 0
        }
        fn release_occupant(&mut self, o: Handle) {
            self.released.push(o);
        }
        fn refresh_terrain(&mut self, r: Rect) {
            self.refreshed.push(r);
        }
    }
    fn occupant(t: u32, cells: Rect) -> TerrainOccupant {
        TerrainOccupant {
            type_id: t,
            cells,
            network_flags: None,
            intersection_cover: None,
            foundation_kind: None,
        }
    }

    #[test]
    fn network_base_texture_and_pipe_bits_preserve_high_flags() {
        let maker = ModelMaker::default();
        let rect = Rect::new(10, 20, 12, 21);
        let mut network = occupant(0xC772_BF98, Rect::new(10, 20, 10, 21));
        network.network_flags = Some(0x100_0000 | 0x200_0000);
        network.intersection_cover = Some([true, false]);
        let mut host = Terrain {
            entries: vec![
                network,
                occupant(0x497F_6D9D, Rect::new(11, 20, 12, 20)),
                occupant(0x2A18_3167, Rect::new(12, 21, 14, 22)),
            ],
            ..Default::default()
        };
        host.flags.insert([10, 20], 0x108);
        assert!(maker.hide_terrain_cells(rect, false, &mut host));
        assert_eq!(host.flags[&[10, 20]], 0x10B);
        assert_eq!(host.flags[&[11, 20]], 4);
        assert_eq!(host.flags[&[12, 21]], 3);
        assert_eq!(host.cover_queries, [[1, 0], [2, 0]]);
        assert_eq!(host.released, [0, 1, 2]);
        host.writes.clear();
        assert!(!maker.hide_terrain_cells(rect, false, &mut host));
        assert!(host.writes.is_empty());
    }

    #[test]
    fn foundation_mode_rebuilds_flags_and_refreshes_end_exclusive_clipped_rect() {
        let mut maker = ModelMaker::default();
        maker.saved.city = Rect::new(0, 0, 11, 20);
        let mut foundation = occupant(0x68FD_0C69, Rect::new(11, 20, 12, 21));
        foundation.foundation_kind = Some(2);
        let mut host = Terrain {
            entries: vec![foundation],
            ..Default::default()
        };
        // Mode writes 3, then grid rebuild finds the same covered cell: no final write.
        assert!(!maker.hide_occupant_terrain(1, 0x68FD_0C69, true, &mut host));
        assert_eq!(host.writes, [([11, 20], 3)]);
        assert!(host.refreshed.is_empty());
        // Removing all covering occupants rebuilds to zero and refreshes.
        host.entries.clear();
        assert!(maker.hide_occupant_terrain(1, 0x68FD_0C69, false, &mut host));
        assert_eq!(host.refreshed, [Rect::new(11, 20, 12, 21)]);
        assert_eq!(host.flags[&[11, 20]], 0);
    }
}

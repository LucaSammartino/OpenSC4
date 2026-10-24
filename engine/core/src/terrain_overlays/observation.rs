// SPDX-License-Identifier: GPL-3.0-or-later
//! OV12 snapshot diagnostics, deliberately separate from `OverlayManager` replay.
//! Native matrices, rectangles and ordered lists here are observations, never
//! creation inputs or independent evidence of the owner's arithmetic/history.

use std::collections::{BTreeMap, BTreeSet};

const NONE: u32 = u32::MAX;
const LIMIT: usize = 1_048_576;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub checks: u32,
    pub differences: u32,
    pub overlays: u32,
    pub handles: u32,
    pub groups: u32,
    pub cells: u32,
    pub entries: u32,
    pub bindings: u32,
    pub sources: u32,
    pub unknown_bindings: u32,
    pub observed_words: u32,
    pub digest: u64,
}

impl Report {
    fn check(&mut self, equal: bool) {
        self.checks += 1;
        self.differences += u32::from(!equal);
    }
}

struct Input<'a>(&'a [u32]);
impl<'a> Input<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u32]> {
        let words = self.0.get(..n)?;
        self.0 = &self.0[n..];
        Some(words)
    }
    fn word(&mut self) -> Option<u32> {
        Some(self.take(1)?[0])
    }
    fn count(&mut self) -> Option<usize> {
        let n = self.word()? as usize;
        (n <= LIMIT).then_some(n)
    }
}

struct Arena<'a> {
    records: Vec<&'a [u32]>,
    live: Vec<bool>,
}
impl<'a> Arena<'a> {
    fn read(input: &mut Input<'a>, stride: usize, report: &mut Report) -> Option<Self> {
        let count = input.count()?;
        let capacity = input.word()?;
        let head = input.word()?;
        let tail = input.word()?;
        if capacity < count as u32 {
            return None;
        }
        let records: Vec<_> = input.take(count.checked_mul(stride)?)?.chunks_exact(stride).collect();
        let mut free = BTreeSet::new();
        if head == NONE || tail == NONE {
            if head != tail {
                return None;
            }
        } else {
            let mut index = head;
            loop {
                let record = records.get(index as usize)?;
                if !free.insert(index) {
                    return None;
                }
                if index == tail {
                    if record[0] != 0 {
                        return None;
                    }
                    break;
                }
                index = record[0];
            }
        }
        let live: Vec<_> = records.iter().map(|r| r[0] == NONE).collect();
        for (index, &is_live) in live.iter().enumerate() {
            // An unqueued non-live record is incomplete, not an exact prefix.
            if is_live == free.contains(&(index as u32)) {
                return None;
            }
            report.check(true);
        }
        Some(Self { records, live })
    }
    fn get(&self, id: u32) -> Option<&'a [u32]> {
        self.live.get(id as usize).copied().filter(|b| *b)?;
        self.records.get(id as usize).copied()
    }
    fn count(&self) -> u32 {
        self.live.iter().filter(|b| **b).count() as u32
    }
}

fn resolve<'a>(id: u32, overlays: &Arena<'a>, handles: &Arena<'a>, groups: &Arena<'a>) -> Option<&'a [u32]> {
    if id & 0x8000_0000 == 0 {
        return overlays.get(id);
    }
    let target = handles.get(id & 0x7fff_ffff)?[11];
    if target & 0x8000_0000 == 0 {
        overlays.get(target)
    } else {
        overlays.get(groups.get(target & 0x7fff_ffff)?[1])
    }
}

/// Parse the bounded, pointer-free v1 observer transport. `None` means incomplete
/// or unsafe structure, never a successful comparison of a truncated prefix.
/// Layout is shared only with FC4OverlaysObservation.cpp, not the public C ABI.
pub fn inspect(words: &[u32]) -> Option<Report> {
    let mut input = Input(words);
    if input.word()? != 1 {
        return None;
    }
    let dynamic = input.word()? != 0;
    let shift = input.word()?;
    let nx = input.count()?;
    let nz = input.count()?;
    if shift > 30 {
        return None;
    }
    let mut report = Report::default();
    let overlays = Arena::read(&mut input, 45, &mut report)?;
    let handles = Arena::read(&mut input, 12, &mut report)?;
    let groups = Arena::read(&mut input, 9, &mut report)?;
    report.overlays = overlays.count();
    report.handles = handles.count();
    report.groups = groups.count();
    for (id, live) in groups.live.iter().enumerate() {
        if *live {
            for &part in &groups.records[id][1..] {
                overlays.get(part)?;
                report.check(true);
            }
        }
    }
    for (id, live) in handles.live.iter().enumerate() {
        if *live {
            resolve(0x8000_0000 | id as u32, &overlays, &handles, &groups)?;
            report.check(true);
        }
    }
    let cells = input.count()?;
    if cells != if dynamic { 0 } else { nx.checked_mul(nz)? } {
        return None;
    }
    report.cells = cells as u32;
    // Presence/membership is testable from bounds. Encounter order and duplicate
    // multiplicity are retained in the digest, never reconstructed or sorted.
    let mut memberships = BTreeSet::new();
    for cell in 0..cells {
        let count = input.count()?;
        for &id in input.take(count)? {
            let r = overlays.get(id)?;
            let x = (cell % nx) as i64;
            let z = (cell / nx) as i64;
            let round = (1_i64 << shift) - 1;
            let rect = [r[3] as i32, r[4] as i32, r[5] as i32, r[6] as i32];
            let coarse = [
                i64::from(rect[0]) >> shift,
                i64::from(rect[1]) >> shift,
                (i64::from(rect[2]) + round) >> shift,
                (i64::from(rect[3]) + round) >> shift,
            ];
            report.check(x >= coarse[0] && z >= coarse[1] && x < coarse[2] && z < coarse[3]);
            memberships.insert((cell, id));
            report.entries += 1;
        }
    }
    if !dynamic {
        let round = (1_i64 << shift) - 1;
        for (id, live) in overlays.live.iter().enumerate() {
            if !*live {
                continue;
            }
            let r = overlays.records[id];
            let bounds = [
                i64::from(r[3] as i32) >> shift,
                i64::from(r[4] as i32) >> shift,
                (i64::from(r[5] as i32) + round) >> shift,
                (i64::from(r[6] as i32) + round) >> shift,
            ];
            // Reject corrupt extents before enumerating; do not clamp away gaps.
            if bounds[0] < 0 || bounds[1] < 0 || bounds[2] > nx as i64 || bounds[3] > nz as i64 {
                return None;
            }
            for z in bounds[1]..bounds[3] {
                for x in bounds[0]..bounds[2] {
                    report.check(memberships.contains(&(z as usize * nx + x as usize, id as u32)));
                }
            }
        }
    }
    let mut bindings = BTreeMap::new();
    for _ in 0..input.count()? {
        let b = input.take(15)?;
        if b[0] == 0 || bindings.insert(b[0], b).is_some() {
            return None;
        }
        report.bindings += 1;
        if b[2] == 0 {
            report.unknown_bindings += 1;
        } else {
            report.check(b[3] == 1); // binding's own factory, not manager's
            report.check(b[4] == 1); // frame array equality, not pixel identity
            report.check(b[1] == b[5]);
        }
    }
    let mut sources = BTreeSet::new();
    for _ in 0..input.count()? {
        let s = input.take(2)?;
        if s[0] == 0 || s[1] == 0 || !sources.insert(s[0]) {
            return None;
        }
        report.sources += 1;
    }
    for r in &overlays.records {
        if r[43] != 0 {
            bindings.get(&r[43])?;
        }
        if r[44] != 0 && !sources.contains(&r[44]) {
            return None;
        }
    }
    let probes = input.count()?;
    if probes != (report.overlays + report.handles) as usize {
        return None;
    }
    let mut seen = BTreeSet::new();
    for _ in 0..probes {
        let p = input.take(17)?;
        if !seen.insert(p[0]) {
            return None;
        }
        let r = resolve(p[0], &overlays, &handles, &groups)?;
        let expected = [r[1] & 1, r[39], r[40], r[41], r[42], r[2] & 255, (r[1] >> 6) & 1];
        for (&a, b) in p[1..8].iter().zip(expected) {
            report.check(a == b);
        }
        for (i, &actual) in p[8..17].iter().enumerate() {
            let expected = if p[0] & 0x8000_0000 == 0 {
                0x3e80_0000 + i as u32
            } else {
                handles.get(p[0] & 0x7fff_ffff)?[1 + i]
            };
            report.check(actual == expected);
        }
    }
    if !input.0.is_empty() {
        return None;
    }
    report.observed_words = words.len() as u32;
    report.digest = words.iter().enumerate().fold(0xcbf2_9ce4_8422_2325, |hash, (i, word)| {
            let word = if (9..9 + overlays.records.len() * 45).contains(&i) && (i - 9) % 45 == 2 {
            *word & 0xffff_00ff
        } else {
            *word
        };
        (hash ^ u64::from(word)).wrapping_mul(0x100_0000_01b3)
    });
    Some(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arena(records: &[Vec<u32>], head: u32, tail: u32, capacity: u32) -> Vec<u32> {
        let mut words = vec![records.len() as u32, capacity, head, tail];
        words.extend(records.iter().flatten());
        words
    }
    fn record(stride: usize) -> Vec<u32> {
        let mut r = vec![0; stride];
        r[0] = NONE;
        r
    }
    fn parse_arena(words: &[u32], stride: usize) -> Option<Vec<bool>> {
        Arena::read(&mut Input(words), stride, &mut Report::default()).map(|a| a.live)
    }
    fn snapshot(
        overlays: &[Vec<u32>],
        handles: &[Vec<u32>],
        groups: &[Vec<u32>],
        cells: Option<&[Vec<u32>]>,
    ) -> Vec<u32> {
        let mut w = vec![1, u32::from(cells.is_none()), 0, 2, 2];
        for records in [overlays, handles, groups] {
            w.extend(arena(records, NONE, NONE, records.len() as u32));
        }
        w.push(cells.map_or(0, |c| c.len() as u32));
        if let Some(cells) = cells {
            for c in cells {
                w.push(c.len() as u32);
                w.extend(c);
            }
        }
        w.extend([0, 0]); // bindings, sources
        w.push((overlays.len() + handles.len()) as u32);
        let ids = (0..overlays.len() as u32).chain((0..handles.len() as u32).map(|i| i | 0x8000_0000));
        for id in ids {
            let overlay = if id & 0x8000_0000 == 0 {
                id
            } else {
                let target = handles[(id & 0x7fff_ffff) as usize][11];
                if target & 0x8000_0000 == 0 {
                    target
                } else {
                    groups[(target & 0x7fff_ffff) as usize][1]
                }
            };
            let r = &overlays[overlay as usize];
            w.extend([id, r[1] & 1, r[39], r[40], r[41], r[42], r[2] & 255, (r[1] >> 6) & 1]);
            for i in 0..9 {
                w.push(if id & 0x8000_0000 == 0 {
                    0x3e80_0000 + i
                } else {
                    handles[(id & 0x7fff_ffff) as usize][1 + i as usize]
                });
            }
        }
        w
    }
    #[test]
    fn ov12_a_constructed_not_reserved() {
        let words = arena(&vec![record(45); 3], NONE, NONE, 5);
        assert_eq!(parse_arena(&words, 45), Some(vec![true; 3]));
        assert!(parse_arena(&words[..words.len() - 1], 45).is_none());
    }
    #[test]
    fn ov12_b_singleton_zero_tail() {
        let mut r = vec![record(45); 3];
        r[0][0] = 0;
        assert_eq!(parse_arena(&arena(&r, 0, 0, 3), 45), Some(vec![false, true, true]));
    }
    #[test]
    fn ov12_c_fifo_next_zero_all_strides() {
        for stride in [45, 12, 9] {
            let mut r = vec![record(stride); 3];
            r[1][0] = 0;
            r[0][0] = 0;
            assert_eq!(parse_arena(&arena(&r, 1, 0, 3), stride), Some(vec![false, false, true]));
            r[1][0] = NONE;
            assert_eq!(parse_arena(&arena(&r, 0, 0, 3), stride), Some(vec![false, true, true]));
        }
    }
    #[test]
    fn ov12_d_last_slot_free_despite_payload() {
        let mut r = vec![record(45); 3];
        r[2][0] = 0;
        r[2][43] = 123;
        assert_eq!(parse_arena(&arena(&r, 2, 2, 3), 45), Some(vec![true, true, false]));
        r[2][0] = NONE;
        assert_eq!(parse_arena(&arena(&r, NONE, NONE, 3), 45), Some(vec![true; 3]));
    }
    #[test]
    fn ov12_e_group_and_direct_are_distinct_arenas() {
        let overlays = vec![record(45); 9];
        let mut h = record(12);
        h[11] = 0x8000_0000;
        let mut g = record(9);
        for (i, p) in g[1..].iter_mut().enumerate() {
            *p = i as u32;
        }
        let w = snapshot(&overlays, &[h], &[g], None);
        let r = inspect(&w).unwrap();
        assert_eq!((r.overlays, r.handles, r.groups, r.differences), (9, 1, 1, 0));
    }
    #[test]
    fn ov12_f_padding_ignored_secondary_observed() {
        let a = snapshot(&[record(45)], &[], &[], None);
        let mut b = a.clone();
        b[9 + 2] |= 0xab00;
        assert_eq!(inspect(&a), inspect(&b));
        b[9 + 23] = 0x3f80_0000;
        assert_ne!(inspect(&a).unwrap().digest, inspect(&b).unwrap().digest);
        // Secondary is observed, not an independent model replay comparison.
        assert_eq!(inspect(&b).unwrap().differences, 0);
    }
    #[test]
    fn ov12_g_row_major_prepend_lists_and_missing_membership() {
        let mut a = record(45);
        a[5] = 2;
        a[6] = 1;
        let mut b = record(45);
        b[3] = 1;
        b[5] = 2;
        b[6] = 2;
        let cells = vec![vec![0], vec![1, 0], vec![], vec![1]];
        let w = snapshot(&[a.clone(), b.clone()], &[], &[], Some(&cells));
        let r = inspect(&w).unwrap();
        assert_eq!((r.cells, r.entries, r.differences), (4, 4, 0));
        let cells = vec![vec![0], vec![1], vec![], vec![1]];
        assert_eq!(
            inspect(&snapshot(&[a, b], &[], &[], Some(&cells))).unwrap().differences,
            1
        );
    }
    #[test]
    fn ov12_h_order_is_retained_not_invented() {
        let mut r = record(45);
        r[5] = 1;
        r[6] = 1;
        let records = vec![r; 2];
        let initial = snapshot(&records, &[], &[], Some(&[vec![1, 0], vec![], vec![], vec![]]));
        let moved = snapshot(&records, &[], &[], Some(&[vec![0, 1], vec![], vec![], vec![]]));
        assert_eq!(inspect(&initial).unwrap().differences, 0);
        assert_eq!(inspect(&moved).unwrap().differences, 0);
        assert_ne!(inspect(&initial).unwrap().digest, inspect(&moved).unwrap().digest);
    }
    #[test]
    fn ov12_i_empty_and_dynamic() {
        let empty = snapshot(&[], &[], &[], None);
        assert_eq!(inspect(&empty).unwrap().overlays, 0);
        assert_eq!(inspect(&snapshot(&[record(45)], &[], &[], None)).unwrap().cells, 0);
    }
    #[test]
    fn ov12_j_known_unknown_metadata_not_pixel_bytes() {
        let mut w = snapshot(&[], &[], &[], None);
        w.truncate(w.len() - 3);
        w.extend([1, 1, 3, 1, 1, 1, 3, 2, 64, 32, 42, 99, 4096, 0x0808_0808, 32, 7, 0, 0]);
        let r = inspect(&w).unwrap();
        assert_eq!((r.bindings, r.unknown_bindings, r.differences), (1, 0, 0));
        w[21] = 0; // known flag in binding payload
        let r = inspect(&w).unwrap();
        assert_eq!((r.bindings, r.unknown_bindings), (1, 1));
    }
    #[test]
    fn ov12_k_model_inputs_not_recoverable() {
        let mut r = record(45);
        r[1] = 8;
        r[7] = 0x3f80_0000;
        r[23] = 0x4000_0000;
        let w = snapshot(&[r], &[], &[], None);
        let report = inspect(&w).unwrap();
        assert_eq!((report.overlays, report.handles, report.differences), (1, 0, 0));
        // No geometry/point count or invented projection is accepted as input.
    }
    #[test]
    fn ov12_l_replacement_is_not_recomputed_from_placement() {
        let mut r = record(45);
        r[1] = 32;
        r[7] = (1.0_f32 / 16.0).to_bits();
        let mut h = record(12);
        h[1] = 0x4220_0000;
        let w = snapshot(&[r], &[h], &[], None);
        assert_eq!(inspect(&w).unwrap().differences, 0);
        let mut corrupt = w.clone();
        *corrupt.last_mut().unwrap() ^= 1;
        assert_eq!(inspect(&corrupt).unwrap().differences, 1);
    }
    #[test]
    fn corrupt_queues_links_and_trailing_or_missing_words_fail_closed() {
        let mut r = vec![record(9); 3];
        r[0][0] = 1;
        r[1][0] = 0;
        r[2][0] = 0;
        assert!(parse_arena(&arena(&r, 0, 2, 3), 9).is_none());
        r[0][0] = 4;
        assert!(parse_arena(&arena(&r, 0, 2, 3), 9).is_none());
        let mut w = snapshot(&[], &[], &[], None);
        w.push(0);
        assert!(inspect(&w).is_none());
        w.truncate(5);
        assert!(inspect(&w).is_none());
    }
}

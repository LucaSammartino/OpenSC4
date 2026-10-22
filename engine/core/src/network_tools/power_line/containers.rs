// SPDX-License-Identifier: GPL-3.0-or-later
//! The 13 structural helpers (§6.5). Reference tokens never AddRef/Release on copy/drop.

use super::{PoleInfo, WireCell};
use std::ops::Range;

pub fn copy_wire_range(source: &[WireCell], destination: &mut Vec<WireCell>) -> usize {
    destination.extend_from_slice(source);
    destination.len()
}

pub fn copy_wire_vector(source: &[WireCell]) -> Vec<WireCell> { source.to_vec() }

pub fn copy_wire_vectors(source: &[Vec<WireCell>]) -> Vec<Vec<WireCell>> {
    source.iter().map(|v| copy_wire_vector(v)).collect()
}

pub fn copy_pole_record(source: &PoleInfo) -> PoleInfo { source.clone() }

pub fn assign_wire_vector(destination: &mut Vec<WireCell>, source: &[WireCell]) {
    if destination.capacity() < source.len() { *destination = copy_wire_vector(source); }
    else { destination.clear(); destination.extend_from_slice(source); }
}

pub fn copy_pole_range(source: &[PoleInfo]) -> Vec<PoleInfo> { source.to_vec() }

pub fn repeat_pole_record(source: &PoleInfo, count: usize) -> Vec<PoleInfo> {
    (0..count).map(|_| source.clone()).collect()
}

pub fn assign_pole_range(destination: &mut [PoleInfo], source: &[PoleInfo]) -> usize {
    for (destination, source) in destination.iter_mut().zip(source) { destination.clone_from(source); }
    source.len()
}

pub fn assign_groups(destination: &mut [Vec<usize>], source: &[Vec<usize>]) -> usize {
    for (destination, source) in destination.iter_mut().zip(source) { destination.clone_from(source); }
    source.len()
}

pub fn erase_groups(groups: &mut Vec<Vec<usize>>, range: Range<usize>) -> usize {
    let removed = range.len();
    for source in range.end..groups.len() {
        let value = groups[source].clone();
        groups[source - removed].clone_from(&value);
    }
    groups.truncate(groups.len() - removed);
    range.start
}

pub fn destroy_pole_vector(poles: Vec<PoleInfo>) { drop(poles); }

pub fn erase_pole_range(poles: &mut Vec<PoleInfo>, range: Range<usize>) -> usize {
    let removed = range.len();
    for source in range.end..poles.len() {
        let value = poles[source].clone();
        poles[source - removed].clone_from(&value);
    }
    poles.truncate(poles.len() - removed);
    range.start
}

pub fn insert_pole_copies(poles: &mut Vec<PoleInfo>, position: usize, value: &PoleInfo, count: usize, omit_suffix: bool) {
    let n = poles.len();
    let mut replacement = Vec::with_capacity(n + n.max(count));
    replacement.extend_from_slice(&poles[..position]);
    replacement.extend(repeat_pole_record(value, count));
    if !omit_suffix { replacement.extend_from_slice(&poles[position..]); }
    *poles = replacement;
}

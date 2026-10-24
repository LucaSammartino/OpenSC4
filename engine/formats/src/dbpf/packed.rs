// SPDX-License-Identifier: GPL-3.0-or-later
//! Public packed-segment projection (data-formats Â§2.6.1â€“2.6.2, EX54/EX55).
use super::{Package, Source, DIR_TGI};
use crate::Tgi;

/// Access bits belong to the segment, not the immutable package index.
pub struct PackedView<'a, S> {
    pub package: &'a Package<S>,
    pub read_access: bool,
    pub write_access: bool,
}

impl<S: Source> PackedView<'_, S> {
    /// EX54. A present buffer must cover the caller's entire declared capacity.
    /// `write_open` describes the matching record, not the segment's write bit.
    pub fn read_record_raw(&self, key: Tgi, buffer: Option<&mut [u8]>, size: &mut u32, write_open: bool) -> u32 {
        if !self.read_access {
            return u32::MAX;
        }
        if write_open {
            *size = 0;
            return u32::MAX;
        }
        let Some(entry) = self.package.find(&key) else {
            return u32::MAX;
        };
        let Some(buffer) = buffer else {
            *size = 0;
            return self.package.dir_size(entry).unwrap_or(entry.size);
        };
        assert!(buffer.len() >= *size as usize);
        let count = (*size).min(entry.size);
        *size = 0;
        if self
            .package
            .src
            .read_at(entry.offset as u64, &mut buffer[..count as usize])
            .is_err()
        {
            return 0;
        }
        *size = count;
        count
    }

    /// EX55: unique index keys, type-only exclusion before the filter, no order
    /// promise. A list callback models Insert; its return is deliberately ignored.
    pub fn resource_keys(
        &self,
        mut list: Option<&mut dyn FnMut(Tgi) -> bool>,
        mut filter: Option<&mut dyn FnMut(Tgi) -> bool>,
    ) -> u32 {
        if !self.read_access && !self.write_access {
            return 0;
        }
        let mut count = 0;
        for key in self.package.index_map.keys().copied() {
            if key.type_id == DIR_TGI.type_id || filter.as_mut().is_some_and(|f| !f(key)) {
                continue;
            }
            if let Some(insert) = list.as_mut() {
                insert(key);
            }
            count += 1;
        }
        count
    }

    /// A null filter bypasses enumeration and its access/type checks.
    pub fn record_count(&self, filter: Option<&mut dyn FnMut(Tgi) -> bool>) -> u32 {
        match filter {
            None => self.package.index_map.len() as u32,
            Some(filter) => self.resource_keys(None, Some(filter)),
        }
    }
}

#[cfg(test)]
mod tests;

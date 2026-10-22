// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable persistence.md §§2-7. Logical interface references are explicit;
//! Rust owns allocations. Calls after logical destruction are outside the API.
//! No native layout, DBPF native-order configuration or shipped-image claim.
mod record;
mod serial;
pub use crate::persistence_manager::{Key, KeyList};
pub use record::{DbRecord, RecordInterface, Segment};
pub use serial::{Field, RawStream, SerialRecord, StreamAdapter};

pub const UNKNOWN_IID: u32 = 1;
pub const RECORD_IID: u32 = 0xe56b8f03;
pub const SERIAL_IID: u32 = 0xc56b8f08;
pub const INPUT_IID: u32 = 0x051b11bd;
pub const OUTPUT_IID: u32 = 0xc51b11cb;
pub const RESOURCE_IID: u32 = 0x456b8f1d;
pub const FACTORY_IID: u32 = 0xa56b8f17;
pub const BUFFER_RESOURCE_IID: u32 = 0x22b9da52;
pub const BUFFER_IID: u32 = 0x20732180;
pub const BUFFER_FACTORY_IID: u32 = 0xa2c72982;
pub const BUFFER_TYPE: u32 = 0x62b9da24;

#[cfg(test)]
mod tests;

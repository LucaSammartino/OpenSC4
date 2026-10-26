// SPDX-License-Identifier: GPL-3.0-or-later
use fc4_core::gdriver::shadow::stream::{self, Record};
use fc4_core::gdriver::shadow::{compare, Snapshot};

// Private static-link transport, deliberately absent from the public C ABI.
#[no_mangle]
unsafe extern "C" fn fc4_gdriver_shadow_compare(
    slot: u32,
    args: *const [u32; 20],
    before: *const Snapshot,
    after: *const Snapshot,
) -> u32 {
    if args.is_null() || before.is_null() || after.is_null() {
        return u32::MAX;
    }
    compare(slot, &*args, &*before, &*after).unwrap_or(u32::MAX)
}

// Private static-link transport. A refused capture cannot produce complete credit.
#[no_mangle]
unsafe extern "C" fn fc4_gdriver_shadow_stream(
    slot: u32,
    args: *const [u32; 20],
    before: *const Snapshot,
    after: *const Snapshot,
    records: *const Record,
    count: u32,
) -> u32 {
    if args.is_null() || before.is_null() || after.is_null() || records.is_null() || count as usize > stream::LIMIT {
        return u32::MAX;
    }
    stream::compare(
        slot,
        &*args,
        &*before,
        &*after,
        std::slice::from_raw_parts(records, count as usize),
    )
    .unwrap_or(u32::MAX)
}

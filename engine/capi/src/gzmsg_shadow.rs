// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link bounded lookup diagnostic, framework-gz-services §3/GZ11.
//! Tokens are numeric identities only: this transport never owns native objects.
use fc4_core::gz_messages::{QueueHost, QueueManager2, Token};

#[repr(C)]
#[derive(Clone, Copy)]
struct Entry {
    id: u32,
    token: u32,
    found: u32,
}

struct NumericHost;
impl QueueHost for NumericHost {
    fn add_ref_queue(&mut self, _: Token) {}
    fn release_queue(&mut self, _: Token) {}
    fn new_queue(&mut self) -> Token {
        unreachable!("read-only lookup diagnostic never creates queues")
    }
}

fn compare(baseline: &[Entry], sample: &[Entry]) -> u32 {
    let mut host = NumericHost;
    let mut manager = QueueManager2::new();
    let mut differences = 0;
    for entry in baseline {
        if entry.found > 1 || (entry.found != 0) != (entry.token != 0) {
            differences += 1;
        }
        if entry.found != 0 && !manager.add_queue(&mut host, entry.id, entry.token.into()) {
            differences += 1;
        }
    }
    for (seed, entry) in baseline.iter().zip(sample) {
        let expected = manager.get_queue(&mut host, seed.id);
        differences += u32::from(
            seed.id != entry.id
                || entry.found != u32::from(expected.is_some())
                || expected.unwrap_or(0) != u64::from(entry.token),
        );
    }
    manager.remove_all_queues(&mut host);
    differences
}

// Private static-library transport: deliberately absent from the DLL export list.
#[no_mangle]
unsafe extern "C" fn fc4_gzmsg_shadow_compare(baseline: *const Entry, sample: *const Entry, count: u32) -> u32 {
    if baseline.is_null() || sample.is_null() || count == 0 || count > 512 {
        return u32::MAX;
    }
    // SAFETY: the plugin supplies two readable arrays of count C-layout entries.
    compare(
        unsafe { std::slice::from_raw_parts(baseline, count as usize) },
        unsafe { std::slice::from_raw_parts(sample, count as usize) },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn immutable_baseline_detects_insert_remove_identity_and_bad_return() {
        let baseline = [
            Entry {
                id: 0,
                token: 0,
                found: 0,
            },
            Entry {
                id: 1,
                token: 42,
                found: 1,
            },
            Entry {
                id: u32::MAX,
                token: 42,
                found: 1,
            },
        ];
        assert_eq!(compare(&baseline, &baseline), 0);
        let mut sample = baseline;
        sample[0].found = 1;
        sample[0].token = 7;
        sample[1].found = 0;
        sample[1].token = 0;
        sample[2].token = 43;
        assert_eq!(compare(&baseline, &sample), 3);
        assert_eq!(compare(&baseline, &baseline), 0);
        sample = baseline;
        sample[0].token = 7; // false with a non-null output is not silently accepted.
        assert_eq!(compare(&baseline, &sample), 1);
    }

    #[test]
    fn transport_rejects_missing_or_unbounded_arrays() {
        let entry = Entry {
            id: 0,
            token: 0,
            found: 0,
        };
        unsafe {
            assert_eq!(fc4_gzmsg_shadow_compare(&entry, &entry, 1), 0);
            assert_eq!(fc4_gzmsg_shadow_compare(std::ptr::null(), &entry, 1), u32::MAX);
            assert_eq!(fc4_gzmsg_shadow_compare(&entry, &entry, 513), u32::MAX);
        }
    }
}

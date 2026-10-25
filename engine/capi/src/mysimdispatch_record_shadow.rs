// SPDX-License-Identifier: GPL-3.0-or-later
//! Native dispatch persistence compatibility, mysims.md §6.2.1.
//! Decode a captured record through the production codec and compare its write
//! with the entire capture. This measures persistence, never independent trips.
use std::collections::BTreeMap;

use fc4_core::mysims::dispatch::{DispatchRecordRefs, DispatchState};
use fc4_core::stream::{ByteReader, ByteWriter, OutStream, RefResolver, RefValue};

#[derive(Default)]
struct CapturedRefs(BTreeMap<u64, u32>);

impl RefResolver for CapturedRefs {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        let token = u64::from(object_id);
        if let Some(previous) = self.0.insert(token, class_id) {
            if previous != class_id {
                return None;
            }
        }
        // Tokens are capture-local numbers, never pointers or owned references.
        Some(RefValue::Host { token, class_id })
    }
}

impl DispatchRecordRefs for CapturedRefs {
    fn reference(&self, token: u64) -> RefValue {
        RefValue::Host {
            token,
            class_id: self.0[&token],
        }
    }
    fn token(&self, reference: RefValue) -> Option<u64> {
        match reference {
            RefValue::Host { token, .. } => Some(token),
            _ => None,
        }
    }
}

#[derive(Default)]
struct CapturedWriter(ByteWriter);

macro_rules! forward {
    ($name:ident, $ty:ty) => {
        fn $name(&mut self, value: $ty) -> bool {
            self.0.$name(value)
        }
    };
}

impl OutStream for CapturedWriter {
    forward!(put_u8, u8);
    forward!(put_u16, u16);
    forward!(put_u32, u32);
    forward!(put_u64, u64);
    forward!(put_f32, f32);
    forward!(put_f64, f64);
    forward!(put_bytes, &[u8]);
    forward!(put_str, &[u8]);
    fn put_ref(&mut self, reference: &RefValue) -> bool {
        match reference {
            RefValue::Null => self.0.put_u32(0),
            RefValue::Host { token, class_id } => {
                // Keep the capture's IDs, including sparse/repeated references.
                self.0.put_u32(*token as u32) && self.0.put_u32(*class_id)
            }
            RefValue::Object(_) => {
                self.0.set_error();
                false
            }
        }
    }
    fn error(&self) -> i32 {
        self.0.error()
    }
    fn set_error(&mut self) {
        self.0.set_error();
    }
}

fn compare(native: &[u8]) -> Option<u32> {
    let mut refs = CapturedRefs::default();
    let mut input = ByteReader::with_resolver(native, &mut refs);
    // The read adapter extracts tokens only; the stream resolver records their
    // classes. No state is shared between a mutating resolver and a getter.
    let state = DispatchState::read(&mut input, &CapturedRefs::default())?;
    if input.remaining() != 0 {
        return None;
    }
    let mut output = CapturedWriter::default();
    if !state.write(&mut output, &refs) {
        return None;
    }
    let expected = output.0.bytes();
    Some((native.len().abs_diff(expected.len()) + native.iter().zip(expected).filter(|(a, b)| a != b).count()) as u32)
}

// Private static-link transport. Not a public C ABI export or factory hook.
#[no_mangle]
unsafe extern "C" fn fc4_mysimdispatch_shadow_record(native: *const u8, length: usize) -> u32 {
    crate::guard(u32::MAX, || {
        if native.is_null() || length > 16 * 1024 * 1024 {
            return u32::MAX;
        }
        // SAFETY: the plugin supplies its captured payload and exact byte count.
        compare(unsafe { std::slice::from_raw_parts(native, length) }).unwrap_or(u32::MAX)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fc4_core::mysims::dispatch::DispatchLocation;
    use fc4_core::pathfinder::RouteData;
    use fc4_core::stream::ResKey;

    fn payload() -> Vec<u8> {
        let mut state = DispatchState {
            owner: Some(77),
            auxiliary_occupant: Some(900),
            travel_type: 0xF123_4567,
            path_indices: (4, 3),
            auxiliary_string: b"dispatch codec".to_vec(),
            ..DispatchState::default()
        };
        state.current.occupant = Some(900);
        state.search.home = state.current;
        state.search.route.push_back(RouteData {
            x: 1,
            z: 255,
            travel_type: 3,
        });
        state.models.insert(
            -1,
            (
                ResKey {
                    type_id: 1,
                    group: 2,
                    instance: 3,
                },
                -8,
            ),
        );
        state.queue.push_back(DispatchLocation {
            occupant: Some(900),
            ..DispatchLocation::default()
        });
        let refs = CapturedRefs(BTreeMap::from([(77, 0x4A1DBBBF), (900, 0x12345678)]));
        let mut output = CapturedWriter::default();
        assert!(state.write(&mut output, &refs));
        output.0.into_bytes()
    }

    #[test]
    fn full_record_preserves_sparse_aliases_routes_models_and_queue() {
        let bytes = payload();
        assert_eq!(compare(&bytes), Some(0));
        assert_eq!(
            unsafe { fc4_mysimdispatch_shadow_record(bytes.as_ptr(), bytes.len()) },
            0
        );
    }

    #[test]
    fn rejects_every_truncation_trailing_data_and_conflicting_class_ids() {
        let mut bytes = payload();
        for end in 0..bytes.len() {
            assert_eq!(compare(&bytes[..end]), None, "truncated at {end}");
        }
        bytes.push(0);
        assert_eq!(compare(&bytes), None);
        let mut refs = CapturedRefs::default();
        assert!(refs.resolve(1, 7).is_some());
        assert!(refs.resolve(2, 7).is_none());
        assert_eq!(
            unsafe { fc4_mysimdispatch_shadow_record(std::ptr::null(), 0) },
            u32::MAX
        );
    }

    #[test]
    fn detects_writer_normalization_and_never_claims_trip_replay() {
        let mut bytes = payload();
        // The native reader accepts any version; the writer emits version one.
        bytes[0] = 2;
        assert_eq!(compare(&bytes), Some(1));
        // A noncanonical true byte reads true but writes one.
        bytes[4] = 0xFF;
        assert_eq!(compare(&bytes), Some(2));
    }
}

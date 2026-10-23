// SPDX-License-Identifier: GPL-3.0-or-later
//! Opaque save identities are distinct from class IDs and live pointer tokens.
use super::{LifecycleCall, LifecycleHost, View3DSave, View3DWin};

const REFERENCES: [(u32, u32, u32); 4] = [
    (0xF0, 0, 0xC9C6_28EC),
    (0xF4, 8, 0xE9C6_22D8),
    (0xF8, 4, 0xA9C7_41B3),
    (0x154, 0, 0x8695_664E),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectRecord {
    pub header: (u32, u32, u32, u32),
    pub identities: [Option<u64>; 4],
    pub triggers: Vec<(u32, u32)>,
}

pub trait ReferenceHost: LifecycleHost {
    fn read_view_record(&mut self, request: u32) -> Option<ObjectRecord>;
    fn reference_body_available(&mut self, request: u32) -> bool;
    fn write_view_record(&mut self, request: u32, record: Option<ObjectRecord>);
    /// Foreign serializer maps a non-null subobject to an opaque save identity.
    fn save_identity(&mut self, object: u32, subobject_offset: u32) -> u64;
    /// Resolve and QI the saved object. Success returns an owned reference to
    /// the requested interface; None is a null/missing reference, never a class.
    fn resolve_identity(&mut self, identity: u64, iid: u32) -> Option<u32>;
}

impl View3DWin {
    pub fn save_object_record(
        &self,
        host: &mut impl ReferenceHost,
        body_available: bool,
    ) -> Option<ObjectRecord> {
        if !body_available {
            return None;
        }
        let identities = REFERENCES.map(|(field, offset, _)| {
            self.owner
                .lifecycle
                .objects
                .get(&field)
                .map(|&object| host.save_identity(object, offset))
        });
        Some(ObjectRecord {
            header: View3DSave::header(),
            identities,
            // Logical pairs only: byte bucket order remains open as VW12.
            triggers: self
                .owner
                .view
                .triggers
                .iter()
                .map(|(&t, &n)| (t, n))
                .collect(),
        })
    }

    pub fn load_object_record(
        &mut self,
        host: &mut impl ReferenceHost,
        record: &ObjectRecord,
    ) -> Result<(), String> {
        let expected = View3DSave::header();
        if record.header.0 != expected.0
            || record.header.1 != expected.1
            || record.header.2 != expected.2
            || record.header.3 >= 5
        {
            return Err("invalid View3D record identity/version".into());
        }
        for ((field, _, iid), identity) in REFERENCES.into_iter().zip(record.identities) {
            let resolved = identity.and_then(|id| host.resolve_identity(id, iid));
            // Resolve before releasing: the saved identity may refer to the
            // same object as the current owner. Each successful QI owns a ref.
            if let Some(old) = self.owner.lifecycle.objects.remove(&field) {
                host.lifecycle(LifecycleCall::Release(old));
            }
            if let Some(object) = resolved {
                self.owner.lifecycle.objects.insert(field, object);
            }
        }
        self.owner.view.triggers = record.triggers.iter().copied().collect();
        self.owner.view.not_loaded = false;
        Ok(())
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
use super::{ByteCtype, CLocale, RenderProperties};

/// Symbolic interface pointers relative to this owner (spec §7.1). A native
/// adapter must implement the actual P / P+4 adjustment, not cast these tokens.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterfaceView {
    RenderProperties,
    Unknown,
}

impl InterfaceView {
    pub fn offset(self) -> u32 {
        match self {
            Self::RenderProperties => 0,
            Self::Unknown => 4,
        }
    }
}

/// Observation of lifetime termination, never a live returned interface reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Deletion {
    pub returned_view: InterfaceView,
    pub allocation_freed: bool,
    pub count_at_destruction: u32,
}

#[derive(Debug)]
struct Storage {
    count: u32,
    properties: Option<RenderProperties>,
}

/// Portable owner for slots 0–2 and the Unknown view's deleting destructor.
/// Both view tokens address the same count and tables. Mutations require an
/// exclusive borrow; immutable getters use no RefCell or unsynchronized mutation.
/// Rust drop also reclaims a retained outer allocation after a flags-0/2 call.
/// Operations after termination return None/false as a core safety extension;
/// native stale-pointer calls are outside RP3's supported lifetime domain.
#[derive(Debug)]
pub struct RenderPropertiesOwner {
    storage: Option<Box<Storage>>,
    deletion: Option<Deletion>,
}

impl Default for RenderPropertiesOwner {
    fn default() -> Self {
        Self::new(&CLocale)
    }
}

impl RenderPropertiesOwner {
    pub fn new(ctype: &dyn ByteCtype) -> Self {
        Self {
            storage: Some(Box::new(Storage {
                count: 0,
                properties: Some(RenderProperties::with_ctype(ctype)),
            })),
            deletion: None,
        }
    }

    pub fn properties(&self) -> Option<&RenderProperties> {
        self.storage.as_ref()?.properties.as_ref()
    }

    pub fn properties_mut(&mut self) -> Option<&mut RenderProperties> {
        self.storage.as_mut()?.properties.as_mut()
    }

    pub fn reference_count(&self) -> Option<u32> {
        self.properties()?;
        Some(self.storage.as_ref()?.count)
    }

    /// RP3: only IID 1, through either view; failure leaves output untouched.
    /// A mutable output reference enforces the native success-path writable cell.
    pub fn query_interface(
        &mut self,
        view: InterfaceView,
        iid: u32,
        output: &mut Option<InterfaceView>,
    ) -> bool {
        if iid != 1 || self.properties().is_none() {
            return false;
        }
        *output = Some(InterfaceView::Unknown);
        self.add_ref(view);
        true
    }

    pub fn add_ref(&mut self, _view: InterfaceView) -> Option<u32> {
        self.properties()?;
        let storage = self.storage.as_mut()?;
        storage.count = storage.count.wrapping_add(1);
        Some(storage.count)
    }

    pub fn release(&mut self, view: InterfaceView) -> Option<u32> {
        let count = self.reference_count()?;
        if count > 1 {
            self.storage.as_mut()?.count = count - 1;
            Some(count - 1)
        } else {
            self.deleting_destructor(view, 1)?;
            Some(0)
        }
    }

    /// U slot 3 or the complete-object destructor body. Every flag destroys the
    /// tables; only bit 0 controls freeing the outer allocation. Return is P.
    pub fn deleting_destructor(&mut self, _view: InterfaceView, flags: u32) -> Option<Deletion> {
        let count = self.reference_count()?;
        let storage = self.storage.as_mut()?;
        drop(storage.properties.take());
        let deletion = Deletion {
            returned_view: InterfaceView::RenderProperties,
            allocation_freed: flags & 1 != 0,
            count_at_destruction: count,
        };
        if deletion.allocation_freed {
            drop(self.storage.take());
        }
        self.deletion = Some(deletion);
        Some(deletion)
    }

    pub fn deletion(&self) -> Option<Deletion> {
        self.deletion
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use InterfaceView::{RenderProperties as P, Unknown as U};

    #[test]
    fn both_views_share_count_and_final_release_drops_tables_and_allocation() {
        let mut owner = RenderPropertiesOwner::default();
        let other = RenderPropertiesOwner::default();
        assert_eq!((P.offset(), U.offset()), (0, 4));
        assert_eq!(owner.reference_count(), Some(0));
        assert_eq!(owner.add_ref(P), Some(1));
        assert_eq!(owner.add_ref(U), Some(2));
        assert_eq!(owner.release(P), Some(1));
        assert_eq!(owner.properties().unwrap().num_bool_properties(), 38);
        assert_eq!(owner.release(U), Some(0));
        assert!(owner.storage.is_none()); // actual outer Box was released
        assert!(owner.properties().is_none());
        assert_eq!(owner.deletion(), Some(Deletion {
            returned_view: P,
            allocation_freed: true,
            count_at_destruction: 1, // no decrement before destruction
        }));
        assert_eq!(other.properties().unwrap().find_bool("rs"), 9);
    }

    #[test]
    fn successful_queries_return_unknown_and_acquire_exactly_one_reference() {
        for first_view in [P, U] {
            let mut owner = RenderPropertiesOwner::default();
            let mut out = None;
            assert!(owner.query_interface(first_view, 1, &mut out));
            assert_eq!(out, Some(U));
            assert_eq!(owner.reference_count(), Some(1));
            assert!(owner.query_interface(out.unwrap(), 1, &mut out));
            assert_eq!(out, Some(U));
            assert_eq!(owner.reference_count(), Some(2));
            assert_eq!(owner.release(U), Some(1));
            assert_eq!(owner.release(U), Some(0));
            assert!(owner.storage.is_none());
        }
    }

    #[test]
    fn rejected_iids_preserve_output_and_count_from_either_view() {
        let mut owner = RenderPropertiesOwner::default();
        owner.add_ref(P);
        for view in [P, U] {
            for iid in [0, 2, u32::MAX] {
                for sentinel in [None, Some(P), Some(U)] {
                    let mut out = sentinel;
                    assert!(!owner.query_interface(view, iid, &mut out));
                    assert_eq!(out, sentinel);
                    assert_eq!(owner.reference_count(), Some(1));
                }
            }
        }
        assert_eq!(owner.release(P), Some(0));
    }

    #[test]
    fn zero_count_release_through_either_view_destroys_without_wrapping() {
        for view in [P, U] {
            let mut owner = RenderPropertiesOwner::default();
            assert_eq!(owner.release(view), Some(0));
            assert!(owner.storage.is_none());
            assert_eq!(owner.deletion().unwrap().count_at_destruction, 0);
            assert!(owner.deletion().unwrap().allocation_freed);
        }
    }

    #[test]
    fn destructor_always_drops_contents_and_only_flag_bit_zero_frees_outer_box() {
        for view in [P, U] {
            for flags in [0, 1, 2, 3, 0xffff_fffe, u32::MAX] {
                let mut owner = RenderPropertiesOwner::default();
                owner.add_ref(P);
                owner.add_ref(U);
                let outer = std::ptr::from_ref(owner.storage.as_deref().unwrap());
                let deletion = owner.deleting_destructor(view, flags).unwrap();
                assert_eq!(deletion.returned_view, P);
                assert_eq!(deletion.count_at_destruction, 2);
                assert_eq!(deletion.allocation_freed, flags & 1 != 0);
                assert!(owner.properties().is_none());
                if flags & 1 == 0 {
                    let storage = owner.storage.as_deref().unwrap();
                    assert_eq!(std::ptr::from_ref(storage), outer);
                    assert!(storage.properties.is_none()); // tables dropped, Box retained
                    assert_eq!(storage.count, 2);
                } else {
                    assert!(owner.storage.is_none());
                }
                // Core safety only: terminal calls cannot destroy twice. Native
                // stale-pointer calls are explicitly outside the contract.
                assert_eq!(owner.deleting_destructor(view, 1), None);
                assert_eq!(owner.deletion(), Some(deletion));
                assert_eq!(owner.release(view), None);
            }
        }
    }

    #[test]
    fn increment_is_literal_u32_wrapping_arithmetic() {
        // Core arithmetic discriminator; not a supported reference-ownership recipe.
        let mut owner = RenderPropertiesOwner::default();
        owner.storage.as_mut().unwrap().count = u32::MAX;
        assert_eq!(owner.add_ref(U), Some(0));
        assert_eq!(owner.release(P), Some(0));
        assert_eq!(owner.deletion().unwrap().count_at_destruction, 0);
    }

    #[test]
    fn immutable_owner_getters_are_send_sync_and_instances_own_separate_tables() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RenderPropertiesOwner>();
        let mut first = RenderPropertiesOwner::default();
        let second = RenderPropertiesOwner::default();
        assert!(first.properties_mut().unwrap().set_int(23, -1));
        assert_eq!(second.properties().unwrap().get_int(23), Some(1024));
        let reader = std::thread::spawn(move || second.properties().unwrap().get_int(23));
        first.release(P);
        assert_eq!(reader.join().unwrap(), Some(1024));
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
//! Emanation identity/lifetime, audio §3.4 and audio-owner-contracts §19.
//! Numeric slots 5/6 and SEM creation/rollback remain explicitly open (AU41).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelKind {
    TwoD,
    ThreeD,
    ThreeDEffects,
}

impl ModelKind {
    pub const fn id(self) -> u32 {
        match self {
            Self::TwoD => 0x7DCD_ED2D,
            Self::ThreeD => 0x7DCD_ED3D,
            Self::ThreeDEffects => 0x0A63_A543,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::TwoD => "2D SC4K",
            Self::ThreeD => "3D SC4K",
            Self::ThreeDEffects => "3DEffect",
        }
    }
}

/// Offsets are interface views, not native pointers into this Rust object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelView {
    Base,
    Lifecycle,
}

#[derive(Debug)]
pub struct EmanationModel {
    pub kind: ModelKind,
    pub refcount: u32,
    /// Constructor words +10/+14/+18/+1C, all zero.
    pub words: [u32; 4],
    /// +24; null until a later, unsettled lifecycle operation.
    pub dependency: Option<u64>,
    destroyed: bool,
}

impl EmanationModel {
    pub fn new(kind: ModelKind) -> Self {
        Self {
            kind,
            refcount: 0,
            words: [0; 4],
            dependency: None,
            destroyed: false,
        }
    }

    pub fn query_interface(&mut self, iid: u32) -> Option<ModelView> {
        let view = match iid {
            1 | 0xDDCF_3BD7 => ModelView::Base,
            0x89E5_1472 => ModelView::Lifecycle,
            _ => return None,
        };
        self.add_ref();
        Some(view)
    }

    pub fn add_ref(&mut self) -> u32 {
        assert!(!self.destroyed);
        self.refcount = self.refcount.wrapping_add(1);
        self.refcount
    }

    /// True means deletion. Zero is not silently treated as a live reference.
    pub fn release(&mut self) -> bool {
        assert!(!self.destroyed);
        if self.refcount == 1 {
            self.refcount = 0;
            self.destroyed = true;
            true
        } else {
            self.refcount = self.refcount.wrapping_sub(1);
            false
        }
    }

    pub const fn identifier(&self) -> u32 {
        self.kind.id()
    }
    pub const fn capability(&self) -> bool {
        true
    }
}

/// Each handle represents an owned reference, including repeated handles.
/// The host implements only foreign QI/slot/reference operations.
pub trait SemHost {
    fn unregister(&mut self, hit: u64, model_id: u32);
    fn query_lifecycle(&mut self, model: u64) -> Option<u64>;
    fn shutdown_model(&mut self, lifecycle: u64);
    fn release(&mut self, reference: u64);
}

/// Retained state, not a fabricated constructor: defaults/order remain AU41.
#[derive(Debug)]
pub struct SemManager {
    pub refcount: u32,
    pub models: [Option<u64>; 3],
    pub auxiliary: [Option<u64>; 2],
    pub hit: Option<u64>,
}

impl SemManager {
    pub fn query_interface(&mut self, iid: u32) -> bool {
        if !matches!(iid, 1 | 0xAA35_5871) {
            return false;
        }
        self.add_ref();
        true
    }

    pub fn add_ref(&mut self) -> u32 {
        self.refcount = self.refcount.wrapping_add(1);
        self.refcount
    }

    pub fn release(&mut self, host: &mut impl SemHost) -> bool {
        if self.refcount == 1 {
            self.destroy(host);
            self.refcount = 0;
            true
        } else {
            self.refcount = self.refcount.wrapping_sub(1);
            false
        }
    }

    /// Recovered `Init` is teardown, not registration (§19).
    pub fn teardown(&mut self, host: &mut impl SemHost) {
        if let Some(hit) = self.hit {
            for kind in [ModelKind::TwoD, ModelKind::ThreeD, ModelKind::ThreeDEffects] {
                host.unregister(hit, kind.id());
            }
        }
        for model in self.models.iter().flatten() {
            if let Some(lifecycle) = host.query_lifecycle(*model) {
                host.shutdown_model(lifecycle);
                host.release(lifecycle);
            }
        }
        self.destroy(host);
    }

    /// Destructor only releases retained objects; it does not unregister.
    pub fn destroy(&mut self, host: &mut impl SemHost) {
        for reference in [
            self.models[0].take(),
            self.auxiliary[0].take(),
            self.models[1].take(),
            self.auxiliary[1].take(),
            self.models[2].take(),
            self.hit.take(),
        ]
        .into_iter()
        .flatten()
        {
            host.release(reference);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn au9_06_constructors_and_shared_interfaces() {
        for (kind, name, id) in [
            (ModelKind::TwoD, "2D SC4K", 0x7DCD_ED2D),
            (ModelKind::ThreeD, "3D SC4K", 0x7DCD_ED3D),
            (ModelKind::ThreeDEffects, "3DEffect", 0x0A63_A543),
        ] {
            let mut model = EmanationModel::new(kind);
            assert_eq!((kind.name(), model.identifier()), (name, id));
            assert_eq!((model.words, model.dependency, model.refcount), ([0; 4], None, 0));
            assert!(model.capability());
            assert_eq!(model.query_interface(99), None);
            assert_eq!(model.query_interface(1), Some(ModelView::Base));
            assert_eq!(model.query_interface(0xDDCF_3BD7), Some(ModelView::Base));
            assert_eq!(model.query_interface(0x89E5_1472), Some(ModelView::Lifecycle));
            assert_eq!(model.refcount, 3);
            assert!(!model.release());
            assert!(!model.release());
            assert!(model.release());
        }
    }

    #[derive(Default)]
    struct Host(Vec<(u8, u64)>);
    impl SemHost for Host {
        fn unregister(&mut self, _: u64, id: u32) {
            self.0.push((0, id as u64));
        }
        fn query_lifecycle(&mut self, model: u64) -> Option<u64> {
            self.0.push((1, model));
            (model != 2).then_some(model + 10)
        }
        fn shutdown_model(&mut self, lifecycle: u64) {
            self.0.push((2, lifecycle));
        }
        fn release(&mut self, reference: u64) {
            self.0.push((3, reference));
        }
    }

    #[test]
    fn sem_teardown_queries_balances_and_destruction_only_releases() {
        let mut sem = SemManager {
            refcount: 0,
            models: [Some(1), Some(2), Some(3)],
            auxiliary: [Some(4), None],
            hit: Some(5),
        };
        assert!(!sem.query_interface(99));
        assert!(sem.query_interface(0xAA35_5871));
        let mut host = Host::default();
        sem.teardown(&mut host);
        assert_eq!(
            host.0,
            [
                (0, 0x7DCD_ED2D),
                (0, 0x7DCD_ED3D),
                (0, 0x0A63_A543),
                (1, 1),
                (2, 11),
                (3, 11),
                (1, 2),
                (1, 3),
                (2, 13),
                (3, 13),
                (3, 1),
                (3, 4),
                (3, 2),
                (3, 3),
                (3, 5)
            ]
        );
        host.0.clear();
        assert!(sem.release(&mut host));
        assert!(host.0.is_empty());
        sem.refcount = 0;
        assert!(!sem.release(&mut host));
        assert_eq!(sem.refcount, u32::MAX);
    }
}

\\?\C:\Users\tlori\FreeCity4\.claude\worktrees\core-network-type-info4\engine\core\src\network_geometry\type_info.rs:

//! Bounded process-global TypeInfo owner (network-type-info §§1–8).
//!
//! One `TypeRecords` models one process array, never a city-owned table. There is
//! no mutable Rust global and no native pointer arithmetic. Foreign content and
//! property conversion are raw primitives, not implementations of their parsers.
//! TI1–4 remain unsupported domains. In particular nonzero derivation does not
//! borrow the older geometry helper's approximate f64 tangent.
use super::params::{widths, PlacementParams, TypeInfo, DEG_TO_RAD, TYPE_COUNT};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token(pub u64);

/// Logical backing identities describe retention, not Rust/native Vec layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Storage {
    Foreign(Token),
    Logical { record: usize, field: u8 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unsupported {
    PropertyVariant,       // TI1: no invented conversion/rejection policy.
    NonfiniteAngle,        // TI3: admitted conversion precision domain is finite.
    NonreturningPrimitive, // TI2: reached prefix only, no unwind guarantee.
    NullEvaluator,         // TI2: AddRules(null) was reached; never safe-success credit.
    NonzeroDerivation { source_offset: u8, completed_stores: usize }, // TI3
    SupportEdit,           // TI4 or an unadmitted container operation.
    PrimitiveReply,
    Destroyed,
}

/// Presence, writes and boolean result are independent. Vector words are the
/// admitted getter output (possibly partial even on false), not parsed owners.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Property {
    Absent,
    Scalar { write: Option<u32>, success: bool },
    Vector { words: Vec<u32>, success: bool },
    UnsupportedVariant,
}

/// Raw container callback insertion; bucket/chain position come from the
/// foreign container primitive. No key hashing or duplicate policy is inferred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupportEntry {
    pub bucket: usize,
    pub position: usize,
    pub node: Token,
    pub allocation_14: Option<Token>,
    pub allocation_8: Option<Token>,
}

/// Resource callsites are opaque identities where the lifecycle publication
/// does not name the resource strings. No guessed resource labels/TGIs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Primitive {
    SupportSetup {
        record: usize,
        initial_size: usize,
    },
    AllocateEvaluator {
        record: usize,
        bytes: usize,
    },
    ConstructEvaluator {
        allocation: Token,
    },
    AddEvaluatorRules {
        evaluator: Option<Token>,
        record: usize,
        callsite: u8,
    },
    DeleteEvaluator {
        token: Token,
        deleting: u8,
    },
    BeginBridgeEnumeration {
        record: usize,
    },
    BridgeEntry {
        record: usize,
        index: usize,
    },
    DestroyBridgeSectionString {
        record: usize,
    },
    SpecializeBridges {
        record: usize,
        width: u32,
    },
    DestroyBridge(Token),
    BeginSupportEnumeration {
        record: usize,
        group: u32,
    },
    SupportEntry {
        record: usize,
        group: u32,
        index: usize,
    },
    Free(Storage),
    AcquirePlacement {
        type_id: usize,
        resource_type: u32,
        group: u32,
        instance: u32,
        interface: u32,
        zeros: [u32; 2],
    },
    Property {
        resource: Token,
        id: u32,
    },
    ReleasePlacement(Token),
    ClearVerifierStore,
    EnumerateVerifiers,
    EnumerateConflicts,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Bool(bool),
    Token(Option<Token>),
    SupportSetup { buckets: usize, storage: Token },
    Support(Option<SupportEntry>),
    Property(Property),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Construct,
    Clear,
    Metadata,
    Complete,
    Destroy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    pub phase: Phase,
    pub record: Option<usize>,
    pub loaded: [u8; TYPE_COUNT],
}

pub trait Foreign {
    /// Execute just this raw operation. Err stops at the reached prefix without
    /// fabricated rollback, publication, cleanup or exception-safety promises.
    fn call(&mut self, operation: Primitive) -> Result<Reply, Unsupported>;
    fn observe(&mut self, _observation: Observation) {}
}

const PLACEMENT_INSTANCES: [u32; TYPE_COUNT] = [
    0x0781214f, 0x0814e612, 0xa8433f6b, 0x492a03a3, 0x095117db, 0, 0x2b730a17, 0x0a367ca7, 0xcb79e089, 0xebe08882,
    0x8bda0c2b, 0xebe08cda, 0x0be08c87,
];
pub const RELOAD_ORDER: [usize; TYPE_COUNT] = [0, 10, 11, 1, 8, 9, 3, 2, 12, 6, 4, 7, 5];
const METADATA: [[u32; 6]; TYPE_COUNT] = [
    [1, 0xa766c101, 0x2821ed93, 0, 0xe17bb266, 0x49e13260],
    [2, 0xc8433ecb, 0xe8347989, 0, 0x217bb308, 0],
    [0, 0x08433f47, 0xa8434037, 0, 0x8bf30a8b, 0],
    [1, 0x092a047d, 0xa92a02ea, 0, 0x2a920bf9, 0],
    [1, 0x09511847, 0xa951171c, 0xf0, 0x82c2ff49, 0],
    [0, 0xa88e1592, 0x088e1962, 0, 0xa16f1463, 0x69e13770],
    [0, 0x0b730a26, 0xcb730fac, 0, 0x6b730b83, 0],
    [1, 0x8a15ee13, 0x8a15f3f2, 0xf1, 0xc17bb329, 0],
    [2, 0x0b79e2e1, 0x2b79dffb, 0, 0xab79e761, 0],
    [2, 0xabe08433, 0xebe084c2, 0, 0xebe091b6, 0],
    [1, 0x2be08438, 0xcbe084cb, 0, 0xabe091a8, 0],
    [1, 0x4be08afb, 0x6be08658, 0, 0x4be091b2, 0],
    [0, 0x6be0843b, 0xebe084d1, 0, 0x6be091b4, 0],
];
const VECTOR_OFFSETS: [u8; 5] = [0x28, 0x54, 0x60, 0x6c, 0xa0];

/// Mutable native placement: the five angle members in `geometry` store radians
/// after admission, including repeated conversion on missing properties. The
/// degree-valued public geometry input can only be obtained via `degree_view`.
/// Existing semantic fields have one authoritative storage location here.
#[derive(Clone, Debug, PartialEq)]
pub struct NativePlacement {
    geometry: PlacementParams,
    extra: BTreeMap<u8, u32>,
    vectors: [Vec<u32>; 3], // only fields absent from PlacementParams: 28,54,60
    backing: [Option<Storage>; 5],
}

impl NativePlacement {
    fn static_zero() -> Self {
        // Static-zero origin, deliberately NOT PlacementParams::default(): its
        // geometry caller defaults are guesses and are not native construction.
        Self {
            geometry: PlacementParams {
                max_slope: 0.0,
                slope_04: 0.0,
                max_slope_change: 0.0,
                max_raise: 0.0,
                max_cut: 0.0,
                resolve_tolerance: 0.0,
                tunnel_trigger_slope: 0.0,
                tunnel_end_slope: 0.0,
                straight_only: false,
                new_cell_tolerance: 0.0,
                support_threshold: 0.0,
                max_support_height: 0.0,
                tunnel_clearance: Vec::new(),
                cell_cost: 0,
                bridge_cost: 0,
                tunnel_cost: 0,
                earthwork_cost: 0.0,
                elevated_ids: Vec::new(),
            },
            extra: BTreeMap::new(),
            vectors: std::array::from_fn(|_| Vec::new()),
            backing: [None; 5],
        }
    }

    /// Explicit lossy radians -> degrees compatibility view; does not run the
    /// older approximate TypeInfo derivation or modify the mutable native state.
    pub fn degree_view(&self) -> PlacementParams {
        let mut p = self.geometry.clone();
        p.max_slope /= DEG_TO_RAD;
        p.slope_04 /= DEG_TO_RAD;
        p.max_slope_change /= DEG_TO_RAD;
        p.tunnel_trigger_slope /= DEG_TO_RAD;
        p.tunnel_end_slope /= DEG_TO_RAD;
        p
    }

    pub fn straight_only(&self) -> bool {
        self.geometry.straight_only
    }

    pub fn word(&self, offset: u8) -> u32 {
        let p = &self.geometry;
        match offset {
            0 => p.max_slope.to_bits(),
            4 => p.slope_04.to_bits(),
            8 => p.max_slope_change.to_bits(),
            0x0c => p.max_raise.to_bits(),
            0x10 => p.max_cut.to_bits(),
            0x14 => p.resolve_tolerance.to_bits(),
            0x18 => p.tunnel_trigger_slope.to_bits(),
            0x1c => p.tunnel_end_slope.to_bits(),
            0x25 => u32::from(p.straight_only),
            0x34 => p.new_cell_tolerance.to_bits(),
            0x38 => p.support_threshold.to_bits(),
            0x3c => p.max_support_height.to_bits(),
            0x8c => p.cell_cost as u32,
            0x90 => p.bridge_cost as u32,
            0x94 => p.tunnel_cost as u32,
            0x98 => p.earthwork_cost.to_bits(),
            _ => *self.extra.get(&offset).unwrap_or(&0),
        }
    }

    fn store(&mut self, offset: u8, word: u32) {
        let p = &mut self.geometry;
        let f = f32::from_bits(word);
        match offset {
            0 => p.max_slope = f,
            4 => p.slope_04 = f,
            8 => p.max_slope_change = f,
            0x0c => p.max_raise = f,
            0x10 => p.max_cut = f,
            0x14 => p.resolve_tolerance = f,
            0x18 => p.tunnel_trigger_slope = f,
            0x1c => p.tunnel_end_slope = f,
            0x25 => p.straight_only = word != 0,
            0x34 => p.new_cell_tolerance = f,
            0x38 => p.support_threshold = f,
            0x3c => p.max_support_height = f,
            0x8c => p.cell_cost = word as i32,
            0x90 => p.bridge_cost = word as i32,
            0x94 => p.tunnel_cost = word as i32,
            0x98 => p.earthwork_cost = f,
            _ => {
                self.extra.insert(offset, word);
            }
        }
    }

    pub fn vector_words(&self, offset: u8) -> Vec<u32> {
        match offset {
            0x28 => self.vectors[0].clone(),
            0x54 => self.vectors[1].clone(),
            0x60 => self.vectors[2].clone(),
            0x6c => self.geometry.tunnel_clearance.iter().map(|v| v.to_bits()).collect(),
            0xa0 => self.geometry.elevated_ids.clone(),
            _ => Vec::new(),
        }
    }

    fn set_vector(&mut self, record: usize, offset: u8, words: Vec<u32>) {
        let slot = VECTOR_OFFSETS
            .iter()
            .position(|o| *o == offset)
            .expect("owned vector offset");
        if !words.is_empty() && self.backing[slot].is_none() {
            self.backing[slot] = Some(Storage::Logical { record, field: offset });
        }
        match offset {
            0x28 => self.vectors[0] = words,
            0x54 => self.vectors[1] = words,
            0x60 => self.vectors[2] = words,
            0x6c => self.geometry.tunnel_clearance = words.into_iter().map(f32::from_bits).collect(),
            0xa0 => self.geometry.elevated_ids = words,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub struct TypeRecord {
    loaded: u8,
    metadata: [u32; 6],
    widths: (u32, u32),
    evaluators: Vec<Option<Token>>,
    bridges: Vec<Token>,
    supports: Vec<Vec<SupportEntry>>,
    backing: [Option<Storage>; 3], // evaluator, bridge, support buckets
    placement: NativePlacement,
    derived: [u32; 5],
}

impl TypeRecord {
    fn static_zero() -> Self {
        Self {
            loaded: 0,
            metadata: [0; 6],
            widths: (0, 0),
            evaluators: Vec::new(),
            bridges: Vec::new(),
            supports: Vec::new(),
            backing: [None; 3],
            placement: NativePlacement::static_zero(),
            derived: [0; 5],
        }
    }
    pub fn loaded(&self) -> u8 {
        self.loaded
    }
    pub fn metadata(&self) -> [u32; 6] {
        self.metadata
    }
    pub fn widths(&self) -> (u32, u32) {
        self.widths
    }
    pub fn placement(&self) -> &NativePlacement {
        &self.placement
    }
    pub fn derived_words(&self) -> [u32; 5] {
        self.derived
    }
    pub fn evaluator_entries(&self) -> &[Option<Token>] {
        &self.evaluators
    }
    pub fn bridge_entries(&self) -> &[Token] {
        &self.bridges
    }
    pub fn support_count(&self) -> usize {
        self.supports.iter().map(Vec::len).sum()
    }
    pub fn backing_identities(&self) -> ([Option<Storage>; 3], [Option<Storage>; 5]) {
        (self.backing, self.placement.backing)
    }

    /// Compatibility view using the already owned five derived stores. Tunnel
    /// helpers retain their old geometry approximation qualification; this view
    /// makes no new bit-exact tangent claim for them.
    pub fn geometry_view(&self, type_id: u32) -> TypeInfo {
        let mut view = TypeInfo::new(type_id, self.placement.degree_view());
        view.widths = self.widths;
        [
            view.slope_rise,
            view.diagonal_slope_rise,
            view.smoothness,
            view.diagonal_smoothness,
            view.rise_04,
        ] = self.derived.map(f32::from_bits);
        view
    }
}

#[derive(Debug)]
pub struct TypeRecords {
    records: [TypeRecord; TYPE_COUNT],
    destroyed: bool,
}

fn token(reply: Reply) -> Result<Option<Token>, Unsupported> {
    if let Reply::Token(value) = reply {
        Ok(value)
    } else {
        Err(Unsupported::PrimitiveReply)
    }
}

impl TypeRecords {
    pub fn construct(f: &mut impl Foreign) -> Result<Self, Unsupported> {
        let mut owner = Self {
            records: std::array::from_fn(|_| TypeRecord::static_zero()),
            destroyed: false,
        };
        for record in 0..TYPE_COUNT {
            let Reply::SupportSetup { buckets, storage } = f.call(Primitive::SupportSetup {
                record,
                initial_size: 100,
            })?
            else {
                return Err(Unsupported::PrimitiveReply);
            };
            let r = &mut owner.records[record];
            r.supports = vec![Vec::new(); buckets];
            r.backing[2] = Some(Storage::Foreign(storage));
            // Constructor-only writes, distinct from zero-before-construction.
            r.derived = [0x41200000, 0x40e24630, 0x41200000, 0x40e24630, 0x41a00000];
            owner.observe(record, Phase::Construct, f);
        }
        Ok(owner)
    }
    pub fn record(&self, record: usize) -> &TypeRecord {
        &self.records[record]
    }
    pub fn records(&self) -> &[TypeRecord; TYPE_COUNT] {
        &self.records
    }
    fn live(&self) -> Result<(), Unsupported> {
        if self.destroyed {
            Err(Unsupported::Destroyed)
        } else {
            Ok(())
        }
    }
    fn observe(&self, record: usize, phase: Phase, f: &mut impl Foreign) {
        f.observe(Observation {
            phase,
            record: Some(record),
            loaded: std::array::from_fn(|i| self.records[i].loaded),
        });
    }

    /// Raw imported callback entry, including null. No deduplication.
    pub fn append_evaluator_entry(&mut self, record: usize, entry: Option<Token>) {
        let r = &mut self.records[record];
        r.backing[0].get_or_insert(Storage::Logical { record, field: 0x24 });
        r.evaluators.push(entry);
    }

    fn clear_supports(&mut self, record: usize, f: &mut impl Foreign) -> Result<(), Unsupported> {
        for bucket in &mut self.records[record].supports {
            // Remove only after all three returning frees; errors expose a
            // reached prefix, never an exception-safe resumable contract (TI2).
            while let Some(node) = bucket.first() {
                for allocation in [node.allocation_14, node.allocation_8, Some(node.node)]
                    .into_iter()
                    .flatten()
                {
                    f.call(Primitive::Free(Storage::Foreign(allocation)))?;
                }
                bucket.remove(0);
            }
        }
        Ok(())
    }

    pub fn clear(&mut self, record: usize, f: &mut impl Foreign) -> Result<(), Unsupported> {
        self.live()?;
        let r = &mut self.records[record];
        while let Some(entry) = r.evaluators.last().copied() {
            if let Some(token) = entry {
                f.call(Primitive::DeleteEvaluator { token, deleting: 1 })?;
            }
            r.evaluators.pop();
        }
        while let Some(entry) = r.bridges.first().copied() {
            f.call(Primitive::DestroyBridge(entry))?;
            r.bridges.remove(0);
        }
        self.clear_supports(record, f)?;
        self.observe(record, Phase::Clear, f);
        Ok(())
    }

    /// Explicit exit, not Drop: primitive calls cannot be made from Rust Drop.
    /// A failed primitive cannot be retried as though cleanup had been safe.
    pub fn destroy(&mut self, f: &mut impl Foreign) -> Result<(), Unsupported> {
        self.live()?;
        for record in (0..TYPE_COUNT).rev() {
            self.observe(record, Phase::Destroy, f);
            self.clear(record, f)?;
            for slot in (0..5).rev() {
                if let Some(storage) = self.records[record].placement.backing[slot] {
                    f.call(Primitive::Free(storage))?;
                    self.records[record].placement.backing[slot] = None;
                }
            }
            self.clear_supports(record, f)?;
            if let Some(storage) = self.records[record].backing[2] {
                f.call(Primitive::Free(storage))?;
                self.records[record].backing[2] = None;
            }
            // Clear supplied an empty bridge suffix; general suffix move/copy
            // (the partial 0x006412B0 body) is intentionally not implemented.
            for slot in [1, 0] {
                if let Some(storage) = self.records[record].backing[slot] {
                    f.call(Primitive::Free(storage))?;
                    self.records[record].backing[slot] = None;
                }
            }
        }
        self.destroyed = true;
        Ok(())
    }

    pub fn load_evaluator(&mut self, record: usize, callsite: u8, f: &mut impl Foreign) -> Result<(), Unsupported> {
        self.live()?;
        let allocation = token(f.call(Primitive::AllocateEvaluator { record, bytes: 16 })?)?;
        let evaluator = match allocation {
            Some(allocation) => token(f.call(Primitive::ConstructEvaluator { allocation })?)?,
            None => None,
        };
        f.call(Primitive::AddEvaluatorRules {
            evaluator,
            record,
            callsite,
        })?;
        if evaluator.is_none() {
            return Err(Unsupported::NullEvaluator);
        }
        self.append_evaluator_entry(record, evaluator);
        Ok(())
    }

    pub fn load_bridges(&mut self, record: usize, f: &mut impl Foreign) -> Result<(), Unsupported> {
        self.live()?;
        // Bool and callback effects are independent primitive inputs.
        f.call(Primitive::BeginBridgeEnumeration { record })?;
        let mut index = 0;
        while let Some(entry) = token(f.call(Primitive::BridgeEntry { record, index })?)? {
            let r = &mut self.records[record];
            r.backing[1].get_or_insert(Storage::Logical { record, field: 0x30 });
            r.bridges.push(entry);
            index += 1;
        }
        f.call(Primitive::DestroyBridgeSectionString { record })?;
        f.call(Primitive::SpecializeBridges {
            record,
            width: self.records[record].widths.0,
        })?;
        Ok(())
    }

    pub fn load_supports(&mut self, record: usize, f: &mut impl Foreign) -> Result<(), Unsupported> {
        self.live()?;
        f.call(Primitive::BeginSupportEnumeration {
            record,
            group: 0x8974f80f,
        })?;
        let mut index = 0;
        loop {
            let Reply::Support(entry) = f.call(Primitive::SupportEntry {
                record,
                group: 0x8974f80f,
                index,
            })?
            else {
                return Err(Unsupported::PrimitiveReply);
            };
            let Some(entry) = entry else {
                break;
            };
            let Some(bucket) = self.records[record].supports.get_mut(entry.bucket) else {
                return Err(Unsupported::SupportEdit);
            };
            if entry.position > bucket.len() {
                return Err(Unsupported::SupportEdit);
            }
            bucket.insert(entry.position, entry);
            index += 1;
        }
        Ok(())
    }

    fn property(
        &mut self,
        record: usize,
        resource: Token,
        id: u32,
        offset: u8,
        vector: bool,
        f: &mut impl Foreign,
    ) -> Result<bool, Unsupported> {
        let Reply::Property(property) = f.call(Primitive::Property { resource, id })? else {
            return Err(Unsupported::PrimitiveReply);
        };
        match property {
            Property::Absent => Ok(false),
            Property::Scalar { write, success } if !vector => {
                if let Some(word) = write {
                    self.records[record].placement.store(offset, word);
                }
                Ok(success)
            }
            Property::Vector { words, success } if vector => {
                self.records[record].placement.set_vector(record, offset, words);
                Ok(success)
            }
            _ => Err(Unsupported::PropertyVariant),
        }
    }

    pub fn load_placement(&mut self, record: usize, f: &mut impl Foreign) -> Result<bool, Unsupported> {
        self.live()?;
        let acquired = token(f.call(Primitive::AcquirePlacement {
            type_id: record,
            resource_type: 0x6534284a,
            group: 0x084344e0,
            instance: PLACEMENT_INSTANCES[record],
            interface: 0xa52160f5,
            zeros: [0, 0],
        })?)?;
        let Some(resource) = acquired else {
            return Ok(false);
        };
        for (offset, id) in [
            (0, 0x879d12e7),
            (4, 0x279d1707),
            (8, 0x07b36c71),
            (0x0c, 0xc7b36ca4),
            (0x10, 0xc7b36ca5),
            (0x14, 0xc804dd86),
            (0x18, 0xc804dd87),
            (0x1c, 0xc804dd88),
            (0x20, 0xc804dd89),
            (0x24, 0xc804dd90),
        ] {
            self.property(record, resource, id, offset, false, f)?;
            if [0, 4, 8, 0x18, 0x1c].contains(&offset) {
                let p = &mut self.records[record].placement;
                let angle = f32::from_bits(p.word(offset));
                // TI3: finite binary32 product, exact f64 intermediate followed
                // by one round-nearest binary32 store. No tangent claim.
                if !angle.is_finite() {
                    return Err(Unsupported::NonfiniteAngle);
                }
                let radians = (f64::from(angle) * f64::from(DEG_TO_RAD)) as f32;
                p.store(offset, radians.to_bits());
            }
        }
        self.records[record].placement.store(0x25, 0);
        if !self.property(record, resource, 0xc804dd9b, 0x50, false, f)? {
            self.records[record].placement.store(0x50, 1000);
        }
        self.property(record, resource, 0xc804dd92, 0x28, true, f)?;
        for (offset, id) in [
            (0x34, 0xc804dd93),
            (0x38, 0xc804dd94),
            (0x3c, 0xc804dd95),
            (0x40, 0xc804dd96),
            (0x44, 0xc804dd97),
            (0x48, 0xc804dd98),
            (0x4c, 0xc804dd99),
        ] {
            self.property(record, resource, id, offset, false, f)?;
        }
        for (offset, id) in [
            (0x54, 0xe974eaa4),
            (0x60, 0xe974eaa5),
            (0x6c, 0xe974eaa6),
            (0xa0, 0xe974eaa7),
        ] {
            self.records[record].placement.set_vector(record, offset, Vec::new());
            let success = self.property(record, resource, id, offset, true, f)?;
            if !success && [0x60, 0x6c].contains(&offset) {
                let p = &mut self.records[record].placement;
                let mut words = p.vector_words(offset);
                words.extend(std::iter::repeat_n(
                    if offset == 0x60 { 0 } else { 0x41200000 },
                    p.vectors[1].len(),
                ));
                p.set_vector(record, offset, words);
            }
        }
        for (offset, id, initial) in [
            (0x78, 0x4a270db0, 0x3fc00000),
            (0x7c, 0x4a270e94, 0x42000000),
            (0x80, 0x4a271248, 0),
            (0x84, 0x4a27124f, 0),
            (0x88, 0x4a275748, 1),
        ] {
            self.records[record].placement.store(offset, initial);
            self.property(record, resource, id, offset, false, f)?;
        }
        for (offset, id, default) in [
            (0x8c, 0x4a2d49b9, 1),
            (0x90, 0x8a2d49ea, 10),
            (0x94, 0x8a2d49f0, 20),
            (0x98, 0x6a2d49f4, 0x3dcccccd),
        ] {
            if !self.property(record, resource, id, offset, false, f)? {
                self.records[record].placement.store(offset, default);
            }
        }
        if !self.property(record, resource, 0xea86e3b5, 0x9c, false, f)? {
            self.records[record].placement.store(0x8c, 1);
        }
        f.call(Primitive::ReleasePlacement(resource))?;
        Ok(true)
    }

    /// Exact zero-angle branch only. Every preceding store remains visible if a
    /// later angle hits TI3. We never multiply an already rounded straight field.
    pub fn derive(&mut self, record: usize) -> Result<(), Unsupported> {
        self.live()?;
        let r = &mut self.records[record];
        for (offset, slot, count) in [(0, 0, 2), (8, 2, 2), (4, 4, 1)] {
            let angle = f32::from_bits(r.placement.word(offset));
            if angle != 0.0 {
                return Err(Unsupported::NonzeroDerivation {
                    source_offset: offset,
                    completed_stores: slot,
                });
            }
            for field in slot..slot + count {
                r.derived[field] = angle.to_bits();
            }
        }
        Ok(())
    }

    pub fn load_complete(&mut self, record: usize, f: &mut impl Foreign) {
        self.records[record].loaded = 1;
        self.observe(record, Phase::Complete, f);
    }
    pub fn load_verifiers(&mut self, f: &mut impl Foreign) -> Result<(), Unsupported> {
        f.call(Primitive::ClearVerifierStore)?;
        f.call(Primitive::EnumerateVerifiers)?;
        Ok(())
    }
    pub fn load_conflicts(&mut self, f: &mut impl Foreign) -> Result<(), Unsupported> {
        f.call(Primitive::EnumerateConflicts)?;
        Ok(())
    }
    pub fn reload(&mut self, f: &mut impl Foreign) -> Result<(), Unsupported> {
        self.live()?;
        for record in RELOAD_ORDER {
            self.clear(record, f)?;
            self.records[record].metadata = METADATA[record];
            self.records[record].widths = widths(record as u32);
            self.observe(record, Phase::Metadata, f);
            if record != 5 {
                self.load_evaluator(record, 0, f)?;
                self.load_evaluator(record, 1, f)?;
                self.load_bridges(record, f)?;
                if [0, 10, 11, 1, 8, 9, 3].contains(&record) {
                    self.load_supports(record, f)?;
                }
                self.load_placement(record, f)?;
                self.derive(record)?;
                if record == 3 {
                    self.records[record].placement.store(0x25, 1);
                }
            }
            self.load_complete(record, f);
        }
        self.load_verifiers(f)
    }
}

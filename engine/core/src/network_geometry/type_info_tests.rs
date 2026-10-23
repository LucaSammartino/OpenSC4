//! Literal expectations transcribed from TI-V01–27 before production code.
use super::type_info::*;
use std::collections::BTreeMap;

#[derive(Default)]
struct Fixture {
    trace: Vec<Primitive>,
    next: u64,
    acquired: bool,
    properties: BTreeMap<u32, Property>,
    bridge: Vec<Token>,
    support: Vec<SupportEntry>,
    null_evaluator: bool,
    fail: Option<Primitive>,
    observations: Vec<Observation>,
}

impl Foreign for Fixture {
    fn call(&mut self, op: Primitive) -> Result<Reply, Unsupported> {
        self.trace.push(op.clone());
        if self.fail.as_ref() == Some(&op) {
            return Err(Unsupported::NonreturningPrimitive);
        }
        Ok(match op {
            Primitive::SupportSetup { record, .. } => Reply::SupportSetup {
                buckets: 3,
                storage: Token(900 + record as u64),
            },
            Primitive::AllocateEvaluator { .. } if self.null_evaluator => Reply::Token(None),
            Primitive::AllocateEvaluator { .. } => {
                self.next += 1;
                Reply::Token(Some(Token(self.next)))
            }
            Primitive::ConstructEvaluator { allocation } => Reply::Token(Some(allocation)),
            Primitive::AcquirePlacement { .. } => Reply::Acquired {
                success: self.acquired,
                // False acquisition deliberately leaves a raw out token too.
                // The owner must branch on the boolean and must not release it.
                resource: Some(Token(800)),
            },
            Primitive::Property { id, .. } => {
                Reply::Property(self.properties.get(&id).cloned().unwrap_or(Property::Absent))
            }
            Primitive::BridgeEntry { index, .. } => Reply::Token(self.bridge.get(index).copied()),
            Primitive::SupportEntry { index, .. } => Reply::Support(self.support.get(index).cloned()),
            // Deliberately false: no outer owner branch may suppress append/publication.
            _ => Reply::Bool(false),
        })
    }
    fn observe(&mut self, observation: Observation) {
        self.observations.push(observation);
    }
}

fn fresh() -> (TypeRecords, Fixture) {
    let mut f = Fixture::default();
    let owner = TypeRecords::construct(&mut f).unwrap();
    (owner, f)
}

#[test]
fn ti_v01_v04_static_constructor_and_reverse_exit() {
    let (mut owner, mut f) = fresh();
    assert_eq!(
        f.trace,
        (0..13)
            .map(|record| Primitive::SupportSetup {
                record,
                initial_size: 100
            })
            .chain([Primitive::RegisterExit])
            .collect::<Vec<_>>()
    );
    for r in owner.records() {
        assert_eq!(r.loaded(), 0);
        assert_eq!(r.metadata(), [0; 6]);
        assert_eq!(r.widths(), (0, 0));
        assert_eq!(
            r.derived_words(),
            [0x41200000, 0x40e24630, 0x41200000, 0x40e24630, 0x41a00000]
        );
        for offset in [0, 4, 8, 0x20] {
            assert_eq!(r.placement().word(offset), 0);
        }
    }
    f.trace.clear();
    owner.destroy(&mut f).unwrap();
    assert_eq!(
        f.observations
            .iter()
            .filter_map(|o| match o.phase {
                Phase::Destroy => Some(o.record.unwrap()),
                _ => None,
            })
            .collect::<Vec<_>>(),
        (0..13).rev().collect::<Vec<_>>()
    );
    assert_eq!(f.trace.iter().filter(|op| matches!(op, Primitive::Free(_))).count(), 13);
}

#[test]
fn ti_v02_v03_v04_owned_destruction_order_and_backing_retention() {
    let (mut owner, mut f) = fresh();
    owner.load_evaluator(0, 0, &mut f).unwrap();
    owner.append_evaluator_entry(0, None);
    owner.load_evaluator(0, 1, &mut f).unwrap();
    f.bridge = vec![Token(21), Token(22)];
    owner.load_bridges(0, &mut f).unwrap();
    f.support = vec![
        SupportEntry {
            bucket: 0,
            position: 0,
            node: Token(31),
            allocation_14: Some(Token(41)),
            allocation_8: Some(Token(51)),
        },
        SupportEntry {
            bucket: 0,
            position: 1,
            node: Token(32),
            allocation_14: None,
            allocation_8: Some(Token(52)),
        },
        SupportEntry {
            bucket: 2,
            position: 0,
            node: Token(33),
            allocation_14: Some(Token(43)),
            allocation_8: None,
        },
    ];
    owner.load_supports(0, &mut f).unwrap();
    owner.load_complete(0, &mut f);
    let backing = owner.record(0).backing_identities();
    let before = owner.record(0).placement().clone();
    f.trace.clear();
    owner.clear(0, &mut f).unwrap();
    assert_eq!(
        f.trace,
        vec![
            Primitive::DeleteEvaluator {
                token: Token(2),
                deleting: 1
            },
            Primitive::DeleteEvaluator {
                token: Token(1),
                deleting: 1
            },
            Primitive::DestroyBridge(Token(21)),
            Primitive::DestroyBridge(Token(22)),
            Primitive::Free(Storage::Foreign(Token(41))),
            Primitive::Free(Storage::Foreign(Token(51))),
            Primitive::Free(Storage::Foreign(Token(31))),
            Primitive::Free(Storage::Foreign(Token(52))),
            Primitive::Free(Storage::Foreign(Token(32))),
            Primitive::Free(Storage::Foreign(Token(43))),
            Primitive::Free(Storage::Foreign(Token(33))),
        ]
    );
    assert_eq!(owner.record(0).loaded(), 1);
    assert_eq!(owner.record(0).placement(), &before);
    assert_eq!(owner.record(0).backing_identities(), backing);
    f.trace.clear();
    owner.clear(0, &mut f).unwrap();
    assert!(f.trace.is_empty());
}

#[test]
fn ti_v05_v10_reload_latch_order_metadata_and_powerline() {
    let (mut owner, mut f) = fresh();
    owner.reload(&mut f).unwrap();
    let published = f
        .observations
        .iter()
        .filter(|o| o.phase == Phase::Complete)
        .collect::<Vec<_>>();
    assert_eq!(
        published.iter().map(|o| o.record.unwrap()).collect::<Vec<_>>(),
        [0, 10, 11, 1, 8, 9, 3, 2, 12, 6, 4, 7, 5]
    );
    assert_eq!(published[0].loaded, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(owner.record(0).derived_words(), [0; 5]);
    assert_eq!(
        owner.record(5).derived_words(),
        [0x41200000, 0x40e24630, 0x41200000, 0x40e24630, 0x41a00000]
    );
    assert_eq!(
        owner.record(4).metadata(),
        [1, 0x09511847, 0xa951171c, 0xf0, 0x82c2ff49, 0]
    );
    assert_eq!(
        owner.record(7).metadata(),
        [1, 0x8a15ee13, 0x8a15f3f2, 0xf1, 0xc17bb329, 0]
    );
    assert_eq!(owner.record(5).widths(), (1, 2));
    assert!(owner.record(3).placement().straight_only());
    assert_eq!(
        f.trace
            .iter()
            .filter(|p| matches!(p, Primitive::AllocateEvaluator { bytes: 16, .. }))
            .count(),
        24
    );
    assert_eq!(
        f.trace
            .iter()
            .filter(|p| matches!(
                p,
                Primitive::SupportEntry {
                    group: 0x8974f80f,
                    index: 0,
                    ..
                }
            ))
            .count(),
        7
    );
    assert_eq!(
        &f.trace[f.trace.len() - 2..],
        &[Primitive::ClearVerifierStore, Primitive::EnumerateVerifiers]
    );
    f.observations.clear();
    owner.reload(&mut f).unwrap();
    assert!(f.observations.iter().all(|o| o.loaded == [1; 13]));
    assert_eq!(owner.record(0).evaluator_entries().len(), 2);
}

#[test]
fn ti_v11_v12_absent_defaults_and_vector_reset() {
    let (mut owner, mut f) = fresh();
    f.acquired = true;
    f.properties.insert(
        0xc804dd92,
        Property::Vector {
            words: vec![7, 8],
            success: true,
        },
    );
    owner.load_placement(0, &mut f).unwrap();
    f.properties.clear();
    owner.load_placement(0, &mut f).unwrap();
    let p = owner.record(0).placement();
    assert_eq!(p.vector_words(0x28), [7, 8]);
    for offset in [0x54, 0x60, 0x6c, 0xa0] {
        assert!(p.vector_words(offset).is_empty());
    }
    for (offset, word) in [
        (0x50, 1000),
        (0x78, 0x3fc00000),
        (0x7c, 0x42000000),
        (0x80, 0),
        (0x84, 0),
        (0x88, 1),
        (0x8c, 1),
        (0x90, 10),
        (0x94, 20),
        (0x98, 0x3dcccccd),
        (0x9c, 0),
    ] {
        assert_eq!(p.word(offset), word, "offset {offset:x}");
    }
    assert_eq!(
        f.trace
            .iter()
            .filter(|op| matches!(op, Primitive::ReleasePlacement(Token(800))))
            .count(),
        2
    );
}

#[test]
fn ti_v13_v14_v15_v23_vectors_and_partial_false() {
    let (mut owner, mut f) = fresh();
    f.acquired = true;
    f.properties.insert(
        0xe974eaa4,
        Property::Vector {
            words: vec![0x11, 0x22],
            success: true,
        },
    );
    owner.load_placement(0, &mut f).unwrap();
    assert_eq!(owner.record(0).placement().vector_words(0x60), [0, 0]);
    assert_eq!(owner.record(0).placement().vector_words(0x6c), [0x41200000, 0x41200000]);
    f.properties.insert(
        0xe974eaa5,
        Property::Vector {
            words: vec![],
            success: true,
        },
    );
    f.properties.insert(
        0xe974eaa6,
        Property::Vector {
            words: vec![0x40e00000],
            success: true,
        },
    );
    owner.load_placement(0, &mut f).unwrap();
    assert!(owner.record(0).placement().vector_words(0x60).is_empty());
    assert_eq!(owner.record(0).placement().vector_words(0x6c), [0x40e00000]);
    f.properties.insert(
        0xe974eaa5,
        Property::Vector {
            words: vec![99],
            success: false,
        },
    );
    owner.load_placement(0, &mut f).unwrap();
    assert_eq!(owner.record(0).placement().vector_words(0x60), [99, 0, 0]);
    f.properties.insert(
        0xe974eaa4,
        Property::Vector {
            words: vec![],
            success: true,
        },
    );
    owner.load_placement(0, &mut f).unwrap();
    assert!(owner.record(0).placement().vector_words(0x54).is_empty());
    assert_eq!(owner.record(0).placement().vector_words(0x60), [99]);
}

#[test]
fn ti_v16_v19_v20_angle_conversion_and_cross_field_conflict() {
    let (mut owner, mut f) = fresh();
    f.acquired = true;
    for id in [0x879d12e7, 0x279d1707, 0x07b36c71, 0xc804dd87, 0xc804dd88] {
        f.properties.insert(
            id,
            Property::Scalar {
                write: Some(30f32.to_bits()),
                success: true,
            },
        );
    }
    f.properties.insert(
        0x4a2d49b9,
        Property::Scalar {
            write: Some(77),
            success: true,
        },
    );
    f.properties.insert(
        0xea86e3b5,
        Property::Scalar {
            write: Some(91),
            success: false,
        },
    );
    owner.load_placement(0, &mut f).unwrap();
    for offset in [0, 4, 8, 0x18, 0x1c] {
        assert_eq!(owner.record(0).placement().word(offset), 0x3f060a92);
    }
    assert_eq!(owner.record(0).placement().word(0x8c), 1);
    assert_eq!(owner.record(0).placement().word(0x9c), 91);
    let before = owner.record(0).placement().clone();
    f.acquired = false;
    let releases = f
        .trace
        .iter()
        .filter(|op| matches!(op, Primitive::ReleasePlacement(_)))
        .count();
    assert!(!owner.load_placement(0, &mut f).unwrap());
    assert_eq!(owner.record(0).placement(), &before);
    assert_eq!(
        releases,
        f.trace
            .iter()
            .filter(|op| matches!(op, Primitive::ReleasePlacement(_)))
            .count()
    );
    f.acquired = true;
    f.properties.clear();
    owner.load_placement(0, &mut f).unwrap();
    assert_eq!(owner.record(0).placement().word(0), 0x3c15b9be);
    assert!(matches!(
        owner.derive(0),
        Err(Unsupported::NonzeroDerivation {
            source_offset: 0,
            completed_stores: 0
        })
    ));
}

#[test]
fn ti_v21_v22_v24_v25_v26_v27_partial_foreign_and_unsupported() {
    let (mut owner, mut f) = fresh();
    owner.load_evaluator(0, 0, &mut f).unwrap();
    owner.load_evaluator(0, 0, &mut f).unwrap();
    assert_eq!(owner.record(0).evaluator_entries(), &[Some(Token(1)), Some(Token(2))]);
    f.bridge = vec![Token(40), Token(40)];
    owner.load_bridges(0, &mut f).unwrap();
    owner.load_bridges(0, &mut f).unwrap();
    assert_eq!(
        owner.record(0).bridge_entries(),
        &[Token(40), Token(40), Token(40), Token(40)]
    );
    f.trace.clear();
    owner.load_verifiers(&mut f).unwrap();
    owner.load_conflicts(&mut f).unwrap();
    assert_eq!(
        f.trace,
        [
            Primitive::ClearVerifierStore,
            Primitive::EnumerateVerifiers,
            Primitive::EnumerateConflicts
        ]
    );
    f.null_evaluator = true;
    f.trace.clear();
    assert_eq!(owner.load_evaluator(0, 1, &mut f), Err(Unsupported::NullEvaluator));
    assert_eq!(
        f.trace,
        [
            Primitive::AllocateEvaluator { record: 0, bytes: 16 },
            Primitive::AddEvaluatorRules {
                evaluator: None,
                record: 0,
                callsite: 1
            }
        ]
    );
    assert_eq!(owner.record(0).evaluator_entries().len(), 2);
    f.null_evaluator = false;
    f.fail = Some(Primitive::BridgeEntry { record: 10, index: 0 });
    assert_eq!(owner.reload(&mut f), Err(Unsupported::NonreturningPrimitive));
    assert_eq!(owner.record(0).loaded(), 1);
    assert_eq!(owner.record(10).loaded(), 0);
    assert_eq!(owner.record(11).loaded(), 0);
}

#[test]
fn ti_v04_nonempty_placement_backings_survive_clear_then_free_in_field_order() {
    let (mut owner, mut f) = fresh();
    f.acquired = true;
    for id in [0xc804dd92, 0xe974eaa4, 0xe974eaa5, 0xe974eaa6, 0xe974eaa7] {
        f.properties.insert(
            id,
            Property::Vector {
                words: vec![7],
                success: true,
            },
        );
    }
    owner.load_placement(0, &mut f).unwrap();
    owner.load_evaluator(0, 0, &mut f).unwrap();
    f.bridge = vec![Token(20)];
    owner.load_bridges(0, &mut f).unwrap();
    let p = owner.record(0).placement().clone();
    let backing = owner.record(0).backing_identities();
    owner.clear(0, &mut f).unwrap();
    assert_eq!(owner.record(0).placement(), &p);
    assert_eq!(owner.record(0).backing_identities(), backing);
    f.trace.clear();
    owner.destroy(&mut f).unwrap();
    assert_eq!(
        &f.trace[f.trace.len() - 8..],
        &[
            Primitive::Free(Storage::Logical { record: 0, field: 0xa0 }),
            Primitive::Free(Storage::Logical { record: 0, field: 0x6c }),
            Primitive::Free(Storage::Logical { record: 0, field: 0x60 }),
            Primitive::Free(Storage::Logical { record: 0, field: 0x54 }),
            Primitive::Free(Storage::Logical { record: 0, field: 0x28 }),
            Primitive::Free(Storage::Foreign(Token(900))),
            Primitive::Free(Storage::Logical { record: 0, field: 0x30 }),
            Primitive::Free(Storage::Logical { record: 0, field: 0x24 }),
        ]
    );
    assert_eq!(owner.clear(0, &mut f), Err(Unsupported::Destroyed));
}

#[test]
fn ti_v16_v18_v20_v21_getter_order_plane_and_typed_geometry_view() {
    let (mut owner, mut f) = fresh();
    f.acquired = true;
    for (id, word) in [
        (0x879d12e7, 0x42b40000),
        (0xc804dd89, 0x420c0000),
        (0x4a2d49b9, 9),
        (0xea86e3b5, 77),
    ] {
        f.properties.insert(
            id,
            Property::Scalar {
                write: Some(word),
                success: true,
            },
        );
    }
    f.properties.insert(
        0x4a27124f,
        Property::Scalar {
            write: None,
            success: false,
        },
    );
    owner.load_placement(0, &mut f).unwrap();
    let p = owner.record(0).placement();
    assert_eq!(p.word(0), 0x3fc90fdb);
    assert_eq!(p.word(0x20), 0x420c0000);
    assert_eq!(p.word(0x8c), 9);
    assert_eq!(p.word(0x9c), 77);
    assert_eq!(p.word(0x84), 0);
    assert_eq!(p.degree_view().max_slope, 90.0);
    let ids = f
        .trace
        .iter()
        .filter_map(|op| match op {
            Primitive::Property { id, .. } => Some(*id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        [
            0x879d12e7, 0x279d1707, 0x07b36c71, 0xc7b36ca4, 0xc7b36ca5, 0xc804dd86, 0xc804dd87, 0xc804dd88, 0xc804dd89,
            0xc804dd90, 0xc804dd9b, 0xc804dd92, 0xc804dd93, 0xc804dd94, 0xc804dd95, 0xc804dd96, 0xc804dd97, 0xc804dd98,
            0xc804dd99, 0xe974eaa4, 0xe974eaa5, 0xe974eaa6, 0xe974eaa7, 0x4a270db0, 0x4a270e94, 0x4a271248, 0x4a27124f,
            0x4a275748, 0x4a2d49b9, 0x8a2d49ea, 0x8a2d49f0, 0x6a2d49f4, 0xea86e3b5
        ]
    );
    f.properties.clear();
    f.properties.insert(
        0x4a2d49b9,
        Property::Scalar {
            write: Some(9),
            success: true,
        },
    );
    owner.load_placement(0, &mut f).unwrap();
    assert_eq!(owner.record(0).placement().word(0x8c), 1);
    assert_eq!(owner.record(0).placement().word(0x9c), 77);
}

#[test]
fn ti_v07_v08_ti1_ti3_partial_stores_never_claim_publication() {
    let (mut owner, mut f) = fresh();
    f.acquired = true;
    f.properties.insert(
        0x07b36c71,
        Property::Scalar {
            write: Some(0x41f00000),
            success: true,
        },
    );
    assert_eq!(
        owner.reload(&mut f),
        Err(Unsupported::NonzeroDerivation {
            source_offset: 8,
            completed_stores: 2
        })
    );
    assert_eq!(
        owner.record(0).derived_words(),
        [0, 0, 0x41200000, 0x40e24630, 0x41a00000]
    );
    assert_eq!(owner.record(0).loaded(), 0);
    f.properties.clear();
    f.properties.insert(
        0x07b36c71,
        Property::Scalar {
            write: Some(0),
            success: true,
        },
    );
    owner.reload(&mut f).unwrap();
    f.properties.insert(0x07b36c71, Property::UnsupportedVariant);
    assert_eq!(owner.reload(&mut f), Err(Unsupported::PropertyVariant));
    assert!(owner.records().iter().all(|r| r.loaded() == 1));
    assert_eq!(owner.record(0).evaluator_entries().len(), 2);
}

#[test]
fn ti_v09_all_literal_metadata_and_widths() {
    let (mut owner, mut f) = fresh();
    owner.reload(&mut f).unwrap();
    let expected = [
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
    let expected_widths = [
        (1, 1),
        (1, 1),
        (2, 2),
        (1, 1),
        (1, 1),
        (1, 2),
        (2, 2),
        (1, 1),
        (1, 1),
        (1, 1),
        (1, 1),
        (1, 1),
        (2, 2),
    ];
    for record in 0..13 {
        assert_eq!(owner.record(record).metadata(), expected[record]);
        assert_eq!(owner.record(record).widths(), expected_widths[record]);
    }
    let view = owner.record(0).geometry_view(0);
    assert_eq!(view.slope_rise.to_bits(), 0);
    assert_eq!(view.diagonal_slope_rise.to_bits(), 0);
    assert_eq!(view.params.max_slope, 0.0);
}

#[test]
fn ti_v24_v25_returning_failure_keeps_each_raw_callback_effect() {
    let (mut owner, mut f) = fresh();
    f.bridge = vec![Token(11)];
    owner.load_bridges(0, &mut f).unwrap();
    assert_eq!(owner.record(0).bridge_entries(), &[Token(11)]);
    let end = f
        .trace
        .iter()
        .position(|op| *op == Primitive::EndBridgeEnumeration { record: 0 })
        .unwrap();
    assert_eq!(
        &f.trace[end..],
        &[
            Primitive::EndBridgeEnumeration { record: 0 },
            Primitive::DestroyBridgeSectionString { record: 0 },
            Primitive::SpecializeBridges { record: 0, width: 0 },
        ]
    );
    f.support = vec![SupportEntry {
        bucket: 2,
        position: 0,
        node: Token(30),
        allocation_14: None,
        allocation_8: None,
    }];
    owner.load_supports(0, &mut f).unwrap();
    assert_eq!(owner.record(0).support_count(), 1);
    f.trace.clear();
    owner.clear(0, &mut f).unwrap();
    assert_eq!(
        f.trace,
        [
            Primitive::DestroyBridge(Token(11)),
            Primitive::Free(Storage::Foreign(Token(30)))
        ]
    );
}

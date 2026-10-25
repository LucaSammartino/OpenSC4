// SPDX-License-Identifier: GPL-3.0-or-later
// VW19 vectors run through both logical and opaque-reference record adapters.
fn load_pairs(v: &mut View3DWin, h: &mut Host, opaque: bool, pairs: Vec<(u32, u32)>) {
    try_load_pairs(v, h, opaque, View3DSave::header(), pairs).unwrap();
    assert!(!h.ops.iter().any(|op| matches!(op, Op::Send(0xAA375FE8, _, _))));
}

fn try_load_pairs(
    v: &mut View3DWin,
    h: &mut Host,
    opaque: bool,
    header: (u32, u32, u32, u32),
    pairs: Vec<(u32, u32)>,
) -> Result<(), String> {
    if opaque {
        v.load_object_record(
            h,
            &ObjectRecord {
                header,
                identities: [None; 4],
                triggers: pairs,
            },
        )
    } else {
        let mut record = v.save_record();
        record.header.group = header.0;
        record.header.type_id = header.1;
        record.header.instance = header.2;
        record.header.version = header.3;
        record.triggers = pairs;
        v.load_record(&record)
    }
}

#[test]
fn trigger_load_rejects_bad_headers_before_any_mutation_or_reference_resolution() {
    let (group, kind, instance, version) = View3DSave::header();
    for opaque in [false, true] {
        for header in [
            (group ^ 1, kind, instance, version),
            (group, kind ^ 1, instance, version),
            (group, kind, instance ^ 1, version),
            (group, kind, instance, 5),
            (group, kind, instance, u32::MAX),
        ] {
            let mut v = view();
            let mut h = Host::default();
            *v.owner.view.triggers.entry(1, || 7) = 7;
            v.owner.lifecycle.objects.insert(0xF0, 71);
            let before = v.save_record();
            assert!(try_load_pairs(&mut v, &mut h, opaque, header, vec![(1, 0), (194, 9)]).is_err());
            assert_eq!(v.save_record(), before);
            assert_eq!(v.owner.view.triggers.bucket_count(), 193);
            assert!(v.owner.view.not_loaded);
            assert_eq!(v.owner.lifecycle.objects.get(&0xF0), Some(&71));
            assert!(h.reference_calls.is_empty());
            assert!(h.ops.is_empty());
        }
    }
}

#[test]
fn trigger_load_accepts_every_legacy_version_and_sends_no_notifications() {
    let (group, kind, instance, _) = View3DSave::header();
    for opaque in [false, true] {
        for version in 0..5 {
            let mut v = view();
            let mut h = Host::default();
            let header = (group, kind, instance, version);
            try_load_pairs(&mut v, &mut h, opaque, header, vec![(1, 9), (194, 7), (1, 0)]).unwrap();
            assert_eq!(v.save_record().triggers, [(194, 7), (1, 0)]);
            assert!(!v.owner.view.not_loaded);
            assert!(h.ops.is_empty());
        }
    }
}

#[test]
fn vw_t40_trigger_collision_round_trip() {
    for opaque in [false, true] {
        let mut v = view();
        let mut h = Host::default();
        load_pairs(&mut v, &mut h, opaque, vec![(194, 7), (1, 9)]);
        assert_eq!(v.save_record().triggers, [(1, 9), (194, 7)]);
        let mut loaded = view();
        load_pairs(&mut loaded, &mut h, opaque, v.save_record().triggers);
        assert_eq!(loaded.save_record().triggers, [(194, 7), (1, 9)]);
    }
}

#[test]
fn vw_t41_trigger_duplicate_overwrite() {
    for opaque in [false, true] {
        let mut v = view();
        let mut h = Host::default();
        load_pairs(&mut v, &mut h, opaque, vec![(1, 2), (194, 3), (1, 0), (194, u32::MAX)]);
        assert_eq!(v.save_record().triggers, [(194, u32::MAX), (1, 0)]);
        assert!(v.global_trigger_state(1));
        assert!(v.global_trigger_state(194));
    }
}

#[test]
fn vw_t42_trigger_load_preserves_existing_and_empty_stream() {
    for opaque in [false, true] {
        let mut v = view();
        let mut h = Host::default();
        for (key, count) in [(1, 5), (194, 8), (2, 6)] {
            *v.owner.view.triggers.entry(key, || count) = count;
        }
        load_pairs(&mut v, &mut h, opaque, vec![(1, 4)]);
        let expected = [(194, 8), (1, 4), (2, 6)];
        assert_eq!(v.save_record().triggers, expected);
        load_pairs(&mut v, &mut h, opaque, vec![]);
        assert_eq!(v.save_record().triggers, expected);
        assert_eq!(v.owner.view.triggers.bucket_count(), 193);
    }
}

#[test]
fn vw_t43_trigger_pair_count_does_not_reserve_or_grow_for_duplicate() {
    for opaque in [false, true] {
        let mut v = view();
        let mut h = Host::default();
        let pairs = (1..=193).map(|key| (key, 1)).chain([(1, 0)]).collect();
        load_pairs(&mut v, &mut h, opaque, pairs);
        assert_eq!(v.owner.view.triggers.bucket_count(), 193);
        let expected: Vec<_> = [(193, 1), (1, 0)]
            .into_iter()
            .chain((2..=192).map(|key| (key, 1)))
            .collect();
        assert_eq!(v.save_record().triggers, expected);
    }
}

#[test]
fn vw_t44_trigger_load_grows_only_when_inserting_new_key() {
    for opaque in [false, true] {
        let mut v = view();
        let mut h = Host::default();
        let pairs = [(390, 1), (1, 1)]
            .into_iter()
            .chain((2..=193).map(|key| (key, 1)))
            .collect();
        load_pairs(&mut v, &mut h, opaque, pairs);
        assert_eq!(v.owner.view.triggers.bucket_count(), 389);
        let expected: Vec<_> = [(390, 1), (1, 1)]
            .into_iter()
            .chain((2..=193).map(|key| (key, 1)))
            .collect();
        assert_eq!(v.save_record().triggers, expected);
    }
}

#[test]
fn trigger_load_uses_unsigned_keys_and_preserves_zero_counts_in_both_save_adapters() {
    for opaque in [false, true] {
        let mut v = view();
        let mut h = Host::default();
        load_pairs(
            &mut v,
            &mut h,
            opaque,
            vec![(u32::MAX, u32::MAX), (107, 3), (0x8000_0000, 7), (0, 9), (u32::MAX, 0)],
        );
        // Unsigned modulo 193 puts these keys in buckets 0, 54 and 107.
        // The duplicate overwrites the tail of bucket 107 without moving it.
        let expected = [(0, 9), (0x8000_0000, 7), (107, 3), (u32::MAX, 0)];
        assert_eq!(v.owner.view.triggers.bucket_count(), 193);
        assert_eq!(v.save_record().triggers, expected);
        assert_eq!(v.save_object_record(&mut h, true).unwrap().triggers, expected);
        for key in [0, 107, 0x8000_0000, u32::MAX] {
            assert!(v.global_trigger_state(key));
        }
        assert!(!v.global_trigger_state(1));
        assert!(h.ops.is_empty());
    }
}

#[test]
fn trigger_load_at_grown_capacity_preserves_nodes_and_unmentioned_keys() {
    for opaque in [false, true] {
        let mut v = view();
        let mut h = Host::default();
        let pairs = [(390, 7), (1, 9)]
            .into_iter()
            .chain((2..=388).map(|key| (key, key)))
            .collect();
        load_pairs(&mut v, &mut h, opaque, pairs);
        assert_eq!(v.owner.view.triggers.bucket_count(), 389);
        let updates = vec![(1, 0); 1024].into_iter().chain([(390, u32::MAX)]).collect();
        load_pairs(&mut v, &mut h, opaque, updates);
        let expected: Vec<_> = [(390, u32::MAX), (1, 0)]
            .into_iter()
            .chain((2..=388).map(|key| (key, key)))
            .collect();
        assert_eq!(v.owner.view.triggers.bucket_count(), 389);
        assert_eq!(v.save_record().triggers, expected);
        assert_eq!(v.save_object_record(&mut h, true).unwrap().triggers, expected);
        load_pairs(&mut v, &mut h, opaque, vec![]);
        assert_eq!(v.owner.view.triggers.bucket_count(), 389);
        assert_eq!(v.save_record().triggers, expected);
        assert!(v.global_trigger_state(1));
        assert!(h.ops.is_empty());
    }
}

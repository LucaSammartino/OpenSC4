// SPDX-License-Identifier: GPL-3.0-or-later
// VW19 vectors run through both logical and opaque-reference record adapters.
fn load_pairs(v: &mut View3DWin, h: &mut Host, opaque: bool, pairs: Vec<(u32, u32)>) {
    if opaque {
        v.load_object_record(
            h,
            &ObjectRecord {
                header: View3DSave::header(),
                identities: [None; 4],
                triggers: pairs,
            },
        )
        .unwrap();
    } else {
        let mut record = v.save_record();
        record.triggers = pairs;
        v.load_record(&record).unwrap();
    }
    assert!(!h.ops.iter().any(|op| matches!(op, Op::Send(0xAA375FE8, _, _))));
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

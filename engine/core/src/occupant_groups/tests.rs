// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::sync::Arc;

fn group(id: u32, class_id: u32, source: u32) -> Arc<OccupantGroupTemplate> {
    let g = Arc::new(OccupantGroupTemplate::new(0));
    g.edit(|s| {
        s.group_id = id;
        s.class_id = class_id;
        s.source_building = source;
    });
    g
}

fn effect(fx: &str, flags: u32) -> KeyEffect {
    KeyEffect {
        fx: fx.into(),
        bone: String::new(),
        controller: String::new(),
        flags,
    }
}

#[test]
fn constructor_published_state() {
    let g = OccupantGroupTemplate::new(3);
    g.inspect(|s| {
        assert!(s.parent.upgrade().is_none());
        assert_eq!((s.group_id, s.class_id, s.csi_image, s.source_building), (0, 0, 0, 0));
        assert_eq!(s.percentage, 1.0);
        assert_eq!((s.own_ref, s.parent_ref), (-2, -2));
        assert_eq!(s.kph, None);
        assert!(s.controllers.is_empty() && s.models.is_empty() && s.anims.is_empty());
        assert!(s.trailer_ids.is_empty() && s.trailer_names.is_empty() && s.key_effects.is_empty());
        assert!(s.drive_params.is_none() && s.player_drive_params.is_none());
    });
    assert_eq!(g.draws.load(Ordering::Relaxed), 0);
    assert!(g.get_group_id_variant().is_empty());
}

#[test]
fn chain_queries_include_zero_and_keep_self_first() {
    let r = group(1, 0, 7);
    let c = group(0, 5, 0);
    let g = group(3, 0, 0);
    c.edit(|s| s.parent = Arc::downgrade(&r));
    g.edit(|s| s.parent = Arc::downgrade(&c));
    for id in [3, 0, 1] {
        assert!(g.is_occupant_group(id));
    }
    assert!(!g.is_occupant_group(7));
    assert_eq!(g.get_class_id(), 5);
    assert_eq!(g.get_source_building_group(), 7);
    assert_eq!(OccupantGroupTemplate::get_group_hierarchy_depth(Some(&g)), 3);
    assert_eq!(OccupantGroupTemplate::get_group_hierarchy_depth(Some(&r)), 1);
    assert_eq!(OccupantGroupTemplate::get_group_hierarchy_depth(None), 0);
    assert_eq!(g.get_group_id_variant(), [3, 1]);
    assert_eq!(r.get_class_id(), 0);
    g.edit(|s| {
        s.class_id = 9;
        s.source_building = 10;
    });
    assert_eq!(g.get_class_id(), 9);
    assert_eq!(g.get_source_building_group(), 10);
}

#[test]
fn cache_is_stale_but_empty_cache_rebuilds_and_duplicates_survive() {
    let p = group(0, 0, 0);
    let g = group(0, 0, 0);
    g.edit(|s| s.parent = Arc::downgrade(&p));
    assert!(g.get_group_id_variant().is_empty());
    p.edit(|s| s.group_id = 4);
    g.edit(|s| s.group_id = 4);
    let mut copy = g.get_group_id_variant();
    assert_eq!(copy, [4, 4]);
    copy.clear();
    g.edit(|s| {
        s.group_id = 8;
        s.parent = Weak::new();
    });
    assert_eq!(g.get_group_id_variant(), [4, 4]);
    assert!(!g.is_occupant_group(4));
}

#[test]
fn filter_shortcuts_never_draw_and_ignore_ancestors() {
    let p = group(2, 0, 0);
    let g = group(5, 0, 0);
    g.edit(|s| s.parent = Arc::downgrade(&p));
    for ids in [None, Some(&[][..]), Some(&[2][..])] {
        assert!(!g.is_occupant_included_with_draw(ids, || panic!("draw on failed match")));
    }
    for pct in [1.0, 1.1, f32::INFINITY] {
        g.edit(|s| s.percentage = pct);
        assert!(g.is_occupant_included_with_draw(Some(&[2, 5]), || panic!("draw at >=1")));
    }
    g.edit(|s| {
        s.group_id = 0;
        s.percentage = 0.5;
    });
    assert!(!g.is_occupant_included_with_draw(Some(&[0]), || panic!("draw for zero ID")));
    assert!(g.is_occupant_type_included(u32::MAX));
    assert!(g.is_property_holder_included(&()));
}

#[test]
fn filter_exact_comparisons_no_clamp_or_premature_f32_rounding() {
    let g = group(5, 0, 0);
    for (pct, draw, expected) in [
        (0.5, 0.5, true),
        (0.5, 0.5 + f64::EPSILON, false),
        (0.5, 0.5 - f64::EPSILON, true),
        (0.0, 0.0, true),
        (-0.0, 0.0, true),
        (-0.01, 0.0, false),
        (f32::NEG_INFINITY, 0.0, false),
        (f32::NAN, 0.5, false),
        (0.5, f64::NAN, false),
    ] {
        g.edit(|s| s.percentage = pct);
        let mut calls = 0;
        assert_eq!(
            g.is_occupant_included_with_draw(Some(&[5]), || {
                calls += 1;
                draw
            }),
            expected
        );
        assert_eq!(calls, 1);
    }
}

struct Properties(Option<Vec<u32>>, AtomicU32);
impl OccupantProperties for Properties {
    fn u32_property(&self, id: u32) -> Option<Vec<u32>> {
        assert_eq!(id, OCCUPANT_GROUPS_PROPERTY);
        self.1.fetch_add(1, Ordering::Relaxed);
        self.0.clone()
    }
}

#[test]
fn temporary_seed_sequence_counts_only_actual_draws() {
    let g = group(5, 0, 0);
    let p = Properties(Some(vec![5]), AtomicU32::new(0));
    assert!(g.is_occupant_included(&p));
    assert_eq!(g.draws.load(Ordering::Relaxed), 0);
    g.edit(|s| s.percentage = 0.5);
    for n in 0..4 {
        // Independent expected first multiplicative states for session 0:
        // (A9C74DC9 XOR n) * 278DDE6D mod 2^32, foundations §6.1.
        let state: u32 = [0x96056C95, 0x6E778E28, 0xE521296F, 0xBD934B02][n];
        let x = state as i32 as f64 / 4_294_967_296.0 + 0.5;
        assert_eq!(g.is_occupant_included(&p), 0.5 >= x);
    }
    assert_eq!(g.draws.load(Ordering::Relaxed), 4);
    assert_eq!(p.1.load(Ordering::Relaxed), 5);
}

#[test]
fn key_copy_is_deep_preserves_flags_and_failure_output() {
    let g = group(3, 0, 0);
    let mut original = effect("a", 0xFEDC_BA98);
    original.bone = "bone".into();
    original.controller = "controller".into();
    g.edit(|s| {
        s.key_effects.insert(7, original.clone());
    });
    let mut out = effect("sentinel", 0xABCD);
    assert!(!g.get_key_effect(8, &mut out));
    assert_eq!(out, effect("sentinel", 0xABCD));
    assert!(g.get_key_effect(0xFFFF_FF07, &mut out));
    assert_eq!(out, original);
    out.fx.push('!');
    out.bone.clear();
    assert!(g.get_key_effect(7, &mut out));
    assert_eq!(out, original);
    g.clear_key_effects();
    assert!(!g.get_key_effect(7, &mut out));
    assert_eq!(out, original);
}

struct Tracked(&'static str, Arc<Mutex<Vec<&'static str>>>);
impl ParameterBlock for Tracked {}
impl Drop for Tracked {
    fn drop(&mut self) {
        self.1.lock().unwrap().push(self.0);
    }
}

fn tracked(log: &Arc<Mutex<Vec<&'static str>>>) -> OccupantGroupTemplate {
    let g = OccupantGroupTemplate::new(1);
    g.edit(|s| {
        s.drive_params = Some(Box::new(Tracked("drive", log.clone())));
        s.player_drive_params = Some(Box::new(Tracked("player", log.clone())));
        s.controllers.push(ControllerHandle(123));
        s.models.push(ResKey {
            type_id: 1,
            group: 2,
            instance: 3,
        });
        s.trailer_ids.push(4);
        s.trailer_names.push("trailer".into());
        s.key_effects.insert(1, effect("owned", 0));
    });
    g
}

#[test]
fn destruction_order_and_deleting_flag_bit_zero() {
    for flags in [0, 2, 1, 3, u32::MAX] {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut allocation = Some(Box::new(Some(tracked(&log))));
        deleting_destroy(&mut allocation, flags);
        assert_eq!(*log.lock().unwrap(), ["drive", "player"]);
        assert_eq!(allocation.is_none(), flags & 1 != 0);
        if let Some(shell) = &allocation {
            assert!(shell.is_none());
        }
        drop(allocation);
        assert_eq!(*log.lock().unwrap(), ["drive", "player"]);
    }
}

#[test]
fn release_zero_one_two_unsigned_and_addref_wrap() {
    for initial in [0, 1, 2, 0x8000_0000, u32::MAX] {
        let log = Arc::new(Mutex::new(Vec::new()));
        let g = tracked(&log);
        g.references.store(initial, Ordering::Relaxed);
        let result = g.release();
        if initial <= 1 {
            assert_eq!(result, 0);
            assert_eq!(*log.lock().unwrap(), ["drive", "player"]);
            g.inspect(|s| {
                assert_eq!(s.controllers.capacity(), 0);
                assert_eq!(s.models.capacity(), 0);
                assert_eq!(s.trailer_ids.capacity(), 0);
                assert_eq!(s.trailer_names.capacity(), 0);
            });
        } else {
            assert_eq!(result, initial - 1);
            assert!(log.lock().unwrap().is_empty());
        }
        drop(g);
        assert_eq!(*log.lock().unwrap(), ["drive", "player"]);
    }
    let g = OccupantGroupTemplate::new(0);
    assert_eq!(g.add_ref(), 1);
    assert_eq!(g.add_ref(), 2);
    g.references.store(u32::MAX, Ordering::Relaxed);
    assert_eq!(g.add_ref(), 0);
}

#[test]
fn parent_links_and_controllers_do_not_own_references() {
    let p = group(1, 0, 0);
    let g = group(2, 0, 0);
    g.edit(|s| s.parent = Arc::downgrade(&p));
    assert_eq!(Arc::strong_count(&p), 1);
    drop(g);
    assert_eq!(Arc::strong_count(&p), 1);
    assert_eq!(p.references.load(Ordering::Relaxed), 0);
}

#[test]
fn concurrent_releases_have_unique_counts_and_destroy_once() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let g = Arc::new(tracked(&log));
    for expected in 1..=32 {
        assert_eq!(g.add_ref(), expected);
    }
    let threads: Vec<_> = (0..32)
        .map(|_| {
            let g = g.clone();
            std::thread::spawn(move || g.release())
        })
        .collect();
    let mut counts: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    counts.sort_unstable();
    assert_eq!(counts, (0..32).collect::<Vec<_>>());
    assert_eq!(*log.lock().unwrap(), ["drive", "player"]);
    drop(g);
    assert_eq!(*log.lock().unwrap(), ["drive", "player"]);
}

#[test]
fn getters_are_safe_on_a_second_thread() {
    let g = group(3, 4, 5);
    g.edit(|s| {
        s.percentage = 0.5;
        s.key_effects.insert(7, effect("a", 0x11));
    });
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let g = g.clone();
            std::thread::spawn(move || {
                for _ in 0..100 {
                    assert_eq!(g.get_group_id_variant(), [3]);
                    assert_eq!(g.get_class_id(), 4);
                    let mut out = effect("", 0);
                    assert!(g.get_key_effect(7, &mut out));
                    g.is_occupant_included(&Properties(Some(vec![3]), AtomicU32::new(0)));
                }
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
    assert_eq!(g.draws.load(Ordering::Relaxed), 400);
}

#[test]
fn published_interface_identity_and_base_record_only() {
    use crate::stream::{ByteReader, ByteWriter};
    assert_eq!(interface_view(1), Some(InterfaceView::Filter));
    assert_eq!(interface_view(0x46A8_0026), Some(InterfaceView::Filter));
    assert_eq!(interface_view(0xE4FD_A3D4), Some(InterfaceView::Serializable));
    for iid in [0, GZCLSID, u32::MAX] {
        assert_eq!(interface_view(iid), None);
    }
    let g = group(9, 10, 11);
    assert_eq!(g.get_gzclsid(), GZCLSID);
    let mut out = ByteWriter::new();
    assert!(g.write_base_record(&mut out));
    assert_eq!(out.bytes(), [3, 0]);
    for bytes in [&[][..], &[3][..], &[2, 0][..], &[0, 3][..]] {
        assert!(!g.read_base_record(&mut ByteReader::new(bytes)));
    }
    assert!(g.read_base_record(&mut ByteReader::new(&[3, 0])));
    assert_eq!(g.get_class_id(), 10);
    assert_eq!(g.get_group_id_variant(), [9]);
}

#[test]
fn temporary_seed_fixups_are_used_and_shortcuts_do_not_consume_them() {
    for (session, expected) in [(GZCLSID, false), (!GZCLSID, true)] {
        let g = OccupantGroupTemplate::new(session);
        g.edit(|s| {
            s.group_id = 5;
            s.percentage = 0.5;
        });
        assert!(!g.is_occupant_included(&Properties(None, AtomicU32::new(0))));
        let p = Properties(Some(vec![5]), AtomicU32::new(0));
        // derive(0) -> 1 produces 0x278DDE6D (>0.5); derive(FFFFFFFF)
        // -> FFFFFFFE produces 0xB0E44326 (<0.5). No helper computes expected.
        assert_eq!(g.is_occupant_included(&p), expected);
        assert_eq!(g.draws.load(Ordering::Relaxed), 1);
    }
    let g = OccupantGroupTemplate::new(0);
    let p = Properties(Some(vec![0]), AtomicU32::new(0));
    assert!(!g.is_occupant_included(&p));
    assert_eq!(p.1.load(Ordering::Relaxed), 1);
    assert_eq!(g.draws.load(Ordering::Relaxed), 0);
}

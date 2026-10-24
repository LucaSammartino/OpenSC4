// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec §11.20's 23 new vectors and §11.19 primary-loader vectors.
use super::tests::boat;
use super::*;
use crate::automata::Vec3;
use crate::stream::ResKey;
use std::collections::{BTreeMap, VecDeque};

fn key(type_id: u32, instance: u32) -> ResKey {
    ResKey {
        type_id,
        group: 7,
        instance,
    }
}
fn mesh() -> ResKey {
    key(0x29A5_D1EC, 8)
}
fn sprite() -> ResKey {
    key(0x5AD0_E817, 9)
}

struct Models {
    calls: Vec<String>,
    lookup: bool,
    creation: bool,
    created: Option<u64>,
    loads: VecDeque<bool>,
    visible: bool,
    set_b0: bool,
    clear_b0: bool,
    transform: Transform,
    written: Vec<Transform>,
    bounds: [Vec3; 2],
    properties: BTreeMap<u32, ResKey>,
    selectors: Option<Vec<u32>>,
}
impl Default for Models {
    fn default() -> Self {
        Self {
            calls: vec![],
            lookup: true,
            creation: true,
            created: Some(20),
            loads: VecDeque::new(),
            visible: true,
            set_b0: false,
            clear_b0: false,
            transform: Transform::default(),
            written: vec![],
            bounds: [[0.0; 3], [2.0, 3.0, 80.0]],
            properties: BTreeMap::new(),
            selectors: None,
        }
    }
}
impl ModelServices for Models {
    fn sprite_lookup(&mut self, k: ResKey, iid: u32, tail: [u32; 2], out: &mut Option<u64>) -> bool {
        assert_eq!(k, sprite());
        assert_eq!(iid, 0x9AD0_E7E3);
        assert_eq!(tail, [0; 2]);
        self.calls.push("lookup".into());
        if self.lookup {
            *out = Some(30);
        }
        self.lookup
    }
    fn sprite_create(&mut self, r: u64, a: u32, out: &mut Option<u64>) -> bool {
        assert_eq!((r, a), (30, 0));
        self.calls.push("create".into());
        if let Some(created) = self.created {
            *out = Some(created);
        }
        self.creation
    }
    fn mesh_construct(&mut self, a: u32) -> u64 {
        assert_eq!(a, 1);
        self.calls.push("construct:1".into());
        10
    }
    fn mesh_load(&mut self, m: u64, _: ResKey) -> bool {
        assert_eq!(m, 10);
        self.calls.push("load".into());
        self.loads.pop_front().unwrap_or(true)
    }
    fn add_ref(&mut self, r: u64) {
        self.calls.push(format!("add:{r}"));
    }
    fn release(&mut self, r: u64) {
        self.calls.push(format!("release:{r}"));
    }
    fn instance(&mut self, m: u64) -> u64 {
        self.calls.push(format!("instance:{m}"));
        m + 100
    }
    fn bounds(&mut self, i: u64) -> [Vec3; 2] {
        self.calls.push(format!("bounds:{i}"));
        self.bounds
    }
    fn read_transform(&mut self, _: u64, out: &mut Transform) {
        assert_eq!(*out, Transform::default());
        self.calls.push("read".into());
        *out = self.transform.clone();
    }
    fn write_transform(&mut self, _: u64, t: &Transform) {
        self.calls.push("write".into());
        self.written.push(t.clone());
    }
    fn associate(&mut self, _: u64) {
        self.calls.push("associate".into());
    }
    fn instance_mode(&mut self, _: u64, mode: u32) {
        self.calls.push(format!("mode:{mode}"));
    }
    fn primary_mode(&mut self, _: u64, a: u32, b: u32) {
        assert_eq!((a, b), (1, 0));
        self.calls.push("primary".into());
    }
    fn update_view(&mut self, _: u64) {
        self.calls.push("view".into());
    }
    fn auxiliary_selectors(&mut self, property: u32) -> Option<Vec<u32>> {
        assert_eq!(property, 0xCC21_420D);
        self.calls.push("selectors".into());
        self.selectors.clone()
    }
    fn property_key(&mut self, property: u32) -> Option<ResKey> {
        self.calls.push(format!("property:{property:08X}"));
        self.properties.get(&property).copied()
    }
    fn visibility(&mut self, f: u32) -> bool {
        assert_eq!(f, 4);
        self.calls.push("visible:4".into());
        self.visible
    }
    fn render_full(&mut self, i: u64) {
        self.calls.push(format!("full:{i}"));
    }
    fn render_partial(&mut self, i: u64, f: u32) {
        assert_eq!(f, 3);
        self.calls.push(format!("partial:{i}:3"));
    }
    fn occupant_update(&mut self, a: u32, flags: &mut u8) {
        assert_eq!(a, 0);
        self.calls.push("occupant:0".into());
        if self.clear_b0 {
            *flags &= !1;
        }
    }
    fn owner70(&mut self, e: bool, flags: &mut u8) {
        assert!(e);
        self.calls.push("70:true".into());
        if self.set_b0 {
            *flags |= 1;
        }
    }
    fn owner71(&mut self, e: bool, _: &mut u8) {
        self.calls.push(format!("71:{e}"));
    }
    fn owner83(&mut self, t: f32) {
        assert_eq!(t, 0.0);
        self.calls.push("83:0".into());
    }
    fn owner106(&mut self) {
        self.calls.push("106".into());
    }
}

#[test]
fn wc10_r01_unsupported_preserves_output() {
    let mut s = Models::default();
    let mut out = Some(99);
    assert!(!acquire_model(key(123, 4), &mut out, &mut s));
    assert_eq!(out, Some(99));
    assert!(s.calls.is_empty());
}
#[test]
fn wc10_r02_sprite_lookup_failure_preserves_output() {
    let mut s = Models {
        lookup: false,
        ..Models::default()
    };
    let mut out = Some(99);
    assert!(!acquire_model(sprite(), &mut out, &mut s));
    assert_eq!(out, Some(99));
    assert_eq!(s.calls, ["lookup"]);
}
#[test]
fn wc10_r03_sprite_creation_releases_only_resource() {
    let mut s = Models {
        creation: false,
        created: None,
        ..Models::default()
    };
    let mut out = Some(99);
    assert!(!acquire_model(sprite(), &mut out, &mut s));
    assert_eq!(out, Some(99));
    assert_eq!(s.calls, ["lookup", "create", "release:30"]);
    s.calls.clear();
    s.creation = true;
    s.created = Some(20);
    assert!(acquire_model(sprite(), &mut out, &mut s));
    assert_eq!(out, Some(20));
    assert_eq!(s.calls, ["lookup", "create", "release:30"]);
}
#[test]
fn wc10_r04_mesh_failure_null_and_success_transfer() {
    let mut s = Models {
        loads: [false, true].into(),
        ..Models::default()
    };
    let mut out = Some(99);
    assert!(!acquire_model(mesh(), &mut out, &mut s));
    assert_eq!(out, None);
    assert_eq!(s.calls, ["construct:1", "add:10", "load", "release:10"]);
    s.calls.clear();
    assert!(acquire_model(mesh(), &mut out, &mut s));
    assert_eq!(out, Some(10));
    assert_eq!(s.calls, ["construct:1", "add:10", "load"]);
}
#[test]
fn wc10_a01_auxiliary_property_mapping_and_untouched_output() {
    let mut b = boat();
    let mut s = Models::default();
    for selector in [1, 2, 3, 0x1234_5678] {
        let mut out = Some(99);
        assert!(!b.load_auxiliary_model(selector, &mut out, &mut s));
        assert_eq!(out, Some(99));
    }
    assert_eq!(
        s.calls,
        [
            "property:09C80417",
            "property:C9E69AF6",
            "property:EC214180",
            "property:12345678"
        ]
    );
    assert!(b.models.entries.is_empty());
}
#[test]
fn wc10_a02_zero_key_clears_output_without_owner_work() {
    let mut b = boat();
    let mut s = Models::default();
    s.properties.insert(1, key(0x29A5_D1EC, 0));
    let mut out = Some(99);
    assert!(!b.load_auxiliary_model(1, &mut out, &mut s)); // 1 maps to the property below.
    assert_eq!(out, Some(99));
    s.calls.clear();
    s.properties.insert(0x09C8_0417, key(0x29A5_D1EC, 0));
    assert!(!b.load_auxiliary_model(1, &mut out, &mut s));
    assert_eq!(out, None);
    assert_eq!(s.calls, ["property:09C80417"]);
}
#[test]
fn wc10_a03_failure_retains_saved_key_and_restores_registration() {
    let mut b = boat();
    b.record.base.occupant.flags |= 2;
    let mut s = Models {
        loads: [false].into(),
        ..Models::default()
    };
    assert_eq!(b.add_model_instance(mesh(), &mut s), None);
    assert_eq!(b.record.base.model, mesh());
    assert_eq!(b.record.base.u2, 8);
    assert_eq!(
        s.calls,
        [
            "71:false",
            "construct:1",
            "add:10",
            "load",
            "release:10",
            "71:true",
            "106"
        ]
    );
    assert!(b.models.entries.is_empty());
}
#[test]
fn wc10_a04_success_order_and_auxiliary_output_reference() {
    let mut b = boat();
    b.record.base.occupant.flags |= 2;
    let mut s = Models::default();
    s.properties.insert(0x09C8_0417, mesh());
    let mut out = None;
    assert!(b.load_auxiliary_model(1, &mut out, &mut s));
    assert_eq!(
        s.calls,
        [
            "property:09C80417",
            "71:false",
            "construct:1",
            "add:10",
            "load",
            "instance:10",
            "associate",
            "mode:1",
            "view",
            "read",
            "write",
            "71:true",
            "106",
            "add:10"
        ]
    );
    assert_eq!(b.models.entries, [Some(10)]);
    assert_eq!(b.record.base.model_keys, [mesh()]);
    s.release(out.unwrap());
    b.remove_models(&mut s);
    assert_eq!(&s.calls[s.calls.len() - 2..], ["release:10", "release:10"]);
}
#[test]
fn wc10_a05_repeated_key_appends_without_overwriting_saved_key() {
    let mut b = boat();
    b.record.base.model = sprite();
    let mut s = Models::default();
    s.properties.insert(0x09C8_0417, mesh());
    for _ in 0..2 {
        assert!(b.load_auxiliary_model(1, &mut None, &mut s));
    }
    assert_eq!(b.record.base.model, sprite());
    assert_eq!(b.models.entries.len(), 2);
    assert_eq!(b.record.base.model_keys, [mesh(), mesh()]);
    assert_eq!(s.calls.iter().filter(|c| *c == "add:10").count(), 4);
}
#[test]
fn wc11_p01_inherited_pose_scale_revision_and_flags() {
    let mut b = boat();
    b.inherited_heading = [0.0, 0.0, 1.0];
    b.inherited_position = [1.0, 2.0, 3.0];
    b.position = [9.0, 8.0, 7.0];
    let mut s = Models::default();
    s.transform.scale = 3.0;
    s.transform.revision = 254;
    s.transform.flags = 0x20;
    b.position_model_instance(110, &mut s);
    assert_eq!(s.calls, ["read", "write"]);
    let t = &s.written[0];
    assert_eq!(t.basis, Transform::default().basis);
    assert_eq!(t.translation, [1.0, 2.0, 3.0]);
    assert_eq!((t.scale, t.revision, t.flags), (3.0, 0, 0x25));
}
#[test]
fn wc11_p02_nonunit_heading_keeps_up_and_forward_lengths() {
    let mut b = boat();
    b.inherited_heading = [3.0, 0.0, 4.0];
    let mut s = Models::default();
    b.position_model_instance(110, &mut s);
    assert_eq!(
        s.written[0].basis,
        [[0.8, -0.0, -0.6], [0.0, 5.0, -0.0], [3.0, 0.0, 4.0]]
    );
}
#[test]
fn wc11_p03_vertical_and_zero_fallback() {
    let mut b = boat();
    let mut s = Models::default();
    for h in [[0.0, -2.0, 0.0], [0.0; 3]] {
        b.inherited_heading = h;
        b.position_model_instance(110, &mut s);
        assert_eq!(s.written.last().unwrap().basis, [[0.0, 0.0, 1.0], [1.0, 0.0, 0.0], h]);
    }
}
#[test]
fn wc11_p04_equal_pose_still_publishes_and_wraps_revision() {
    let mut b = boat();
    b.inherited_heading = [0.0, 0.0, 1.0];
    let mut s = Models::default();
    s.transform.revision = 255;
    s.transform.flags = 0xFF;
    b.position_model_instance(110, &mut s);
    assert_eq!(s.written.len(), 1);
    assert_eq!((s.written[0].revision, s.written[0].flags), (1, 255));
}
#[test]
fn wc11_u01_bounds_precede_visibility_and_event_is_independent() {
    let mut b = boat();
    b.models.entries = vec![Some(10)];
    b.models.event_counter = 101.0;
    let mut s = Models {
        visible: false,
        ..Models::default()
    };
    b.publish_models(true, true, &mut s);
    assert_eq!(s.calls, ["instance:10", "bounds:110", "visible:4"]);
    assert_eq!(b.occupant_bounds, s.bounds);
    assert_eq!(b.models.event_counter, 0.0);
    assert_eq!(b.events.last(), Some(&Event::Notify(9, 0)));
}
fn publication(flags: u8, full: bool, partial: bool) -> Models {
    let mut b = boat();
    b.record.base.occupant.flags = flags;
    b.models.entries = vec![Some(10), None, Some(20)];
    let mut s = Models::default();
    b.publish_models(full, partial, &mut s);
    s
}
#[test]
fn wc11_u02_full_precedes_partial_and_only_loop_skips_null() {
    assert_eq!(
        publication(2, true, true).calls,
        [
            "instance:10",
            "bounds:110",
            "visible:4",
            "instance:10",
            "full:110",
            "instance:20",
            "full:120"
        ]
    );
}
#[test]
fn wc11_u03_partial_then_occupant_update() {
    assert_eq!(
        publication(3, false, true).calls,
        [
            "instance:10",
            "bounds:110",
            "visible:4",
            "instance:10",
            "partial:110:3",
            "instance:20",
            "partial:120:3",
            "occupant:0"
        ]
    );
}
#[test]
fn wc11_u04_no_render_with_both_arguments_false() {
    assert_eq!(
        publication(3, false, false).calls,
        ["instance:10", "bounds:110", "visible:4", "occupant:0"]
    );
}
#[test]
fn wc11_u05_owner70_flag_reread() {
    for set_b0 in [true, false] {
        let mut b = boat();
        b.record.base.occupant.flags = 0;
        let mut s = Models {
            set_b0,
            ..Models::default()
        };
        b.publish_models(false, false, &mut s);
        assert_eq!(
            s.calls,
            if set_b0 {
                vec!["visible:4", "70:true", "71:true"]
            } else {
                vec!["visible:4", "70:true"]
            }
        );
    }
}
#[test]
fn wc11_u06_occupant_update_flag_reread() {
    for clear_b0 in [true, false] {
        let mut b = boat();
        b.record.base.occupant.flags = 1;
        let mut s = Models {
            clear_b0,
            ..Models::default()
        };
        b.publish_models(false, false, &mut s);
        assert_eq!(
            s.calls,
            if clear_b0 {
                vec!["visible:4", "occupant:0"]
            } else {
                vec!["visible:4", "occupant:0", "71:true"]
            }
        );
    }
}
#[test]
fn wc11_u07_strict_counter_and_nan_without_models() {
    for counter in [100.0, f32::from_bits(100.0_f32.to_bits() + 1), f32::NAN] {
        let mut b = boat();
        b.models.event_counter = counter;
        let mut s = Models {
            visible: false,
            ..Models::default()
        };
        b.publish_models(false, false, &mut s);
        assert_eq!(s.calls, ["visible:4"]);
        assert_eq!(b.events.contains(&Event::Notify(9, 0)), counter > 100.0);
        if counter > 100.0 {
            assert_eq!(b.models.event_counter, 0.0);
        } else {
            assert_eq!(b.models.event_counter.to_bits(), counter.to_bits());
        }
    }
}
#[test]
fn wc3_ml01_existing_models_skip_all_loading() {
    let mut b = boat();
    b.models.entries = vec![Some(10)];
    b.record.base.model_keys = vec![mesh(); 3];
    let mut s = Models::default();
    assert!(!b.load_models(&mut s));
    assert_eq!(s.calls, ["83:0", "106"]);
}
#[test]
fn wc3_ml02_empty_keys_do_not_claim_model_success() {
    let mut b = boat();
    let mut s = Models::default();
    assert!(!b.load_models(&mut s));
    assert_eq!(s.calls, ["selectors", "83:0", "106"]);
}
#[test]
fn wc3_ml03_partial_failure_keeps_first_model_and_retry_does_not_resume() {
    let mut b = boat();
    b.record.base.model_keys = vec![mesh(); 2];
    let mut s = Models {
        loads: [true, false].into(),
        ..Models::default()
    };
    assert!(!b.load_models(&mut s));
    assert_eq!(b.models.entries, [Some(10)]);
    assert_eq!(
        s.calls,
        [
            "construct:1",
            "add:10",
            "load",
            "instance:10",
            "bounds:110",
            "primary",
            "read",
            "write",
            "associate",
            "mode:3",
            "view",
            "construct:1",
            "add:10",
            "load",
            "release:10"
        ]
    );
    s.calls.clear();
    assert!(!b.load_models(&mut s));
    assert_eq!(s.calls, ["83:0", "106"]);
}
#[test]
fn wc3_ml04_radius_cap_and_auxiliary_filter_order() {
    let mut b = boat();
    b.record.base.model = mesh();
    let mut s = Models {
        selectors: Some(vec![0, 1, 4, 3, 2, u32::MAX]),
        ..Models::default()
    };
    s.properties.insert(0x09C8_0417, mesh());
    assert!(b.load_models(&mut s));
    assert_eq!(b.radius, 32.0);
    assert_eq!(
        s.calls
            .iter()
            .filter(|c| c.starts_with("property:"))
            .cloned()
            .collect::<Vec<_>>(),
        ["property:09C80417", "property:EC214180", "property:C9E69AF6"]
    );
    assert_eq!(s.calls.iter().filter(|c| *c == "release:10").count(), 1);
    assert_eq!(b.models.entries.len(), 2);
    for z in [-4.0, f32::NAN] {
        let mut b = boat();
        b.record.base.model = mesh();
        let mut s = Models::default();
        s.bounds[1][2] = z;
        assert!(b.load_models(&mut s));
        assert_eq!(b.radius.to_bits(), (z * 0.5).to_bits());
    }
}

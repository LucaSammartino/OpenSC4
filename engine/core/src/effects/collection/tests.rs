// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::effects::{ComponentRecord, ParamBinding};

#[test]
fn ef18_7_8_11_copies_stable_slots_fixups_only_at_publication() {
    let mut owner = Collection::default();
    let mut source = EffectDescription::new(
        1,
        vec![
            ComponentRecord::new(0, u32::MAX),
            ComponentRecord::new(2, u32::MAX),
            ComponentRecord::new(0, 7),
            ComponentRecord::new(99, u32::MAX),
        ],
    );
    for c in &mut source.components {
        c.name = "MiXeD".into();
    }
    source.text = "TrAiL".into();
    let index = owner.add_description("Smoke", &source);
    assert_eq!(index, 0);
    let stored = &owner.descriptions[0];
    assert_eq!(stored.text, "trail");
    assert_eq!(stored.components[0].index, u32::MAX);
    assert_eq!(stored.components[0].name, "mixed");
    assert_eq!(stored.components[1].index, 0);
    assert_eq!(stored.components[2].name, "MiXeD");
    assert_eq!(stored.components[2].index, 7);
    assert_eq!(stored.components[3].name, "mixed");
    assert_eq!(stored.components[3].index, u32::MAX);
    owner.components.entry(0).or_default().insert("mixed".into(), 4);
    assert_eq!(owner.descriptions[0].components[0].index, u32::MAX);
    assert_eq!(owner.add_description("SMOKE", &source), 0);
    assert_eq!(owner.descriptions[0].components[0].index, 4);
    assert_eq!(source.components[0].index, u32::MAX);
    source.priority = 2;
    assert_eq!(owner.add_description("smoke", &source), 1);
    source.components.clear();
    assert_eq!(owner.descriptions[1].components.len(), 4);
    let mut out = EffectDescription::new(99, vec![]);
    let untouched = out.clone();
    assert!(!owner.get_description("missing", Some(&mut out)));
    assert_eq!(out, untouched);
    assert!(owner.get_description("sMoKe", None));
    assert!(owner.get_description("SMOKE", Some(&mut out)));
    assert_eq!(out.priority, 1);
    out.components.clear();
    assert_eq!(owner.descriptions[0].components.len(), 4);
}

#[test]
fn collection_parameter_binding_precedence_and_each_component_directory() {
    let mut owner = Collection::default();
    let mut source = EffectDescription::new(1, vec![]);
    for kind in [0, 1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16] {
        owner
            .components
            .entry(kind)
            .or_default()
            .insert("a".into(), i32::from(kind) + 100);
        let mut c = ComponentRecord::new(kind, u32::MAX);
        c.name = "A".into();
        source.components.push(c);
    }
    owner.shakes.insert("asset".into(), 11);
    owner.lights.insert("asset".into(), 22);
    for flags in [1, 4, 8, 13, 0, 2] {
        source.bindings.push(ParamBinding {
            index: flags,
            name: "AsSeT".into(),
            value: 3.,
            extra: 99,
        });
    }
    source.bindings.push(ParamBinding {
        index: 1,
        name: "MiSs".into(),
        value: 4.,
        extra: 98,
    });
    owner.add_description("a", &source);
    let stored = &owner.descriptions[0];
    for c in &stored.components {
        assert_eq!(c.index, u32::from(c.kind) + 100);
    }
    assert_eq!(
        stored.bindings.iter().map(|b| b.extra).collect::<Vec<_>>(),
        [11, 22, 22, 11, 99, 99, 98]
    );
    assert_eq!(stored.bindings[6].name, "miss");
    assert_eq!(source.bindings[0].name, "AsSeT");
}

#[derive(Default)]
struct Service(Vec<(bool, u32)>);
impl Messages for Service {
    fn subscribe(&mut self, id: u32) {
        self.0.push((true, id));
    }
    fn unsubscribe(&mut self, id: u32) {
        self.0.push((false, id));
    }
}
#[test]
fn ef18_9_10_ids_duplicate_triggers_and_empty_reload() {
    let mut owner = Collection::default();
    let mut service = Service::default();
    owner.begin(Some(&mut service));
    let mut camera = CameraParams {
        curve: vec![1., 2.],
        scalars: [3, 4, 5, 6],
    };
    owner.set_camera(&camera);
    camera.curve.clear();
    owner.add_description("A", &EffectDescription::new(1, vec![]));
    owner.add_description("B", &EffectDescription::new(99, vec![]));
    owner.add_binding("a", 1, 2);
    owner.add_binding("b", 1, 2);
    owner.add_binding("missing", 1, 3);
    // The binding operation copies but deliberately does not fold names.
    owner.add_binding("A", 1, 4);
    owner.add_trigger(7, "first");
    owner.add_trigger(7, "second");
    owner.end(Some(&mut service));
    assert_eq!(owner.ids.len(), 1);
    assert_eq!(owner.ids[&(1, 2)], 0);
    assert_eq!(owner.trigger_lookup[&7], 1);
    assert_eq!(owner.manager_camera.as_ref().unwrap().curve, [1., 2.]);
    assert_eq!(service.0, [(true, 7), (true, 7)]);
    owner.begin(Some(&mut service));
    assert_eq!(service.0, [(true, 7), (true, 7), (false, 7), (false, 7)]);
    assert!(
        owner.bindings.is_empty()
            && owner.triggers.is_empty()
            && owner.ids.is_empty()
            && owner.trigger_lookup.is_empty()
    );
    assert!(!owner.camera_present);
    assert_eq!(owner.camera.curve, [1., 2.]);
    assert_eq!(owner.descriptions.len(), 2);
    owner.end(None);
    assert_eq!(owner.manager_camera, None);
    owner.begin(None); // no nesting/transaction state
    owner.begin(None);
    assert!(owner.get_description("b", None));
}

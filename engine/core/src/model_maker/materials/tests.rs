// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::collections::BTreeMap;

#[derive(Default)]
struct Host {
    refs: Vec<(bool, Handle)>,
    values: BTreeMap<Handle, Material>,
    holder: Option<Handle>,
    service: bool,
    loads: Vec<ModelKey>,
    changes: Vec<(Handle, u32)>,
    type_reads: usize,
    frame_reads: usize,
}
impl ObjectHost for Host {
    fn retain(&mut self, h: Handle) {
        self.refs.push((true, h));
    }
    fn release(&mut self, h: Handle) {
        self.refs.push((false, h));
    }
}
impl MaterialHost for Host {
    fn holder(&mut self, _: Handle) -> Option<Handle> {
        self.holder
    }
    fn occupant_type(&mut self, _: Handle) -> u32 {
        self.type_reads += 1;
        0
    }
    fn instances(&mut self, _: Handle) -> Vec<Handle> {
        vec![10, 11]
    }
    fn renderer_contains(&mut self, i: Handle) -> bool {
        i == 10
    }
    fn renderer_change(&mut self, i: Handle, mode: u32) {
        self.changes.push((i, mode));
    }
    fn material_list(&mut self, i: Handle) -> Option<Handle> {
        (i == 10).then_some(20)
    }
    fn materials(&mut self, _: Handle) -> Vec<Handle> {
        self.values.keys().copied().collect()
    }
    fn material(&mut self, id: Handle) -> Material {
        self.values[&id].clone()
    }
    fn write_material(&mut self, id: Handle, value: Material) {
        self.values.insert(id, value);
    }
    fn frame_counter(&mut self) -> u32 {
        self.frame_reads += 1;
        123
    }
    fn resource_service(&mut self) -> Option<Handle> {
        self.service.then_some(30)
    }
    fn texture_name(&mut self, _: Handle, name: u32) -> (u32, u32) {
        (456, name - 1)
    }
    fn load_texture(&mut self, _: Handle, key: ModelKey) -> Option<Handle> {
        self.loads.push(key);
        (key.instance == 0x8002).then_some(40)
    }
}

#[test]
fn counted_assignment_keeps_identity_and_acquires_before_release() {
    let mut host = Host::default();
    let mut slot = Some(1);
    assign_counted(&mut slot, Some(1), &mut host);
    assert!(host.refs.is_empty());
    assign_counted(&mut slot, Some(2), &mut host);
    assign_counted(&mut slot, None, &mut host);
    assert_eq!(host.refs, [(true, 2), (false, 1), (false, 2)]);
}

#[test]
fn change_models_default_mode_checks_registration_and_retains_holder() {
    let maker = ModelMaker::default();
    let mut host = Host::default();
    assert!(!maker.change_models(1, 0, 0, &mut host));
    host.holder = Some(2);
    assert!(maker.change_models(1, 0, 0, &mut host));
    assert_eq!(host.type_reads, 2);
    assert_eq!(host.changes, [(10, 3)]);
    assert_eq!(host.refs, [(true, 2), (false, 2)]);
}

#[test]
fn material_apply_reads_frame_once_and_restore_changes_only_swapped_materials() {
    let maker = ModelMaker::default();
    let mut host = Host::default();
    for id in 1..=3 {
        host.values.insert(
            id,
            Material {
                state: id as u8 - 1,
                binding: Some(40),
                byte_2d: 9,
                ..Material::default()
            },
        );
    }
    assert!(maker.restore_static_textures(2, &mut host));
    assert_eq!(host.values[&1].state, 0);
    assert_eq!(host.values[&2].binding, Some(40));
    assert_eq!(host.values[&3].binding, None);
    assert!(!maker.restore_static_textures(2, &mut host));
    host.refs.clear();
    maker.apply_texture_to_models(2, Some(40), 2, false, &mut host);
    assert_eq!(host.frame_reads, 1);
    for value in host.values.values() {
        assert_eq!(value.state, 2);
        assert_eq!(value.sampler, [3, 3, 1, 1]);
        assert_eq!(value.word_30, 123);
        assert_eq!(value.byte_2d, 9);
        assert_eq!(value.byte_2e, 1);
    }
    assert_eq!(host.refs, [(true, 40), (false, 20)]);
}

#[test]
fn offset_swap_leaves_missing_textures_static_and_counts_the_loaded_reference() {
    let maker = ModelMaker::default();
    let mut host = Host::default();
    for id in 0..=4 {
        host.values.insert(
            id,
            Material {
                state: if id == 0 { 0 } else { 2 },
                name_id: id as u32,
                binding: Some(50),
                byte_2d: 9,
                ..Material::default()
            },
        );
    }
    assert!(!maker.swap_textures_by_offset(2, 0x8000, 2, true, &mut host));
    assert!(host.refs.is_empty());
    host.service = true;
    assert!(maker.swap_textures_by_offset(2, 0x8000, 2, true, &mut host));
    assert_eq!(
        host.loads.iter().map(|k| k.instance).collect::<Vec<_>>(),
        [0x8001, 0x8002, 0x8003]
    );
    assert_eq!(host.values[&0].state, 0);
    assert_eq!(host.values[&0].binding, Some(50));
    for id in [1, 2, 4] {
        assert_eq!(host.values[&id].state, 1);
        assert_eq!(host.values[&id].binding, None);
    }
    let value = &host.values[&3];
    assert_eq!(
        (value.state, value.binding, value.swapped_instance),
        (2, Some(40), 0x8002)
    );
    assert_eq!(value.sampler, [2, 2, 0, 0]);
    assert_eq!((value.byte_2c, value.byte_2d, value.byte_2e), (2, 0, 0));
    assert_eq!(value.word_30, 1.0f32.to_bits());
    assert_eq!(host.refs.iter().filter(|e| **e == (false, 50)).count(), 4);
    assert_eq!(
        &host.refs[host.refs.len() - 4..],
        [(false, 40), (false, 50), (false, 20), (false, 30)]
    );
}

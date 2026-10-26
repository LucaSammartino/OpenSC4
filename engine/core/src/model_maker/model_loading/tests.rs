// SPDX-License-Identifier: GPL-3.0-or-later
use super::super::TerrainOccupant;
use super::*;
use std::collections::{BTreeMap, BTreeSet};

type FoundationArrays = (Option<[u32; 5]>, Option<[u32; 5]>);

struct Host {
    refs: BTreeMap<Handle, i32>,
    released: Vec<Handle>,
    interfaces: BTreeMap<u32, Handle>,
    properties: BTreeSet<u32>,
    present: BTreeSet<u32>,
    type_id: u32,
    holder: Option<Handle>,
    cell: [i32; 2],
    cells: Rect,
    lot_lookups: Vec<[i32; 2]>,
    keys: Vec<ModelKey>,
    available: Vec<ModelKey>,
    no_instance: bool,
    threshold: f32,
    bounds: Bounds,
    model_ids: Vec<(u32, Vector)>,
    id_queries: usize,
    flags: BTreeMap<Handle, u32>,
    flag_reads: Vec<Handle>,
    bytes: BTreeMap<Handle, u8>,
    transforms: BTreeMap<Handle, Transform>,
    added: Vec<Handle>,
    bindings: Vec<(Handle, Handle)>,
    placeholder_queries: Vec<(u32, i32, i32)>,
    class: u32,
    animation: bool,
    animation_view: Option<[i32; 2]>,
    animation_speed: Option<f32>,
    animation_byte: Option<u8>,
    style: bool,
    foundation_style: bool,
    foundation_data: Option<FoundationArrays>,
    foundation_dimensions: Option<[f32; 2]>,
    foundation_box: Option<Bounds>,
    unused_property: Vec<u32>,
    filled: Vec<Handle>,
    fill_queries: Vec<(Handle, InstanceSource, i32, i32)>,
    development: Vec<(u32, ModelKey)>,
    notifications: Vec<Handle>,
    base_resets: usize,
    clears: usize,
    terrain_queries: usize,
    shadow_detail: i32,
    shadow_threshold: f32,
    shadow_queries: Vec<(i32, i32, i32, bool)>,
    meshes: BTreeMap<Handle, Vec<ShadowMesh>>,
    overlays: Vec<(Bounds, Transform)>,
    shadow_ids: Vec<i32>,
}

impl Default for Host {
    fn default() -> Self {
        Self {
            refs: BTreeMap::new(),
            released: vec![],
            interfaces: BTreeMap::new(),
            properties: BTreeSet::new(),
            present: BTreeSet::new(),
            type_id: BUILDING,
            holder: Some(10),
            cell: [0, 0],
            cells: Rect::new(0, 0, 1, 1),
            lot_lookups: vec![],
            keys: vec![],
            available: vec![ModelKey::s3d(1)],
            no_instance: false,
            threshold: 0.0,
            bounds: Bounds {
                min: [0.0; 3],
                max: [1.0, 2.0, 3.0],
            },
            model_ids: vec![(1, [0.0; 3])],
            id_queries: 0,
            flags: BTreeMap::new(),
            flag_reads: vec![],
            bytes: BTreeMap::new(),
            transforms: BTreeMap::new(),
            added: vec![],
            bindings: vec![],
            placeholder_queries: vec![],
            class: 0,
            animation: false,
            animation_view: None,
            animation_speed: None,
            animation_byte: None,
            style: true,
            foundation_style: false,
            foundation_data: None,
            foundation_dimensions: None,
            foundation_box: None,
            unused_property: vec![],
            filled: vec![],
            fill_queries: vec![],
            development: vec![],
            notifications: vec![],
            base_resets: 0,
            clears: 0,
            terrain_queries: 0,
            shadow_detail: 3,
            shadow_threshold: 0.0,
            shadow_queries: vec![],
            meshes: BTreeMap::new(),
            overlays: vec![],
            shadow_ids: vec![],
        }
    }
}
impl Host {
    fn owned(&mut self, h: Handle) -> Handle {
        *self.refs.entry(h).or_default() += 1;
        h
    }
    fn placement() -> Placement {
        Placement {
            rotation: 99,
            orientation: 1,
            flip: false,
            position: [10.0, 20.0, 30.0],
            offset: Some([2.0, 3.0, 4.0]),
        }
    }
    fn mesh(state: u8, count: usize) -> ShadowMesh {
        ShadowMesh {
            material_state: state,
            binding: Some(90),
            positions: vec![[0.0; 3]; count],
            coordinates: vec![[0.0; 2]; count],
        }
    }
    fn no_temporary_refs(&self) {
        for (&h, &count) in &self.refs {
            assert_eq!(
                count,
                self.added.iter().filter(|i| **i == h).count() as i32,
                "reference {h}"
            );
        }
    }
}
impl ObjectHost for Host {
    fn retain(&mut self, h: Handle) {
        self.owned(h);
    }
    fn release(&mut self, h: Handle) {
        let count = self.refs.get_mut(&h).expect("release without acquire");
        assert!(*count > 0, "over-release {h}");
        *count -= 1;
        self.released.push(h);
    }
}
impl TransformHost for Host {
    fn camera_yaw(&mut self) -> f32 {
        0.0
    }
    fn object_normal(&mut self, _: Vector, _: u32, _: bool) -> Option<Vector> {
        None
    }
    fn terrain_blocks_normal(&mut self, _: f32, _: f32) -> bool {
        true
    }
    fn terrain_normal(&mut self, _: f32, _: f32) -> Vector {
        panic!("blocked terrain")
    }
}
impl ModelHost for Host {
    fn exemplar(&mut self, _: Handle) -> Handle {
        20
    }
    fn type_id(&mut self, _: Handle) -> u32 {
        self.type_id
    }
    fn position(&mut self, _: Handle) -> Vector {
        [10.0, 20.0, 30.0]
    }
    fn cells(&mut self, _: Handle) -> Rect {
        self.cells
    }
    fn occupant_bounds(&mut self, _: Handle) -> Bounds {
        Bounds {
            min: [10.0; 3],
            max: [20.0; 3],
        }
    }
    fn query_interface(&mut self, _: Handle, iid: u32) -> Option<Handle> {
        self.interfaces.get(&iid).copied().map(|h| self.owned(h))
    }
    fn interface_word(&mut self, _: Handle, slot: u32) -> u32 {
        assert!(matches!(slot, 9 | 14));
        1
    }
    fn property_bool(&mut self, _: Handle, id: u32) -> bool {
        self.properties.contains(&id)
    }
    fn property_present(&mut self, _: Handle, id: u32) -> bool {
        self.present.contains(&id)
    }
    fn property_byte(&mut self, _: Handle, _: u32) -> u8 {
        7
    }
    fn property_float(&mut self, _: Handle, _: u32, default: f32) -> f32 {
        default
    }
    fn property_five(&mut self, _: Handle, id: u32) -> Option<[u32; 5]> {
        (id == 0x68FC_FF37).then_some([1, 2, 3, 4, 5])
    }
    fn observe_property(&mut self, _: Handle, id: u32) {
        self.unused_property.push(id);
    }
    fn orientation_and_state(&mut self, _: Handle) -> (i32, bool) {
        (1, false)
    }
    fn model_class(&mut self, _: Handle) -> u32 {
        self.class
    }
    fn uses_building_style(&mut self, _: Handle) -> bool {
        self.style
    }
    fn foundation_uses_building_style(&mut self, _: Handle) -> bool {
        self.foundation_style
    }
    fn model_ids(&mut self, _: Handle, _: i32, _: i32) -> ModelIds {
        self.id_queries += 1;
        ModelIds {
            type_id: 0x5AD0_E817,
            group: 0xBADB_57F1,
            entries: self.model_ids.clone(),
        }
    }
    fn property1_instance(&mut self, base: u32, zoom: i32, rotation: i32) -> u32 {
        self.placeholder_queries.push((base, zoom, rotation));
        999
    }
    fn holder(&mut self, _: Handle) -> Option<Handle> {
        self.holder
    }
    fn holder_add(&mut self, h: Handle, model: Handle) {
        assert_eq!(Some(h), self.holder);
        self.retain(model);
        self.added.push(model);
    }
    fn holder_clear(&mut self, _: Handle) {
        self.clears += 1;
    }
    fn hiding_filter_accepts(&mut self, _: u32) -> bool {
        true
    }
    fn reset_base_texture(&mut self, _: Handle, arg: u32) {
        assert_eq!(arg, 0);
        self.base_resets += 1;
    }
    fn world_to_cell(&mut self, _: f32, _: f32) -> [i32; 2] {
        self.cell
    }
    fn lot_at_cell(&mut self, cell: [i32; 2]) -> Option<Handle> {
        self.lot_lookups.push(cell);
        Some(30)
    }
    fn lot_occupant(&mut self, lot: Handle) -> Option<Handle> {
        assert_eq!(lot, 30);
        Some(31)
    }
    fn fill_instances(
        &mut self,
        source: Handle,
        kind: InstanceSource,
        zoom: i32,
        rotation: i32,
        limit: usize,
    ) -> Vec<Handle> {
        assert_eq!(limit, 10);
        self.fill_queries.push((source, kind, zoom, rotation));
        let values = self.filled.clone();
        for &h in &values {
            self.owned(h);
        }
        values
    }
    fn load_s3d(&mut self, key: ModelKey) -> Option<Handle> {
        self.keys.push(key);
        self.available
            .contains(&key)
            .then(|| self.owned(1000 + u64::from(key.instance)))
    }
    fn resource_bounds(&mut self, _: Handle) -> Bounds {
        self.bounds
    }
    fn resource_instance(&mut self, resource: Handle) -> Option<Handle> {
        (!self.no_instance).then(|| self.owned(resource + 1000))
    }
    fn camera_scale(&mut self) -> f32 {
        1.0
    }
    fn model_threshold(&mut self) -> f32 {
        self.threshold
    }
    fn animated_model(&mut self, _: ModelKey) -> Option<Handle> {
        self.animation.then(|| self.owned(700))
    }
    fn animated_instance(&mut self, animation: Handle) -> Option<Handle> {
        Some(animation)
    }
    fn animated_view(&mut self, _: Handle, zoom: i32, rotation: i32) {
        self.animation_view = Some([zoom, rotation]);
    }
    fn animated_speed(&mut self, _: Handle, value: f32) {
        self.animation_speed = Some(value);
    }
    fn animated_byte(&mut self, _: Handle, value: u8) {
        self.animation_byte = Some(value);
    }
    fn set_transform(&mut self, instance: Handle, transform: Transform) {
        self.transforms.insert(instance, transform);
    }
    fn occupant_flag(&mut self, _: Handle, flag: u32) -> bool {
        assert_eq!(flag, 0x10);
        self.present.contains(&flag)
    }
    fn instance_flags(&mut self, instance: Handle) -> u32 {
        self.flag_reads.push(instance);
        *self.flags.get(&instance).unwrap_or(&1)
    }
    fn set_instance_flags(&mut self, instance: Handle, flags: u32) {
        self.flags.insert(instance, flags);
    }
    fn instance_byte(&mut self, instance: Handle) -> u8 {
        *self.bytes.get(&instance).unwrap_or(&0x80)
    }
    fn set_instance_byte(&mut self, instance: Handle, value: u8) {
        self.bytes.insert(instance, value);
    }
    fn bind_occupant(&mut self, instance: Handle, occupant: Handle) {
        self.bindings.push((instance, occupant));
    }
    fn development_check(&mut self, category: u32, key: ModelKey) -> i32 {
        self.development.push((category, key));
        0
    }
    fn development_accepts(&mut self, _: Handle) -> bool {
        true
    }
    fn development_notify(&mut self, building: Handle, arg: u32) {
        assert_eq!(arg, 0);
        self.notifications.push(building);
    }
    fn create_foundation(&mut self, arg: u32) -> Handle {
        assert_eq!(arg, 1);
        self.owned(500)
    }
    fn foundation_arrays(&mut self, _: Handle, a: Option<[u32; 5]>, b: Option<[u32; 5]>) {
        self.foundation_data = Some((a, b));
    }
    fn foundation_dimensions(&mut self, _: Handle, a: f32, b: f32) {
        self.foundation_dimensions = Some([a, b]);
    }
    fn foundation_bounds(&mut self, _: Handle, bounds: Bounds) {
        self.foundation_box = Some(bounds);
    }
    fn foundation_adjust_height(&mut self, _: Handle, enabled: bool) {
        assert!(enabled);
    }
}
impl ShadowHost for Host {
    fn shadow_detail(&mut self) -> i32 {
        self.shadow_detail
    }
    fn shadow_threshold(&mut self) -> f32 {
        self.shadow_threshold
    }
    fn shadow_model_id(
        &mut self,
        _: Handle,
        zoom: i32,
        view_rotation: i32,
        orientation: i32,
        flag: bool,
        key: &mut ModelKey,
    ) {
        assert_eq!(*key, ModelKey::s3d(0));
        self.shadow_queries.push((zoom, view_rotation, orientation, flag));
        key.instance = zoom as u32 + 1;
    }
    fn resource_model(&mut self, resource: Handle) -> Option<Handle> {
        Some(self.owned(resource + 2000))
    }
    fn model_bounds(&mut self, _: Handle) -> Bounds {
        self.bounds
    }
    fn shadow_meshes(&mut self, model: Handle) -> Vec<ShadowMesh> {
        self.meshes.get(&model).cloned().unwrap_or_default()
    }
    fn add_model_shadow(&mut self, mesh: &ShadowMesh, bounds: Bounds, transform: Transform) -> i32 {
        assert_eq!(mesh.positions.len(), mesh.coordinates.len());
        self.overlays.push((bounds, transform));
        if self.overlays.len() == 1 {
            -1
        } else {
            42
        }
    }
    fn holder_add_shadow(&mut self, _: Handle, shadow: i32) {
        self.shadow_ids.push(shadow);
    }
}
impl TerrainHost for Host {
    fn occupant_cells(&mut self, _: Handle) -> Rect {
        self.terrain_queries += 1;
        Rect::new(1, 1, 0, 0)
    }
    fn foundation_mode(&mut self, _: Handle) -> Option<u32> {
        unreachable!()
    }
    fn flags(&mut self, _: [i32; 2]) -> u32 {
        unreachable!()
    }
    fn set_flags(&mut self, _: [i32; 2], _: u32) {
        unreachable!()
    }
    fn occupants(&mut self, _: Rect, _: Option<Handle>) -> Vec<Handle> {
        unreachable!()
    }
    fn occupant(&mut self, _: Handle) -> TerrainOccupant {
        unreachable!()
    }
    fn lot_flags(&mut self, _: [i32; 2]) -> Option<u32> {
        unreachable!()
    }
    fn base_texture_cover(&mut self, _: Handle, _: i32, _: i32) -> bool {
        unreachable!()
    }
    fn release_occupant(&mut self, _: Handle) {
        unreachable!()
    }
    fn refresh_terrain(&mut self, _: Rect) {
        unreachable!()
    }
}

#[test]
fn placeholder_uses_cell_differences_all_size_classes_and_c_remainder() {
    let maker = ModelMaker::default();
    let mut host = Host {
        available: vec![ModelKey::s3d(999)],
        ..Host::default()
    };
    for (size, base) in [
        (1, 0x2A0E_0000),
        (2, 0x2A0C_0000),
        (3, 0x2A0B_0000),
        (4, 0x2A0A_0000),
        (9, 0x2A0A_0000),
    ] {
        host.cells = Rect::new(5, 8, 5 + size, 9);
        let model = maker.create_placeholder(1, 3, -1, 0, false, &mut host).unwrap();
        assert_eq!(host.placeholder_queries.last(), Some(&(base, 3, -3)));
        host.release(model);
    }
    let model = maker.create_placeholder(1, 2, 1, 2, true, &mut host).unwrap();
    assert_eq!(host.placeholder_queries.last().unwrap().2, 3);
    host.release(model);
    host.no_temporary_refs();
}

#[test]
fn add_model_fallback_mutates_key_and_size_cut_does_not_make_a_placeholder() {
    let maker = ModelMaker::default();
    let mut host = Host::default();
    let mut key = ModelKey {
        type_id: 5,
        group: 6,
        instance: 1,
    };
    let original = key;
    assert_eq!(
        maker.add_model(1, &mut key, Host::placement(), &mut host),
        Ok(Some(2001))
    );
    assert_eq!(host.keys, [original, ModelKey::s3d(1)]);
    assert_eq!(key, ModelKey::s3d(1));
    assert_eq!(host.transforms[&2001].translation, [6.0, 23.0, 32.0]);
    host.no_temporary_refs();
    host.threshold = 100.0;
    assert_eq!(maker.add_model(1, &mut key, Host::placement(), &mut host), Ok(None));
    assert!(host.placeholder_queries.is_empty());
    host.no_temporary_refs();
}

#[test]
fn animated_class_forces_flip_sets_mirrored_view_and_keeps_only_flag_one() {
    let mut maker = ModelMaker::default();
    maker.saved.target_view = [4, 2];
    let mut host = Host {
        class: 0xAA12_3BF9,
        animation: true,
        ..Host::default()
    };
    host.present.extend([0x6A9E_DD07, 0x10]);
    host.flags.insert(700, 0xFF);
    let mut key = ModelKey::s3d(1);
    assert_eq!(
        maker.add_model(1, &mut key, Host::placement(), &mut host),
        Ok(Some(700))
    );
    assert_eq!(host.animation_view, Some([4, 2]));
    assert_eq!(host.animation_speed, Some(0.01));
    assert_eq!(host.animation_byte, Some(7));
    assert_eq!(host.transforms[&700].translation, [12.0, 23.0, 34.0]);
    assert_eq!(host.flags[&700], 1);
    assert!(host.keys.is_empty());
    host.no_temporary_refs();
    host.animation = false;
    key.type_id = 5;
    key.group = 6;
    assert_eq!(
        maker.add_model(1, &mut key, Host::placement(), &mut host),
        Ok(Some(2001))
    );
    assert_eq!(host.keys, [ModelKey::s3d(1)]);
    host.no_temporary_refs();
}

#[test]
fn ignore_missing_data_is_overridden_by_exemplar_and_missing_holder_releases_all_refs() {
    let mut maker = ModelMaker {
        ignore_missing_model_data: true,
        ..ModelMaker::default()
    };
    maker.saved.target_view = [2, 1];
    let mut host = Host {
        available: vec![ModelKey::s3d(999)],
        ..Host::default()
    };
    let mut key = ModelKey::s3d(1);
    assert_eq!(maker.add_model(1, &mut key, Host::placement(), &mut host), Ok(None));
    assert!(host.placeholder_queries.is_empty());
    host.present.insert(0x6A87_1B82);
    host.holder = None;
    assert_eq!(
        maker.add_model(1, &mut key, Host::placement(), &mut host),
        Err(LoadError::MissingHolder)
    );
    assert_eq!(host.placeholder_queries, [(0x2A0E_0000, 2, 0)]);
    host.no_temporary_refs();
}

#[test]
fn common_result_comes_only_from_first_id_and_prop_flags_and_bytes_apply_to_later_models() {
    let maker = ModelMaker::default();
    let mut host = Host {
        model_ids: vec![(0, [0.0; 3]), (1, [0.0; 3])],
        ..Host::default()
    };
    host.present.insert(0x4A70_D491);
    host.properties.insert(0xE9F0_FA86);
    assert_eq!(maker.load_models_common(1, 4, 2, PROP, &mut host), Ok(false));
    assert_eq!(host.added, [2001]);
    assert_eq!(host.flags[&2001], 0x22);
    assert_eq!(host.bytes[&2001], 0x84);
    assert_eq!(host.transforms[&2001].translation, [10.0, 20.0, 30.0]);
    host.no_temporary_refs();
}

#[test]
fn common_building_first_model_notifies_development_and_flora_sets_flags() {
    let maker = ModelMaker::default();
    let mut host = Host {
        model_ids: vec![(1, [0.0; 3]), (2, [0.0; 3])],
        available: vec![ModelKey::s3d(1), ModelKey::s3d(2)],
        ..Host::default()
    };
    host.interfaces.insert(0x87DD_A3A9, 50);
    assert_eq!(maker.load_models_common(1, 4, 2, BUILDING, &mut host), Ok(true));
    assert_eq!(host.flags[&2001], 0x41);
    assert!(!host.flags.contains_key(&2002));
    assert_eq!(host.flag_reads, [2001]);
    assert_eq!(host.development, [(2, ModelKey::s3d(1)), (1, ModelKey::s3d(1))]);
    assert_eq!(host.notifications, [50]);
    assert_eq!(host.refs[&50], 0);
    host.no_temporary_refs();
    assert_eq!(maker.load_models_common(1, 4, 2, FLORA, &mut host), Ok(true));
    assert_eq!(host.flags[&2002], 0x81);
    host.no_temporary_refs();
}

#[test]
fn common_other_types_leave_instance_flags_untouched() {
    let maker = ModelMaker::default();
    let mut host = Host::default();
    for type_id in [FOUNDATION, 0x6A11_BD2B] {
        assert_eq!(maker.load_models_common(1, 4, 2, type_id, &mut host), Ok(true));
    }
    assert!(host.flag_reads.is_empty());
    assert!(host.flags.is_empty());
    host.no_temporary_refs();
}

#[test]
fn foundation_copies_optional_arrays_defaults_and_box_or_uses_common_with_y_offset() {
    let maker = ModelMaker::default();
    let mut host = Host::default();
    assert_eq!(maker.load_foundation_models(1, 4, 2, FOUNDATION, &mut host), Ok(true));
    assert_eq!(host.foundation_data, Some((Some([1, 2, 3, 4, 5]), None)));
    assert_eq!(host.foundation_dimensions, Some([16.0, 16.0]));
    assert_eq!(host.foundation_box.unwrap().max, [20.0, 20.2, 20.0]);
    assert_eq!(host.unused_property, [0x88FC_C3E3]);
    assert_eq!(host.bindings, [(500, 1)]);
    host.no_temporary_refs();
    host.foundation_style = true;
    assert_eq!(maker.load_foundation_models(1, 4, 2, FOUNDATION, &mut host), Ok(true));
    assert_eq!(host.transforms[&2001].translation[1], 20.2);
    host.no_temporary_refs();
}

#[test]
fn network_and_underground_fill_sources_and_owned_list_cleanup_follow_type() {
    let maker = ModelMaker::default();
    let mut host = Host {
        filled: vec![71, 72],
        ..Host::default()
    };
    assert_eq!(maker.load_network_models(1, 2, 3, &mut host), Ok(false));
    host.interfaces.insert(0xA821_EF94, 50);
    assert_eq!(maker.load_network_models(1, 2, 3, &mut host), Ok(true));
    assert_eq!(host.fill_queries, [(50, InstanceSource::NetworkInterface, 2, 3)]);
    host.type_id = POWER_POLE;
    assert_eq!(maker.load_network_models(1, 2, 3, &mut host), Ok(true));
    assert_eq!(host.fill_queries.last().unwrap().1, InstanceSource::PowerPole);
    assert_eq!(maker.load_underground_models(1, 2, 3, &mut host), Ok(false));
    host.type_id = 0x2A18_3167;
    host.holder = None;
    assert_eq!(
        maker.load_underground_models(1, 2, 3, &mut host),
        Err(LoadError::MissingHolder)
    );
    assert_eq!(host.fill_queries.last().unwrap().1, InstanceSource::Underground);
    host.no_temporary_refs();
}

#[test]
fn lot_lookup_uses_signed_inclusive_city_bounds_and_interface_constructor_does_not_assign() {
    let mut maker = ModelMaker::default();
    maker.saved.city = Rect::new(-2, -3, 4, 5);
    let mut host = Host::default();
    for cell in [[-2, -3], [4, 5]] {
        host.cell = cell;
        assert_eq!(maker.lot_at_occupant(1, &mut host), Some(31));
    }
    for cell in [[-3, 0], [0, -4], [5, 0], [0, 6]] {
        host.cell = cell;
        assert_eq!(maker.lot_at_occupant(1, &mut host), None);
    }
    assert_eq!(host.lot_lookups, [[-2, -3], [4, 5]]);
    let mut out = Some(99);
    maker.construct_interface(&mut out, 1, 0x097F_6D98, &mut host);
    assert_eq!(out, None);
    host.interfaces.insert(0x097F_6D98, 50);
    maker.construct_interface(&mut out, 1, 0x097F_6D98, &mut host);
    assert_eq!(out, Some(50));
    host.release(50);
    maker.reset_lot_base_texture(1, &mut host);
    assert_eq!(host.base_resets, 1);
    host.no_temporary_refs();
}

#[test]
fn remove_models_runs_hiding_before_clear_and_bulk_reload_skips_hiding() {
    let mut maker = ModelMaker::default();
    let mut host = Host::default();
    maker.remove_models(1, FOUNDATION, &mut host);
    assert_eq!((host.terrain_queries, host.clears), (1, 1));
    maker.bulk_reload = true;
    maker.remove_models(1, FOUNDATION, &mut host);
    assert_eq!((host.terrain_queries, host.clears), (1, 2));
    host.no_temporary_refs();
}

#[test]
fn shadow_prop_gates_then_searches_lower_zoom_and_filters_materials_and_overlay_ids() {
    let maker = ModelMaker::default();
    let mut host = Host {
        available: vec![ModelKey::s3d(1), ModelKey::s3d(2), ModelKey::s3d(3)],
        ..Host::default()
    };
    host.interfaces.insert(0xE979_3A65, 50);
    assert_eq!(maker.create_occupant_shadow(1, 2, 3, &mut host), Ok(false));
    assert!(host.keys.is_empty());
    host.properties.insert(0x8A5E_5DB8);
    host.meshes.insert(3003, vec![Host::mesh(1, 3), Host::mesh(1, 3)]);
    host.meshes.insert(3002, vec![Host::mesh(0, 3)]);
    assert_eq!(maker.create_occupant_shadow(1, 2, 3, &mut host), Ok(true));
    assert_eq!(host.shadow_queries, [(2, 0, 1, false), (1, 0, 1, false)]);
    assert!(host.overlays.is_empty());
    host.shadow_detail = 4;
    host.meshes.insert(
        3003,
        vec![Host::mesh(0, 3), Host::mesh(1, 2), Host::mesh(1, 3), Host::mesh(2, 4)],
    );
    assert_eq!(maker.create_occupant_shadow(1, 2, 3, &mut host), Ok(true));
    assert_eq!(host.overlays.len(), 2);
    assert_eq!(host.shadow_ids, [42]);
    assert_eq!(host.overlays[0].0.min, [10.0; 3]);
    assert_eq!(host.overlays[0].1.counter, 3);
    host.no_temporary_refs();
}

#[test]
fn shadow_size_cut_stops_search_and_detail_five_uses_transformed_model_bounds() {
    let maker = ModelMaker::default();
    let mut host = Host {
        type_id: POWER_POLE,
        shadow_detail: 5,
        shadow_threshold: 3.0,
        ..Host::default()
    };
    host.meshes.insert(3001, vec![Host::mesh(1, 3)]);
    assert_eq!(maker.create_occupant_shadow(1, 0, 0, &mut host), Ok(false));
    assert!(host.overlays.is_empty());
    host.no_temporary_refs();
    host.shadow_threshold = 0.0;
    assert_eq!(maker.create_occupant_shadow(1, 0, 0, &mut host), Ok(true));
    let (bounds, _) = host.overlays[0];
    assert_eq!(bounds.min, [7.0, 20.0, 30.0]);
    assert_eq!(bounds.max, [10.0, 22.0, 31.0]);
    host.no_temporary_refs();
}

#[test]
fn prop_style_gate_and_shadow_wrapper_keep_the_common_result() {
    let maker = ModelMaker::default();
    let mut host = Host {
        style: false,
        ..Host::default()
    };
    assert_eq!(maker.load_prop_models(1, 4, 2, PROP, &mut host), Ok(false));
    assert_eq!(host.id_queries, 0);
    host.style = true;
    host.properties.insert(0x0292_44DB);
    assert_eq!(maker.load_prop_models(1, 4, 2, PROP, &mut host), Ok(true));
    assert!(host.shadow_queries.is_empty());
    host.no_temporary_refs();
}

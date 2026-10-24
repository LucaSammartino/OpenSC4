// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::sync::{Arc, Mutex};

type Events = Arc<Mutex<Vec<String>>>;

struct Owner(&'static str, Events);
impl ModelOwner for Owner {
    fn add_ref(&self) {
        self.1.lock().unwrap().push(format!("acquire {}", self.0));
    }
    fn release(&self) {
        self.1.lock().unwrap().push(format!("release {}", self.0));
    }
}

struct Lease {
    identity: u32,
    serial: u32,
    events: Events,
}
impl Texture for Lease {
    fn same_binding(&self, other: &Self) -> bool {
        self.identity == other.identity
    }
    fn id(&self) -> u32 {
        self.identity
    }
    fn has_alpha(&self) -> bool {
        self.serial > 1
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        self.events.lock().unwrap().push(format!("binding {}", self.serial));
    }
}
struct Factory {
    events: Events,
    serial: u32,
}
impl TextureFactory for Factory {
    type Binding = Lease;
    fn load(&mut self, _: TextureKey) -> Option<Lease> {
        self.serial += 1;
        Some(Lease {
            identity: 42,
            serial: self.serial,
            events: self.events.clone(),
        })
    }
}

fn record() -> CellRecord {
    CellRecord {
        texture_key: 100,
        cell: [2, 3],
        orientation: Orientation::new(1).unwrap(),
        mirror: true,
        rgb: [255; 3],
        alpha: 128,
        mode: 255,
    }
}

#[test]
fn basic_defaults_owner_identity_and_release_before_acquire() {
    let events = Events::default();
    let a: Arc<dyn ModelOwner> = Arc::new(Owner("a", events.clone()));
    let b: Arc<dyn ModelOwner> = Arc::new(Owner("b", events.clone()));
    let mut basic = BasicModel::default();
    assert_eq!(basic.reference_count(), 0);
    assert!(basic.owner().is_none());
    assert_eq!(
        (basic.custom_data(), basic.properties(), basic.initial_flags()),
        (0, 2, [0, 0])
    );
    assert_eq!(basic.initial_scale().to_bits(), 0x3f800000);
    basic.set_owner(Some(a.clone()));
    basic.set_owner(Some(a.clone()));
    assert!(Arc::ptr_eq(basic.owner().unwrap(), &a));
    basic.set_custom_data(u32::MAX);
    assert_eq!(basic.custom_data(), u32::MAX);
    basic.set_owner(Some(b));
    basic.set_owner(None);
    drop(basic);
    assert_eq!(
        *events.lock().unwrap(),
        ["acquire a", "release a", "acquire b", "release b"]
    );
}

#[test]
fn owner_qi_preserves_failure_output_and_word_refcount_wraps() {
    let mut cell = CellTextureModel::<Lease>::from_record(record());
    let mut output = Some(ModelInterface::Wall);
    assert!(!cell.query_interface(RETAINING_WALL_IID, &mut output));
    assert_eq!(output, Some(ModelInterface::Wall));
    assert_eq!(cell.basic().reference_count(), 0);
    assert!(cell.query_interface(CELL_TEXTURE_IID, &mut output));
    assert_eq!(output, Some(ModelInterface::Cell));
    for iid in [1, 0x26a68449] {
        assert!(cell.query_interface(iid, &mut output));
        assert_eq!(output, Some(ModelInterface::Basic));
    }
    assert_eq!(cell.release(), 2);
    assert_eq!(cell.release(), 1);
    assert_eq!(cell.release(), 0);
    let mut basic = BasicModel::default();
    for n in 1..=65535 {
        assert_eq!(basic.add_ref(), n);
    }
    assert_eq!(basic.add_ref(), 0);
    assert_eq!(basic.add_ref(), 1);
}

#[test]
fn terminal_release_and_drop_order_for_every_owner() {
    let events = Events::default();
    let mut factory = Factory {
        events: events.clone(),
        serial: 0,
    };
    let owner: Arc<dyn ModelOwner> = Arc::new(Owner("owner", events.clone()));
    let keys = FoundationTextureKeys {
        sides: [Some(10); 5],
        tops: [Some(20); 5],
    };
    let mut foundation = FoundationModel::empty(keys);
    assert_eq!(
        foundation.world_bounds(),
        Bounds {
            min: [1.; 3],
            max: [0.; 3]
        }
    );
    let mut out = Some(ModelInterface::Cell);
    assert!(!foundation.query_interface(CELL_TEXTURE_IID, &mut out));
    assert!(!foundation.query_interface(RETAINING_WALL_IID, &mut out));
    foundation.basic_mut().set_owner(Some(owner.clone()));
    foundation.load_textures(Zoom::new(0).unwrap(), &mut factory);
    assert_eq!(foundation.release(), 0); // Initial zero still destroys.
    assert!(foundation.side_texture().is_none() && foundation.top_texture().is_none());
    drop(foundation); // No duplicate releases.
    assert_eq!(
        *events.lock().unwrap(),
        ["acquire owner", "binding 1", "binding 2", "release owner"]
    );

    events.lock().unwrap().clear();
    let mut cell = CellTextureModel::from_record(record());
    cell.basic_mut().set_owner(Some(owner.clone()));
    cell.load_texture(Zoom::new(0).unwrap(), &mut factory).unwrap();
    cell.add_ref();
    assert_eq!(cell.release(), 0);
    drop(cell);
    assert_eq!(*events.lock().unwrap(), ["acquire owner", "binding 3", "release owner"]);

    events.lock().unwrap().clear();
    let samples = [
        WallSample {
            position: [0., 0.],
            terrain_height: 12.,
        },
        WallSample {
            position: [16., 0.],
            terrain_height: 14.,
        },
    ];
    let mut wall = RetainingWallModel::init(Edge::North, 10., &samples, 0.0625, [Some(1); 5], true).unwrap();
    wall.basic_mut().set_owner(Some(owner));
    wall.load_texture(Zoom::new(0).unwrap(), &mut factory);
    assert!(wall.query_interface(RETAINING_WALL_IID, &mut out));
    assert_eq!(out, Some(ModelInterface::Wall));
    assert_eq!(wall.release(), 0);
    assert_eq!(wall.shape().records.capacity(), 0);
    drop(wall);
    assert_eq!(*events.lock().unwrap(), ["acquire owner", "binding 4", "release owner"]);
}

#[test]
fn rust_drop_releases_resources_before_basic_owner_without_terminal_release() {
    let events = Events::default();
    let mut factory = Factory {
        events: events.clone(),
        serial: 0,
    };
    let mut cell = CellTextureModel::from_record(record());
    cell.basic_mut()
        .set_owner(Some(Arc::new(Owner("owner", events.clone()))));
    cell.load_texture(Zoom::new(0).unwrap(), &mut factory).unwrap();
    drop(cell);
    assert_eq!(*events.lock().unwrap(), ["acquire owner", "binding 1", "release owner"]);
}

#[test]
fn cell_equal_binding_releases_extra_acquisition_and_retains_old_lease() {
    let events = Events::default();
    let mut factory = Factory {
        events: events.clone(),
        serial: 0,
    };
    let mut cell = CellTextureModel::from_record(record());
    cell.load_texture(Zoom::new(0).unwrap(), &mut factory).unwrap();
    assert!(!cell.texture_has_alpha());
    cell.load_texture(Zoom::new(1).unwrap(), &mut factory).unwrap();
    assert!(cell.texture_has_alpha()); // Metadata from the new acquisition.
    assert_eq!(cell.texture().unwrap().serial, 1);
    assert_eq!(*events.lock().unwrap(), ["binding 2"]);
    drop(cell);
    assert_eq!(*events.lock().unwrap(), ["binding 2", "binding 1"]);
}

#[test]
fn cell_invalid_orientation_retains_uvs_then_mirrors_each_update() {
    let events = Events::default();
    let mut factory = Factory { events, serial: 0 };
    let mut cell = CellTextureModel::from_record(record());
    assert_eq!(cell.corners(), [[1., 1.], [1., 0.], [0., 1.], [0., 0.]]);
    let mut rec = record();
    rec.orientation = Orientation::from_byte(255);
    cell.set_texture(rec, Zoom::new(0).unwrap(), &mut factory).unwrap();
    assert_eq!(cell.corners(), [[0., 1.], [0., 0.], [1., 1.], [1., 0.]]);
    cell.set_texture(rec, Zoom::new(0).unwrap(), &mut factory).unwrap();
    assert_eq!(cell.corners(), [[1., 1.], [1., 0.], [0., 1.], [0., 0.]]);
    rec.mirror = false;
    cell.set_texture(rec, Zoom::new(0).unwrap(), &mut factory).unwrap();
    assert_eq!(cell.corners(), [[1., 1.], [1., 0.], [0., 1.], [0., 0.]]);
}

#[test]
fn init_absent_service_preserves_state_and_success_packs_literals() {
    let mut init = CellRenderInitialization::default();
    assert!(!init.init(false));
    assert!(init.states.is_none() && init.combiner.is_none());
    assert!(init.init(true));
    let states = init.states.unwrap();
    assert_eq!(
        states.map(|s| (s.value, s.mask)),
        [
            (0x020001a7, 0x1e0003ff),
            (0x020a91bf, 0x1fffffff),
            (0x000a80b5, 0x101fe3ff)
        ]
    );
    assert_eq!(init.combiner.unwrap(), [0, 0, 2, 0x21, 1, 0, 3, 1, 0, 0, 0, 0]);
    assert!(!init.init(false));
    assert_eq!(init.states.unwrap(), states);
}

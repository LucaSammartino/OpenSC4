// SPDX-License-Identifier: GPL-3.0-or-later
//! The formats→core adapter uses actual parsed Model.regions, with no reparse or
//! owner-output delegation. This test also checks parser case folding and append.
use fc4_core::model_reg_effects::{
    EffectFactory, InstanceFrame, ModelInstance, ModelRegEffects, Regions, Transform, VisualEffect,
};
use fc4_formats::s3d::Model;
use std::sync::{Arc, Mutex};

struct ParsedInstance(Model);

impl ModelInstance for ParsedInstance {
    fn regions(&self) -> &Regions {
        &self.0.regions
    }

    fn frame(&self) -> InstanceFrame {
        InstanceFrame {
            transform: Transform::IDENTITY,
            animated_frame: Some(1),
        }
    }
}

type Transforms = Arc<Mutex<Vec<(Transform, u8)>>>;

struct Effect(Transforms);

impl VisualEffect for Effect {
    fn start(&mut self, _: bool, _: u32) {}
    fn stop(&mut self, _: bool) {}
    fn set_transform(&mut self, transform: Transform, flags: u8) {
        self.0.lock().unwrap().push((transform, flags));
    }
    fn set_parameter(&mut self, _: u32, _: f32) {}
}

struct Factory {
    names: Vec<String>,
    transforms: Transforms,
}

impl EffectFactory for Factory {
    fn create(&mut self, name: &str) -> Option<Box<dyn VisualEffect>> {
        self.names.push(name.to_owned());
        Some(Box::new(Effect(Arc::clone(&self.transforms))))
    }
}

/// A constructed, minimal valid S3D 1.2 fixture. Chunk lengths are ignored by
/// the existing parser; no meshes/materials are needed to exercise REGP.
fn model_bytes() -> Vec<u8> {
    fn chunk(bytes: &mut Vec<u8>, tag: &[u8; 4]) {
        bytes.extend_from_slice(tag);
        bytes.extend_from_slice(&0u32.to_le_bytes());
    }
    let mut bytes = Vec::new();
    chunk(&mut bytes, b"3DMD");
    chunk(&mut bytes, b"HEAD");
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    for tag in [b"VERT", b"INDX", b"PRIM", b"MATS"] {
        chunk(&mut bytes, tag);
        bytes.extend_from_slice(&0u32.to_le_bytes());
    }
    chunk(&mut bytes, b"ANIM");
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&30u16.to_le_bytes());
    bytes.extend_from_slice(&255u16.to_le_bytes()); // Owner doesn't interpret playback.
    bytes.extend_from_slice(&123u32.to_le_bytes());
    bytes.extend_from_slice(&42f32.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    chunk(&mut bytes, b"PROP");
    bytes.extend_from_slice(&0u32.to_le_bytes());
    chunk(&mut bytes, b"REGP");
    bytes.extend_from_slice(&3u32.to_le_bytes());
    for (name, x) in [
        ("FX_00_RotOR\0", 1.0f32),
        ("fx_00_rotor\0", 9.0),
        ("regpoint01\0", 77.0),
    ] {
        bytes.push(name.len() as u8);
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        for value in [x, 2.0, 3.0, 0.0, 0.0, 0.0, 1.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes
}

#[test]
fn parsed_regions_feed_owner_and_duplicate_records_select_current_frame() {
    let model = Model::parse(&model_bytes()).unwrap();
    assert_eq!(model.regions["fx_00_rotor"].len(), 2);
    let instance = Arc::new(ParsedInstance(model));
    let mut factory = Factory {
        names: Vec::new(),
        transforms: Arc::default(),
    };
    let mut owner = ModelRegEffects::new();
    owner.attach(Some(instance), &mut factory);
    assert_eq!(factory.names, ["rotor"]);
    owner.start_all(false, 0);
    let report = owner.update();
    assert_eq!(report.transformed, 1);
    assert!(report.missing_records.is_empty());
    let actual = factory.transforms.lock().unwrap();
    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0].0.translation, [9.0, 2.0, 3.0]);
    assert_eq!(actual[0].0.rotation, Transform::IDENTITY.rotation);
    assert_eq!(actual[0].1, 5);
}

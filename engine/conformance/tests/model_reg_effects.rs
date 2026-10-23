// SPDX-License-Identifier: GPL-3.0-or-later
//! graphics-formats §3.11 / §8.2 vector 37 / §8.5 vectors 48–49.
use fc4_core::model_reg_effects::*;
use fc4_formats::s3d::Model;
use std::sync::{Arc, Mutex};

struct Instance(Model);
impl ModelInstance for Instance {
    fn regions(&self) -> Option<&Regions> {
        Some(&self.0.regions)
    }
    fn frame(&self) -> InstanceFrame {
        InstanceFrame {
            transform: Transform {
                translation: [100., 0., 200.],
                ..Transform::IDENTITY
            },
            flags: 5,
            animated_frame: None,
        }
    }
}
type Transforms = Arc<Mutex<Vec<(Transform, u8, u8)>>>;
struct Effect {
    started: bool,
    transforms: Transforms,
}
impl VisualEffect for Effect {
    fn is_started(&self) -> bool {
        self.started
    }
    fn start(&mut self, _: u32) {
        self.started = true;
    }
    fn stop(&mut self, _: u32) {
        self.started = false;
    }
    fn set_transform(&mut self, transform: Transform, flags: u8, version: u8) {
        self.transforms.lock().unwrap().push((transform, flags, version));
    }
    fn set_parameter(&mut self, _: u32, _: Vector3, _: u32) {}
}
struct Factory {
    names: Vec<String>,
    transforms: Transforms,
}
impl EffectFactory for Factory {
    fn acquire_manager(&mut self) -> bool {
        true
    }
    fn create(&mut self, name: &str) -> Creation {
        self.names.push(name.to_owned());
        Creation {
            success: true,
            effect: Some(Box::new(Effect {
                started: false,
                transforms: self.transforms.clone(),
            })),
        }
    }
}

fn chunk(out: &mut Vec<u8>, tag: &[u8; 4], body: &[u8]) {
    out.extend(tag);
    out.extend((8 + body.len() as u32).to_le_bytes());
    out.extend(body);
}

fn resource(entries: &[(&str, [f32; 7])]) -> Vec<u8> {
    // Minimal geometry-free version 1.5 resource, with real REGP serial data.
    let mut body = Vec::new();
    chunk(&mut body, b"HEAD", &[1, 0, 5, 0]);
    for tag in [b"VERT", b"INDX", b"PRIM", b"MATS"] {
        chunk(&mut body, tag, &[0; 4]);
    }
    // One frame, rate 30, mode 3, flags/displacement zero, zero meshes.
    chunk(&mut body, b"ANIM", &[1, 0, 30, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    chunk(&mut body, b"PROP", &[0; 4]);
    let mut regp = (entries.len() as u32).to_le_bytes().to_vec();
    for (name, record) in entries {
        regp.push((name.len() + 1) as u8);
        regp.extend(name.as_bytes());
        regp.push(0);
        regp.extend(1u16.to_le_bytes());
        for component in record {
            regp.extend(component.to_le_bytes());
        }
    }
    chunk(&mut body, b"REGP", &regp);
    let mut bytes = b"3DMD".to_vec();
    bytes.extend((body.len() as u32 + 8).to_le_bytes());
    bytes.extend(body);
    bytes
}

#[test]
fn gf37_48_49_resource_to_registration_to_transform() {
    let record = [1., 2., 3., 0., 0., 2., std::f32::consts::FRAC_PI_2];
    let bytes = resource(&[
        ("FX_00_RotOR", record),
        ("fx_00_rotor", [9.; 7]),
        ("regpoint01", record),
    ]);
    let model = Arc::new(Instance(Model::parse(&bytes).unwrap()));
    assert_eq!(model.0.regions["fx_00_rotor"], [record, [9.; 7]]);
    let mut factory = Factory {
        names: Vec::new(),
        transforms: Arc::default(),
    };
    let mut owner = ModelRegEffects::new();
    owner.set_model_instance(Some(model.clone()));
    assert!(factory.names.is_empty());
    assert!(owner.init(&mut factory));
    assert_eq!(factory.names, ["rotor"]);
    assert!(std::ptr::eq(
        owner.records("fx_00_rotor").unwrap(),
        model.0.regions["fx_00_rotor"].as_slice()
    ));
    owner.start_all(1);
    assert_eq!(owner.update().transformed, 1);
    let values = factory.transforms.lock().unwrap();
    let (transform, flags, version) = values[0];
    assert_eq!((flags, version), (5, 0));
    assert_eq!(transform.translation, [101., 2., 203.]);
    for (a, b) in transform
        .rotation
        .into_iter()
        .flatten()
        .zip([[0., -1., 0.], [1., 0., 0.], [0., 0., 1.]].into_iter().flatten())
    {
        assert!((a - b).abs() <= 1e-6);
    }
    owner.shutdown();
}

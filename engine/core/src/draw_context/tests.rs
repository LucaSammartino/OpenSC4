// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent observations derived from draw-context.md 13 and sections 1-9.
use super::*;
use std::sync::atomic::{AtomicI32, Ordering};

#[test]
fn settled_complete_constructor_init_and_stage_records() {
    // Literal tables in public sections 1.2 and 1.4, independent of constants.
    let mut c = DrawContext::new(Device::default(), props());
    let constructor = [1, 0, 3, 1, 1, 0, 3, 1, 0, 0, 0, 0].map(Some);
    assert_eq!(*c.default_combiner_bytes(), constructor);
    assert_eq!(*c.highlight_combiner_bytes(), constructor);
    assert_eq!(c.default_shade(), [1.0; 3]);
    c.init().unwrap();
    assert_eq!(*c.default_combiner_bytes(), [2, 0, 1, 1, 0, 1, 3, 1, 0, 0, 0, 0].map(Some));
    assert_eq!(*c.highlight_combiner_bytes(), [2, 2, 0, 1, 1, 2, 0, 1, 0, 0, 0, 0].map(Some));
    for stage in c.stages() {
        assert_eq!(stage.texture.as_ref().unwrap().token, 0xffff_ffff);
        assert!(stage.texture_known && stage.transform_known);
        assert_eq!(stage.coordinate_set, Some(0));
        assert_eq!(stage.wrap, [Some(-1); 2]);
        assert_eq!(stage.filter, [Some(-1); 2]);
        assert_eq!(stage.combiner, [1, 0, 3, 1, 1, 0, 3, 1, 0, 0, 1, 0]);
        assert_eq!(stage.saved_matrix, [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.]);
        assert_eq!(stage.transform, None);
    }
}

#[derive(Default)]
struct Device {
    commands: Vec<Command>,
    reads: usize,
    fail_info: bool,
    fail_next: bool,
    attempts: usize,
    fail_at: Option<usize>,
    stage_count: usize,
    normal_failure: bool,
}
impl Driver for Device {
    fn submit_returned(&mut self, command: Command) -> Result<NormalReturn, Error> {
        self.submit(command)?;
        Ok(NormalReturn { register: if self.normal_failure { 99 } else { 0 } })
    }
    fn info(&mut self) -> Result<DriverInfo, Error> {
        self.reads += 1;
        if self.fail_info {
            return Err(Error::Provider(64));
        }
        Ok(DriverInfo {
            stage_count: self.stage_count,
            bytes: [0xa7, 0xe2],
        })
    }
    fn submit(&mut self, command: Command) -> Result<(), Error> {
        let attempt = self.attempts;
        self.attempts += 1;
        if std::mem::take(&mut self.fail_next) || self.fail_at == Some(attempt) {
            return Err(Error::Provider(99));
        }
        self.commands.push(command);
        Ok(())
    }
}

#[test]
fn settled_literal_cache_order_suppression_and_normal_returns() {
    let mut c = DrawContext::new(Device { stage_count: 3, normal_failure: true, ..Device::default() }, props());
    c.init().unwrap();
    c.set_tex_coord(0, 0).unwrap();
    assert!(c.driver.commands.is_empty());
    c.set_tex_coord(3, 0).unwrap();
    c.set_tex_coord(3, 0).unwrap();
    c.set_texture(None, 0).unwrap();
    c.set_texture(None, 0).unwrap();
    assert_eq!(c.driver.commands, [Command::SelectStage(0), Command::TexCoord(3), Command::Texture(None), Command::Texture(None)]);
    c.driver.commands.clear();
    c.set_tex_wrap(1, 2, 2).unwrap();
    c.set_tex_filter(0, 4, 2).unwrap();
    c.properties.use_second_stage = true;
    c.set_tex_wrap(1, 2, 2).unwrap();
    c.set_tex_filter(0, 4, 2).unwrap();
    assert!(c.driver.commands.is_empty());
    c.set_tex_wrap(1, 3, 2).unwrap();
    assert_eq!(c.driver.commands, [Command::SelectStage(2), Command::TexParameter { parameter: 2, value: 1 }, Command::TexParameter { parameter: 3, value: 3 }]);
    c.driver.commands.clear();
    c.set_default_render_state_unilaterally().unwrap();
    assert_eq!(&c.driver.commands[..9], &[
        Command::SelectStage(0), Command::TexMatrix(None), Command::TexCoord(0),
        Command::SelectStage(1), Command::TexMatrix(None), Command::TexCoord(0),
        Command::SelectStage(2), Command::TexMatrix(None), Command::TexCoord(0),
    ]);
    assert_eq!(c.state_word(), 0x230a_8587);
    assert_eq!(c.stages[2].wrap, [Some(-1); 2]);
    assert_eq!(c.stages[0].filter, [Some(1), Some(7)]);
}

#[test]
fn settled_transform_duplicate_selection_and_suppressed_clear() {
    let mut c = context();
    let matrix = [2.0; 16];
    c.set_tex_transform(matrix, 0).unwrap();
    c.set_tex_transform(matrix, 0).unwrap();
    assert_eq!(c.driver.commands, [Command::SelectStage(0), Command::TexCoord(16), Command::SelectStage(0), Command::TexMatrix(Some(matrix)), Command::SelectStage(0), Command::TexMatrix(Some(matrix))]);
    assert_eq!(c.stages[0].saved_matrix[0], 1.0);
    c.driver.commands.clear();
    c.set_tex_coord(7, 1).unwrap();
    c.set_tex_transform(matrix, 1).unwrap();
    assert_eq!(c.stages[1].coordinate_set, Some(7));
    assert!(c.driver.commands.is_empty());
    c.clear_tex_transform(1).unwrap();
    assert_eq!(c.driver.commands, [Command::SelectStage(1), Command::TexMatrix(None)]);
    assert_eq!(c.stages[1].saved_matrix, matrix);
}

#[test]
fn settled_compact_whole_flags_counters_spills_and_raw_scale() {
    let mut c = context();
    c.model_view_valid = true;
    c.model_counter = 7;
    c.composed_counter = 40;
    c.composed = Transform { scale: 7.0, translation: [10., 20., 30.], flags: 2, ..Transform::IDENTITY };
    c.set_model_transform(Some(Transform { rotation: [[9.; 3]; 3], translation: [99.; 3], scale: 99., flags: 0 })).unwrap();
    assert_eq!((c.model_counter, c.composed_counter), (8, 40));
    assert_eq!(c.matrix, [7., 0., 0., 0., 0., 7., 0., 0., 0., 0., 7., 0., 10., 20., 30., 1.]);
    assert!(!c.model_view_valid);
    c.model_view_valid = true;
    c.set_model_matrix([1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.]).unwrap();
    assert_eq!(c.matrix, [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 10., 20., 30., 1.]);
    assert_eq!((c.model_counter, c.composed_counter), (8, 40));
    c.model_view_valid = true;
    c.composed.flags = 0;
    c.composed_counter = 255;
    let model = Transform { rotation: [[3., 0., 0.], [0., 3., 0.], [0., 0., 3.]], translation: [4., 5., 6.], scale: 5., flags: 0x80 };
    c.set_model_transform(Some(model)).unwrap();
    assert_eq!(c.composed, model);
    assert_eq!(c.composed_counter, 0);
    assert_eq!(c.matrix[0], 3.0);

    // Public DC-V19: Di=1+2^-24; x/y spill after scaling, z before.
    let view = Transform { rotation: [[1., 1., 0.]; 3], translation: [-3.; 3], scale: 3., flags: 1 };
    let model = Transform { translation: [1., f32::from_bits(0x3380_0000), 0.], flags: 1, ..Transform::IDENTITY };
    let composed = view.followed_by(model).unwrap();
    assert_eq!(composed.translation.map(f32::to_bits), [0x3480_0000, 0x3480_0000, 0]);
    // DC-V21: (1+2^-23)(1-2^-24)=1+2^-24-2^-47 -> Q=1.
    let left = Transform { scale: f32::from_bits(0x3f80_0001), flags: 2, ..Transform::IDENTITY };
    let right = Transform { scale: f32::from_bits(0x3f7f_ffff), flags: 2, ..Transform::IDENTITY };
    assert_eq!(left.followed_by(right).unwrap().scale.to_bits(), 0x3f80_0000);
}

#[test]
fn settled_numeric_qualification_and_original_alpha_reference() {
    assert_eq!(state::quantize(f32::from_bits(0x3f33_3333)), Ok(10));
    assert_eq!(state::quantize(0.5), Ok(8));
    let mut c = context();
    c.set_alpha_func(1, 0.7).unwrap();
    c.set_alpha_func(1, 0.7).unwrap();
    assert_eq!(c.driver.commands, vec![Command::AlphaFunc { function: 1, reference: 0.7 }; 2]);
    assert_eq!((c.word >> 21) & 15, 10);
    // Wider cancellation and nonfinite raw zero products are outside exact53,
    // not claimed as portable reproductions of public x87-64 vectors 18/23/24.
    let a = f64::from(2.0_f32.powi(80));
    assert_eq!(numeric::add(a, 1.0), Err(Error::UnsupportedNumeric));
    c.model_view_valid = true;
    c.composed = Transform::IDENTITY;
    let before = c.driver.commands.len();
    let mut raw = [0.; 16];
    raw[0] = f32::INFINITY;
    assert_eq!(c.set_model_matrix(raw), Err(Error::UnsupportedNumeric));
    assert_eq!(c.driver.commands.len(), before);
    c.set_numeric_control(NumericControl::Unqualified);
    assert_eq!(c.set_model_transform(Some(Transform::IDENTITY)), Err(Error::UnsupportedNumeric));
}
fn props() -> Properties {
    Properties {
        force_no_alpha_test: false,
        force_no_blend: false,
        use_second_stage: false,
        force_point: false,
        force_bilinear: false,
        force_repeat: false,
        show_boxes: false,
        bit30: false,
        model_threshold: 12.0,
        depth_offset: 2,
    }
}
fn context() -> DrawContext<Device> {
    let mut c = DrawContext::new(Device::default(), props());
    c.set_numeric_control(NumericControl::NearestEven64Gradual);
    c.init().unwrap();
    c
}
fn flags() -> DrawFlags {
    DrawFlags {
        bits: 0,
        highlight_color: 0,
        color: [1.0; 3],
        a: 1.0,
    }
}
fn slot(mode: u8, token: u64) -> TextureSlot {
    TextureSlot {
        texture: Some(Arc::new(Texture { token })),
        coordinate_set: 2,
        wrap: [2, 3],
        filter: [0, 1],
        mode,
        flags: 0,
        scale: 1.0,
    }
}
fn material() -> Material {
    Material {
        flags: 0x1a3,
        depth_func: 3,
        alpha_func: 1,
        alpha_threshold: 128,
        blend: [1, 0],
        textures: Vec::new(),
    }
}
fn mesh() -> Mesh {
    Mesh {
        format: 2,
        vertices: Some(0x1000),
        indices: Some(0x2000),
        groups: vec![Group {
            primitive: 0,
            first: 3,
            count: 6,
        }],
        material: material(),
    }
}
struct Cam {
    view: Transform,
    bias: AtomicI32,
}
impl Camera for Cam {
    fn view_transform(&self) -> Result<Transform, Error> {
        Ok(self.view)
    }
    fn projection(&self) -> Result<[[f32; 4]; 4], Error> {
        Ok([
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ])
    }
    fn set_depth_bias(&self, bias: i32) -> Result<(), Error> {
        self.bias.store(bias, Ordering::SeqCst);
        Ok(())
    }
}
fn camera(c: &mut DrawContext<Device>, view: Transform) -> Arc<Cam> {
    let camera = Arc::new(Cam {
        view,
        bias: AtomicI32::new(0),
    });
    c.set_camera(camera.clone()).unwrap();
    camera
}
struct Light;
impl Lighting for Light {
    fn model_light(&self, instance: u64) -> Result<Rgb, Error> {
        assert_eq!(instance, 42);
        Ok([0.5, 0.25, 1.0])
    }
}
struct Pixels;
impl View for Pixels {
    fn pixels_per_metre(&self) -> Result<f32, Error> {
        Ok(2.0)
    }
}
struct ModelInstance;
impl Instance for ModelInstance {
    fn token(&self) -> u64 {
        42
    }
    fn bounds(&self) -> Result<Bounds, Error> {
        Ok(Bounds {
            min: [0.0; 3],
            max: [2.0, 3.0, 4.0],
        })
    }
    fn effects_collection_bounds(&self) -> Result<Option<Bounds>, Error> {
        Ok(None)
    }
    fn transform(&self) -> Result<Transform, Error> {
        Ok(Transform::IDENTITY)
    }
}

#[test]
fn constructor_init_once_and_dc1_bytes() {
    let mut c = DrawContext::new(Device::default(), props());
    assert!(!c.is_initialized());
    assert_eq!(c.state_word(), 0);
    assert!(c.stages().is_empty());
    assert_eq!(c.highlights(), &[[0.0; 4]; 11]);
    assert_eq!(c.enable_depth_test(1), Err(Error::NotInitialized));
    c.init().unwrap();
    c.init().unwrap();
    assert_eq!(c.driver().reads, 1);
    assert_eq!(
        c.driver_info(),
        DriverInfo {
            stage_count: 0,
            bytes: [0xa7, 0xe2]
        }
    );
    assert_eq!(c.stages().len(), 2);
    assert_eq!(c.highlights()[9], [0.0, 0.7, 0.2, 0.3]);
    assert!(c.driver().commands.is_empty());
    let mut c = DrawContext::new(
        Device {
            stage_count: 4,
            ..Device::default()
        },
        props(),
    );
    c.init().unwrap();
    assert_eq!(c.stages().len(), 4);
}

#[test]
fn init_failure_is_retryable_and_does_not_publish_state() {
    let mut c = DrawContext::new(
        Device {
            fail_info: true,
            ..Device::default()
        },
        props(),
    );
    assert_eq!(c.init(), Err(Error::Provider(64)));
    assert!(!c.is_initialized());
    assert!(c.stages().is_empty());
    c.driver.fail_info = false;
    c.init().unwrap();
    assert!(c.is_initialized());
    assert_eq!(c.driver.reads, 2);
}

#[test]
fn cache_and_force_switches_and_keep_sentinels() {
    let mut c = context();
    c.enable_depth_test(1).unwrap();
    c.enable_depth_test(1).unwrap();
    c.enable_depth_test(-1).unwrap();
    assert_eq!(
        c.driver.commands,
        [Command::Capability {
            capability: 1,
            enabled: true
        }]
    );
    c.properties.force_no_alpha_test = true;
    c.properties.force_no_blend = true;
    c.enable_alpha_test(1).unwrap();
    c.enable_blend(1).unwrap();
    assert_eq!(c.state_word(), 4);
    c.enable_color_mask(1).unwrap();
    c.enable_depth_mask(1).unwrap();
    c.enable_cull_face(1).unwrap();
    c.set_depth_func(3).unwrap();
    c.set_blend_func(4, 5).unwrap();
    let word = c.state_word();
    let n = c.driver.commands.len();
    c.set_depth_func(-1).unwrap();
    c.set_blend_func(-1, -1).unwrap();
    assert_eq!(c.state_word(), word);
    assert_eq!(c.driver.commands.len(), n);
}

#[test]
fn alpha_is_unrounded_on_input_and_quantized_on_replay() {
    let mut c = context();
    c.set_alpha_func(1, 0.5).unwrap();
    assert_eq!(c.state_word() & 0x01e0_0000, 8 << 21);
    assert_eq!(
        c.driver.commands,
        [Command::AlphaFunc {
            function: 1,
            reference: 0.5
        }]
    );
    c.set_alpha_func(0, 0.0).unwrap();
    c.set_render_state(PackedState {
        value: (1 << 10) | (8 << 21),
        mask: (7 << 10) | (15 << 21),
    })
    .unwrap();
    assert!(c.driver.commands.contains(&Command::AlphaFunc {
        function: 1,
        reference: 8.0 / 15.0
    }));
    c.driver.commands.clear();
    c.set_alpha_func(1, 0.51).unwrap(); // same quantized value
    assert!(c.driver.commands.is_empty());
}

#[test]
fn packed_masks_and_published_default_value() {
    let input = PackInput {
        color_write: true,
        depth_write: true,
        depth_test: true,
        alpha_test: false,
        blend: false,
        textures: 0,
        cull: false,
        depth_func: 3,
        source_blend: 4,
        destination_blend: 5,
        alpha_func: 1,
        alpha_reference: 8.0 / 15.0,
        stage0_mode: 1,
        stage1_mode: 1,
    };
    assert_eq!(
        PackedState::pack(input).unwrap(),
        PackedState {
            value: 0x230a_8587,
            mask: 0xffff_ffff
        }
    );
    let partial = PackedState::pack(PackInput {
        depth_func: -1,
        source_blend: -1,
        destination_blend: -1,
        alpha_func: -1,
        alpha_reference: -1.0,
        stage0_mode: -1,
        stage1_mode: -1,
        ..input
    })
    .unwrap();
    assert_eq!(partial.mask, 0x1000_007f);
    let mut c = context();
    c.set_depth_func(6).unwrap();
    c.set_render_state(partial).unwrap();
    assert_eq!((c.state_word() >> 7) & 7, 6);
    c.set_default_render_state().unwrap();
    assert_eq!(c.state_word(), 0x230a_8587);
    c.push_packed_state(PackedState { value: 0, mask: 1 });
    c.set_default_render_state().unwrap();
    assert_eq!(c.state_word(), 0x230a_8586);
    assert!(c.pop_packed_state().is_some());
    assert!(c.pop_packed_state().is_none());
}

#[test]
fn packed_dispatch_order_and_mandatory_resets() {
    let mut c = context();
    c.properties.use_second_stage = true;
    c.set_default_render_state().unwrap();
    let observed: Vec<_> = c
        .driver
        .commands
        .iter()
        .filter(|command| !matches!(command, Command::SelectStage(_) | Command::TexParameter { .. }))
        .cloned()
        .collect();
    assert_eq!(
        observed,
        [
            Command::ColorMask(true),
            Command::DepthMask(true),
            Command::Capability {
                capability: 1,
                enabled: true
            },
            Command::DepthFunc(3),
            Command::AlphaFunc {
                function: 1,
                reference: 8.0 / 15.0
            },
            Command::BlendFunc {
                source: 4,
                destination: 5
            },
            Command::TexEnv(1),
            Command::TexEnv(1)
        ]
    );
    let matrix = Transform::IDENTITY.matrix();
    c.set_tex_transform(matrix, 0).unwrap();
    c.set_tex_transform(matrix, 1).unwrap();
    c.set_default_render_state().unwrap();
    assert_eq!(c.stages[0].transform, None);
    assert_eq!(c.stages[1].transform, None);
    assert_eq!(c.stages[0].wrap, [Some(3), Some(3)]);
    assert_eq!(c.stages[0].filter, [Some(1), Some(7)]);
    c.push_packed_state(PackedState {
        value: 0,
        mask: u32::MAX,
    });
    c.set_default_render_state_unilaterally().unwrap();
    assert_eq!(c.state_word(), 0x230a_8587); // unilateral ignores the stack
}

#[test]
fn texture_dependencies_suppressed_stage_and_higher_stage_behavior() {
    let mut c = context();
    c.enable_texture(1, 1).unwrap();
    assert_eq!(c.state_word() & 0x1000_0020, 0x1000_0020);
    assert_eq!(
        c.driver.commands,
        [
            Command::SelectStage(0),
            Command::Capability {
                capability: 5,
                enabled: true
            }
        ]
    );
    c.enable_texture(0, 1).unwrap();
    assert_eq!(c.state_word() & 0x1000_0020, 0x20);
    c.enable_texture(1, 1).unwrap();
    c.enable_texture(0, 0).unwrap();
    assert_eq!(c.state_word() & 0x1000_0020, 0);
    c.driver.commands.clear();
    c.properties.use_second_stage = true;
    c.enable_texture(1, 5).unwrap();
    c.enable_texture(0, 5).unwrap();
    assert_eq!(
        c.driver.commands,
        [
            Command::SelectStage(5),
            Command::Capability {
                capability: 5,
                enabled: true
            }
        ]
    );
}

#[test]
fn suppressed_stage_records_all_resources_and_second_pass_replays_them() {
    let mut c = context();
    let tex = Arc::new(Texture { token: 77 });
    let weak = Arc::downgrade(&tex);
    c.set_texture(Some(tex.clone()), 1).unwrap();
    drop(tex);
    c.clear_tex_transform(1).unwrap();
    c.set_tex_coord(7, 1).unwrap();
    c.set_tex_wrap(2, 3, 1).unwrap();
    c.set_tex_filter(0, 1, 1).unwrap();
    c.set_tex_env_mode(1, 1).unwrap();
    c.enable_texture(1, 1).unwrap();
    assert!(weak.upgrade().is_some());
    assert!(!c
        .driver
        .commands
        .iter()
        .any(|command| matches!(command, Command::SelectStage(1))));
    c.driver.commands.clear();
    c.prepare_second_pass().unwrap();
    assert_eq!(c.stages[0], c.stages[1]);
    assert_eq!((c.state_word() >> 7) & 7, 2);
    assert_eq!(c.state_word() & 8, 0);
    assert_eq!((c.state_word() >> 13) & 255, 2 << 4);
    assert_ne!(c.state_word() & 16, 0);
    // DIFFERS DC5/DC6: binding is an explicit-stage call without selection.
    assert!(matches!(c.driver.commands.first(), Some(Command::Texture(_))));
    assert!(c.driver.commands.contains(&Command::BlendFunc {
        source: 0,
        destination: 2
    }));
    drop(c);
    assert!(weak.upgrade().is_none());
}

#[test]
fn texture_overrides_and_transform_clear_cache() {
    let mut c = context();
    c.properties.force_point = true;
    c.properties.force_bilinear = true;
    c.properties.force_repeat = true;
    c.set_tex_wrap(2, 2, 0).unwrap();
    c.set_tex_filter(7, 7, 0).unwrap();
    // DIFFERS DC7: requested cache values survive driver force overrides.
    assert_eq!(c.stages[0].wrap, [Some(2); 2]);
    assert_eq!(c.stages[0].filter, [Some(7); 2]);
    c.properties.force_point = false;
    c.set_tex_filter(0, 0, 0).unwrap();
    assert_eq!(c.stages[0].filter, [Some(0), Some(0)]);
    c.set_tex_transform(Transform::IDENTITY.matrix(), 1).unwrap();
    assert_eq!(c.stages[1].coordinate_set, Some(0));
    assert!(c.stages[1].transform.is_some());
    c.clear_tex_transform(1).unwrap();
    assert_eq!(c.stages[1].coordinate_set, Some(0));
    let count = c.driver.commands.len();
    c.clear_tex_transform(1).unwrap();
    assert_eq!(c.driver.commands.len(), count);
    c.set_tex_combiner(&[7; 12], 1).unwrap();
    assert_eq!((c.state_word() >> 29) & 7, 4);
}

#[test]
fn transforms_compose_numerically_and_restore_plain_view() {
    let mut c = context();
    let view = Transform {
        translation: [1.0, 2.0, 3.0],
        scale: 2.0,
        flags: 3,
        ..Transform::IDENTITY
    };
    let camera = camera(&mut c, view);
    assert_eq!(c.driver.commands[0], Command::MatrixMode(1));
    assert_eq!(
        c.driver.commands[1],
        Command::LoadMatrix([1.0, 5.0, 9.0, 13.0, 2.0, 6.0, 10.0, 14.0, 3.0, 7.0, 11.0, 15.0, 4.0, 8.0, 12.0, 16.0])
    );
    c.set_model_transform(Some(Transform {
        translation: [1.0, 0.0, 0.0],
        flags: 1,
        ..Transform::IDENTITY
    }))
    .unwrap();
    assert_eq!(c.composed.translation, [3.0, 2.0, 3.0]);
    assert_eq!(c.composed.scale, 2.0);
    assert_eq!(
        c.matrix,
        [2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 3.0, 2.0, 3.0, 1.0]
    );
    assert!(!c.model_view_valid());
    c.set_model_transform(None).unwrap();
    assert_eq!(c.composed, view);
    assert!(c.model_view_valid());
    let count = c.driver.commands.len();
    c.set_model_transform(None).unwrap();
    assert_eq!(c.driver.commands.len(), count);
    c.set_depth_offset(4).unwrap();
    assert_eq!(camera.bias.load(Ordering::SeqCst), 8);
    let count = c.driver.commands.len();
    c.set_depth_offset(4).unwrap();
    assert_eq!(c.driver.commands.len(), count);
}

#[test]
fn noncommuting_rotations_and_flagged_identity_fields() {
    let a = Transform {
        rotation: [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        translation: [10.0, 20.0, 30.0],
        scale: 2.0,
        flags: 7,
    };
    let b = Transform {
        rotation: [[1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]],
        translation: [1.0, 2.0, 3.0],
        scale: 3.0,
        flags: 7,
    };
    let c = a.followed_by(b).unwrap();
    assert_eq!(c.rotation, [[0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
    assert_eq!(c.translation, [6.0, 22.0, 36.0]);
    assert_eq!(c.scale, 6.0);
    // DIFFERS DC8: zero flags do not replace stored matrix payload by identity.
    assert_eq!(Transform::ZERO.matrix()[0], 0.0);
}

#[test]
fn shade_sources_tint_and_reset_once() {
    let mut c = context();
    c.set_default_shade([0.5, 0.25, 1.0]);
    c.set_shade([0.2, 0.4, 0.6]).unwrap();
    assert_eq!(
        c.driver.commands.last(),
        Some(&Command::ColorMultiplier([0.1, 0.1, 0.6]))
    );
    c.reset_shade().unwrap();
    let count = c.driver.commands.len();
    c.reset_shade().unwrap();
    assert_eq!(c.driver.commands.len(), count);
    c.set_lighting(Some(Arc::new(Light)));
    c.set_model_shade(42, Some([0.4, 0.8, 0.5])).unwrap();
    assert_eq!(
        c.driver.commands.last(),
        Some(&Command::ColorMultiplier([0.2, 0.2, 0.5]))
    );
}

#[test]
fn material_fade_grey_depth_bits_and_texture_matrix() {
    let mut c = context();
    camera(
        &mut c,
        Transform {
            translation: [4.0, 6.0, 0.0],
            flags: 1,
            ..Transform::IDENTITY
        },
    );
    let mut m = material();
    m.textures.push(slot(1, 5));
    m.textures[0].flags = 1;
    m.textures[0].scale = 2.0;
    c.set_material_state(
        &m,
        DrawFlags {
            bits: 2,
            a: 1.0,
            ..flags()
        },
    )
    .unwrap();
    assert_ne!(c.state_word() & 16, 0);
    assert_eq!((c.state_word() >> 13) & 255, 0x54);
    assert!(!c.driver.commands.contains(&Command::AlphaMultiplier(1.0))); // fade at 1 still blends, no multiplier
    let matrix = c.stages[0].transform.unwrap();
    assert_eq!([matrix[0], matrix[5], matrix[12], matrix[13]], [0.5, -0.5, -2.0, 3.0]);
    c.set_material_state(
        &m,
        DrawFlags {
            bits: 2,
            a: 0.5,
            ..flags()
        },
    )
    .unwrap();
    assert_eq!((c.state_word() >> 21) & 15, 4);
    assert!(c.driver.commands.contains(&Command::AlphaMultiplier(0.5)));
    c.driver.commands.clear();
    c.set_material_state(
        &m,
        DrawFlags {
            bits: 10,
            a: 0.3,
            ..flags()
        },
    )
    .unwrap();
    assert!(c.driver.commands.contains(&Command::ColorMultiplier([0.3; 3])));
    assert!(!c
        .driver
        .commands
        .iter()
        .any(|command| matches!(command, Command::AlphaMultiplier(_))));
    c.set_material_state(
        &m,
        DrawFlags {
            bits: 1 << 5,
            ..flags()
        },
    )
    .unwrap();
    assert_eq!(c.state_word() & 6, 4);
    assert_eq!((c.state_word() >> 7) & 7, 4);
    c.set_material_state(
        &m,
        DrawFlags {
            bits: 1 << 4,
            ..flags()
        },
    )
    .unwrap();
    assert_eq!(c.state_word() & 6, 0);
}

#[test]
fn primitive_mesh_order_and_index_arithmetic() {
    let mut c = context();
    c.draw_prims(1, 7, 9, 0x4321).unwrap();
    assert_eq!(
        c.driver.commands,
        [
            Command::InterleavedArrays {
                format: 7,
                stride: 0,
                data: 0x4321
            },
            Command::DrawArrays {
                primitive: 1,
                first: 0,
                count: 9
            }
        ]
    );
    c.driver.commands.clear();
    let mut mesh = mesh();
    mesh.groups.push(Group {
        primitive: 2,
        first: 8,
        count: 3,
    });
    c.render_mesh(&mesh).unwrap();
    assert_eq!(
        c.driver.commands,
        [
            Command::InterleavedArrays {
                format: 2,
                stride: 0,
                data: 0x1000
            },
            Command::DrawElements {
                primitive: 0,
                count: 6,
                index_type: 3,
                indices: 0x2006
            },
            Command::DrawElements {
                primitive: 2,
                count: 3,
                index_type: 3,
                indices: 0x2010
            }
        ]
    );
    c.driver.commands.clear();
    mesh.vertices = None;
    c.render_mesh(&mesh).unwrap();
    assert!(c.driver.commands.is_empty());
    mesh.vertices = Some(0x1000);
    mesh.indices = None;
    c.render_mesh(&mesh).unwrap();
    assert_eq!(
        c.driver.commands[1],
        Command::DrawArrays {
            primitive: 0,
            first: 3,
            count: 6
        }
    );
}

#[test]
fn model_culling_highlight_skip_and_draw_pass_order() {
    let mut c = context();
    c.properties.bit30 = true;
    c.set_view(Some(Arc::new(Pixels)));
    c.render_model_instance(&ModelInstance, &[mesh()], flags(), false)
        .unwrap();
    assert!(c.driver.commands.is_empty()); // 4 metres * 2 pixels < 12
    c.properties.model_threshold = 8.0;
    camera(&mut c, Transform::IDENTITY);
    c.set_lighting(Some(Arc::new(Light)));
    let mut m = mesh();
    m.material.textures = vec![slot(0, 1), slot(1, 2)];
    c.driver.commands.clear();
    c.render_model_instance(&ModelInstance, &[m], flags(), false).unwrap();
    let draw_positions: Vec<_> = c
        .driver
        .commands
        .iter()
        .enumerate()
        .filter_map(|(index, command)| matches!(command, Command::DrawElements { .. }).then_some(index))
        .collect();
    assert_eq!(draw_positions.len(), 2);
    assert!(c.driver.commands[draw_positions[0] + 1..draw_positions[1]].contains(&Command::DepthFunc(2)));
    assert_eq!(c.driver.commands.last(), Some(&Command::AlphaMultiplier(1.0)));
    c.driver.commands.clear();
    c.render_model_instance(&ModelInstance, &[], DrawFlags { bits: 8, ..flags() }, true)
        .unwrap();
    assert!(c.driver.commands.is_empty());
}

#[test]
fn bounding_box_geometry_face_order_wrapping_and_draw_state() {
    let bounds = Bounds {
        min: [0.0; 3],
        max: [1.0, 2.0, 3.0],
    };
    let v = boxes::bounding_box_vertices(bounds, [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(v.len(), 24);
    assert_eq!(
        v[..4].iter().map(|v| v.position).collect::<Vec<_>>(),
        [[0.0, 2.0, 0.0], [1.0, 2.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0]]
    );
    assert_eq!(v[0].bgra, [0, 0, 155, 255]);
    assert_eq!(v[4].bgra, [0, 0, 255, 255]);
    let mut c = context();
    c.draw_bounding_box(bounds, [1.0; 4]).unwrap();
    assert_eq!(c.state_word() & 0x1e1f_e3ff, 0x020a_8197 & 0x1e1f_e3ff);
    assert!(
        matches!(&c.driver.commands[c.driver.commands.len()-2], Command::InterleavedOwned {format:1,stride:0,vertices} if vertices.len()==24)
    );
    assert_eq!(
        c.driver.commands.last(),
        Some(&Command::DrawArrays {
            primitive: 6,
            first: 0,
            count: 24
        })
    );
}

#[test]
fn errors_do_not_fabricate_success_cache_or_missing_owned_constants() {
    let mut c = context();
    c.driver.fail_next = true;
    assert_eq!(c.enable_depth_test(1), Err(Error::Provider(99)));
    assert_eq!(c.state_word(), 0);
    c.enable_depth_test(1).unwrap();
    assert_eq!(c.state_word(), 4);
    assert_eq!(c.set_model_transform(None), Err(Error::MissingCamera));
    assert_eq!(c.set_model_shade(42, None), Ok(())); // DC8 null-manager fallback
    assert_eq!(c.set_tex_coord(0, 2), Err(Error::InvalidStage));
    assert_eq!(c.set_alpha_func(0, f32::NAN), Err(Error::InvalidValue));
    c.set_index_buffer(usize::MAX);
    assert_eq!(c.draw_indexed(0, 1, 1), Err(Error::AddressOverflow));
    let count = c.driver.commands.len();
    assert_eq!(c.begin(), Err(Error::Gap(ContractGap::BeginEnd)));
    assert_eq!(c.end(), Err(Error::Gap(ContractGap::BeginEnd)));
    assert_eq!(c.driver.commands.len(), count);
    c.set_render_state_highlight(0).unwrap(); // DIFFERS DC4: full record settled
    let mut m = material();
    m.textures.push(slot(4, 0));
    c.set_material_state(&m, flags()).unwrap();
    assert!(c.driver.commands.contains(&Command::Combiner(vec![2, 0, 1, 1, 0, 1, 3, 1, 0, 0, 0, 0])));
}

#[test]
fn partial_packed_mask_does_not_apply_force_switch_to_excluded_bits() {
    let mut c = context();
    c.enable_alpha_test(1).unwrap();
    c.enable_blend(1).unwrap();
    c.properties.force_no_alpha_test = true;
    c.properties.force_no_blend = true;
    c.set_render_state(PackedState { value: 1, mask: 1 }).unwrap();
    assert_eq!(c.state_word() & 0x19, 0x19);
    c.enable_alpha_test(1).unwrap();
    c.enable_blend(1).unwrap();
    assert_eq!(c.state_word() & 0x19, 1);
}

#[test]
fn remaining_second_pass_modes_and_hardware_stage() {
    for (mode, expected_blend, expected_pair, expected_env) in
        [(0, false, 0x54, 0), (2, true, 0x54, 0), (3, true, 0x54, 1)]
    {
        let mut c = context();
        let mut m = material();
        m.flags |= 0x10;
        m.blend = [4, 5];
        m.textures = vec![slot(1, 1), slot(mode, 2)];
        c.set_material_state(&m, flags()).unwrap();
        c.prepare_second_pass().unwrap();
        assert_eq!(c.state_word() & 16 != 0, expected_blend);
        assert_eq!((c.state_word() >> 13) & 255, expected_pair);
        assert_eq!((c.state_word() >> 25) & 7, expected_env);
    }
    let mut c = context();
    c.properties.use_second_stage = true;
    camera(&mut c, Transform::IDENTITY);
    c.set_lighting(Some(Arc::new(Light)));
    let mut m = mesh();
    m.material.textures = vec![slot(1, 1), slot(1, 2)];
    c.driver.commands.clear();
    c.render_model_instance(&ModelInstance, &[m], flags(), false).unwrap();
    assert_eq!(
        c.driver
            .commands
            .iter()
            .filter(|cmd| matches!(cmd, Command::DrawElements { .. }))
            .count(),
        1
    );
    assert!(c.driver.commands.contains(&Command::SelectStage(1)));
}

#[test]
fn per_field_device_failure_preserves_completed_prefix_and_retry() {
    let mut c = context();
    c.driver.fail_at = Some(c.driver.attempts + 2); // selection, mag succeeds, min fails
    assert_eq!(c.set_tex_filter(1, 7, 0), Err(Error::Provider(99)));
    // Nonreturn boundary preserves completed calls, but pair cache is after both.
    assert_eq!(c.stages[0].filter, [Some(-1), Some(-1)]);
    c.driver.fail_at = None;
    c.driver.commands.clear();
    c.set_tex_filter(1, 7, 0).unwrap();
    assert_eq!(
        c.driver.commands,
        [
            Command::SelectStage(0),
            Command::TexParameter { parameter: 0, value: 1 },
            Command::TexParameter { parameter: 1, value: 7 }
        ]
    );
    let first = Arc::new(Texture { token: 1 });
    let weak = Arc::downgrade(&first);
    c.set_texture(Some(first.clone()), 1).unwrap();
    drop(first);
    c.properties.use_second_stage = true;
    c.driver.fail_at = Some(c.driver.attempts); // binding has no selection
    assert_eq!(
        c.set_texture(Some(Arc::new(Texture { token: 2 })), 1),
        Err(Error::Provider(99))
    );
    assert_eq!(c.stages[1].texture.as_ref().unwrap().token, 1);
    c.driver.fail_at = None;
    c.properties.use_second_stage = false;
    c.set_texture(None, 1).unwrap();
    assert!(weak.upgrade().is_none());
}

#[test]
fn model_failure_preserves_prefix_without_exception_cleanup() {
    let mut c = context();
    camera(&mut c, Transform::IDENTITY);
    c.set_default_shade([0.2, 0.3, 0.4]);
    c.driver.commands.clear();
    // Model matrix, constant shade, then fail the first material write.
    c.driver.fail_at = Some(c.driver.attempts + 2);
    let result = c.render_model_instance(
        &ModelInstance,
        &[mesh()],
        DrawFlags {
            bits: 4,
            color: [0.6, 0.7, 0.8],
            ..flags()
        },
        false,
    );
    assert_eq!(result, Err(Error::Provider(99)));
    // DIFFERS DC6: cleanup belongs only to the normal loop end.
    assert_eq!(c.driver.commands.last(), Some(&Command::ColorMultiplier([0.6, 0.7, 0.8])));
    assert!(c.shade_override());
    assert!(!c
        .driver
        .commands
        .iter()
        .any(|cmd| matches!(cmd, Command::DrawElements { .. })));
}

struct EffectsInstance;
impl Instance for EffectsInstance {
    fn token(&self) -> u64 {
        panic!("debug box must not ask for shading")
    }
    fn bounds(&self) -> Result<Bounds, Error> {
        Ok(Bounds {
            min: [0.0; 3],
            max: [1.0; 3],
        })
    }
    fn effects_collection_bounds(&self) -> Result<Option<Bounds>, Error> {
        Ok(Some(Bounds {
            min: [2.0; 3],
            max: [4.0; 3],
        }))
    }
    fn transform(&self) -> Result<Transform, Error> {
        panic!("debug box must not set model transform")
    }
}

#[test]
fn debug_boxes_precede_culling_and_do_not_draw_model() {
    let mut c = context();
    c.properties.show_boxes = true;
    c.properties.bit30 = true; // no View is needed in this early path
    c.render_model_instance(&EffectsInstance, &[mesh()], flags(), false)
        .unwrap();
    let arrays: Vec<_> = c
        .driver
        .commands
        .iter()
        .filter_map(|cmd| match cmd {
            Command::InterleavedOwned { vertices, .. } => Some(vertices),
            _ => None,
        })
        .collect();
    assert_eq!(arrays.len(), 2);
    assert_eq!(arrays[0][4].bgra, [0, 0, 255, 255]);
    assert_eq!(arrays[1][4].bgra, [255, 0, 0, 128]);
    assert!(!c
        .driver
        .commands
        .iter()
        .any(|cmd| matches!(cmd, Command::DrawElements { .. })));
}

#[test]
fn unknown_default_values_cannot_be_used_to_fabricate_second_pass() {
    let mut c = context();
    assert_eq!(c.default_packed_state(), DEFAULT_STATE);
    assert_eq!(c.highlight_packed_state(), HIGHLIGHT_STATE);
    assert_eq!(c.default_combiner_bytes()[3], Some(1));
    assert_eq!(c.default_combiner_bytes()[4], Some(0));
    assert_eq!(c.highlight_combiner_bytes()[6], Some(0));
    c.enable_texture(1, 1).unwrap();
    c.prepare_second_pass().unwrap(); // DIFFERS DC5: defaults are fully settled
    c.clear_tex_transform(1).unwrap();
    c.set_tex_coord(2, 1).unwrap();
    c.set_tex_wrap(3, 3, 1).unwrap();
    c.set_tex_filter(1, 7, 1).unwrap();
    c.prepare_second_pass().unwrap();
    assert!(c.stages[1].texture_known);
    c.set_texture(None, 1).unwrap(); // explicitly unbound is different from unknown
    c.prepare_second_pass().unwrap();
    assert!(c.stages[0].texture_known);
    assert!(c.stages[0].texture.is_none());
}

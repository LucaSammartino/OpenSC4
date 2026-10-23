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
    assert_eq!(c.model_transform(), Transform::IDENTITY);
    assert_eq!(c.composed_transform(), Transform::IDENTITY);
    c.init().unwrap();
    assert_eq!(
        *c.default_combiner_bytes(),
        [2, 0, 1, 1, 0, 1, 3, 1, 0, 0, 0, 0].map(Some)
    );
    assert_eq!(
        *c.highlight_combiner_bytes(),
        [2, 2, 0, 1, 1, 2, 0, 1, 0, 0, 0, 0].map(Some)
    );
    for stage in c.stages() {
        assert_eq!(stage.texture.as_ref().unwrap().token, 0xffff_ffff);
        assert!(stage.texture_known && stage.transform_known);
        assert_eq!(stage.coordinate_set, Some(0));
        assert_eq!(stage.wrap, [Some(-1); 2]);
        assert_eq!(stage.filter, [Some(-1); 2]);
        assert_eq!(stage.combiner, [1, 0, 3, 1, 1, 0, 3, 1, 0, 0, 1, 0]);
        assert_eq!(
            stage.saved_matrix,
            [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.]
        );
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
    fog_color_pointers: Vec<usize>,
    fog_scalar_pointers: Vec<(u32, usize)>,
}
impl Driver for Device {
    fn fog_color(&mut self, color: &Rgba) -> Result<NormalReturn, Error> {
        self.fog_color_pointers.push(color.as_ptr() as usize);
        self.submit_returned(Command::FogColor(*color))
    }
    fn fog_scalar(&mut self, selector: u32, value: &f32) -> Result<NormalReturn, Error> {
        self.fog_scalar_pointers.push((selector, value as *const f32 as usize));
        self.submit_returned(Command::FogScalar(selector, *value))
    }
    fn submit_returned(&mut self, command: Command) -> Result<NormalReturn, Error> {
        self.submit(command)?;
        Ok(NormalReturn {
            register: if self.normal_failure { 99 } else { 0 },
        })
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
    let mut c = DrawContext::new(
        Device {
            stage_count: 3,
            normal_failure: true,
            ..Device::default()
        },
        props(),
    );
    c.init().unwrap();
    c.set_tex_coord(0, 0).unwrap();
    assert!(c.driver.commands.is_empty());
    c.set_tex_coord(3, 0).unwrap();
    c.set_tex_coord(3, 0).unwrap();
    c.set_texture(None, 0).unwrap();
    c.set_texture(None, 0).unwrap();
    assert_eq!(
        c.driver.commands,
        [
            Command::SelectStage(0),
            Command::TexCoord(3),
            Command::Texture(None, 0),
            Command::Texture(None, 0)
        ]
    );
    c.driver.commands.clear();
    c.set_tex_wrap(1, 2, 2).unwrap();
    c.set_tex_filter(0, 4, 2).unwrap();
    c.properties.use_second_stage = true;
    c.set_tex_wrap(1, 2, 2).unwrap();
    c.set_tex_filter(0, 4, 2).unwrap();
    assert!(c.driver.commands.is_empty());
    c.set_tex_wrap(1, 3, 2).unwrap();
    assert_eq!(
        c.driver.commands,
        [
            Command::SelectStage(2),
            Command::TexParameter { parameter: 2, value: 1 },
            Command::TexParameter { parameter: 3, value: 3 }
        ]
    );
    c.driver.commands.clear();
    c.set_default_render_state_unilaterally().unwrap();
    assert_eq!(
        &c.driver.commands[..9],
        &[
            Command::SelectStage(0),
            Command::TexMatrix(None),
            Command::TexCoord(0),
            Command::SelectStage(1),
            Command::TexMatrix(None),
            Command::TexCoord(0),
            Command::SelectStage(2),
            Command::TexMatrix(None),
            Command::TexCoord(0),
        ]
    );
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
    assert_eq!(
        c.driver.commands,
        [
            Command::SelectStage(0),
            Command::TexCoord(16),
            Command::SelectStage(0),
            Command::TexMatrix(Some(matrix)),
            Command::SelectStage(0),
            Command::TexMatrix(Some(matrix))
        ]
    );
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
    c.composed = Transform {
        scale: 7.0,
        translation: [10., 20., 30.],
        flags: 2,
        ..Transform::IDENTITY
    };
    c.set_model_transform(Some(Transform {
        rotation: [[9.; 3]; 3],
        translation: [99.; 3],
        scale: 99.,
        flags: 0,
    }))
    .unwrap();
    assert_eq!((c.model_counter, c.composed_counter), (8, 40));
    assert_eq!(
        c.matrix,
        [7., 0., 0., 0., 0., 7., 0., 0., 0., 0., 7., 0., 10., 20., 30., 1.]
    );
    assert!(!c.model_view_valid);
    c.model_view_valid = true;
    c.set_model_matrix([1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.])
        .unwrap();
    assert_eq!(
        c.matrix,
        [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 10., 20., 30., 1.]
    );
    assert_eq!((c.model_counter, c.composed_counter), (8, 40));
    c.model_view_valid = true;
    c.composed.flags = 0;
    c.composed_counter = 255;
    let model = Transform {
        rotation: [[3., 0., 0.], [0., 3., 0.], [0., 0., 3.]],
        translation: [4., 5., 6.],
        scale: 5.,
        flags: 0x80,
    };
    c.set_model_transform(Some(model)).unwrap();
    assert_eq!(c.composed, model);
    assert_eq!(c.composed_counter, 0);
    assert_eq!(c.matrix[0], 3.0);

    // Public DC-V19: Di=1+2^-24; x/y spill after scaling, z before.
    let view = Transform {
        rotation: [[1., 1., 0.]; 3],
        translation: [-3.; 3],
        scale: 3.,
        flags: 1,
    };
    let model = Transform {
        translation: [1., f32::from_bits(0x3380_0000), 0.],
        flags: 1,
        ..Transform::IDENTITY
    };
    let composed = view.followed_by(model).unwrap();
    assert_eq!(composed.translation.map(f32::to_bits), [0x3480_0000, 0x3480_0000, 0]);
    // DC-V21: (1+2^-23)(1-2^-24)=1+2^-24-2^-47 -> Q=1.
    let left = Transform {
        scale: f32::from_bits(0x3f80_0001),
        flags: 2,
        ..Transform::IDENTITY
    };
    let right = Transform {
        scale: f32::from_bits(0x3f7f_ffff),
        flags: 2,
        ..Transform::IDENTITY
    };
    assert_eq!(left.followed_by(right).unwrap().scale.to_bits(), 0x3f80_0000);
}

#[test]
fn settled_numeric_qualification_and_original_alpha_reference() {
    assert_eq!(state::quantize(f32::from_bits(0x3f33_3333)), Ok(10));
    assert_eq!(state::quantize(0.5), Ok(8));
    let mut c = context();
    c.set_alpha_func(1, 0.7).unwrap();
    c.set_alpha_func(1, 0.7).unwrap();
    assert_eq!(
        c.driver.commands,
        vec![
            Command::AlphaFunc {
                function: 1,
                reference: 0.7
            };
            2
        ]
    );
    assert_eq!((c.word >> 21) & 15, 10);
    // An x87-64 operation rounds A+1 back to A, without rejecting the input.
    let a = f64::from(2.0_f32.powi(80));
    assert_eq!(numeric::add(a, 1.0), numeric::add(a, 0.0));
    c.model_view_valid = true;
    c.composed = Transform::IDENTITY;
    let before = c.driver.commands.len();
    let mut raw = [0.; 16];
    raw[0] = f32::INFINITY;
    c.set_model_matrix(raw).unwrap();
    assert!(c.matrix[3].is_nan());
    assert_eq!(c.driver.commands.len(), before + 1);
    c.set_numeric_control(NumericControl::Unqualified);
    assert_eq!(
        c.set_model_transform(Some(Transform::IDENTITY)),
        Err(Error::UnsupportedNumeric)
    );
}

#[test]
fn settled_returned_load_failures_retry_and_lifecycle() {
    let mut c = context();
    camera(
        &mut c,
        Transform {
            flags: 1,
            ..Transform::IDENTITY
        },
    );
    c.composed = Transform {
        flags: 1,
        ..Transform::IDENTITY
    };
    c.model_view_valid = true;
    c.composed_counter = 40;
    c.model_counter = 7;
    c.driver.commands.clear();
    c.driver.normal_failure = true;
    c.set_model_transform(Some(Transform {
        flags: 1,
        ..Transform::IDENTITY
    }))
    .unwrap();
    assert_eq!(
        (c.model_counter, c.composed_counter, c.model_view_valid),
        (8, 41, false)
    );
    c.set_model_transform(Some(Transform {
        flags: 1,
        ..Transform::IDENTITY
    }))
    .unwrap();
    assert_eq!((c.model_counter, c.composed_counter), (9, 43));
    assert_eq!(c.driver.commands.len(), 2);
    assert!(c
        .driver
        .commands
        .iter()
        .all(|command| matches!(command, Command::LoadMatrix(_))));
    c.driver.commands.clear();
    c.model_view_valid = true;
    c.model_counter = 7;
    c.composed_counter = 40;
    let raw = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.];
    c.set_model_matrix(raw).unwrap();
    assert_eq!(
        (c.model_counter, c.composed_counter, c.model_view_valid),
        (7, 40, false)
    );
    c.set_model_matrix(raw).unwrap();
    assert_eq!((c.model_counter, c.composed_counter), (7, 41));
    assert_eq!(c.driver.commands, [Command::LoadMatrix(raw), Command::LoadMatrix(raw)]);
    let stages = c.stages.clone();
    c.driver.commands.clear();
    c.shutdown();
    c.shutdown();
    assert!(!c.is_initialized());
    assert!(!c.properties_present);
    assert_eq!(c.stages, stages);
    assert!(c.driver.commands.is_empty());
    assert!(context().model_view_matrix().is_none());
}

#[test]
fn settled_shade_checked_fallback_grey_and_full_material_matrix() {
    struct Declined;
    impl Lighting for Declined {
        fn model_light(&self, _: u64) -> Result<Rgb, Error> {
            panic!("use checked AL result")
        }
        fn checked_model_light(&self, _: u64) -> Result<Option<Rgb>, Error> {
            Ok(None)
        }
        fn default_light(&self) -> Result<Rgb, Error> {
            Ok([0.5, 0.25, 1.0])
        }
    }
    let mut c = context();
    c.set_lighting(Some(Arc::new(Declined)));
    c.set_lighting_enabled(true).unwrap();
    c.set_lighting_enabled(false).unwrap();
    assert_eq!(c.default_shade(), [0.5, 0.25, 1.]);
    c.set_model_shade(1, Some([0.5, 2., 0.25])).unwrap();
    assert_eq!(
        c.driver.commands.last(),
        Some(&Command::ColorMultiplier([0.25, 0.5, 0.25]))
    );
    assert!(c.shade_override());
    c.reset_shade().unwrap();
    assert!(!c.shade_override());
    c.driver.commands.clear();
    c.set_material_state(
        &material(),
        DrawFlags {
            bits: 2,
            a: 0.25,
            ..flags()
        },
    )
    .unwrap();
    c.set_material_state(
        &material(),
        DrawFlags {
            bits: 10,
            a: 0.75,
            ..flags()
        },
    )
    .unwrap();
    let multipliers: Vec<_> = c
        .driver
        .commands
        .iter()
        .filter(|command| matches!(command, Command::AlphaMultiplier(_)))
        .collect();
    assert_eq!(multipliers, [&Command::AlphaMultiplier(0.25)]);
    assert!(!c.shade_override()); // grey never writes override
    c.composed.translation = [3., -4., 0.];
    let mut m = material();
    m.flags |= 0x20;
    let mut texture = slot(4, 3);
    texture.flags = 1;
    texture.scale = 2.0;
    m.textures = vec![texture];
    c.set_material_state(&m, flags()).unwrap();
    assert!(c.driver.commands.contains(&Command::TexMatrix(Some([
        0.5, 0., 0., 0., 0., -0.5, 0., 0., 0., 0., 0., 0., -1.5, -2., 0., 0.
    ]))));
    assert_eq!(numeric::material_threshold(1).to_bits(), 0x3b80_8081);
    assert_eq!(numeric::material_threshold(128).to_bits(), 0x3f00_8081);
    assert_eq!(numeric::material_threshold(255).to_bits(), 0x3f80_0000);
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
    assert_eq!(c.highlights(), &[[1.0; 4]; 11]); // published constructor vector
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
                                        // DIFFERS DC8: direct equality compares original reference, not just q.
    assert_eq!(
        c.driver.commands,
        [Command::AlphaFunc {
            function: 1,
            reference: 0.51
        }]
    );
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
    c.properties.use_second_stage = false; // higher-stage enable bypasses boolean 11
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
    assert!(matches!(c.driver.commands.first(), Some(Command::Texture(_, 0))));
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
    let matrix = c
        .driver
        .commands
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::TexMatrix(Some(matrix)) => Some(*matrix),
            _ => None,
        })
        .unwrap();
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
    c.set_alpha_func(0, f32::NAN).unwrap(); // native packing does not clamp
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
    assert!(c
        .driver
        .commands
        .contains(&Command::Combiner(vec![2, 0, 1, 1, 0, 1, 3, 1, 0, 0, 0, 0], 0)));
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
    assert_eq!(
        c.driver.commands.last(),
        Some(&Command::ColorMultiplier([0.6, 0.7, 0.8]))
    );
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

#[test]
fn closure_dc_v17_v18_v20_v23_v24() {
    let mut c = context();
    c.model_view_valid = true;
    c.composed = Transform {
        rotation: [[2., 0., 0.], [0., 2., 0.], [0., 0., 2.]],
        translation: [1., 2., 3.],
        scale: 5.,
        flags: 0x80,
    };
    c.composed_counter = 40;
    c.model_counter = 7;
    c.set_model_transform(Some(Transform {
        rotation: [[3., 0., 0.], [0., 3., 0.], [0., 0., 3.]],
        translation: [4., 5., 6.],
        scale: 7.,
        flags: 1,
    }))
    .unwrap();
    assert_eq!(c.composed.translation, [41., 52., 63.]);
    assert_eq!((c.composed.scale, c.composed.flags), (35., 0x81));
    assert_eq!((c.composed_counter, c.model_counter), (41, 8));
    assert_eq!([c.matrix[0], c.matrix[5], c.matrix[10]], [6.; 3]);

    // DC-V18 fixes nine different summation orders at x87-64 precision.
    let a = 2.0_f32.powi(80);
    c.model_view_valid = true;
    c.composed = Transform {
        rotation: [[1.; 3]; 3],
        flags: 1,
        ..Transform::IDENTITY
    };
    c.set_model_transform(Some(Transform {
        rotation: [[a; 3], [-a; 3], [1.; 3]],
        flags: 1,
        ..Transform::IDENTITY
    }))
    .unwrap();
    assert_eq!(
        c.composed.rotation.map(|row| row.map(f32::to_bits)),
        [[0, 0, 0x3f80_0000], [0, 0x3f80_0000, 0], [0, 0, 0],]
    );
    assert_eq!(
        c.matrix,
        [0., 0., 0., 0., 0., 1., 0., 0., 1., 0., 0., 0., 0., 0., 0., 1.]
    );

    // DC-V20: completed rotation spills before scale expansion.
    c.model_view_valid = true;
    c.composed = Transform {
        rotation: [[1., 1., 0.], [0., 1., 0.], [0., 0., 1.]],
        flags: 2,
        ..Transform::IDENTITY
    };
    c.set_model_transform(Some(Transform {
        rotation: [[1., 0., 0.], [f32::from_bits(0x3380_0000), 1., 0.], [0., 0., 1.]],
        scale: 3.,
        flags: 2,
        ..Transform::IDENTITY
    }))
    .unwrap();
    assert_eq!(c.composed.rotation[0][0].to_bits(), 0x3f80_0000);
    assert_eq!(c.matrix[0].to_bits(), 0x4040_0000);

    // DC-V23: the raw first column has a different four-term order.
    c.model_view_valid = true;
    c.composed = Transform {
        rotation: [[0., 1., 1.], [0.; 3], [0.; 3]],
        translation: [1., 0., 0.],
        ..Transform::IDENTITY
    };
    let mut raw = [0.; 16];
    raw[4] = a;
    raw[8] = 1.;
    raw[12] = -a;
    raw[15] = 1.;
    c.set_model_matrix(raw).unwrap();
    assert_eq!(c.matrix[0].to_bits(), 0x3f80_0000);

    // DC-V24: infinity times the explicit zero must remain NaN.
    c.model_view_valid = true;
    c.composed = Transform::IDENTITY;
    raw = [0.; 16];
    raw[0] = f32::INFINITY;
    raw[12] = 9.;
    c.set_model_matrix(raw).unwrap();
    assert!(c.matrix[3].is_nan());
}

#[test]
fn closure_texture_reciprocals_and_x87_controls() {
    let mut c = context();
    c.composed.translation = [7., 5., 0.];
    let mut m = material();
    m.flags |= 0x20;
    let mut texture = slot(0, 3);
    texture.flags = 1;
    texture.scale = 3.;
    m.textures = vec![texture];
    c.set_material_state(&m, flags()).unwrap();
    let last_matrix = |c: &DrawContext<Device>| {
        c.driver
            .commands
            .iter()
            .rev()
            .find_map(|command| {
                if let Command::TexMatrix(Some(matrix)) = command {
                    Some(*matrix)
                } else {
                    None
                }
            })
            .unwrap()
    };
    let matrix = last_matrix(&c);
    assert_eq!(matrix[0].to_bits(), 0x3eaa_aaab);
    // The live reciprocal is not rounded to binary32 before multiplication.
    assert_eq!(matrix[12].to_bits(), 0xc015_5555);
    assert_eq!(matrix[13].to_bits(), 0x3fd5_5555);
    m.textures[0].scale = 0.;
    c.composed.translation = [0.; 3];
    c.set_material_state(&m, flags()).unwrap();
    assert!(last_matrix(&c)[0].is_infinite());
    assert!(last_matrix(&c)[12].is_nan());

    for precision in [Precision::Bits24, Precision::Bits53, Precision::Bits64] {
        for (rounding, expected) in [
            (Rounding::NearestEven, 8),
            (Rounding::Down, 7),
            (Rounding::Up, 8),
            (Rounding::TowardZero, 7),
        ] {
            let arithmetic = Arithmetic { precision, rounding };
            c.set_numeric_control(NumericControl::X87(arithmetic));
            c.set_alpha_func(1, 0.5).unwrap();
            assert_eq!((c.word >> 21) & 15, expected);
            assert_eq!(state::quantize_with_control(0.5, arithmetic).unwrap(), expected);
        }
    }
    assert_eq!(state::quantize(2.).unwrap(), 14); // no saturation
    assert_eq!(state::quantize(-0.5).unwrap(), 8);
    let a = Arithmetic::default();
    let tiny = f64::from(f32::from_bits(1));
    assert_eq!(a.store(a.mul(tiny, 0.5).unwrap()).unwrap().to_bits(), 0);
    let up = Arithmetic {
        rounding: Rounding::Up,
        ..a
    };
    assert_eq!(up.store(up.mul(tiny, 0.5).unwrap()).unwrap().to_bits(), 1);
    let zero = Arithmetic {
        rounding: Rounding::TowardZero,
        ..a
    };
    assert_eq!(
        zero.store(zero.mul(f64::from(f32::MAX), 2.).unwrap()).unwrap(),
        f32::MAX
    );
}

#[test]
fn closure_fog_constructor_and_indexed_highlight_write() {
    let mut c = DrawContext::new(Device::default(), props());
    assert_eq!(c.fog_cache(), (0, [f32::MAX; 4], [f32::MAX; 2]));
    assert_eq!(c.opaque_f4(), &[0; 0x38]);
    c.turn_on_fog(0, None, f32::MAX, f32::MAX).unwrap();
    assert!(c.driver.commands.is_empty());
    c.init().unwrap();
    c.set_highlight_color(2, [0.25, 0.5, 0.75, 1.]).unwrap();
    c.set_render_state_highlight(2).unwrap();
    assert!(c.driver.commands.contains(&Command::TexColor([0.25, 0.5, 0.75, 1.])));
    assert_eq!(c.highlights[1], [0., 1., 0., 1.]);
    assert_eq!(c.set_highlight_color(11, [0.; 4]), Err(Error::InvalidValue));
}

#[test]
fn fog_dc_v27_through_v32_and_v35_order_cache_and_normal_failures() {
    let color = [0.25, 0.5, 0.75, 1.];
    let expected = [
        Command::Capability {
            capability: 6,
            enabled: true,
        },
        Command::FogInteger(0, 2),
        Command::FogColor(color),
        Command::FogScalar(3, 10.),
        Command::FogScalar(4, 20.),
    ];
    // DC-V31 must match DC-V28 even when every normal return is failure-valued.
    for normal_failure in [false, true] {
        let mut c = DrawContext::new(
            Device {
                normal_failure,
                ..Device::default()
            },
            props(),
        );
        c.turn_on_fog(0, None, f32::MAX, f32::MAX).unwrap(); // V27
        c.turn_on_fog(0, Some(&[f32::MAX, f32::MAX, f32::MAX, 0.]), f32::MAX, f32::MAX)
            .unwrap(); // V32
        assert!(c.driver.commands.is_empty());
        assert_eq!(c.fog_cache(), (0, [f32::MAX; 4], [f32::MAX; 2]));

        c.turn_on_fog(1, Some(&color), 10., 20.).unwrap();
        assert_eq!(c.driver.commands, expected);
        assert_eq!(c.fog_cache(), (1, [0.25, 0.5, 0.75, f32::MAX], [10., 20.]));
        assert_eq!(c.driver.fog_color_pointers, [color.as_ptr() as usize]);
        assert_eq!(c.driver.fog_scalar_pointers.len(), 2);
        assert_eq!(c.driver.fog_scalar_pointers[0].0, 3);
        assert_eq!(c.driver.fog_scalar_pointers[1].0, 4);
        assert_ne!(c.driver.fog_scalar_pointers[0].1, c.driver.fog_scalar_pointers[1].1);
        assert_eq!(c.driver.reads, 0); // No Init, properties or manager queries.
        c.driver.commands.clear();
        c.turn_on_fog(1, Some(&color), 10., 20.).unwrap();
        c.turn_on_fog(1, Some(&[0.25, 0.5, 0.75, 0.]), f32::MAX, f32::MAX)
            .unwrap();
        assert!(c.driver.commands.is_empty());
        c.turn_on_fog(0, None, f32::MAX, f32::MAX).unwrap();
        c.turn_on_fog(1, Some(&color), 10., 20.).unwrap();
        assert_eq!(
            c.driver.commands,
            [
                Command::Capability {
                    capability: 6,
                    enabled: false
                },
                expected[0].clone(),
                expected[1].clone(),
            ]
        );
        c.driver.commands.clear();
        c.turn_on_fog(2, None, f32::MAX, f32::MAX).unwrap(); // V35
        c.turn_on_fog(2, None, f32::MAX, f32::MAX).unwrap();
        assert_eq!(c.driver.commands, expected[..2]);
        assert_eq!(c.fog_cache().0, 2);
        assert_eq!(c.fog_cache().1[3], f32::MAX);
        assert_eq!(c.fog_cache().2, [10., 20.]);
    }

    // DC-V30: independently update parameters with fog disabled, without sorting.
    let mut c = DrawContext::new(Device::default(), props());
    let color = [1., 0., 0., 0.5];
    c.turn_on_fog(0, Some(&color), 20., 10.).unwrap();
    assert_eq!(
        c.driver.commands,
        [
            Command::FogColor(color),
            Command::FogScalar(3, 20.),
            Command::FogScalar(4, 10.)
        ]
    );
    assert_eq!(c.fog_cache(), (0, [1., 0., 0., f32::MAX], [20., 10.]));
}

#[test]
fn fog_dc_v33_v34_float_comparisons_preserve_payloads() {
    let mut c = DrawContext::new(Device::default(), props());
    c.turn_on_fog(0, Some(&[0.; 4]), 0., 0.).unwrap();
    c.driver.commands.clear();
    c.turn_on_fog(0, Some(&[-0., -0., -0., 1.]), -0., -0.).unwrap();
    assert!(c.driver.commands.is_empty());
    assert_eq!(c.fog_cache().1.map(f32::to_bits), [0, 0, 0, f32::MAX.to_bits()]);
    assert_eq!(c.fog_cache().2.map(f32::to_bits), [0, 0]);

    // Equality also preserves negative zeros when those were submitted first.
    let mut c = DrawContext::new(Device::default(), props());
    c.turn_on_fog(0, Some(&[-0.; 4]), -0., -0.).unwrap();
    c.driver.commands.clear();
    c.turn_on_fog(0, Some(&[0.; 4]), 0., 0.).unwrap();
    assert!(c.driver.commands.is_empty());
    assert_eq!(c.fog_cache().1[0].to_bits(), (-0.0f32).to_bits());
    assert_eq!(c.fog_cache().2.map(f32::to_bits), [(-0.0f32).to_bits(); 2]);

    let nan = f32::from_bits(0x7fc1_2345);
    let mut c = DrawContext::new(Device::default(), props());
    for _ in 0..2 {
        c.turn_on_fog(0, Some(&[nan, 0., 0., 1.]), nan, nan).unwrap();
        let commands = std::mem::take(&mut c.driver.commands);
        assert!(
            matches!(commands.as_slice(), [Command::FogColor(v), Command::FogScalar(3, s), Command::FogScalar(4, e)]
            if v[0].to_bits() == nan.to_bits() && s.to_bits() == nan.to_bits() && e.to_bits() == nan.to_bits())
        );
        assert_eq!(c.fog_cache().1[0].to_bits(), nan.to_bits());
        assert_eq!(c.fog_cache().2.map(f32::to_bits), [nan.to_bits(); 2]);
        assert_eq!(c.fog_cache().1[3], f32::MAX);
    }
    // A NaN in the old cache also compares unequal to a finite new request.
    c.turn_on_fog(0, Some(&[0.; 4]), 0., 0.).unwrap();
    assert_eq!(c.driver.commands.len(), 3);
}

#[test]
fn fog_nonreturning_boundary_preserves_completed_prefix_and_retries() {
    let color = [0.25, 0.5, 0.75, 1.];
    for fail_at in 0..5 {
        let mut c = DrawContext::new(
            Device {
                fail_at: Some(fail_at),
                ..Device::default()
            },
            props(),
        );
        assert_eq!(c.turn_on_fog(1, Some(&color), 10., 20.), Err(Error::Provider(99)));
        assert_eq!(
            c.fog_cache(),
            (
                u8::from(fail_at >= 2),
                if fail_at >= 3 {
                    [0.25, 0.5, 0.75, f32::MAX]
                } else {
                    [f32::MAX; 4]
                },
                [if fail_at >= 4 { 10. } else { f32::MAX }, f32::MAX],
            )
        );
        c.driver.fail_at = None;
        c.driver.commands.clear();
        c.turn_on_fog(1, Some(&color), 10., 20.).unwrap();
        assert_eq!(c.driver.commands.len(), if fail_at < 2 { 5 } else { 5 - fail_at });
        assert_eq!(c.fog_cache(), (1, [0.25, 0.5, 0.75, f32::MAX], [10., 20.]));
    }
}

#[test]
fn closure_arithmetic_precision_and_binary32_boundaries() {
    let a = Arithmetic::default();
    for (precision, power, expected) in [
        (Precision::Bits24, 24, 0.),
        (Precision::Bits53, 24, 1.),
        (Precision::Bits53, 53, 0.),
        (Precision::Bits64, 53, 1.),
    ] {
        let arithmetic = Arithmetic { precision, ..a };
        let big = 2.0_f64.powi(power);
        let sum = arithmetic.add(big, 1.).unwrap();
        assert_eq!(arithmetic.store(arithmetic.add(sum, -big).unwrap()).unwrap(), expected);
    }
    let up = Arithmetic {
        rounding: Rounding::Up,
        ..a
    };
    let big = 2.0_f64.powi(80);
    assert_eq!(
        up.store(up.add(up.add(big, 1.).unwrap(), -big).unwrap()).unwrap(),
        131_072.
    );
    let down = Arithmetic {
        rounding: Rounding::Down,
        ..a
    };
    assert_eq!(down.store(down.add(1., -1.).unwrap()).unwrap().to_bits(), 0x8000_0000);

    // Independent host IEEE binary32 results at PC24/nearest-even. These
    // exercise exponent alignment, sticky rounding, signs, subnormals/overflow.
    let pc24 = Arithmetic {
        precision: Precision::Bits24,
        ..a
    };
    let mut seed = 0x57a3_901d_u32;
    for _ in 0..512 {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let x = f32::from_bits(seed);
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let y = f32::from_bits(seed);
        if x.is_finite() && y.is_finite() {
            let sum = pc24.store(pc24.add(f64::from(x), f64::from(y)).unwrap()).unwrap();
            assert_eq!(sum.to_bits(), (x + y).to_bits());
            // x87 precision control does not restrict the exponent range. At
            // underflow a subsequent binary32 store can round a second time;
            // compare products at PC64 instead, where the 48-bit product is exact.
            let product = a.store(a.mul(f64::from(x), f64::from(y)).unwrap()).unwrap();
            assert_eq!(product.to_bits(), (x * y).to_bits());
        }
    }
    for scale in [1., 3., 10., 0.1, -3., f32::MIN_POSITIVE, f32::from_bits(1), f32::MAX] {
        assert_eq!(
            a.store(a.reciprocal(scale).unwrap()).unwrap().to_bits(),
            ((1.0_f64 / f64::from(scale)) as f32).to_bits()
        );
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
// Scoped prefix/projection tests, additional to the 19 published ledger groups.

#[test]
fn rotation_boundaries_keep_offset_and_exact_lookup_prefix() {
    let rotations = ["rotateX", "rotateY", "rotateZ", "rotateXYZ", "rotateZXY"];
    for (n, key) in rotations.into_iter().enumerate() {
        let mut f = Fixture::default();
        f.hits.insert("offset", vec![0]);
        f.hits.insert(key, vec![0]);
        f.vectors.insert(0, [0x80000000, 0x7FC01234, 0x00000001]);
        let mut r = asymmetric();
        r.component.placement[0] = 255;
        r.component.placement[1] = 0xA6;
        assert_eq!(run(&mut f, &["raw-vector"], 0, &mut r), Err(Unsupported::Rotation(key)));
        assert_eq!([bits(&r, 40), bits(&r, 44), bits(&r, 48)], [0x80000000, 0x7FC01234, 1]);
        assert_eq!(&r.component.placement[..2], &[0, 0xA7]);
        assert_eq!((r.component.lod_min, r.component.lod_max), (7, 9));
        let mut expected = vec!["find:offset:1", "vector:0"];
        expected.extend_from_slice(&ORDER[1..n + 2]);
        assert_eq!(f.calls, expected);
    }
}
#[test]
fn scale_compares_original_after_rounded_store() {
    let mut f = Fixture::default();
    f.hits.insert("scale", vec![0]);
    f.floats.insert(0, RawFloat::Dyadic(33554433, -25)); // 1 + 2^-25
    let mut r = asymmetric();
    r.component.placement[1] = 0xA5;
    run(&mut f, &["pre-store"], 0, &mut r).unwrap();
    assert_eq!(bits(&r, 52), 0x3F800000);
    assert_eq!(&r.component.placement[..2], &[0xFF, 0xA7]);
}
#[test]
fn numeric_unsupported_retains_owned_prefix_not_rollback() {
    let mut f = Fixture::default();
    f.hits.insert("emitScale", vec![0, 1]);
    f.floats.insert(0, RawFloat::Binary32(0x3FC00000));
    f.floats.insert(1, RawFloat::Unsupported);
    let mut r = asymmetric();
    assert_eq!(
        run(&mut f, &["1.5", "unqualified"], 0, &mut r),
        Err(Unsupported::Numeric)
    );
    assert_eq!(r.component.render, [1.5, 12., 13., 14.]);
    assert_eq!((r.component.lod_min, r.component.lod_max), (1, 6));
    assert_eq!(f.calls.last().unwrap(), "float:1");
    assert!(!f.calls.iter().any(|x| x == "find:sizeScale:1"));
    let mut f = Fixture::default();
    f.hits.insert("offset", vec![0]);
    f.vectors.insert(0, [1, 2, 3]);
    f.hits.insert("scale", vec![1]);
    f.floats.insert(1, RawFloat::Dyadic(1, 0));
    let mut r = asymmetric();
    assert_eq!(
        parse_options(
            &tokens(&["vector", "scalar"]),
            &mut vec![],
            0,
            &mut r,
            NumericPolicy::Unqualified,
            &mut f
        ),
        Err(Unsupported::Numeric)
    );
    assert_eq!(
        [bits(&r, 40), bits(&r, 44), bits(&r, 48), bits(&r, 52)],
        [1, 2, 3, 0x40400000]
    );
    assert_eq!(&r.component.placement[..2], &[255, 0xA7]);
}
#[test]
fn forbidden_lod_and_probability_stop_at_error_boundary() {
    for key in ["lodRange", "lod"] {
        let mut f = Fixture::default();
        f.hits.insert("scale", vec![0]);
        f.floats.insert(0, RawFloat::Dyadic(2, 0));
        f.hits.insert(key, vec![0, 0]);
        let mut r = asymmetric();
        assert_eq!(run(&mut f, &["2"], 1, &mut r), Err(Unsupported::LodInSelect));
        assert_eq!(
            (
                bits(&r, 52),
                r.component.placement[0],
                r.component.lod_min,
                r.component.lod_max
            ),
            (0x40000000, 255, 7, 9)
        );
        assert!(!f.calls.iter().any(|x| x.starts_with("uint:")));
    }
    let mut f = Fixture::default();
    f.hits.insert("prob", vec![0]);
    f.hits.insert("respectLength", vec![]);
    let mut r = asymmetric();
    assert_eq!(
        run(&mut f, &["1"], 0, &mut r),
        Err(Unsupported::ProbabilityOutsideSelect)
    );
    assert_eq!(
        (
            r.component.lod_min,
            r.component.lod_max,
            r.component.flags,
            r.component.select_weight
        ),
        (1, 6, 0xA5A50001, 0x1234)
    );
    assert_eq!(f.calls.last().unwrap(), "find:prob:1");
}
#[test]
fn raw_binary32_and_exact_probability_domain() {
    let mut f = Fixture::default();
    f.hits.insert("emitScale", vec![0, 1]);
    f.floats.insert(0, RawFloat::Binary32(0x80000000));
    f.floats.insert(1, RawFloat::Binary32(1));
    f.hits.insert("prob", vec![2]);
    f.floats.insert(2, RawFloat::Dyadic(1, -1));
    let mut r = asymmetric();
    run(&mut f, &["negative-zero", "subnormal", "half"], 7, &mut r).unwrap();
    assert_eq!(
        [r.component.render[0].to_bits(), r.component.render[1].to_bits()],
        [0x80000000, 1]
    );
    // 12582912 + 32767.5 rounds nearest-even to bits 4B408000.
    assert_eq!(r.component.select_weight, 0x8000);
    for raw in [
        RawFloat::Binary32(0x7FC00000),
        RawFloat::Dyadic(1, -25),
        RawFloat::Dyadic(2, 0),
    ] {
        let mut f = Fixture::default();
        f.hits.insert("prob", vec![0]);
        f.floats.insert(0, raw);
        let mut r = asymmetric();
        assert_eq!(run(&mut f, &["raw"], 7, &mut r), Err(Unsupported::Numeric));
        assert_eq!(r.component.select_weight, 0x1234);
    }
}
#[test]
fn lod_single_lowbyte_wrap_and_absent_finder_stale_scratch() {
    let mut f = Fixture::default();
    f.hits.insert("lod", vec![0]);
    f.uints.insert(0, 511);
    let mut r = asymmetric();
    run(&mut f, &["511"], 0, &mut r).unwrap();
    assert_eq!((r.component.lod_min, r.component.lod_max), (255, 0));
    let mut f = Fixture::default();
    let mut r = asymmetric();
    parse_options(
        &[],
        &mut vec![Index(900)],
        7,
        &mut r,
        NumericPolicy::Unqualified,
        &mut f,
    )
    .unwrap();
    assert_eq!((r.component.lod_min, r.component.lod_max), (1, 6));
}
#[test]
fn shared_constructor_projection_is_not_direct_option_initialization() {
    let r = Record::shared_constructor_projection();
    assert_eq!(
        (
            r.type_word,
            r.component.flags,
            r.component.lod_min,
            r.component.lod_max,
            r.component.copies,
            r.component.spacing,
            r.component.select_group,
            r.component.select_weight,
            r.component.index
        ),
        (0, 0, 0, 0, 1, 16, 0, 0, u32::MAX)
    );
    assert_eq!(r.component.name, "");
    assert_eq!(r.component.render, [1.; 4]);
    let mut expected = [0u8; 56];
    for at in [4, 20, 36, 52] {
        expected[at..at + 4].copy_from_slice(&[0, 0, 128, 63]);
    }
    assert_eq!(r.component.placement, expected);
}
#[test]
fn named_rereads_shared_first_index_after_validation() {
    let (mut p, mut f, t) = named_fixture();
    p.scratch = vec![Index(1)];
    f.validation_reindex = Some(2);
    p.add_named(&t, 7, NumericPolicy::NearestAtLeast53, &mut f).unwrap();
    assert_eq!(p.components[0].component.name, "other");
    assert_eq!(p.components[0].component.flags, 1);
    assert_eq!(&f.calls[..3], ["ctor", "validate:loopA:6:null", "name:other"]);
}
#[test]
fn unsupported_command_and_validation_keep_precise_prefix() {
    let (mut p, mut f, t) = named_fixture();
    p.current_block = Text::default();
    assert_eq!(
        p.sequence_effect(&t, NumericPolicy::NearestAtLeast53, &mut f),
        Err(Unsupported::NotInEffectsBlock)
    );
    assert!(f.calls.is_empty());
    p.current_block = Text("parent".into());
    f.selected.clear();
    assert_eq!(
        p.sequence_effect(&t, NumericPolicy::NearestAtLeast53, &mut f),
        Err(Unsupported::MissingIndex)
    );
    assert_eq!(f.calls, ["nonswitch:1"]);
    f.calls.clear();
    p.scratch = vec![Index(1)];
    f.validation = 0;
    assert_eq!(
        p.add_named(&t, 0, NumericPolicy::NearestAtLeast53, &mut f),
        Err(Unsupported::NamedValidation)
    );
    assert_eq!(f.calls, ["ctor", "validate:loopA:6:null"]);
    assert!(p.pending.is_some());
    assert!(p.components.is_empty());
}
#[test]
fn query_fallback_is_raw_and_missing_adjustment_unsupported() {
    let mut f = Fixture {
        qi: Some(Identity(17)),
        ..Fixture::default()
    };
    let mut out = Some(Identity(3));
    assert!(query_interface(42, Some(&mut out), None, &mut f).unwrap());
    assert_eq!(out, Some(Identity(17)));
    assert_eq!(f.calls, ["base:42"]);
    f.calls.clear();
    assert_eq!(
        query_interface(0x6956BABC, Some(&mut out), None, &mut f),
        Err(Unsupported::PhysicalQueryAdjustment)
    );
    assert!(f.calls.is_empty());
    assert_eq!(out, Some(Identity(17)));
}

#[derive(Default)]
struct RegistrationFixture {
    events: Vec<String>,
    registrations: Vec<Registration>,
    fail_at: Option<usize>,
}
impl Registrar for RegistrationFixture {
    fn base_register(&mut self) {
        self.events.push("base".into());
    }
    fn allocate(&mut self, bytes: u32) -> Option<Descriptor> {
        self.events.push(format!("allocate:{bytes}"));
        if self.fail_at == Some(self.registrations.len()) {
            None
        } else {
            Some(Descriptor(100 + self.registrations.len() as u32))
        }
    }
    fn shared_block_construct(&mut self, d: Descriptor) {
        self.events.push(format!("block:{}", d.0));
    }
    fn install_vptr(&mut self, d: Descriptor, v: u32) {
        self.events.push(format!("vptr:{}:{v:08X}", d.0));
    }
    fn register(&mut self, d: Descriptor, r: Registration) -> u8 {
        self.events.push(format!("register:{}:{}", d.0, r.name));
        self.registrations.push(r);
        0
    }
}
#[test]
fn root_registration_order_literals_descriptors_hooks_and_construction() {
    // Independently transcribed §6.1, not selected through ROOT_COMMANDS.
    let expected = [
        ("renderprop", 0x00A9F85C, 0x00596630, None),
        ("textureID", 0x00A9F86C, 0x0059F890, None),
        ("modelID", 0x00A9F87C, 0x0059FE50, None),
        ("soundID", 0x00A9F88C, 0x0059FF40, None),
        ("brushID", 0x00A9F89C, 0x005A0030, None),
        ("effectID", 0x00A9F8AC, 0x0059A640, None),
        ("effectGroup", 0x00A9F8BC, 0x00596750, Some(0x00648F10)),
        ("instance", 0x00A9FA08, 0x0059A730, None),
        ("testEffect", 0x00A9F8D8, 0x0059D1D0, None),
        ("messageTrigger", 0x00A9F8E8, 0x0059E820, None),
        ("camera", 0x00A9F8F8, 0x0059A2A0, None),
        ("setPriority", 0x00A9F908, 0x005A24C0, None),
        ("effectsResource", 0x00A9F918, 0x00596810, Some(0x00596970)),
        ("loadResource", 0x00A9F934, 0x00596AB0, None),
        ("effect", 0x00A9F944, 0x005A0D40, Some(0x0059EB50)),
        ("particles", 0x00A9F960, 0x005A1A50, Some(0x0059D9B0)),
        ("dynamicParticle", 0x00A9F97C, 0x0059DB00, Some(0x0059DCF0)),
        ("decal", 0x00A9F998, 0x0059DD80, Some(0x0059DF50)),
        ("shake", 0x00A9F9B4, 0x0059DFE0, Some(0x0059E260)),
        ("light", 0x00A9F9D0, 0x0059E2F0, Some(0x0059E4D0)),
        ("sequence", 0x00A9F9EC, 0x0059E600, Some(0x0059E780)),
    ];
    let mut f = RegistrationFixture::default();
    register_commands(&mut f).unwrap();
    assert_eq!(f.registrations.len(), 21);
    let mut trace = vec!["base".into()];
    for (i, (name, vptr, first, last)) in expected.into_iter().enumerate() {
        assert_eq!(
            f.registrations[i],
            Registration {
                name,
                descriptor: vptr,
                hooks: match last {
                    Some(b) => Hooks::Block(first, b),
                    None => Hooks::Direct(first),
                }
            }
        );
        let id = 100 + i;
        trace.push(format!("allocate:{}", if last.is_some() { 24 } else { 4 }));
        if last.is_some() {
            trace.push(format!("block:{id}"));
        }
        trace.push(format!("vptr:{id}:{vptr:08X}"));
        trace.push(format!("register:{id}:{name}"));
    }
    assert_eq!(f.events, trace);
}
#[test]
fn nested_registration_and_allocation_boundary_are_separate() {
    let mut f = RegistrationFixture::default();
    register_sequence_effect(&mut f).unwrap();
    assert_eq!(
        f.registrations,
        [Registration {
            name: "sequenceEffect",
            descriptor: 0x00A9FDFC,
            hooks: Hooks::Direct(0x005A3890)
        }]
    );
    assert_eq!(
        f.events,
        ["allocate:4", "vptr:100:00A9FDFC", "register:100:sequenceEffect"]
    );
    let mut f = RegistrationFixture {
        fail_at: Some(1),
        ..RegistrationFixture::default()
    };
    assert_eq!(register_commands(&mut f), Err(Unsupported::Allocation));
    assert_eq!(
        f.events,
        [
            "base",
            "allocate:4",
            "vptr:100:00A9F85C",
            "register:100:renderprop",
            "allocate:4"
        ]
    );
    assert_eq!(f.registrations.len(), 1);
}

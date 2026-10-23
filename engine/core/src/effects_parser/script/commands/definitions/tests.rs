// SPDX-License-Identifier: GPL-3.0-or-later
use super::super::tests::{bytes, Host};
use super::*;
use crate::effects_parser::script::tests::tokens;
use DefinitionKind::*;

#[test]
fn ec51_ec52_ec53_ec54_ec55_definition_defaults_and_inherit_failure() {
    let mut c = Commands::default();
    let mut h = Host::default();
    for kind in [Particles, DynamicParticle, Decal, Shake] {
        c.definitions.get_mut(kind).name = bytes("Old");
        assert!(c
            .enter_definition(kind, &mut tokens(&["cmd"]), &mut Ascii, &mut h)
            .is_err());
        assert_eq!(c.definitions.get(kind).name, b"Old");
    }
    c.enter_definition(Decal, &mut tokens(&["decal", "Name", ":"]), &mut Ascii, &mut h)
        .unwrap();
    let d = &c.definitions.get(Decal).description;
    assert_eq!(d.words[&0], 0);
    assert_eq!(d.bytes[&8], 0);
    assert_eq!(d.bytes[&9], 1);
    assert!(!d.bytes.contains_key(&10));
    assert_eq!(d.get_scalar(0x0c), Some(5.));
    assert_eq!(d.get_scalar(0x58), Some(1.));
    for (at, value) in [(0x10, 0.), (0x1c, 1.), (0x28, 1.), (0x40, 1.)] {
        assert_eq!(d.floats[&at], [value]);
    }
    assert_eq!(d.vectors[&0x34], [[1.; 3]]);
    assert_eq!(
        c.enter_definition(
            Shake,
            &mut tokens(&["shake", "Name", "wrong", "Parent"]),
            &mut Ascii,
            &mut h
        ),
        Err(Error::Parse("Expecting ':'".into()))
    );
    let d = &c.definitions.get(Shake).description;
    assert_eq!(d.get_scalar(0), Some(5.));
    assert_eq!(d.floats[&8], [1.]);
    assert_eq!(d.floats[&0x14], [60.]);
    c.collection = Some(Identity(1));
    let reset = Description::reset(DynamicParticle, &Description::default());
    let mut changed = reset.clone();
    changed.scalar(0x18, 7.);
    let mut h = Host::returning(vec![(
        Call::Inherit(Identity(1), DynamicParticle, bytes("Parent"), reset),
        Reply::Inherited(false, changed.clone()),
    )]);
    assert_eq!(
        c.enter_definition(
            DynamicParticle,
            &mut tokens(&["dynamicParticle", "Name", ":", "Parent", "ignored"]),
            &mut Ascii,
            &mut h
        ),
        Err(Error::Parse("Unknown particle definition: Parent".into()))
    );
    assert_eq!(c.definitions.get(DynamicParticle).description, changed);
    assert_eq!(c.definitions.get(DynamicParticle).name, b"Name");
    h.done();
    c.collection = None;
    c.enter_definition(
        Decal,
        &mut tokens(&["decal", "Name", ":", "Missing"]),
        &mut Ascii,
        &mut h,
    )
    .unwrap();
    assert_eq!(
        c.definitions.get(Decal).description,
        Description::reset(Decal, &Description::default())
    );
}
#[test]
fn ec56_ec57_particle_retained_word_indeterminate_word_and_scratch_tail() {
    let mut c = Commands::default();
    c.definitions.get_mut(Particles).description.words.insert(0x204, 123);
    c.definitions.get_mut(Particles).description.words.insert(0x1c8, 99);
    let mut h = Host::default();
    c.enter_definition(Particles, &mut tokens(&["particles", "-Name"]), &mut Ascii, &mut h)
        .unwrap();
    let definition = c.definitions.get(Particles);
    assert_eq!(definition.name, b"-Name");
    assert_eq!(definition.description.words[&0x204], 123);
    assert!(!definition.description.words.contains_key(&0x1c8));
    assert_eq!(definition.description.words[&0x14c], 0xce6e6b28);
    assert_eq!(definition.description.words[&0x16c], 0x10000);
    assert_eq!(definition.description.floats[&0x88], [30.]);
    assert_eq!(definition.description.vectors[&0x94], [[1.; 3]]);
    assert_eq!(definition.description.get_scalar(0x158), Some(-10000.));
    assert_eq!(c.word_494, 0);
    assert_eq!(c.scalar_498, 1.);
    c.collection = Some(Identity(1));
    let reset = Description::reset(Particles, &c.definitions.get(Particles).description);
    let mut inherited = reset.clone();
    inherited.tractors.push([0., 0., 0., 0., 0., 0., 4., 7.]);
    let mut h = Host::returning(vec![(
        Call::Inherit(Identity(1), Particles, bytes("Parent"), reset),
        Reply::Inherited(true, inherited.clone()),
    )]);
    c.enter_definition(
        Particles,
        &mut tokens(&["particles", "Name", ":", "Parent"]),
        &mut Ascii,
        &mut h,
    )
    .unwrap();
    assert_eq!(c.word_494, 7f32.to_bits());
    assert_eq!(c.definitions.get(Particles).description, inherited);
    h.done();
}
#[test]
fn ec58_ec59_ec60_ec61_light_exit_fallbacks_and_definition_publication() {
    let mut c = Commands {
        collection: Some(Identity(1)),
        ..Commands::default()
    };
    let reset = Description::reset(Light, &Description::default());
    let mut h = Host::returning(vec![(
        Call::Inherit(Identity(1), Light, bytes("Parent"), reset.clone()),
        Reply::Inherited(true, reset.clone()),
    )]);
    c.enter_definition(
        Light,
        &mut tokens(&["light", "Name", ":", "Parent", "ignored"]),
        &mut Ascii,
        &mut h,
    )
    .unwrap();
    assert!(c.definitions.get(Light).description.floats[&0x0c].is_empty());
    assert!(c.definitions.get(Light).description.vectors[&0].is_empty());
    c.collection = None;
    c.exit_definition(Light, &mut h).unwrap();
    let d = c.definitions.get(Light);
    assert_eq!(d.description.floats[&0x0c], [1.]);
    assert_eq!(d.description.vectors[&0], [[0., 0., 1.]]);
    assert_eq!(d.description.get_scalar(0x18), Some(2.));
    assert!(d.name.is_empty());
    c.collection = Some(Identity(1));
    c.definitions.get_mut(Light).name = bytes("Name");
    c.definitions.get_mut(Light).description.floats.insert(0x0c, vec![7.]);
    c.definitions
        .get_mut(Light)
        .description
        .vectors
        .insert(0, vec![[8.; 3]]);
    let old = c.definitions.get(Light).clone();
    h.expected.push_back((
        Call::PublishDefinition(Identity(1), Light, bytes("Name"), old.description.clone()),
        Err(Error::Foreign("publish".into())),
    ));
    assert!(c.exit_definition(Light, &mut h).is_err());
    assert_eq!(c.definitions.get(Light), &old);
    for kind in [Particles, DynamicParticle, Decal, Shake] {
        c.definitions.get_mut(kind).name = bytes("Name");
        let description = c.definitions.get(kind).description.clone();
        h.expected.push_back((
            Call::PublishDefinition(Identity(1), kind, bytes("Name"), description.clone()),
            Ok(Reply::Bool(false)),
        ));
        c.exit_definition(kind, &mut h).unwrap();
        assert!(c.definitions.get(kind).name.is_empty());
        assert_eq!(c.definitions.get(kind).description, description);
    }
    h.done();
}
#[test]
fn ec62_ec63_ec64_sequence_reference_replacement_exit_and_throw() {
    let mut c = Commands::default();
    c.definitions.get_mut(Sequence).object = Some(Identity(1));
    let mut h = Host::returning(vec![
        (Call::AllocateSequence, Reply::Object(Some(Identity(2)))),
        (Call::Retain(Identity(2)), Reply::Void),
        (Call::Release(Identity(1)), Reply::Void),
    ]);
    let mut t = tokens(&["sequence", "Name", ":", "Parent", "-loop", "-noOverlap", "-hardStart"]);
    c.enter_definition(Sequence, &mut t, &mut Ascii, &mut h).unwrap();
    assert_eq!(c.definitions.get(Sequence).object, Some(Identity(2)));
    assert_eq!(c.definitions.get(Sequence).description.words[&0x18], 7);
    assert!(c.definitions.get(Sequence).description.lists[&0x0c].is_empty());
    assert_eq!(t[4].bytes, b"+loop");
    c.collection = Some(Identity(3));
    h.expected.push_back((
        Call::PublishSequence(Identity(3), bytes("Name"), Some(Identity(2))),
        Err(Error::Foreign("publish".into())),
    ));
    assert!(c.exit_definition(Sequence, &mut h).is_err());
    assert_eq!(c.definitions.get(Sequence).name, b"Name");
    assert_eq!(c.definitions.get(Sequence).object, Some(Identity(2)));
    c.collection = None;
    h.expected.push_back((Call::Release(Identity(2)), Ok(Reply::Void)));
    c.exit_definition(Sequence, &mut h).unwrap();
    assert!(c.definitions.get(Sequence).name.is_empty());
    assert_eq!(c.definitions.get(Sequence).object, None);
    h.done();
}
#[test]
fn ec65_ec66_ec67_ec68_effect_base_guard_copy_empty_and_post_check() {
    let mut c = Commands::default();
    assert_eq!(
        c.effect_base(&tokens(&["effectBase"]), &mut Ascii),
        Err(Error::Parse("Not in particles block".into()))
    );
    c.definitions.get_mut(Particles).name = bytes("P");
    for (input, expected) in [
        (vec!["effectBase", "MixedCase", "ignored"], "MixedCase"),
        (vec!["effectBase", ""], ""),
        (vec!["effectBase", "New", "-unknown"], "New"),
    ] {
        let t = tokens(&input);
        c.effect_base(&t, &mut Ascii).unwrap();
        assert_eq!(
            c.definitions.get(Particles).description.strings[&0x160],
            bytes(expected)
        );
        assert_eq!(c.definitions.get(Particles).name, b"P");
        if expected == "New" {
            assert!(check_switches(&t, &mut Ascii).is_err());
        }
    }
}

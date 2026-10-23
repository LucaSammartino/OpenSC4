// SPDX-License-Identifier: GPL-3.0-or-later
//! Published child registration identities; aliases allocate distinct objects.
use super::{register_one, Hooks, Registrar, Registration, Unsupported};
const fn direct(name: &'static str, descriptor: u32, handler: u32) -> Registration {
    Registration {
        name,
        descriptor,
        hooks: Hooks::Direct(handler),
    }
}
pub const PARTICLES: [Registration; 27] = [
    direct("zoom", 0x00a9fb78, 0x005970e0),
    direct("color", 0x00a9fb68, 0x0059bdc0),
    direct("alpha", 0x00a9fb58, 0x0059c060),
    direct("size", 0x00a9fb48, 0x005998e0),
    direct("aspect", 0x00a9fb38, 0x00599a10),
    direct("rotate", 0x00a9fb28, 0x00599b60),
    direct("source", 0x00a9fb18, 0x00597140),
    direct("emit", 0x00a9fb08, 0x0059c280),
    direct("force", 0x00a9faf8, 0x0059a820),
    direct("warp", 0x00a9fae8, 0x0059b680),
    direct("randomWalk", 0x00a9fad8, 0x005979b0),
    direct("stretch", 0x00a9fac8, 0x00597c60),
    direct("life", 0x00a9fab8, 0x00597cd0),
    direct("rate", 0x00a9faa8, 0x0059ca70),
    direct("inject", 0x00a9fa98, 0x0059cbb0),
    direct("maintain", 0x00a9fa88, 0x0059cc20),
    direct("texture", 0x00a9fa78, 0x005a10b0),
    direct("model", 0x00a9fa68, 0x005a1340),
    direct("align", 0x00a9fa58, 0x00597fb0),
    direct("collide", 0x00a9fa48, 0x00598120),
    direct("terrainRepel", 0x00a9fa38, 0x00598320),
    direct("effectBase", 0x00a9fa28, 0x0059d8f0),
    direct("timedEffect", 0x00a9fa18, 0x005a16b0),
    direct("color255", 0x00a9fb68, 0x0059bdc0),
    direct("colour", 0x00a9fb68, 0x0059bdc0),
    direct("colour255", 0x00a9fb68, 0x0059bdc0),
    direct("alpha255", 0x00a9fb58, 0x0059c060),
];
pub const DYNAMIC_PARTICLE: [Registration; 4] = [
    direct("effectBase", 0x00a9fbb8, 0x0059da40),
    direct("model", 0x00a9fba8, 0x005a1c50),
    direct("mass", 0x00a9fb98, 0x00598430),
    direct("friction", 0x00a9fb88, 0x005984d0),
];
pub const SEQUENCE: [Registration; 2] = [
    direct("wait", 0x00a9fcc8, 0x005a2400),
    direct("play", 0x00a9fcb8, 0x005a20c0),
];
pub const SHAKE: [Registration; 5] = [
    direct("length", 0x00a9fc78, 0x005986a0),
    direct("amplitude", 0x00a9fc68, 0x0059a060),
    direct("frequency", 0x00a9fc58, 0x0059a120),
    direct("shakeAspect", 0x00a9fc48, 0x00598760),
    direct("table", 0x00a9fc38, 0x005987d0),
];
pub const LIGHT: [Registration; 6] = [
    direct("strength", 0x00a9fca8, 0x0059a1e0),
    direct("length", 0x00a9fc98, 0x00598840),
    direct("color", 0x00a9fc88, 0x0059d060),
    direct("color255", 0x00a9fc88, 0x0059d060),
    direct("colour", 0x00a9fc88, 0x0059d060),
    direct("colour255", 0x00a9fc88, 0x0059d060),
];
pub const DECAL: [Registration; 11] = [
    direct("color", 0x00a9fc28, 0x0059ccd0),
    direct("color255", 0x00a9fc28, 0x0059ccd0),
    direct("colour", 0x00a9fc28, 0x0059ccd0),
    direct("colour255", 0x00a9fc28, 0x0059ccd0),
    direct("alpha", 0x00a9fc18, 0x0059ce40),
    direct("alpha255", 0x00a9fc18, 0x0059ce40),
    direct("size", 0x00a9fc08, 0x00599ce0),
    direct("aspect", 0x00a9fbf8, 0x00599e50),
    direct("rotate", 0x00a9fbe8, 0x00599f10),
    direct("life", 0x00a9fbd8, 0x005985c0),
    direct("texture", 0x00a9fbc8, 0x005a1e80),
];
// Literal §19.52 association. effects-parser §6.1 gives this same descriptor
// a different root handler; do not silently reconcile conflicting specs (EF16).
pub const EFFECT_GROUP: [Registration; 1] = [direct("instance", 0x00a9fa08, 0x00596980)];
pub const EFFECT: [Registration; 19] = [
    direct("particleEffect", 0x00a9feac, 0x005a0120),
    direct("dynamicParticleEffect", 0x00a9fe9c, 0x005a0350),
    direct("decalEffect", 0x00a9fe8c, 0x005a0480),
    direct("shakeEffect", 0x00a9fe7c, 0x005a05f0),
    direct("flashEffect", 0x00a9fe6c, 0x005a08e0),
    direct("tintEffect", 0x00a9fe5c, 0x005a0a90),
    direct("soundEffect", 0x00a9fe4c, 0x005a2a50),
    direct("chainEffect", 0x00a9fe3c, 0x0059d830),
    direct("brushEffect", 0x00a9fe2c, 0x005a2cf0),
    direct("scrubberEffect", 0x00a9fe1c, 0x005a30b0),
    direct("demolishEffect", 0x00a9fe1c, 0x005a30b0),
    direct("gameEffect", 0x00a9fe1c, 0x005a30b0),
    direct("mapEffect", 0x00a9fe1c, 0x005a30b0),
    direct("automataEffect", 0x00a9fe0c, 0x005a3710),
    direct("sequenceEffect", 0x00a9fdfc, 0x005a3890),
    direct("cameraEffect", 0x00a9fdec, 0x005a3930),
    direct("visualEffect", 0x00a9fddc, 0x005a0bd0),
    Registration {
        name: "select",
        descriptor: 0x00a9fdc0,
        hooks: Hooks::Block(0x00596e70, 0x00596fe0),
    },
    Registration {
        name: "particleSequence",
        descriptor: 0x00a9fed4,
        hooks: Hooks::Block(0x00596f50, 0x00596f40),
    },
];
pub fn register(registrations: &[Registration], host: &mut impl Registrar) -> Result<(), Unsupported> {
    for &registration in registrations {
        register_one(registration, host)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects_parser::Descriptor;
    #[derive(Default)]
    struct Host {
        count: u32,
        registrations: Vec<(Descriptor, Registration)>,
        allocations: Vec<u32>,
    }
    impl Registrar for Host {
        fn base_register(&mut self) {
            panic!("not a root registration");
        }
        fn allocate(&mut self, bytes: u32) -> Option<Descriptor> {
            assert!(bytes == 4 || bytes == 24);
            self.allocations.push(bytes);
            self.count += 1;
            Some(Descriptor(self.count))
        }
        fn shared_block_construct(&mut self, _: Descriptor) {
            // Generic block constructor, followed by the registered descriptor.
        }
        fn install_vptr(&mut self, _: Descriptor, _: u32) {}
        fn register(&mut self, d: Descriptor, r: Registration) -> u8 {
            self.registrations.push((d, r));
            0
        }
    }
    #[test]
    fn ec69_ec70_ec71_particle_registration_order_and_distinct_alias_allocations() {
        let mut h = Host::default();
        register(&PARTICLES, &mut h).unwrap();
        assert_eq!(h.count, 27);
        assert_eq!(
            h.registrations.iter().map(|(_, r)| r.name).collect::<Vec<_>>(),
            [
                "zoom",
                "color",
                "alpha",
                "size",
                "aspect",
                "rotate",
                "source",
                "emit",
                "force",
                "warp",
                "randomWalk",
                "stretch",
                "life",
                "rate",
                "inject",
                "maintain",
                "texture",
                "model",
                "align",
                "collide",
                "terrainRepel",
                "effectBase",
                "timedEffect",
                "color255",
                "colour",
                "colour255",
                "alpha255"
            ]
        );
        let unique: std::collections::BTreeSet<_> = h.registrations.iter().map(|(_, r)| r.descriptor).collect();
        assert_eq!(unique.len(), 23);
        let colors: Vec<_> = h
            .registrations
            .iter()
            .filter(|(_, r)| r.descriptor == 0xa9fb68)
            .collect();
        assert_eq!(colors.len(), 4);
        for (d, r) in colors {
            assert_ne!(d.0, 0);
            assert_eq!(r.hooks, Hooks::Direct(0x59bdc0));
        }
        assert_ne!(h.registrations[1].0, h.registrations[23].0);
        assert_ne!(h.registrations[2].0, h.registrations[26].0);
        assert_eq!(h.registrations[26].1.hooks, Hooks::Direct(0x59c060));
    }
    #[test]
    fn ec120_ec131_ec141_dynamic_sequence_shake_light_registration() {
        for (entries, names) in [
            (&DYNAMIC_PARTICLE[..], vec!["effectBase", "model", "mass", "friction"]),
            (&SEQUENCE[..], vec!["wait", "play"]),
            (
                &SHAKE[..],
                vec!["length", "amplitude", "frequency", "shakeAspect", "table"],
            ),
            (
                &LIGHT[..],
                vec!["strength", "length", "color", "color255", "colour", "colour255"],
            ),
        ] {
            let mut h = Host::default();
            register(entries, &mut h).unwrap();
            assert_eq!(h.count as usize, names.len());
            assert_eq!(h.registrations.iter().map(|(_, r)| r.name).collect::<Vec<_>>(), names);
        }
        assert_eq!(SEQUENCE[0].descriptor, 0xa9fcc8);
        assert_eq!(SEQUENCE[1].descriptor, 0xa9fcb8);
        assert_eq!(LIGHT[2].descriptor, LIGHT[5].descriptor);
    }
    #[test]
    fn ec161_ec272_through_ec276_decal_and_group_dictionaries() {
        let mut h = Host::default();
        register(&DECAL, &mut h).unwrap();
        assert_eq!(h.count, 11);
        assert_eq!(
            h.registrations
                .iter()
                .map(|(_, r)| r.descriptor)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            7
        );
        assert_eq!(
            DECAL.iter().map(|r| r.name).collect::<Vec<_>>(),
            [
                "color",
                "color255",
                "colour",
                "colour255",
                "alpha",
                "alpha255",
                "size",
                "aspect",
                "rotate",
                "life",
                "texture"
            ]
        );
        let mut h = Host::default();
        register(&EFFECT, &mut h).unwrap();
        assert_eq!(h.count, 19);
        assert_eq!(&h.allocations[..17], &[4; 17]);
        assert_eq!(&h.allocations[17..], &[24; 2]);
        assert_eq!(
            h.registrations
                .iter()
                .map(|(_, r)| r.descriptor)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            16
        );
        for i in 9..13 {
            assert_eq!(EFFECT[i].descriptor, 0xa9fe1c);
            assert_eq!(EFFECT[i].hooks, Hooks::Direct(0x5a30b0));
            if i > 9 {
                assert_ne!(h.registrations[i - 1].0, h.registrations[i].0);
            }
        }
        assert_eq!(
            [EFFECT[0].descriptor, EFFECT[2].descriptor, EFFECT[15].descriptor],
            [0xa9feac, 0xa9fe8c, 0xa9fdec]
        );
        assert_eq!(EFFECT[17].hooks, Hooks::Block(0x596e70, 0x596fe0));
        assert_eq!(EFFECT[18].hooks, Hooks::Block(0x596f50, 0x596f40));
        let mut h = Host::default();
        register(&EFFECT_GROUP, &mut h).unwrap();
        assert_eq!(h.count, 1);
        assert_eq!(
            EFFECT_GROUP[0].descriptor,
            crate::effects_parser::ROOT_COMMANDS[7].descriptor
        );
        // EC275's claim of equal handlers is unsettled: EF16 records the
        // conflict with effects-parser §6.1. Both literal ledgers are retained.
        assert_eq!(EFFECT_GROUP[0].hooks, Hooks::Direct(0x00596980));
        assert_eq!(crate::effects_parser::ROOT_COMMANDS[7].hooks, Hooks::Direct(0x0059a730));
    }
}

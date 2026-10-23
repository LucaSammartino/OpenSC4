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
    }
    impl Registrar for Host {
        fn base_register(&mut self) {
            panic!("not a root registration");
        }
        fn allocate(&mut self, bytes: u32) -> Option<Descriptor> {
            assert_eq!(bytes, 4);
            self.count += 1;
            Some(Descriptor(self.count))
        }
        fn shared_block_construct(&mut self, _: Descriptor) {
            panic!("ordinary children only");
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
}

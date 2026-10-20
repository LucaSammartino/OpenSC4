// SPDX-License-Identifier: GPL-3.0-or-later
//! The install sweep of the effect directory (`docs/specs/effects.md` §3, §8.4, §9.4, §10): reads the resource
//! (type `0xEA5118B0`, group `0xEA5118B1`, instance 1) from the installed game's packages at run time, parses it with
//! `fc4_core::effects::EffectDirectory`, reports the counts (expected: 25 brush records, 64 attractors, 57 game
//! records, 1154 visual effects) and checks the shipped records and the disaster mapping of `fc4_core::effects::
//! shipped` against it. Nothing from the install is committed.
//!
//! The install is found through `FC4_GAME_DIR`, or the default GOG path; the test passes with a note when neither
//! exists. Game files are only opened for reading.
//!
//! `cargo test -p fc4-formats --test effects_directory -- --nocapture`

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use fc4_core::effects::{
    component_type, shipped, EffectDirectory, DIRECTORY_GROUP, DIRECTORY_INSTANCE, DIRECTORY_TYPE,
};
use fc4_formats::tgi::Tgi;
use fc4_formats::Package;

const DEFAULT_DIR: &str = r"C:\Program Files (x86)\GOG Galaxy\Games\SimCity 4 Deluxe Edition";
const PACKAGES: [&str; 6] = [
    "SimCity_1.dat",
    "SimCity_2.dat",
    "SimCity_3.dat",
    "SimCity_4.dat",
    "SimCity_5.dat",
    "EP1.dat",
];

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FC4_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DIR));
    dir.is_dir().then_some(dir)
}

/// The resource and the package it came from.
fn read_directory() -> Option<(String, Vec<u8>)> {
    let dir = game_dir()?;
    let tgi = Tgi::new(DIRECTORY_TYPE, DIRECTORY_GROUP, DIRECTORY_INSTANCE);
    for name in PACKAGES {
        let path = dir.join(name);
        let Ok(pkg) = Package::open(&path) else { continue };
        if let Ok(data) = pkg.read_tgi(&tgi) {
            return Some((name.to_owned(), data));
        }
    }
    None
}

/// The effects reachable from `i` through meta components (by name, every description of the name), `i` included.
fn reachable(d: &EffectDirectory, i: u32, seen: &mut BTreeSet<u32>) {
    if !seen.insert(i) {
        return;
    }
    for c in &d.effects[i as usize].components {
        if c.kind == component_type::META {
            for &j in d.indices_of(&c.name).unwrap_or(&[]) {
                reachable(d, j, seen);
            }
        }
    }
}

fn has_sim_component(d: &EffectDirectory, i: u32) -> bool {
    d.effects[i as usize]
        .components
        .iter()
        .any(|c| c.kind == component_type::BRUSH || c.kind == component_type::GAME)
}

#[test]
fn effects_directory_sweep() {
    let Some((package, data)) = read_directory() else {
        eprintln!("effects_directory: no installed effect directory (set FC4_GAME_DIR); skipped");
        return;
    };
    let d = EffectDirectory::parse(&data).expect("the shipped effect directory parses");
    let o = d.offsets;
    println!(
        "effects_directory: {package}, {} bytes, root version {}, brush section at 0x{:X}, visual effects at 0x{:X} \
         (version {}), name map 0x{:X}..0x{:X} ({} bytes after it)",
        data.len(),
        d.root_version,
        o.brushes,
        o.effects,
        d.effects_version,
        o.names,
        o.names_end,
        data.len() - o.names_end
    );
    println!(
        "effects_directory: {} brush records, {} attractors, {} game records, {} visual effects, {} names",
        d.brushes.len(),
        d.attractors.len(),
        d.games.len(),
        d.effects.len(),
        d.names.len()
    );
    assert_eq!(package, "SimCity_1.dat");
    assert_eq!(o.brushes, fc4_core::effects::directory::SHIPPED_BRUSH_SECTION);
    assert_eq!(
        (d.brushes.len(), d.attractors.len(), d.games.len(), d.effects.len()),
        (25, 64, 57, 1154)
    );
    assert_eq!(d.effects_version, 2);

    // §3: priorities 1 (882), 3 (271), 5 (1); every description has a name.
    let mut prio: BTreeMap<u32, usize> = BTreeMap::new();
    for e in &d.effects {
        *prio.entry(e.priority).or_default() += 1;
    }
    println!("effects_directory: priorities {prio:?}");
    assert_eq!(prio, BTreeMap::from([(1, 882), (3, 271), (5, 1)]));
    let named: BTreeSet<u32> = d.names.iter().map(|(_, i)| *i).collect();
    assert_eq!(named.len(), d.effects.len());
    let mut kinds: BTreeMap<u8, usize> = BTreeMap::new();
    for c in d.effects.iter().flat_map(|e| &e.components) {
        *kinds.entry(c.kind).or_default() += 1;
    }
    println!("effects_directory: components by type {kinds:?}");

    // §8.4: the brush records.
    for (i, (got, want)) in d.brushes.iter().zip(shipped::BRUSHES.iter()).enumerate() {
        assert_eq!(got, want, "brush record {i}");
    }
    // §9.4: the game records' simulation fields.
    for (i, got) in d.games.iter().enumerate() {
        let want = shipped::game_record(i).unwrap();
        assert!(shipped::same_simulation_fields(got, &want), "game record {i}: {got:?}");
        assert_eq!(got.version, 1, "game record {i}");
    }
    for i in shipped::MAP_ONLY_GAME_RECORDS {
        assert_ne!(d.games[i].map_kind, 0, "game record {i} has an effect map");
    }
    assert_eq!(
        d.attractors[shipped::VOLCANO_ATTRACTOR.1 as usize].name,
        shipped::VOLCANO_ATTRACTOR.0
    );

    // §10.1: volcano_control and its creators.
    let vc = d.indices_of(shipped::VOLCANO_CONTROL).unwrap();
    assert_eq!(vc, &[shipped::VOLCANO_CONTROL_INDEX]);
    let v = &d.effects[vc[0] as usize];
    assert_eq!(v.priority, shipped::VOLCANO_CONTROL_PRIORITY);
    assert_eq!(v.components.len(), 10);
    for (c, want) in v.components.iter().zip(shipped::VOLCANO_CREATORS.iter()) {
        assert_eq!(c.kind, component_type::META);
        assert_eq!(
            (c.name.as_str(), c.select_group, c.select_weight),
            (want.name, shipped::VOLCANO_SELECT_GROUP, want.weight)
        );
        let ci = d.indices_of(want.name).unwrap();
        assert_eq!(ci.len(), 1);
        let creator = &d.effects[ci[0] as usize];
        assert_eq!(creator.priority, 1);
        let idx = |k: u8| -> Vec<u32> {
            creator
                .components
                .iter()
                .filter(|c| c.kind == k)
                .map(|c| c.index)
                .collect()
        };
        assert_eq!(idx(component_type::BRUSH), vec![want.brush as u32], "{}", want.name);
        assert_eq!(idx(component_type::GAME), vec![want.game as u32], "{}", want.name);
    }
    let a = &v.components[9];
    assert_eq!(
        (a.kind, a.index, a.flags),
        (component_type::ATTRACTOR, shipped::VOLCANO_ATTRACTOR.1, 1)
    );

    // §10.2: the control effects; the meteor craters' and the bolts' select groups.
    for c in shipped::CONTROL_EFFECTS {
        assert_eq!(d.indices_of(c.control), Some(&[c.index][..]), "{}", c.menu);
        assert_eq!(d.effects[c.index as usize].priority, 1, "{}", c.menu);
    }
    let weights = |name: &str| -> Vec<u16> {
        let i = d.indices_of(name).unwrap()[0] as usize;
        d.effects[i]
            .components
            .iter()
            .filter(|c| c.select_group > 0)
            .map(|c| c.select_weight)
            .collect()
    };
    assert_eq!(weights("meteor_stuff_terrain"), shipped::METEOR_CRATER_WEIGHTS.to_vec());
    assert_eq!(
        weights("lightningstrike_trigger"),
        shipped::LIGHTNING_BOLT_WEIGHTS.to_vec()
    );
    for (name, b) in shipped::METEOR_CRATERS {
        let i = d.indices_of(name).unwrap()[0] as usize;
        let brushes: Vec<u32> = d.effects[i]
            .components
            .iter()
            .filter(|c| c.kind == component_type::BRUSH)
            .map(|c| c.index)
            .collect();
        assert_eq!(brushes, vec![b as u32], "{name}");
    }

    // §5.2: the components of every effect that reaches a brush or game record (through meta components) have the
    // range [1, 6), so the damage does not depend on the zoom; the range of other components varies (render).
    let mut sim_reaching = 0;
    let mut other_ranges = 0usize;
    for i in 0..d.effects.len() as u32 {
        let mut seen = BTreeSet::new();
        reachable(&d, i, &mut seen);
        let reaches_sim = seen.iter().any(|&j| has_sim_component(&d, j));
        for c in &d.effects[i as usize].components {
            let to_sim = match c.kind {
                component_type::BRUSH | component_type::GAME => true,
                component_type::META => d.indices_of(&c.name).unwrap_or(&[]).iter().any(|&j| {
                    let mut s = BTreeSet::new();
                    reachable(&d, j, &mut s);
                    s.iter().any(|&k| has_sim_component(&d, k))
                }),
                _ => false,
            };
            if to_sim {
                assert_eq!((c.lod_min, c.lod_max), (1, 6), "effect {i} component {}", c.name);
            } else if (c.lod_min, c.lod_max) != (1, 6) {
                other_ranges += 1;
            }
        }
        sim_reaching += usize::from(reaches_sim);
    }
    println!(
        "effects_directory: {sim_reaching} effects reach a brush or game record, all through [1, 6) components; \
         {other_ranges} other components have another range"
    );
    // Every meta component names a known effect.
    for (i, e) in d.effects.iter().enumerate() {
        for c in &e.components {
            if c.kind == component_type::META {
                assert!(d.has(&c.name), "effect {i}: meta '{}' unknown", c.name);
            }
        }
    }
    // The resource re-parses at the offsets found.
    let again = EffectDirectory::parse_at(&data, o.brushes, o.effects).unwrap();
    assert_eq!(again.names, d.names);
    // The locator without the shipped offset (a byte inserted after the root version): the scan finds the same
    // sections one byte later.
    let mut shifted = data[..2].to_vec();
    shifted.push(0);
    shifted.extend_from_slice(&data[2..]);
    let s = EffectDirectory::parse(&shifted).unwrap();
    assert_eq!((s.offsets.brushes, s.offsets.effects), (o.brushes + 1, o.effects + 1));
    assert_eq!(s.effects.len(), d.effects.len());
}

// SPDX-License-Identifier: GPL-3.0-or-later
use fc4_core::save::{SaveRecord, RECORD_SIZE_V4};
use fc4_core::store::archive::{read_class, write_class, ClassIndex, StoredObject};
use fc4_core::store::{INDEX_TYPE, STORE_GROUP};
use fc4_formats::city_save::{first_difference, CitySave, Report};
use fc4_formats::dbpf::Compression;
use fc4_formats::{Package, PackageBuilder, Tgi};
use std::path::PathBuf;

fn fixture() -> Vec<u8> {
    let mut simulator = vec![0; RECORD_SIZE_V4];
    simulator[0] = 4;
    let payload = SaveRecord::decode(&simulator).unwrap().encode();
    let mut builder = PackageBuilder::new();
    let index = ClassIndex {
        entries_written: 2,
        classes: vec![(0x2990_C1E5, 1), (0xDEAD_BEEF, 1)],
    };
    builder
        .add(
            Tgi::new(INDEX_TYPE, STORE_GROUP, 0),
            &index.write(),
            Compression::Always,
        )
        .unwrap();
    for (class, id, payload) in [(0x2990_C1E5, 57, payload), (0xDEAD_BEEF, 99, vec![12, 13, 14])] {
        let bytes = write_class(&[StoredObject { id, payload }]).unwrap();
        builder
            .add(Tgi::new(class, STORE_GROUP, 0), &bytes, Compression::Always)
            .unwrap();
    }
    builder.build().unwrap()
}

#[test]
fn codecs_are_counted_separately_from_unknown_payloads_and_ids_survive() {
    let source = Package::from_bytes(fixture()).unwrap();
    let model = CitySave::read(&source).unwrap();
    let output = model.write().unwrap();
    let report = model.audit(&output).unwrap();
    assert!(report.differences.is_empty());
    assert_eq!(report.types[&0x2990_C1E5].identical, 1);
    assert_eq!(report.types[&0xDEAD_BEEF].identical, 0);
    assert_eq!(report.types[&0xDEAD_BEEF].not_handled, 1);
    let written = Package::from_bytes(&output).unwrap();
    let objects = read_class(&written.read_tgi(&Tgi::new(0x2990_C1E5, STORE_GROUP, 0)).unwrap()).unwrap();
    assert_eq!(objects[0].id, 57);
    let second = CitySave::read(&written).unwrap().write().unwrap();
    assert_eq!(output, second);
}

#[test]
fn damage_and_first_differences_are_reported() {
    let mut record = write_class(&[StoredObject {
        id: 3,
        payload: vec![1, 2],
    }])
    .unwrap();
    record[12] ^= 1;
    assert!(read_class(&record).unwrap_err().contains("CRC"));
    for length in 1..12 {
        assert!(read_class(&record[..length]).is_err());
    }
    assert_eq!(first_difference(&[1, 2], &[1, 3]), Some(1));
    assert_eq!(first_difference(&[1], &[1, 2]), Some(1));
    assert_eq!(first_difference(&[1], &[1]), None);
    let source = Package::from_bytes(fixture()).unwrap();
    let model = CitySave::read(&source).unwrap();
    let mut changed = PackageBuilder::new();
    for entry in source.entries().iter().filter(|e| e.tgi != fc4_formats::dbpf::DIR_TGI) {
        let mut bytes = source.read(entry).unwrap();
        if entry.tgi.type_id == 0x2990_C1E5 {
            bytes[12] ^= 1;
        }
        changed.add(entry.tgi, &bytes, Compression::None).unwrap();
    }
    let report = model.audit(&changed.build().unwrap()).unwrap();
    assert_eq!(report.types[&0x2990_C1E5].differs, 1);
    assert!(report.differences[0].contains("byte 12"));
}

#[test]
fn terrain_field_edges_and_class_references_roundtrip_independently() {
    use fc4_core::store::archive::ObjectReference;
    use fc4_core::stream::{ByteWriter, OutStream};
    use fc4_core::terrain::{SurfaceWaterRecord, Terrain, TerrainRecord, TerrainTuning, TerrainViewRecord};
    let terrain = Terrain::blank(2, 2, TerrainTuning::default(), 308.0).unwrap();
    let mut builder = PackageBuilder::new();
    for (key, bytes) in [
        (Tgi::new(0x8A91_E7E3, 0xE98F_9525, 3), terrain.write_misc()),
        (Tgi::new(0xA9DD_6FF4, 0xE98F_9525, 1), terrain.write_altitudes()),
        (Tgi::new(0x8A91_E7E0, 0xE98F_9525, 2), terrain.write_properties()),
        // These deliberately differ from the interior's edges, including raw float bits.
        (
            Tgi::new(0xA9DD_6FF4, 0xE98F_9525, 0x64),
            Terrain::write_edge_altitudes(&[f32::from_bits(0x8000_0000), f32::from_bits(0x7FC0_1234)]),
        ),
    ] {
        builder.add(key, &bytes, Compression::Always).unwrap();
    }
    let mut w = ByteWriter::new();
    TerrainRecord {
        view: ObjectReference {
            id: 44,
            class: 0xC9B8_4E10,
        },
    }
    .write(&mut w);
    let terrain_payload = w.into_bytes();
    let mut w = ByteWriter::new();
    TerrainViewRecord { value: 1 }.write(&mut w);
    let view_payload = w.into_bytes();
    let mut w = ByteWriter::new();
    SurfaceWaterRecord {
        terrain: ObjectReference {
            id: 59,
            class: 0xE98F_9525,
        },
    }
    .write(&mut w);
    let water_payload = w.into_bytes();
    for (class, id, payload) in [
        (0xE98F_9525, 59, terrain_payload),
        (0xC9B8_4E10, 44, view_payload),
        (0x0B8D_A7DB, 56, water_payload),
    ] {
        let bytes = write_class(&[StoredObject { id, payload }]).unwrap();
        builder
            .add(Tgi::new(class, STORE_GROUP, 0), &bytes, Compression::Always)
            .unwrap();
    }
    let bytes = builder.build().unwrap();
    let package = Package::from_bytes(bytes).unwrap();
    let model = CitySave::read(&package).unwrap();
    let report = model.audit(&model.write().unwrap()).unwrap();
    assert!(report.differences.is_empty(), "{:?}", report.differences);
    assert!(report.types.values().all(|s| s.not_handled == 0 && s.differs == 0));
    // Null references consume exactly the ID word, leaving the next field intact.
    let mut w = ByteWriter::new();
    ObjectReference { id: 0, class: 999 }.write(&mut w);
    w.put_u8(7);
    assert_eq!(w.into_bytes(), [0, 0, 0, 0, 7]);
}

#[test]
fn every_numeric_grid_uses_its_owner_codec_including_ieee_bits() {
    use fc4_core::grid::{CityDims, SimGrid};
    use fc4_core::stream::{ByteWriter, SaveObject};
    let mut builder = PackageBuilder::new();
    macro_rules! add_grid {
        ($ty:ty, $value:expr) => {{
            let grid = SimGrid::<$ty>::create(&CityDims::square(1), 73, 1);
            grid.data.borrow_mut().values[0] = $value;
            let mut w = ByteWriter::new();
            assert!(grid.write(&mut w));
            let bytes = write_class(&[StoredObject {
                id: grid.class_id() + 100,
                payload: w.into_bytes(),
            }])
            .unwrap();
            builder
                .add(Tgi::new(grid.class_id(), STORE_GROUP, 0), &bytes, Compression::Always)
                .unwrap();
        }};
    }
    add_grid!(u8, 255);
    add_grid!(i8, -127);
    add_grid!(u16, 65535);
    add_grid!(i16, -32767);
    add_grid!(u32, 0xCDCD_CDCD);
    add_grid!(i32, i32::MIN);
    add_grid!(u64, 0xBAAD_F00D_CCCC_CCCC);
    add_grid!(i64, i64::MIN);
    add_grid!(f32, f32::from_bits(0x7FC0_1234));
    add_grid!(f64, f64::from_bits(0x7FF8_1234_5678_ABCD));
    let package = Package::from_bytes(builder.build().unwrap()).unwrap();
    let model = CitySave::read(&package).unwrap();
    let report = model.audit(&model.write().unwrap()).unwrap();
    assert!(report.differences.is_empty(), "{:?}", report.differences);
    assert_eq!(report.types.len(), 11); // ten classes plus regenerated DIR
    assert!(report.types.values().all(|s| s.identical == 1));
}

fn golden_directory() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("FC4_GOLDEN_DIR") {
        let path = PathBuf::from(path);
        assert!(path.is_dir(), "FC4_GOLDEN_DIR does not exist");
        return Some(path);
    }
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .map(|path| path.join("build-runs/golden"))
        .find(|path| path.is_dir())
}

#[test]
fn golden_supported_owners_roundtrip_when_corpus_is_present() {
    let Some(directory) = golden_directory() else {
        eprintln!("golden corpus absent; set FC4_GOLDEN_DIR to enable the save sweep");
        return;
    };
    let mut paths: Vec<_> = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path().join("end.sc4"))
        .filter(|p| p.is_file())
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "golden directory contains no end.sc4 saves");
    let mut total = Report::default();
    for path in &paths {
        let package = Package::open(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let model = CitySave::read(&package).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let output = model.write().unwrap();
        let report = model.audit(&output).unwrap();
        assert!(
            report.differences.is_empty(),
            "{}: {:?}",
            path.display(),
            report.differences
        );
        total.merge(report);
    }
    eprintln!("{} golden saves\n{}", paths.len(), total.table());
    assert!(total.types[&0x2990_C1E5].identical > 0);
}

#[test]
fn shipped_regions_reach_stable_second_write_when_present() {
    let install = std::env::var_os("FC4_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)\GOG Galaxy\Games\SimCity 4 Deluxe Edition"));
    let regions = install.join("Regions");
    if !regions.is_dir() {
        eprintln!("shipped Regions absent; set FC4_GAME_DIR to enable the legacy save sweep");
        return;
    }
    fn collect(path: &std::path::Path, files: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect(&path, files);
            } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("sc4")) {
                files.push(path);
            }
        }
    }
    let mut files = Vec::new();
    collect(&regions, &mut files);
    files.sort();
    assert!(!files.is_empty(), "shipped Regions contains no saves");
    for path in &files {
        let original = Package::open(path).unwrap();
        let model = CitySave::read(&original).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let first = model.write().unwrap();
        let package = Package::from_bytes(&first).unwrap();
        let loaded = CitySave::read(&package).unwrap();
        let second = loaded.write().unwrap();
        let report = loaded.audit(&second).unwrap();
        assert!(
            report.differences.is_empty(),
            "{}: {:?}",
            path.display(),
            report.differences
        );
        assert_eq!(first, second, "{}: second output is unstable", path.display());
    }
    eprintln!("{} shipped saves have a stable second write", files.len());
}

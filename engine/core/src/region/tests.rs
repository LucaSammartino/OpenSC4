// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Clone)]
struct Package {
    version: AppVersion,
    city: Option<City>,
}

struct Host {
    directories: BTreeSet<String>,
    files: BTreeMap<String, Vec<u8>>,
    packages: BTreeMap<String, Package>,
    entries: Vec<DirectoryEntry>,
    resources: BTreeMap<(u32, u32, u32), Vec<u8>>,
    events: Vec<String>,
    fail: BTreeSet<String>,
    active: String,
    rng: u32,
    seconds: i64,
}

impl Default for Host {
    fn default() -> Self {
        Self {
            directories: ["R\\".into(), "R\\\\".into(), "R\\A\\".into(), "D\\".into()].into(),
            files: BTreeMap::new(),
            packages: BTreeMap::new(),
            entries: Vec::new(),
            resources: BTreeMap::new(),
            events: Vec::new(),
            fail: BTreeSet::new(),
            active: "A".into(),
            rng: 0,
            seconds: 0,
        }
    }
}

impl Host {
    fn add(&mut self, path: &str, city: City) {
        self.files
            .insert(path.into(), b"opaque DBPF; unrelated records retained".to_vec());
        self.packages.insert(
            path.into(),
            Package {
                version: AppVersion::Value(65),
                city: Some(city),
            },
        );
    }
    fn entry(&mut self, name: &str, city: City) {
        self.add(&format!("R\\A\\{name}"), city);
        self.entries.push(DirectoryEntry {
            name: name.into(),
            regular_file: true,
        });
    }
    fn image(&mut self, image: Image) {
        self.files
            .insert("R\\A\\config.bmp".into(), encode_bmp(&image).unwrap());
    }
    fn event_count(&self, prefix: &str) -> usize {
        self.events.iter().filter(|e| e.starts_with(prefix)).count()
    }
}

impl RegionHost for Host {
    fn region_directory(&self) -> String {
        "R\\".into()
    }
    fn download_directory(&self) -> String {
        "D\\".into()
    }
    fn active_directory_name(&self) -> String {
        self.active.clone()
    }
    fn directory_exists(&mut self, path: &str) -> bool {
        self.directories.contains(path)
    }
    fn create_directory(&mut self, path: &str) -> bool {
        self.events.push(format!("mkdir:{path}"));
        !self.fail.contains("mkdir") && self.directories.insert(path.into())
    }
    fn remove_directory_recursive(&mut self, path: &str) -> bool {
        self.events.push(format!("rmdir:{path}"));
        if self.fail.contains("rmdir") {
            return false;
        }
        self.files.retain(|key, _| !key.starts_with(path));
        self.packages.retain(|key, _| !key.starts_with(path));
        self.directories.remove(path)
    }
    fn list_directory(&mut self, path: &str) -> Vec<DirectoryEntry> {
        self.events.push(format!("list:{path}"));
        self.entries.clone()
    }
    fn file_exists(&mut self, path: &str) -> bool {
        self.files.contains_key(path)
    }
    fn read_file(&mut self, path: &str) -> Option<Vec<u8>> {
        if self.fail.contains("read") {
            None
        } else {
            self.files.get(path).cloned()
        }
    }
    fn write_file(&mut self, path: &str, bytes: &[u8]) -> bool {
        self.events.push(format!("write:{path}"));
        let parent = &path[..=path.rfind('\\').unwrap()];
        if self.fail.contains("write") || !self.directories.contains(parent) {
            return false;
        }
        self.files.insert(path.into(), bytes.to_vec());
        true
    }
    fn make_writable(&mut self, path: &str) {
        self.events.push(format!("writable:{path}"));
    }
    fn delete_file(&mut self, path: &str) -> bool {
        self.events.push(format!("delete:{path}"));
        if self.fail.contains("delete") {
            return false;
        }
        self.packages.remove(path);
        self.files.remove(path).is_some()
    }
    fn rename_file(&mut self, source: &str, target: &str) -> bool {
        self.events.push(format!("rename:{source}:{target}"));
        if self.fail.contains("rename") || self.files.contains_key(target) {
            return false;
        }
        let Some(bytes) = self.files.remove(source) else {
            return false;
        };
        self.files.insert(target.into(), bytes);
        if let Some(package) = self.packages.remove(source) {
            self.packages.insert(target.into(), package);
        }
        true
    }
    fn copy_file_overwriting(&mut self, source: &str, target: &str) -> bool {
        self.events.push(format!("copy:{source}:{target}"));
        if self.fail.contains("copy") {
            return false;
        }
        let Some(bytes) = self.files.get(source).cloned() else {
            return false;
        };
        self.files.insert(target.into(), bytes);
        if let Some(package) = self.packages.get(source).cloned() {
            self.packages.insert(target.into(), package);
        }
        true
    }
    fn load_image(&mut self, path: &str) -> Option<Image> {
        self.events.push(format!("image:{path}"));
        self.files.get(path).and_then(|bytes| decode_bmp(bytes).ok())
    }
    fn resource_image(&mut self, t: u32, g: u32, i: u32) -> Option<Image> {
        self.resources.get(&(t, g, i)).and_then(|bytes| decode_bmp(bytes).ok())
    }
    fn resource_exists(&mut self, t: u32, g: u32, i: u32) -> bool {
        self.resources.contains_key(&(t, g, i))
    }
    fn export_resource(&mut self, t: u32, g: u32, i: u32, path: &str) -> bool {
        self.events.push(format!("export:{i}:{path}"));
        if self.fail.contains("export") {
            self.files.remove(path);
            return false;
        }
        let Some(bytes) = self.resources.get(&(t, g, i)).cloned() else {
            return false;
        };
        self.files.insert(path.into(), bytes);
        true
    }
    fn localized_name(&mut self, group: u32, instance: u32) -> String {
        assert_eq!(group, 0x6A23_1EAA);
        if instance == 0xCAB6_D26E {
            "Berlin".into()
        } else {
            String::new()
        }
    }
    fn read_city_package(&mut self, path: &str) -> Option<CityPackage> {
        self.events.push(format!("package:{path}"));
        if self.fail.contains("package") {
            return None;
        }
        self.packages.get(path).map(|p| CityPackage {
            version: p.version,
            city: p.city.clone().map(City::counted),
        })
    }
    fn write_city_record(&mut self, path: &str, city: &CityRef) -> bool {
        self.events.push(format!("record:{path}"));
        if self.fail.contains("record") {
            return false;
        }
        let Some(package) = self.packages.get_mut(path) else {
            return false;
        };
        package.city = Some(city.read().unwrap().clone());
        true
    }
    fn shutdown_city(&mut self, city: &CityRef) {
        self.events.push(format!("shutdown:{}", city.read().unwrap().name));
    }
    fn activate_tutorial(&mut self, id: i32) {
        self.events.push(format!("activate:{id}"));
    }
    fn random_uniform(&mut self, upper: u32) -> u32 {
        self.events.push(format!("random:{upper}"));
        self.rng % upper
    }
    fn seconds_since_local_2000(&mut self) -> i64 {
        self.seconds
    }
}

fn city(x: i32, z: i32, sx: i32, sz: i32, name: &str, serial: u32) -> City {
    City {
        position: (x, z),
        size: (sx, sz),
        name: name.into(),
        serial,
        tutorial_id: -1,
        path: format!("R\\A\\City - {name}.sc4"),
        rearrange: false,
    }
}
fn region() -> Region {
    let mut region = Region::default();
    region.set_directory_name("A");
    region
}
fn image(width: usize, height: usize, points: &[(usize, usize, [u8; 3])]) -> Image {
    let mut pixels = vec![[0; 3]; width * height];
    for &(x, z, color) in points {
        pixels[z * width + x] = color;
    }
    Image::new(width, height, pixels).unwrap()
}
fn loc(x: i32, z: i32, size_exp: i32) -> Location {
    Location { x, z, size_exp }
}

#[test]
fn identity_fields_and_lifetime() {
    let mut r = Region::default();
    let mut h = Host::default();
    assert_eq!((r.initialized(), r.reference_count(), r.unused_word), (false, 0, 0));
    assert_eq!(
        (
            r.name(),
            r.directory_name(),
            r.base_terrain_type(),
            r.water_preferences()
        ),
        ("", "", 0, (60, 100))
    );
    let mut out = Some(17);
    assert!(!r.query_interface(9, &mut out));
    assert_eq!(out, Some(17));
    assert!(r.query_interface(1, &mut out));
    assert!(r.query_interface(REGION_IID, &mut out));
    assert_eq!(r.release(&mut h), 1);
    assert!(!r.destroyed());
    assert_eq!(r.release(&mut h), 0);
    assert!(r.destroyed());
    assert!(h.events.is_empty());
    let mut r = region();
    r.init(&mut h, &mut RegionGlobals::default());
    assert_eq!(r.release(&mut h), 0); // zero-reference release also shuts down
    assert_eq!(h.event_count("write:"), 4);
    assert_eq!(r.name(), "");
}

#[test]
fn location_vectors_colors_fit_and_discovery_order() {
    assert_eq!(
        image(4, 4, &[(0, 0, [0, 0, 255]), (0, 1, [255, 0, 0])]).locations(),
        [loc(0, 0, 2)]
    );
    assert!(
        image(4, 4, &[(0, 0, [255, 255, 0]), (3, 0, [0, 255, 0]), (0, 3, [0, 255, 0])])
            .locations()
            .is_empty()
    );
    for color in [
        [0, 0, 0],
        [254, 254, 254],
        [255, 255, 255],
        [255, 0, 255],
        [0, 255, 255],
    ] {
        assert!(image(4, 4, &[(0, 0, color)]).locations().is_empty());
    }
    let picture = image(
        4,
        4,
        &[(1, 0, [255, 99, 254]), (2, 0, [0, 255, 7]), (0, 2, [255, 0, 0])],
    );
    assert_eq!(picture.locations(), [loc(1, 0, 0), loc(2, 0, 1), loc(0, 2, 0)]);
    // Only the first pixel determines the square, even if another square previously covered
    // some cells: visited is tested only for this square's top-left.
    assert_eq!(
        image(4, 3, &[(1, 0, [0, 255, 0]), (0, 1, [0, 255, 0])]).locations(),
        [loc(1, 0, 1), loc(0, 1, 1)]
    );
}

#[test]
fn location_and_bmp_same_input_round_trips() {
    for location in [loc(0, 0, 0), loc(-7, i32::MAX, 2), loc(i32::MIN, -1, -3)] {
        assert_eq!(Location::decode(&location.encode()), Ok(location));
    }
    assert_eq!(Location::decode(&[0; 11]), Err(RecordError::Truncated));
    for width in 1..8 {
        let original = image(width, 5, &[(0, 0, [255, 0, 0]), (width - 1, 4, [0, 255, 0])]);
        let bytes = encode_bmp(&original).unwrap();
        let decoded = decode_bmp(&bytes).unwrap();
        assert_eq!(decoded, original);
        assert_eq!(encode_bmp(&decoded).unwrap(), bytes);
        assert_eq!(decoded.locations(), original.locations());
        assert!(decode_bmp(&bytes[..bytes.len() - 1]).is_err());
        // Convert the same file to a top-down bitmap.
        let mut top_down = bytes.clone();
        top_down[22..26].copy_from_slice(&(-5i32).to_le_bytes());
        let stride = (width * 3 + 3) & !3;
        for row in 0..5 {
            top_down[54 + row * stride..54 + (row + 1) * stride]
                .copy_from_slice(&bytes[54 + (4 - row) * stride..54 + (5 - row) * stride]);
        }
        assert_eq!(decode_bmp(&top_down).unwrap(), original);
    }
    assert!(Image::new(2, 2, vec![]).is_err());
    assert!(decode_bmp(b"bad").is_err());
    assert_eq!(
        encode_bmp(&Image::new(0, 0, vec![]).unwrap()),
        Err(RecordError::Invalid)
    );
}

#[test]
fn bounds_ignore_extents_and_degenerate_axes() {
    let mut r = region();
    for locations in [
        vec![],
        vec![loc(3, 4, 2)],
        vec![loc(0, 0, 1), loc(2, 0, 0)],
        vec![loc(0, 0, 1), loc(0, 2, 0)],
    ] {
        r.locations = locations;
        assert_eq!(r.bounding_rect(), [0; 4]);
    }
    r.locations = vec![loc(0, 0, 1), loc(3, 2, 0)];
    assert_eq!(r.bounding_rect(), [0, 0, 3, 2]);
    let mut copied = r.city_locations();
    copied.clear();
    assert_eq!(r.city_locations().len(), 2);
    r.locations = vec![loc(-7, -5, 2), loc(3, 2, 0)];
    assert_eq!(r.bounding_rect(), [-7, -5, 3, 2]);
}

#[test]
fn config_conformance_localization_and_range_patch() {
    let mut r = region();
    let mut h = Host::default();
    r.set_name("old");
    h.files.insert(
        "R\\A\\region.ini".into(),
        b"[regional settings]\rTerrain type = 2\rWater Min = 255\rWater Max = 254\rName = ".to_vec(),
    );
    r.load_config(&mut h);
    assert_eq!(
        (r.name(), r.base_terrain_type(), r.water_preferences()),
        ("old", 0, (60, 254))
    );
    for (name, expected) in [
        ("#0", "#0"),
        ("#zz", "#zz"),
        ("#0xcab6d26e", "Berlin"),
        ("#1", ""),
        ("# 0Xcab6d26e", "Berlin"),
    ] {
        h.files.insert(
            "R\\A\\region.ini".into(),
            format!("[Regional Settings]\nName = {name}\n").into_bytes(),
        );
        r.load_config(&mut h);
        assert_eq!(r.name(), expected);
    }
    h.files.insert(
        "R\\A\\region.ini".into(),
        b"[Regional Settings]\nTerrain type = 0x1\nWater Min = 254\nWater Max = -1\n".to_vec(),
    );
    r.set_base_terrain_type(1);
    r.load_config(&mut h);
    assert_eq!((r.base_terrain_type(), r.water_preferences()), (0, (254, 254)));
    let before = r.config;
    h.files
        .insert("R\\A\\region.ini".into(), b"[Other]\nWater Min=1\n".to_vec());
    r.load_config(&mut h);
    assert_eq!(r.config, before);
    h.files.remove("R\\A\\region.ini");
    r.load_config(&mut h);
    assert_eq!(r.config, before);
}

#[test]
fn config_write_codec_preserves_tail_and_replaces_first_keys() {
    let config = Config {
        terrain_type: 1,
        water_min: 254,
        water_max: 7,
    };
    let original = b"; retained\n[Regional Settings]\nNAME=old\n\n; keep\nWater Min=2\n[Other]\nThing = 1\n";
    let encoded = config.write_ini(original, "Berlin");
    assert!(encoded.starts_with(b"; retained\n[Regional Settings]\nName = Berlin\r\n; keep\n"));
    assert!(encoded.ends_with(b"[Other]\nThing = 1\n"));
    let mut decoded = Config::default();
    assert_eq!(decoded.apply_ini(&encoded).as_deref(), Some("Berlin"));
    assert_eq!(decoded, config);
    assert_eq!(decoded.write_ini(&encoded, "Berlin"), encoded);
    let mut duplicate = Config::default();
    duplicate.apply_ini(b"[Regional Settings]\nWater Min=3\nWater Min=4\n[Regional Settings]\nWater Max=9\n");
    assert_eq!((duplicate.water_min, duplicate.water_max), (3, 100));
    assert_eq!(
        config.write_ini_entry(Some(b"[Other]\n"), "x", 0),
        b"[Other]\n\r\n[Regional Settings]\r\nName = x\r\n"
    );
    assert_eq!(
        config.write_ini_entry(Some(b"[Regional Settings]\n;last"), "x", 0),
        b"[Regional Settings]\n;lastName = x\r\n"
    );
}

#[test]
fn city_boxes_overlap_removal_counted_references_and_list_append() {
    let mut r = region();
    let mut h = Host::default();
    let a = city(1, 2, 2, 2, "a", 1).counted();
    let b = city(-1, 0, 1, 1, "b", 2).counted();
    let a_id = r.insert_city(a.clone());
    r.insert_city(b.clone());
    assert_eq!(Arc::strong_count(&a), 2);
    assert_eq!(r.cell_count(), 5);
    assert_eq!(r.get_city(2, 3), Some(a_id));
    assert!(r.overlaps_occupied_cell(&city(2, 3, 2, 1, "c", 3).counted()));
    let mut list = vec![a.clone()];
    r.get_all_cities(&mut list);
    assert_eq!(list.len(), 3);
    assert!(Arc::ptr_eq(&list[1], &b));
    assert!(Arc::ptr_eq(&list[2], &a));
    r.remove_city(a_id, &mut h);
    assert_eq!(r.cell_count(), 1);
    assert!(r.boxed_city(a_id).is_none());
    assert_eq!(h.events, ["shutdown:a"]);
    drop(list);
    assert_eq!(Arc::strong_count(&a), 1);
    let replacement = city(-1, 0, 1, 1, "new", 3).counted();
    let new_id = r.insert_city(replacement); // replacement does not release old b box
    assert_eq!(Arc::strong_count(&b), 2);
    assert_eq!(r.get_city(-1, 0), Some(new_id));
    let zero = city(0, 0, 0, 2, "zero", 4).counted();
    r.insert_city(zero.clone());
    assert_eq!(Arc::strong_count(&zero), 2);
    assert_eq!(r.cell_count(), 1);
}

#[test]
fn remove_erases_rectangle_by_key_even_after_replacement() {
    let mut r = region();
    let mut h = Host::default();
    let old = r.insert_city(city(0, 0, 2, 1, "a", 0).counted());
    let new = r.insert_city(city(1, 0, 1, 1, "b", 0).counted());
    r.remove_city(old, &mut h);
    assert!(r.get_city(1, 0).is_none());
    assert!(r.boxed_city(new).is_some());
    assert_eq!(h.events, ["shutdown:a"]);
}

#[test]
fn package_version_gate_extensions_and_read_failures() {
    let mut h = Host::default();
    for (i, version) in [
        AppVersion::Missing,
        AppVersion::Unreadable,
        AppVersion::Value(61),
        AppVersion::Value(62),
        AppVersion::Value(63),
        AppVersion::Value(64),
        AppVersion::Value(65),
        AppVersion::Value(66),
    ]
    .into_iter()
    .enumerate()
    {
        let path = format!("R\\A\\v{i}.sc4");
        h.entry(&format!("v{i}.sc4"), city(i as i32, 0, 1, 1, "x", 0));
        h.packages.get_mut(&path).unwrap().version = version;
    }
    h.entry("upper.SC4", city(0, 0, 1, 1, "ignored", 0));
    h.entry("missing.sc4", city(0, 0, 1, 1, "missing", 0));
    h.packages.get_mut("R\\A\\missing.sc4").unwrap().city = None;
    h.entries.push(DirectoryEntry {
        name: "directory.sc4".into(),
        regular_file: false,
    });
    h.entries.push(DirectoryEntry {
        name: "invalid.sc4".into(),
        regular_file: true,
    });
    let loaded = load_cities_from_directory(&mut h, "R\\A\\");
    assert_eq!(loaded.len(), 5);
    assert_eq!(h.event_count("writable:"), 5);
    assert_eq!(loaded[0].read().unwrap().path, "R\\A\\v0.sc4");
    assert!(load_cities_from_directory(&mut h, "absent\\").is_empty());
    let c = city(0, 0, 1, 1, "x", 0).counted();
    assert!(!write_regional_city(&mut h, "R\\A\\v1.sc4", &c));
    assert!(write_regional_city(&mut h, "R\\A\\v0.sc4", &c));
    h.fail.insert("record".into());
    assert!(!write_regional_city(&mut h, "R\\A\\v0.sc4", &c));
}

#[test]
fn load_cities_overflow_and_x_size_only() {
    let mut r = region();
    let mut h = Host::default();
    r.locations = vec![loc(0, 0, 1), loc(3, 0, 0)];
    for (name, c) in [
        ("inside.sc4", city(1, 0, 1, 1, "inside", 1)),
        ("negative.sc4", city(-1, 0, 1, 1, "negative", 2)),
        ("valid.sc4", city(0, 0, 2, 1, "valid", 3)),
        ("duplicate.sc4", city(0, 0, 2, 2, "duplicate", 4)),
        ("wrongsize.sc4", city(3, 0, 2, 2, "wrongsize", 5)),
    ] {
        h.entry(name, c);
    }
    r.load_cities(&mut h);
    assert_eq!(r.cell_count(), 2); // Z size is NOT checked against the location
    assert_eq!(
        r.overflow()
            .iter()
            .map(|c| c.read().unwrap().name.clone())
            .collect::<Vec<_>>(),
        ["inside", "negative", "duplicate", "wrongsize"]
    );
    assert!(r.get_city(0, 1).is_none());
}

#[test]
fn duplicate_timbuktu_vector_and_copy_failure() {
    for fail in [false, true] {
        let mut r = region();
        let mut h = Host::default();
        r.locations = vec![loc(6, 5, 1)];
        h.entry(
            "City - Making Money Tutorial.sc4",
            city(6, 5, 2, 2, "Making Money Tutorial", 1),
        );
        h.entry("City - New City (21).sc4", city(6, 5, 2, 2, "New City", 2));
        r.load_cities(&mut h);
        assert_eq!(r.cell_count(), 4);
        assert_eq!(r.overflow.len(), 1);
        if fail {
            h.fail.insert("copy".into());
        }
        r.move_overflow_to_downloads(&mut h);
        assert!(r.overflow.is_empty());
        assert_eq!(h.files.contains_key("R\\A\\City - New City (21).sc4"), fail);
        assert_eq!(h.files.contains_key("D\\City - New City.sc4"), !fail);
        assert_eq!(h.event_count("shutdown:New City"), 1);
    }
}

#[test]
fn picture_fallback_keeps_old_locations_if_both_fail() {
    let mut r = region();
    let mut h = Host::default();
    r.locations = vec![loc(9, 8, 0)];
    r.load_city_locations(&mut h);
    assert_eq!(r.locations, [loc(9, 8, 0)]);
    let default = image(1, 1, &[(0, 0, [255, 0, 0])]);
    h.resources.insert(
        (PICTURE_TYPE, PICTURE_GROUP, DEFAULT_PICTURE),
        encode_bmp(&default).unwrap(),
    );
    h.files.insert("R\\A\\config.bmp".into(), b"bad".to_vec());
    r.load_city_locations(&mut h);
    assert_eq!(r.locations, [loc(0, 0, 0)]);
    h.image(image(1, 1, &[]));
    r.load_city_locations(&mut h);
    assert!(r.locations.is_empty());
}

#[test]
fn init_idempotent_shutdown_writes_resolved_name_and_retains_locations() {
    let mut h = Host::default();
    let mut r = region();
    let mut g = RegionGlobals::default();
    h.image(image(1, 1, &[(0, 0, [255, 0, 0])]));
    h.files.insert(
        "R\\A\\region.ini".into(),
        b"[Regional Settings]\nName = #0xcab6d26e\n".to_vec(),
    );
    assert!(!r.init(&mut h, &mut g));
    assert!(g.init_seen);
    assert!(r.initialized());
    h.entry("City - a.sc4", city(0, 0, 1, 1, "a", 1));
    assert!(!r.init(&mut h, &mut g));
    assert_eq!(h.event_count("list:"), 1);
    r.shutdown(&mut h);
    assert!(!r.initialized());
    assert_eq!(r.locations.len(), 1);
    assert!(String::from_utf8_lossy(&h.files["R\\A\\region.ini"]).contains("Name = Berlin\r\n"));
    let writes = h.event_count("write:");
    r.shutdown(&mut h);
    assert_eq!(h.event_count("write:"), writes);
    assert!(r.init(&mut h, &mut g));
    r.shutdown(&mut h);
    assert_eq!(r.cell_count(), 0);
    assert_eq!(h.event_count("shutdown:a"), 1);
}

#[test]
fn shutdown_overflow_and_ignored_write_failures_delete_recursive() {
    let mut h = Host::default();
    let mut r = region();
    r.initialized = true;
    r.locations = vec![loc(0, 0, 0)];
    r.overflow.push(city(2, 0, 1, 1, "overflow", 0).counted());
    let c = city(0, 0, 1, 1, "a", 0);
    h.add(&c.path.clone(), c.clone());
    r.insert_city(c.counted());
    h.files.insert("R\\A\\extra.bin".into(), vec![1]);
    r.delete(&mut h);
    assert!(!h.directories.contains("R\\A\\"));
    assert!(!h.files.keys().any(|p| p.starts_with("R\\A\\")));
    assert_eq!(h.event_count("shutdown:overflow"), 1);
    assert_eq!(h.event_count("write:"), 4);
    assert_eq!(r.locations.len(), 1);
    assert!(!r.initialized());
    let remove_at = h.events.iter().position(|s| s.starts_with("rmdir:")).unwrap();
    assert!(h.events[remove_at + 1..].iter().any(|s| s.starts_with("write:")));
}

#[test]
fn create_sanitization_templates_starter_and_failure_paths() {
    let mut h = Host::default();
    h.directories.insert("R\\Berlin\\".into());
    let bitmap = encode_bmp(&image(1, 1, &[(0, 0, [255, 0, 0])])).unwrap();
    h.resources
        .insert((PICTURE_TYPE, PICTURE_GROUP, FIRST_TEMPLATE), bitmap.clone());
    h.resources
        .insert((PICTURE_TYPE, PICTURE_GROUP, FIRST_TEMPLATE + 1), bitmap.clone());
    h.resources
        .insert((PICTURE_TYPE, PICTURE_GROUP, FIRST_TEMPLATE + 3), bitmap.clone()); // gap terminates counting
    h.rng = 1;
    let r = Region::create(&mut h, "Berlin");
    assert_eq!(
        (r.name(), r.directory_name(), r.initialized(), r.reference_count()),
        ("Berlin", "Berlin (2)", false, 0)
    );
    assert_eq!(h.files["R\\Berlin (2)\\config.bmp"], bitmap);
    assert!(h.events.contains(&"random:2".into()));
    assert!(!h.files.contains_key("R\\Berlin (2)\\region.ini"));
    let r = Region::create(&mut h, "a:b?");
    assert_eq!((r.name(), r.directory_name()), ("a:b?", "ab"));
    let starter = Region::create_starter(&mut h);
    assert_eq!((starter.name(), starter.directory_name()), ("", " (2)"));
    h.fail.insert("mkdir".into());
    let failed = Region::create(&mut h, "failed");
    assert_eq!(failed.name(), "failed");
    assert!(!h.files.contains_key("R\\failed\\config.bmp"));
    h.fail.remove("mkdir");
    h.fail.insert("export".into());
    let _ = Region::create(&mut h, "export failure");
    assert!(!h.files.contains_key("R\\export failure\\config.bmp"));
    let mut no_templates = Host::default();
    let _ = Region::create(&mut no_templates, "empty");
    assert!(no_templates.files.is_empty());
    assert_eq!(sanitize_file_name("a<>:\\/\"|*?b"), "ab");
    assert_eq!(sanitize_file_name(&"é".repeat(129)), "é".repeat(128));
    assert_eq!(sanitize_file_name(&("a".repeat(255) + "é")), "a".repeat(255));
}

#[test]
fn filename_conformance_and_serial_generation() {
    for (existing, incoming, expected) in [
        (5, 7, "City - New City (2).sc4"),
        (5, 5, "City - New City.sc4"),
        (0, 7, "City - New City.sc4"),
        (0, 0, "City - New City (2).sc4"),
    ] {
        let mut h = Host::default();
        h.add("R\\A\\City - New City.sc4", city(0, 0, 1, 1, "New City", existing));
        assert_eq!(
            active_save_path_for_city(&mut h, "New City", incoming),
            format!("R\\A\\{expected}")
        );
    }
    let mut h = Host::default();
    h.files.insert("D\\City - New City.sc4".into(), vec![]);
    h.files.insert("D\\City - New City (2).sc4".into(), vec![]);
    assert_eq!(
        save_path_for_city(&mut h, "D\\", "New City", 1),
        "D\\City - New City (3).sc4"
    );
    h.seconds = 1;
    h.rng = 15;
    assert_eq!(create_city_serial_number(&mut h), (28_801 << 4) | 15);
    h.seconds = -28_801;
    h.rng = 3;
    assert_eq!(create_city_serial_number(&mut h), 0xFFFF_FFF3);
}

#[test]
fn reload_success_failure_and_delete_city() {
    let mut h = Host::default();
    let mut r = region();
    let c = city(0, 0, 2, 2, "a", 1);
    h.add(&c.path.clone(), c.clone());
    let original = r.insert_city(c.counted());
    let mut id = Some(original);
    h.packages
        .get_mut("R\\A\\City - a.sc4")
        .unwrap()
        .city
        .as_mut()
        .unwrap()
        .position = (4, 5);
    r.reload_city(&mut id, &mut h);
    assert_ne!(id, Some(original));
    assert_eq!(id, r.get_city(4, 5));
    assert!(r.get_city(0, 0).is_none());
    let stale = id;
    h.fail.insert("package".into());
    r.reload_city(&mut id, &mut h);
    assert_eq!(id, stale);
    assert_eq!(r.cell_count(), 0);
    assert!(r.boxed_city(id.unwrap()).is_none());
    h.fail.remove("package");
    let id = r.insert_city(city(0, 0, 1, 1, "a", 1).counted());
    h.fail.insert("delete".into());
    r.delete_city(id, &mut h);
    assert_eq!(r.cell_count(), 0);
    assert!(h.files.contains_key("R\\A\\City - a.sc4"));
}

#[test]
fn move_city_collision_serial_rewrite_active_folder_and_source_shutdown() {
    let mut h = Host::default();
    let mut dest = region();
    let mut source = region();
    let mut g = RegionGlobals::default();
    source.set_directory_name("Source");
    source.initialized = true;
    dest.initialized = true;
    h.directories.insert("R\\Source\\".into());
    h.image(image(
        3,
        1,
        &[(0, 0, [255, 0, 0]), (1, 0, [255, 0, 0]), (2, 0, [255, 0, 0])],
    ));
    let mut moved = city(0, 0, 1, 1, "moved", 9);
    moved.path = "R\\Source\\original.sc4".into();
    h.add(&moved.path.clone(), moved.clone());
    let moved = moved.counted();
    source.insert_city(moved.clone());
    dest.insert_city(city(0, 0, 1, 1, "existing", 9).counted());
    dest.insert_city(city(1, 0, 1, 1, "second", 28_800 << 4).counted());
    let occupant = city(2, 0, 1, 1, "occupant", 77);
    h.add(&occupant.path.clone(), occupant.clone());
    dest.insert_city(occupant.counted());
    h.entries.push(DirectoryEntry {
        name: "City - moved.sc4".into(),
        regular_file: true,
    });
    let opaque = h.files["R\\Source\\original.sc4"].clone();
    assert!(dest.move_city(Some(&mut source), &moved, (2, 0), &mut h, &mut g));
    assert!(!source.initialized());
    assert!(dest.initialized());
    let state = h.packages["R\\A\\City - moved.sc4"].city.as_ref().unwrap();
    assert_eq!(
        (state.position, state.serial, state.rearrange),
        ((2, 0), (28_800 << 4) - 1, true)
    );
    assert_eq!(h.files["R\\A\\City - moved.sc4"], opaque); // no other record written
    assert!(!h.files.contains_key("R\\A\\City - occupant.sc4"));
    assert!(dest.get_city(2, 0).is_some());
    let city = dest.boxed_city(dest.get_city(2, 0).unwrap()).unwrap();
    let events = h.events.len();
    assert!(dest.move_city(None, &city, (2, 0), &mut h, &mut g));
    assert_eq!(h.events.len(), events);
}

#[test]
fn move_failures_always_reinit_and_same_region_reassigns_serial() {
    for failure in ["rename", "package", "record"] {
        let mut h = Host::default();
        let mut r = region();
        let mut g = RegionGlobals::default();
        r.initialized = true;
        let c = city(0, 0, 1, 1, "a", 7);
        h.add(&c.path.clone(), c.clone());
        let c = c.counted();
        r.insert_city(c.clone());
        h.fail.insert(failure.into());
        let result = r.move_city(None, &c, (2, 0), &mut h, &mut g);
        assert_eq!(result, failure == "record"); // write result intentionally ignored
        assert!(r.initialized());
        assert!(g.init_seen);
        if failure == "record" {
            assert!(h.files.contains_key("R\\A\\City - a (2).sc4"));
        }
    }
}

#[test]
fn tutorial_reset_polarity_reload_failure_and_reactivation() {
    for failure in ["none", "export", "package"] {
        let mut h = Host::default();
        let mut r = region();
        let mut c = city(0, 0, 1, 1, "tutorial", 0);
        c.tutorial_id = 42;
        h.add(&c.path.clone(), c.clone());
        r.insert_city(c.counted());
        h.resources.insert((TUTORIAL_TYPE, TUTORIAL_GROUP, 42), vec![1, 2, 3]);
        if failure != "none" {
            h.fail.insert(failure.into());
        }
        assert!(!r.reset_tutorial_city(-1, &mut h));
        assert!(!r.reset_tutorial_city(99, &mut h));
        assert_eq!(r.reset_tutorial_city(42, &mut h), failure == "export");
        assert_eq!(h.event_count("activate:42"), usize::from(failure != "export"));
        if failure == "none" {
            assert_eq!(
                r.boxed_city(r.get_city(0, 0).unwrap())
                    .unwrap()
                    .read()
                    .unwrap()
                    .tutorial_id,
                42
            );
        }
        if failure == "package" {
            assert_eq!(r.cell_count(), 0);
        }
    }
}

#[test]
fn terrain_height_table_low_water_bytes_and_thread_safe_getters() {
    let mut r = region();
    let mut g = RegionGlobals::default();
    assert_eq!(r.base_terrain_height(&g), Ok(0));
    g.set_height_tuning(17, 255);
    assert_eq!(r.base_terrain_height(&g), Ok(17));
    r.set_base_terrain_type(1);
    assert_eq!(r.base_terrain_height(&g), Ok(255));
    r.set_base_terrain_type(2);
    assert_eq!(r.base_terrain_height(&g), Err(Unsupported::BaseTerrainType(2)));
    r.set_base_terrain_type(-1);
    assert_eq!(r.base_terrain_type(), -1);
    r.config.water_min = 0x123;
    r.config.water_max = -1;
    assert_eq!(r.water_preferences(), (0x23, 255));
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Region>();
    send_sync::<CityRef>();
}

#[test]
fn conformance_distribution_vectors() {
    // Spec §12.1 distributions, represented by synthetic exact-cover pictures. These do not
    // claim to validate any shipped bitmap's particular arrangement or package contents.
    for (name, width, small, medium, large) in [
        ("Berlin", 16, 32, 52, 1),
        ("Fairview", 8, 44, 5, 0),
        ("London", 16, 16, 56, 1),
        ("New York", 16, 0, 44, 5),
        ("San Francisco", 15, 29, 37, 3),
        ("Timbuktu", 16, 36, 55, 0),
    ] {
        let mut pixels = vec![[0; 3]; width * width];
        let mut taken = vec![false; width * width];
        for (side, count, color) in [
            (4, large, [0, 0, 255]),
            (2, medium, [0, 255, 0]),
            (1, small, [255, 0, 0]),
        ] {
            for _ in 0..count {
                let (x, z) = (0..width)
                    .flat_map(|z| (0..width).map(move |x| (x, z)))
                    .find(|&(x, z)| {
                        x + side <= width
                            && z + side <= width
                            && (z..z + side).all(|row| (x..x + side).all(|col| !taken[row * width + col]))
                    })
                    .expect("distribution packs into specified image size");
                pixels[z * width + x] = color;
                for row in z..z + side {
                    for col in x..x + side {
                        taken[row * width + col] = true;
                    }
                }
            }
        }
        let picture = Image::new(width, width, pixels).unwrap();
        let decoded = decode_bmp(&encode_bmp(&picture).unwrap()).unwrap();
        let locations = decoded.locations();
        assert_eq!(locations.len(), small + medium + large, "{name}");
        for (exponent, count) in [(0, small), (1, medium), (2, large)] {
            assert_eq!(
                locations
                    .iter()
                    .filter(|location| location.size_exp == exponent)
                    .count(),
                count,
                "{name}"
            );
        }
        assert!(taken.iter().all(|cell| *cell));
        let mut r = region();
        let mut h = Host::default();
        r.locations = locations.clone();
        for (i, location) in locations.iter().enumerate() {
            let side = 1 << location.size_exp;
            h.entry(
                &format!("{i}.sc4"),
                city(location.x, location.z, side, side, name, i as u32),
            );
        }
        r.load_cities(&mut h);
        assert_eq!(r.cell_count(), width * width);
        assert!(r.overflow().is_empty());
    }
}

#[test]
fn ini_io_failures_do_not_abort_shutdown_or_recreate_deleted_parent() {
    for failure in ["write", "read"] {
        let mut r = region();
        let mut h = Host::default();
        r.initialized = true;
        h.files.insert("R\\A\\region.ini".into(), b"original".to_vec());
        h.fail.insert(failure.into());
        assert!(r.shutdown(&mut h));
        assert!(!r.initialized());
        assert_eq!(h.files["R\\A\\region.ini"], b"original");
        assert_eq!(h.event_count("write:"), if failure == "write" { 4 } else { 0 });
    }
}

#[test]
fn bitmap_32_bit_and_rejected_header_paths() {
    let original = image(1, 1, &[(0, 0, [255, 77, 16])]);
    let mut bytes = encode_bmp(&original).unwrap();
    bytes[28..30].copy_from_slice(&32u16.to_le_bytes());
    bytes[57] = 0; // alpha does not affect location classification
    assert_eq!(decode_bmp(&bytes).unwrap(), original);
    let mut huge_header = bytes.clone();
    huge_header[14..18].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_bmp(&huge_header).is_err()); // checked header addition on 32-bit targets
    for (offset, value) in [(26, 2u8), (28, 16), (30, 1), (18, 0), (22, 0), (10, 1)] {
        let mut bad = bytes.clone();
        bad[offset] = value;
        assert!(decode_bmp(&bad).is_err(), "offset {offset}");
    }
}

#[test]
fn orphaned_box_references_survive_region_destruction() {
    let orphan = city(0, 0, 1, 1, "orphan", 1).counted();
    let active = city(0, 0, 1, 1, "active", 2).counted();
    let empty = city(0, 0, 0, 1, "empty", 3).counted();
    let mut h = Host::default();
    {
        let mut r = region();
        r.initialized = true;
        r.insert_city(orphan.clone());
        r.insert_city(active.clone());
        r.insert_city(empty.clone());
        r.release(&mut h);
        assert_eq!(
            h.events
                .iter()
                .filter(|event| event.starts_with("shutdown:"))
                .collect::<Vec<_>>(),
            [&"shutdown:active".to_owned()]
        );
    }
    assert_eq!(Arc::strong_count(&orphan), 2);
    assert_eq!(Arc::strong_count(&empty), 2);
    assert_eq!(Arc::strong_count(&active), 1);
}

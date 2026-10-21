// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! mysims.md §4.12 (the Sim template pool): the worked examples.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use super::pool::*;

/// A typed pool host: in-memory files, registry, ini files.
#[derive(Default)]
struct Host {
    keys: RefCell<Option<Vec<TemplateKey>>>,
    exemplars: RefCell<HashMap<u32, TemplateExemplar>>,
    dir: Vec<u8>,
    files: RefCell<HashMap<Vec<u8>, Vec<u8>>>,
    strings: RefCell<HashMap<(u32, u32), Vec<u8>>>,
    registry: RefCell<HashMap<Vec<u8>, Vec<u8>>>,
    dirs: RefCell<Vec<Vec<u8>>>,
    converter: Cell<bool>,
    output: RefCell<Option<(Vec<u8>, Vec<u8>)>>,
    listings: RefCell<HashMap<Vec<u8>, Vec<Vec<u8>>>>,
    inis: RefCell<HashMap<Vec<u8>, IniFile>>,
    converted: RefCell<Vec<(Vec<u8>, Vec<u8>)>>,
    deleted: RefCell<Vec<Vec<u8>>>,
}

impl PoolHost for Host {
    fn resource_keys(&self) -> Option<Vec<TemplateKey>> {
        self.keys.borrow().clone()
    }
    fn template_exemplar(&self, key: TemplateKey) -> Option<TemplateExemplar> {
        self.exemplars.borrow().get(&key.instance).cloned()
    }
    fn mysim_directory(&self) -> Vec<u8> {
        self.dir.clone()
    }
    fn read_file(&self, path: &[u8]) -> Option<Vec<u8>> {
        self.files.borrow().get(path).cloned()
    }
    fn localized_string(&self, group: u32, instance: u32) -> Option<Vec<u8>> {
        self.strings.borrow().get(&(group, instance)).cloned()
    }
    fn registry_string(&self, key: &[u8], value: &[u8]) -> Option<Vec<u8>> {
        assert_eq!(key, SIMS_REGISTRY_KEY);
        self.registry.borrow().get(value).cloned()
    }
    fn directory_exists(&self, path: &[u8]) -> bool {
        self.dirs.borrow().iter().any(|d| d == path)
    }
    fn create_directory(&self, path: &[u8]) {
        self.dirs.borrow_mut().push(path.to_vec());
    }
    fn create_image_converter(&self) -> bool {
        self.converter.get()
    }
    fn release_image_converter(&self) {}
    fn create_output(&self, path: &[u8]) -> bool {
        *self.output.borrow_mut() = Some((path.to_vec(), Vec::new()));
        true
    }
    fn write_output(&self, bytes: &[u8]) {
        self.output.borrow_mut().as_mut().unwrap().1.extend_from_slice(bytes);
    }
    fn close_output(&self) {
        let (p, b) = self.output.borrow_mut().take().unwrap();
        self.files.borrow_mut().insert(p, b);
    }
    fn list_files(&self, dir: &[u8], pattern: &[u8]) -> Vec<Vec<u8>> {
        assert_eq!(pattern, b"*.pub");
        self.listings.borrow().get(dir).cloned().unwrap_or_default()
    }
    fn read_ini(&self, path: &[u8]) -> Option<IniFile> {
        self.inis.borrow().get(path).cloned()
    }
    fn convert_image(&self, src: &[u8], dst: &[u8]) -> bool {
        self.converted.borrow_mut().push((src.to_vec(), dst.to_vec()));
        true
    }
    fn delete_file(&self, path: &[u8]) {
        self.deleted.borrow_mut().push(path.to_vec());
    }
}

const DIR: &[u8] = b"C:\\U\\SimCity 4\\MySim\\";

fn host() -> Rc<Host> {
    Rc::new(Host {
        dir: DIR.to_vec(),
        ..Host::default()
    })
}

fn dat(bytes: &[u8]) -> (Vec<u8>, Vec<u8>) {
    ([DIR, b"MySims.dat"].concat(), bytes.to_vec())
}

/// Example "record": one template "Bob Newb", male, sign 5, bitmap 0,
/// portrait `Bob_1a2b3.bmp`.
#[test]
fn p1_record() {
    let bytes = [&[0xAA, 0xE4, 0x32, 0x4A, 8][..], b"Bob Newb", &[1, 5, 9], b"Bob_1a2b3"].concat();
    let t = parse_mysims_dat(&bytes);
    assert_eq!(
        t,
        vec![MySimTemplate {
            bitmap_id: 0,
            portrait: b"Bob_1a2b3.bmp".to_vec(),
            name: b"Bob Newb".to_vec(),
            is_male: true,
            sign: 5,
        }]
    );
    // A truncated second record is dropped; magic only / wrong magic.
    let mut more = bytes.clone();
    more.extend_from_slice(&[3, b'A', b'n']);
    assert_eq!(parse_mysims_dat(&more).len(), 1);
    assert!(parse_mysims_dat(&MYSIMS_DAT_MAGIC).is_empty());
    assert!(parse_mysims_dat(&[0, 0, 0, 0, 8]).is_empty());
    assert!(parse_mysims_dat(&[0xAA, 0xE4]).is_empty());
    // Portrait bytes kept (case), empty name gives `.bmp`; is-male ≠ 0.
    let t = parse_mysims_dat(&[&MYSIMS_DAT_MAGIC[..], &[0, 7, 11, 6], b"BOB_FF", &[0, 0, 0, 0]].concat());
    assert_eq!(t[0].portrait, b"BOB_FF.bmp");
    assert!(t[0].is_male);
    assert_eq!(t[1].portrait, b".bmp");
    assert!(!t[1].is_male);
    // encode_record round-trips.
    let r = encode_record(b"Bob Newb", true, 5, b"Bob_1a2b3");
    assert_eq!([&MYSIMS_DAT_MAGIC[..], &r].concat(), bytes);
}

/// Example "display path": bitmap 0 → MySim dir + portrait; bitmap ≠ 0 →
/// the resource.
#[test]
fn p2_display_path() {
    let t = MySimTemplate {
        portrait: b"Bob_1a2b3.bmp".to_vec(),
        ..MySimTemplate::default()
    };
    assert_eq!(
        portrait_source(&t, DIR),
        PortraitSource::File(b"C:\\U\\SimCity 4\\MySim\\Bob_1a2b3.bmp".to_vec())
    );
    let t = MySimTemplate { bitmap_id: 0x1234, ..t };
    assert_eq!(portrait_source(&t, DIR), PortraitSource::Resource(0x1234));
}

/// Example "gender": "male" text `Male`.
#[test]
fn p3_gender() {
    for v in [&b"male"[..], b"MALE", b"Male"] {
        assert!(is_male_text(v, b"Male"));
    }
    for v in [&b"Male "[..], b"m", b"female", b""] {
        assert!(!is_male_text(v, b"Male"));
    }
    // Only ASCII A–Z fold.
    assert!(!is_male_text(&[0xC4], &[0xE4]));
}

/// `GetMySimTemplates`: built-ins in key order (type and exemplar-type
/// filtered, localized name first), then imports; `out` replaced.
#[test]
fn p4_get_templates() {
    let h = host();
    let key = |t, i| TemplateKey {
        type_id: t,
        group: 1,
        instance: i,
    };
    *h.keys.borrow_mut() = Some(vec![
        key(TEMPLATE_KEY_TYPE, 2),
        key(0x1, 3),
        key(TEMPLATE_KEY_TYPE, 4),
        key(TEMPLATE_KEY_TYPE, 5),
    ]);
    let ex = |ty, loc: Option<&[u8]>| TemplateExemplar {
        exemplar_type: Some(ty),
        bitmap_id: Some(77),
        localized_name: loc.map(<[u8]>::to_vec),
        name: Some(b"Plain".to_vec()),
        is_male: Some(true),
        sign: Some(3),
    };
    h.exemplars.borrow_mut().insert(2, ex(0x26, Some(b"Loc")));
    h.exemplars.borrow_mut().insert(3, ex(0x26, None));
    h.exemplars.borrow_mut().insert(4, ex(0x10, None));
    h.exemplars.borrow_mut().insert(5, ex(0x26, None));
    let (p, b) = dat(&[&MYSIMS_DAT_MAGIC[..], &encode_record(b"Imp", false, 1, b"x")].concat());
    h.files.borrow_mut().insert(p, b);
    let pool = MySimPool::new(h.clone());
    let mut out = vec![MySimTemplate::default(); 5];
    pool.get_mysim_templates(&mut out);
    let names: Vec<&[u8]> = out.iter().map(|t| t.name.as_slice()).collect();
    assert_eq!(names, vec![&b"Loc"[..], b"Plain", b"Imp"]);
    assert_eq!((out[0].bitmap_id, out[0].is_male, out[0].sign), (77, true, 3));
    assert!(pool.is_loaded());
    // No manager: imports still load.
    let h2 = host();
    h2.files.borrow_mut().insert(
        dat(&MYSIMS_DAT_MAGIC).0,
        [&MYSIMS_DAT_MAGIC[..], &encode_record(b"I", true, 0, b"p")].concat(),
    );
    assert_eq!(MySimPool::new(h2).templates().len(), 1);
}

/// A Sims installation with one family of two members in `UserData2`.
fn sims_host() -> Rc<Host> {
    let h = host();
    h.registry
        .borrow_mut()
        .insert(b"InstallPath".to_vec(), b"D:\\Sims".to_vec());
    h.dirs.borrow_mut().push(b"D:\\Sims".to_vec());
    h.dirs.borrow_mut().push(b"D:\\Sims\\UserData2".to_vec());
    h.converter.set(true);
    h.strings.borrow_mut().insert(MALE_TEXT, b"Male".to_vec());
    h.strings
        .borrow_mut()
        .insert((ZODIAC_GROUP, ZODIAC_INSTANCES[4]), b"Leo".to_vec());
    h.listings
        .borrow_mut()
        .insert(b"D:\\Sims\\UserData2\\Export".to_vec(), vec![b"F.pub".to_vec()]);
    let e = |k: &str, v: &str| (b"FamilyInfo".to_vec(), k.as_bytes().to_vec(), v.as_bytes().to_vec());
    let ini = IniFile {
        entries: vec![
            (b"WebPages".to_vec(), b"FamilyDescription".to_vec(), b"Newb/".to_vec()),
            e("sims_family_nummembers", "2"),
            e("sims_family_name", "Newb"),
            e("sims_familymember1_name", "Bob"),
            e("sims_familymember1_gender", "MALE"),
            e("sims_familymember1_zodiac", "LEO"),
            e("sims_familymember1_guid", "107307"),
            e("sims_familymember2_name", "Betty"),
            e("sims_familymember2_gender", "Female"),
        ],
    };
    h.inis
        .borrow_mut()
        .insert(b"D:\\Sims\\UserData2\\Export\\F.ini".to_vec(), ini);
    h
}

/// `Import`: one record per member with a guid; paths by concatenation.
#[test]
fn p5_import() {
    let h = sims_host();
    let pool = MySimPool::new(h.clone());
    assert_eq!(pool.import(), 1);
    let file = h.files.borrow().get(&dat(&[]).0).cloned().unwrap();
    assert_eq!(
        file,
        [
            &MYSIMS_DAT_MAGIC[..],
            &encode_record(b"Bob Newb", true, 4, b"Bob_1a32b")
        ]
        .concat()
    );
    assert_eq!(
        h.converted.borrow()[0],
        (
            b"D:\\Sims\\Web Pages\\FamilyGFX\\Newb\\family1_face.jpg".to_vec(),
            b"C:\\U\\SimCity 4\\MySim\\Bob_1a32b.bmp".to_vec()
        )
    );
    assert_eq!(
        h.deleted.borrow().clone(),
        vec![b"D:\\Sims\\UserData2\\Export\\F.ini".to_vec()]
    );
    assert_eq!(pool.installation(), b"D:\\Sims");
    // No installation: count 0, nothing written.
    let h = host();
    assert_eq!(MySimPool::new(h.clone()).import(), 0);
    assert!(h.files.borrow().is_empty());
}

/// `FindTheSimsInstallation`: SIMS_DATA first, then InstallPath; a missing
/// directory clears the stored path.
#[test]
fn p6_find_installation() {
    let h = sims_host();
    h.registry
        .borrow_mut()
        .insert(b"SIMS_DATA".to_vec(), b"E:\\Gone".to_vec());
    let pool = MySimPool::new(h.clone());
    assert!(!pool.find_the_sims_installation());
    assert!(pool.installation().is_empty());
    h.registry.borrow_mut().remove(&b"SIMS_DATA".to_vec());
    assert!(pool.find_the_sims_installation());
    h.registry.borrow_mut().clear();
    assert!(!pool.find_the_sims_installation());
    assert_eq!(pool.installation(), b"D:\\Sims");
}

/// The cache after `Import` (three examples).
#[test]
fn p7_cache() {
    // count 2, loaded → cleared, next call re-reads.
    let h = sims_host();
    h.inis.borrow_mut().values_mut().for_each(|i| {
        i.entries.push((
            b"FamilyInfo".to_vec(),
            b"sims_familymember2_guid".to_vec(),
            b"255".to_vec(),
        ))
    });
    let pool = MySimPool::new(h.clone());
    assert!(pool.templates().is_empty());
    assert!(pool.is_loaded());
    assert_eq!(pool.import(), 2);
    assert!(!pool.is_loaded());
    assert!(pool.cached().is_empty());
    let t = pool.templates();
    assert_eq!(t.len(), 2);
    assert_eq!(t[1].portrait, b"Betty_ff.bmp");
    assert!(!t[1].is_male);
    // count 0, loaded → old imports kept although the file is magic only.
    h.listings.borrow_mut().clear();
    assert_eq!(pool.import(), 0);
    assert!(pool.is_loaded());
    assert_eq!(pool.templates().len(), 2);
    assert_eq!(
        h.files.borrow().get(&dat(&[]).0).cloned().unwrap(),
        MYSIMS_DAT_MAGIC.to_vec()
    );
    // not loaded → nothing cleared; the first call reads the new file.
    let h = sims_host();
    let pool = MySimPool::new(h.clone());
    assert_eq!(pool.import(), 1);
    assert!(!pool.is_loaded());
    assert_eq!(pool.templates().len(), 1);
}

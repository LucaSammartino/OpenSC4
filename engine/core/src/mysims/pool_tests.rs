// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! mysims.md §4.12 (the Sim template pool): the worked examples.

use std::rc::Rc;

use super::pool::*;

const DIR: &[u8] = b"C:\\U\\SimCity 4\\MySim\\";

fn host() -> Rc<MemoryPoolHost> {
    Rc::new(MemoryPoolHost {
        dir: DIR.to_vec(),
        ..MemoryPoolHost::default()
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
            indeterminate: 0,
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
fn sims_host() -> Rc<MemoryPoolHost> {
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
        .insert(b"D:\\Sims\\UserData2\\Export\\".to_vec(), vec![b"F.pub".to_vec()]);
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
            e("sims_familymember2_zodiac", ""),
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
            b"D:\\Sims\\UserData2\\Web Pages\\Newb\\FamilyGFX\\family1_face.jpg".to_vec(),
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

/// MS12 examples: long name (length mod 256), UserData path, ini next to
/// pub, picture folder.
#[test]
fn p8_ms12_paths_and_lengths() {
    let name = vec![b'a'; 300];
    let r = encode_record(&name, false, 0, b"p");
    assert_eq!(r[0], 44);
    assert_eq!(&r[1..45], &name[..44]);
    assert_eq!(&r[45..], &[0, 0, 1, b'p']);
    let back = parse_mysims_dat(&[&MYSIMS_DAT_MAGIC[..], &r].concat());
    assert_eq!(back[0].name.len(), 44);
    assert_eq!(encode_record(&[], true, 1, &[b'x'; 256])[3..], [0]);
    assert_eq!(userdata_dir(b"D:\\Sims\\", 1), b"D:\\Sims\\\\UserData");
    assert_eq!(userdata_dir(b"D:\\Sims\\", 3), b"D:\\Sims\\\\UserData3");
    // A trailing-`\` install path: the double separator is kept throughout.
    let h = sims_host();
    h.registry
        .borrow_mut()
        .insert(b"InstallPath".to_vec(), b"D:\\Sims\\".to_vec());
    h.dirs.borrow_mut().push(b"D:\\Sims\\".to_vec());
    h.dirs.borrow_mut().push(b"D:\\Sims\\\\UserData".to_vec());
    let ini = h.inis.borrow().values().next().cloned().unwrap();
    h.listings
        .borrow_mut()
        .insert(b"D:\\Sims\\\\UserData\\Export\\".to_vec(), vec![b"Smith.pub".to_vec()]);
    h.inis
        .borrow_mut()
        .insert(b"D:\\Sims\\\\UserData\\Export\\Smith.ini".to_vec(), ini);
    assert_eq!(MySimPool::new(h.clone()).import(), 1);
    assert_eq!(
        h.converted.borrow()[0].0,
        b"D:\\Sims\\\\UserData\\Web Pages\\Newb\\FamilyGFX\\family1_face.jpg".to_vec()
    );
    assert_eq!(
        h.deleted.borrow()[0],
        b"D:\\Sims\\\\UserData\\Export\\Smith.ini".to_vec()
    );
}

/// MS12 b: ini sections/keys case-insensitive, numbers `%ld`, missing name /
/// guid skip; unsigned member loop.
#[test]
fn p9_ms12_ini_reads() {
    let ini = IniFile {
        entries: vec![(b"FamilyInfo".to_vec(), b"Sims_Family_Name".to_vec(), b"N".to_vec())],
    };
    assert_eq!(ini.get(b"FAMILYINFO", b"sims_family_name"), Some(&b"N"[..]));
    let num = |v: &[u8]| {
        IniFile {
            entries: vec![(b"S".to_vec(), b"k".to_vec(), v.to_vec())],
        }
        .read_number(b"s", b"K")
    };
    assert_eq!(num(b"0x1F"), Some(0));
    assert_eq!(num(b" 42abc"), Some(42));
    assert_eq!(num(b"-7"), Some(-7));
    assert_eq!(num(b"abc"), None);
    // Guid `0x1F` → 0, ` 42abc` → 42, missing → member skipped; keys in
    // another case still match.
    let h = sims_host();
    h.inis.borrow_mut().values_mut().for_each(|i| {
        i.entries.retain(|(_, k, _)| !k.starts_with(b"sims_familymember1"));
        let e = |k: &str, v: &str| (b"familyinfo".to_vec(), k.as_bytes().to_vec(), v.as_bytes().to_vec());
        i.entries.extend([
            e("SIMS_FAMILY_NUMMEMBERS", "3"),
            e("sims_familymember1_name", "A"),
            e("sims_familymember1_guid", "0x1F"),
            e("sims_familymember1_gender", ""),
            e("sims_familymember1_zodiac", ""),
            e("sims_familymember2_guid", " 42abc"),
            e("sims_familymember3_guid", "9"),
        ]);
    });
    // nummembers: the first entry ("2") wins, so member 3 is not reached.
    let pool = MySimPool::new(h.clone());
    assert_eq!(pool.import(), 2);
    let t = pool.templates();
    assert_eq!(t[0].portrait, b"A_0.bmp");
    assert_eq!(t[1].portrait, b"Betty_2a.bmp");
    // A finite negative count loops unsigned: member 3 is reached.
    // -1 never terminates (MS14); test formatting separately instead.
    for (n, want) in [("-2", 3), ("0", 0)] {
        h.inis.borrow_mut().values_mut().for_each(|i| {
            i.entries.retain(|(_, k, _)| k != b"sims_family_nummembers");
            i.entries.retain(|(_, k, _)| k != b"SIMS_FAMILY_NUMMEMBERS");
            i.entries.push((
                b"FamilyInfo".to_vec(),
                b"sims_family_nummembers".to_vec(),
                n.as_bytes().to_vec(),
            ));
            i.entries.push((
                b"FamilyInfo".to_vec(),
                b"sims_familymember3_name".to_vec(),
                b"C".to_vec(),
            ));
            for f in ["gender", "zodiac"] {
                i.entries
                    .push((b"FamilyInfo".to_vec(), member_key(3, f).into_bytes(), Vec::new()));
            }
        });
        assert_eq!(MySimPool::new(h.clone()).import(), want, "n = {n}");
    }
}

/// MS12 a: absent bitmap / is-male / sign are flagged indeterminate; an
/// absent name is empty.
#[test]
fn p10_ms12_absent_properties() {
    let h = host();
    *h.keys.borrow_mut() = Some(vec![TemplateKey {
        type_id: TEMPLATE_KEY_TYPE,
        group: 0,
        instance: 1,
    }]);
    h.exemplars.borrow_mut().insert(
        1,
        TemplateExemplar {
            exemplar_type: Some(EXEMPLAR_TYPE_SIM_TEMPLATE),
            sign: Some(7),
            ..TemplateExemplar::default()
        },
    );
    let t = MySimPool::new(h).templates();
    assert_eq!(t[0].name, b"");
    assert_eq!((t[0].bitmap_id, t[0].is_male, t[0].sign), (0, false, 7));
    assert_eq!(t[0].indeterminate, INDETERMINATE_BITMAP | INDETERMINATE_MALE);
}

#[test]
fn p11_ms14_required_reads_and_short_names() {
    for missing in [
        "sims_family_nummembers",
        "sims_familymember1_name",
        "sims_familymember1_gender",
        "sims_familymember1_zodiac",
        "sims_familymember1_guid",
    ] {
        let h = sims_host();
        h.inis
            .borrow_mut()
            .values_mut()
            .for_each(|i| i.entries.retain(|(_, k, _)| k != missing.as_bytes()));
        assert_eq!(MySimPool::new(h.clone()).import(), 0, "{missing}");
        assert!(h.converted.borrow().is_empty());
    }
    let h = sims_host();
    h.inis.borrow_mut().values_mut().for_each(|i| {
        for (_, k, v) in &mut i.entries {
            if k == b"sims_family_nummembers" {
                *v = b"bad".to_vec();
            }
        }
    });
    assert_eq!(MySimPool::new(h).import(), 0);
    for name in [b"abc".as_slice(), b""] {
        let h = sims_host();
        let ini = h
            .inis
            .borrow_mut()
            .remove(b"D:\\Sims\\UserData2\\Export\\F.ini".as_slice())
            .unwrap();
        let path = [b"D:\\Sims\\UserData2\\Export\\".as_slice(), name, b".ini"].concat();
        h.inis.borrow_mut().insert(path.clone(), ini);
        *h.listings.borrow_mut().values_mut().next().unwrap() = vec![name.to_vec()];
        assert_eq!(MySimPool::new(h.clone()).import(), 1);
        assert_eq!(*h.deleted.borrow(), vec![path]);
    }
    assert_eq!(member_key(0x8000_0000, "name"), "sims_familymember-2147483648_name");
    assert_eq!(member_face(0x8000_0000), "family-2147483648_face.jpg");
    let h = sims_host();
    h.strings.borrow_mut().clear();
    h.inis.borrow_mut().values_mut().for_each(|i| {
        for (_, k, v) in &mut i.entries {
            if k == b"sims_familymember1_gender" || k == b"sims_familymember1_zodiac" {
                v.clear();
            }
        }
    });
    let p = MySimPool::new(h);
    assert_eq!(p.import(), 1);
    let t = p.templates();
    assert!(t[0].is_male);
    assert_eq!(t[0].sign, 0);
}

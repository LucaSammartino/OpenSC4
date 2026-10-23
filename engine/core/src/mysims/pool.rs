// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4MySimPool` (vtable `0x00A85B14`), the Sim-template pool of the
//! pick-sim UI (mysims.md §4.12). It never touches the simulation.
//!
//! - `GetMySimTemplates` (slot 6): on first use the built-ins (resource keys
//!   of type `0x6A297266`, exemplar type `0x26`) then the imports of
//!   `<MySim dir>MySims.dat`; `out` is **replaced** by the cached vector.
//! - `Import` (slot 7): rewrites `MySims.dat` from The Sims' families; the
//!   cache is reset only when `count > 0` and it was loaded *[quirk]*.
//! - `FindTheSimsInstallation` (slot 8): the registry look-up.
//!
//! Everything outside the class (registry, files, ini files, the resource
//! manager, the image converter, localized strings) is typed host input
//! ([`PoolHost`]); the module makes no OS call. Paths are byte strings
//! joined by plain concatenation (§4.12 "Path bytes").

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use crate::rz_files::ini::{ini_lower, scan_long};

/// Resource-key type of the built-in templates (`EnumKeysCallback`).
pub const TEMPLATE_KEY_TYPE: u32 = 0x6A29_7266;
/// The interface queried on the resource manager.
pub const RESOURCE_ENUM_IID: u32 = 0x6534_284A;
/// Exemplar type property and the template's value.
pub const PROP_EXEMPLAR_TYPE: u32 = 0x10;
pub const EXEMPLAR_TYPE_SIM_TEMPLATE: u32 = 0x26;
pub const PROP_BITMAP_ID: u32 = 0xEA29_6F8D;
pub const PROP_NAME_STRING: u32 = 0xCA41_6B3F;
pub const PROP_NAME: u32 = 0xEA29_6F8F;
pub const PROP_IS_MALE: u32 = 0xEA29_6F91;
pub const PROP_SIGN: u32 = 0xEA29_6F90;
/// `MySims.dat`, the 10 bytes joined to the MySim directory.
pub const MYSIMS_DAT: &[u8] = b"MySims.dat";
/// The file magic, little-endian `0x4A32E4AA`.
pub const MYSIMS_DAT_MAGIC: [u8; 4] = [0xAA, 0xE4, 0x32, 0x4A];
/// Appended to an imported record's portrait bytes.
pub const PORTRAIT_SUFFIX: &[u8] = b".bmp";
/// The "male" text (localized string group, instance).
pub const MALE_TEXT: (u32, u32) = (0xEA23_1E96, 0x6A76_30C6);
/// The 12 zodiac names (group, instances; table `0x00A85AA4`).
pub const ZODIAC_GROUP: u32 = 0x6A23_1EAA;
pub const ZODIAC_INSTANCES: [u32; 12] = [
    0xEA82_2E90,
    0xCA82_2E93,
    0xCA82_2E97,
    0xAA82_2E9B,
    0xCA82_2EA4,
    0xAA82_2EA8,
    0xCA82_2EAC,
    0xCA82_2EAF,
    0xCA82_2EB3,
    0x6A82_2EB6,
    0x6A82_2EB9,
    0xEA82_2EBD,
];
/// The registry values tried in order under
/// `HKEY_LOCAL_MACHINE\SOFTWARE\Maxis\The Sims`.
pub const SIMS_REGISTRY_KEY: &[u8] = b"SOFTWARE\\Maxis\\The Sims";
pub const SIMS_REGISTRY_VALUES: [&[u8]; 2] = [b"SIMS_DATA", b"InstallPath"];
/// `UserData` directories tried by `Import`: 1…99.
pub const USERDATA_COUNT: u32 = 99;

/// `cISC4MySimPool::tMySimTemplate` (0x30 bytes).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MySimTemplate {
    /// `+0x00`: the portrait bitmap resource ID; 0 for imports.
    pub bitmap_id: u32,
    /// `+0x04`: the portrait file name (imports: record bytes + `.bmp`, no
    /// directory).
    pub portrait: Vec<u8>,
    /// `+0x18`.
    pub name: Vec<u8>,
    /// `+0x2C`.
    pub is_male: bool,
    /// `+0x2D`, 0–11.
    pub sign: u8,
    /// Fields the original leaves **uninitialised** because the exemplar
    /// property is absent (§4.12, MS12 a, *[compat-undefined]*): bits
    /// [`INDETERMINATE_BITMAP`], [`INDETERMINATE_MALE`],
    /// [`INDETERMINATE_SIGN`]. The field itself then holds the
    /// reimplementation's 0 / false / 0; 0 = every field defined.
    pub indeterminate: u8,
}

/// [`MySimTemplate::indeterminate`] bits.
pub const INDETERMINATE_BITMAP: u8 = 1;
pub const INDETERMINATE_MALE: u8 = 2;
pub const INDETERMINATE_SIGN: u8 = 4;

/// A resource key (type, group, instance).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TemplateKey {
    pub type_id: u32,
    pub group: u32,
    pub instance: u32,
}

/// The properties `EnumKeysCallback` reads from a key's property holder
/// (`0xA52160F5`). `None` = the property is absent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TemplateExemplar {
    /// Property `0x10`.
    pub exemplar_type: Option<u32>,
    /// Property `0xEA296F8D`.
    pub bitmap_id: Option<u32>,
    /// The localized string named by property `0xCA416B3F`
    /// (`SC4LoadExemplarString` with the key's group/instance); `None` when
    /// the property is absent or the string does not load.
    pub localized_name: Option<Vec<u8>>,
    /// Property `0xEA296F8F`.
    pub name: Option<Vec<u8>>,
    /// Property `0xEA296F91`.
    pub is_male: Option<bool>,
    /// Property `0xEA296F90`.
    pub sign: Option<u8>,
}

/// A parsed ini file: `(section, key, value)` byte strings as the host's
/// ini reader returns them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IniFile {
    pub entries: Vec<(Vec<u8>, Vec<u8>, Vec<u8>)>,
}

impl IniFile {
    /// `ReadEntry` (framework-rz-files.md §3.5): the value of
    /// `[section] key`, section and key matched case-insensitively (MS12 b,
    /// [`ini_lower`]); the first matching entry wins.
    pub fn get(&self, section: &[u8], key: &[u8]) -> Option<&[u8]> {
        let (section, key) = (ini_lower(section), ini_lower(key));
        self.entries
            .iter()
            .find(|(s, k, _)| ini_lower(s) == section && ini_lower(k) == key)
            .map(|(_, _, v)| v.as_slice())
    }

    /// `ReadNumber` (§3.5): `sscanf("%ld")` of the entry; `None` = no entry
    /// or no number (the caller's variable is then left as it was).
    pub fn read_number(&self, section: &[u8], key: &[u8]) -> Option<i32> {
        self.get(section, key).and_then(scan_long)
    }
}

/// What the pool reads from and writes to the app. Callbacks must not call
/// back into the pool.
pub trait PoolHost {
    /// The resource manager's keys through interface `0x6534284A`
    /// (`EnumKeys`), in enumeration order; `None` = no manager or no
    /// interface.
    fn resource_keys(&self) -> Option<Vec<TemplateKey>>;
    /// A key's property holder; `None` = it does not load.
    fn template_exemplar(&self, key: TemplateKey) -> Option<TemplateExemplar>;
    /// The application's `GetMySimDirectory()` (ends in `\`).
    fn mysim_directory(&self) -> Vec<u8>;
    /// Reads a whole file (open read, open existing, share read); `None` =
    /// the open fails.
    fn read_file(&self, path: &[u8]) -> Option<Vec<u8>>;
    /// A localized string; `None` = it does not load.
    fn localized_string(&self, group: u32, instance: u32) -> Option<Vec<u8>>;
    /// `HKEY_LOCAL_MACHINE\<key>\<value>` as a string, encoding converted.
    fn registry_string(&self, key: &[u8], value: &[u8]) -> Option<Vec<u8>>;
    fn directory_exists(&self, path: &[u8]) -> bool;
    fn create_directory(&self, path: &[u8]);
    /// Creates the image converter (class `0xA2FFB5D3`, interface
    /// `0x02FFBB60`); false = none. Released at the end of `Import`.
    fn create_image_converter(&self) -> bool;
    fn release_image_converter(&self);
    /// Opens the output for write, create-always, share read; false = fails.
    fn create_output(&self, path: &[u8]) -> bool;
    fn write_output(&self, bytes: &[u8]);
    fn close_output(&self);
    /// The file names matching `<dir><pattern>` (`dir` ends in `\`), in
    /// listing order.
    fn list_files(&self, dir: &[u8], pattern: &[u8]) -> Vec<Vec<u8>>;
    /// Opens (read, open existing) and parses an ini file; `None` = missing
    /// or not an ini file.
    fn read_ini(&self, path: &[u8]) -> Option<IniFile>;
    /// Converts an image file to a bmp (`0x0043ABE0`); false = failure.
    fn convert_image(&self, src: &[u8], dst: &[u8]) -> bool;
    fn delete_file(&self, path: &[u8]);
}

/// `cRZString::MakeLower`: ASCII `A`–`Z` folded, every other byte kept.
pub fn make_lower(s: &[u8]) -> Vec<u8> {
    s.to_ascii_lowercase()
}

/// The gender test (`IsEqual(text, caseSensitive = 0)`): same length and
/// equal after [`make_lower`] of both; no trimming.
pub fn is_male_text(value: &[u8], male_text: &[u8]) -> bool {
    value.len() == male_text.len() && make_lower(value) == make_lower(male_text)
}

/// The `MySims.dat` records (`LoadImports`): nothing on a short file or a
/// wrong magic; records until any read fails (a truncated record is
/// dropped). Imported templates have bitmap ID 0.
pub fn parse_mysims_dat(bytes: &[u8]) -> Vec<MySimTemplate> {
    let mut out = Vec::new();
    if bytes.len() < 4 || bytes[..4] != MYSIMS_DAT_MAGIC {
        return out;
    }
    let mut pos = 4;
    let mut take = |n: usize| -> Option<&[u8]> {
        let s = bytes.get(pos..pos + n)?;
        pos += n;
        Some(s)
    };
    while let Some(n) = take(1).map(|b| b[0] as usize) {
        let Some(name) = take(n).map(<[u8]>::to_vec) else { break };
        let Some(male) = take(1).map(|b| b[0] != 0) else { break };
        let Some(sign) = take(1).map(|b| b[0]) else { break };
        let Some(m) = take(1).map(|b| b[0] as usize) else { break };
        let Some(portrait) = take(m) else { break };
        let mut portrait = portrait.to_vec();
        portrait.extend_from_slice(PORTRAIT_SUFFIX);
        out.push(MySimTemplate {
            bitmap_id: 0,
            portrait,
            name,
            is_male: male,
            sign,
            indeterminate: 0,
        });
    }
    out
}

/// One `MySims.dat` record (the `Import` writer). Each string is written as
/// a u8 length = **byte length mod 256** followed by that many bytes (MS12 e):
/// a 300-byte name is stored as 44 and its first 44 bytes.
pub fn encode_record(name: &[u8], is_male: bool, sign: u8, portrait: &[u8]) -> Vec<u8> {
    let name = &name[..name.len() % 256];
    let portrait = &portrait[..portrait.len() % 256];
    let mut r = Vec::with_capacity(name.len() + portrait.len() + 4);
    r.push(name.len() as u8);
    r.extend_from_slice(name);
    r.push(u8::from(is_male));
    r.push(sign);
    r.push(portrait.len() as u8);
    r.extend_from_slice(portrait);
    r
}

/// Where the pick-sim dialog takes a template's portrait from (`PickHead`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortraitSource {
    /// Bitmap ID ≠ 0: the resource keyed by that ID.
    Resource(u32),
    /// Bitmap ID 0: `GetMySimDirectory()` followed by the portrait file
    /// name, no separator added.
    File(Vec<u8>),
}

/// The display path (§4.12): the MySim directory at display time.
pub fn portrait_source(t: &MySimTemplate, mysim_directory: &[u8]) -> PortraitSource {
    if t.bitmap_id != 0 {
        PortraitSource::Resource(t.bitmap_id)
    } else {
        PortraitSource::File([mysim_directory, t.portrait.as_slice()].concat())
    }
}

/// The pool.
pub struct MySimPool<H: PoolHost + ?Sized> {
    host: std::rc::Rc<H>,
    /// `+0x08`.
    loaded: Cell<bool>,
    /// `+0x0C`.
    cache: RefCell<Vec<MySimTemplate>>,
    /// `+0x18`: The Sims' installation path.
    installation: RefCell<Vec<u8>>,
}

impl<H: PoolHost + ?Sized> MySimPool<H> {
    pub fn new(host: std::rc::Rc<H>) -> MySimPool<H> {
        MySimPool {
            host,
            loaded: Cell::new(false),
            cache: RefCell::new(Vec::new()),
            installation: RefCell::new(Vec::new()),
        }
    }

    /// The loaded flag (`+0x08`).
    pub fn is_loaded(&self) -> bool {
        self.loaded.get()
    }

    /// The stored installation path (`+0x18`).
    pub fn installation(&self) -> Vec<u8> {
        self.installation.borrow().clone()
    }

    /// The cached vector (`+0x0C`) as it stands.
    pub fn cached(&self) -> Vec<MySimTemplate> {
        self.cache.borrow().clone()
    }

    /// `GetMySimTemplates(out)` (slot 6): `out` is replaced by the cache.
    pub fn get_mysim_templates(&self, out: &mut Vec<MySimTemplate>) {
        if !self.loaded.get() {
            let mut list = Vec::new();
            if let Some(keys) = self.host.resource_keys() {
                for key in keys {
                    if let Some(t) = self.enum_keys_callback(key) {
                        list.push(t);
                    }
                }
            }
            list.extend(self.load_imports());
            *self.cache.borrow_mut() = list;
            self.loaded.set(true);
        }
        out.clone_from(&self.cache.borrow());
    }

    /// `GetMySimTemplates` into a fresh vector.
    pub fn templates(&self) -> Vec<MySimTemplate> {
        let mut out = Vec::new();
        self.get_mysim_templates(&mut out);
        out
    }

    /// `EnumKeysCallback(key)`. An absent name gives the empty string; an
    /// absent bitmap ID / is-male / sign is indeterminate in the original
    /// (MS12 a): 0 / false / 0 here, flagged in `indeterminate`.
    fn enum_keys_callback(&self, key: TemplateKey) -> Option<MySimTemplate> {
        if key.type_id != TEMPLATE_KEY_TYPE {
            return None;
        }
        let ex = self.host.template_exemplar(key)?;
        if ex.exemplar_type != Some(EXEMPLAR_TYPE_SIM_TEMPLATE) {
            return None;
        }
        Some(MySimTemplate {
            bitmap_id: ex.bitmap_id.unwrap_or(0),
            portrait: Vec::new(),
            name: ex.localized_name.or(ex.name).unwrap_or_default(),
            is_male: ex.is_male.unwrap_or(false),
            sign: ex.sign.unwrap_or(0),
            indeterminate: (u8::from(ex.bitmap_id.is_none()) * INDETERMINATE_BITMAP)
                | (u8::from(ex.is_male.is_none()) * INDETERMINATE_MALE)
                | (u8::from(ex.sign.is_none()) * INDETERMINATE_SIGN),
        })
    }

    /// The data file path: `GetMySimDirectory()` + `MySims.dat`.
    pub fn data_file_path(&self) -> Vec<u8> {
        [self.host.mysim_directory().as_slice(), MYSIMS_DAT].concat()
    }

    /// `LoadImports`.
    fn load_imports(&self) -> Vec<MySimTemplate> {
        self.host
            .read_file(&self.data_file_path())
            .map(|b| parse_mysims_dat(&b))
            .unwrap_or_default()
    }

    /// `FindTheSimsInstallation` (slot 8).
    pub fn find_the_sims_installation(&self) -> bool {
        let value = SIMS_REGISTRY_VALUES
            .iter()
            .find_map(|v| self.host.registry_string(SIMS_REGISTRY_KEY, v));
        let Some(path) = value else {
            return false;
        };
        if !self.host.directory_exists(&path) {
            self.installation.borrow_mut().clear();
            return false;
        }
        *self.installation.borrow_mut() = path;
        true
    }

    /// `Import(count)` (slot 7): the number of records written.
    pub fn import(&self) -> u32 {
        let mut count = 0u32;
        let male = self.host.localized_string(MALE_TEXT.0, MALE_TEXT.1).unwrap_or_default();
        if !self.find_the_sims_installation() {
            return count;
        }
        let mysim_dir = self.host.mysim_directory();
        if !self.host.directory_exists(&mysim_dir) {
            self.host.create_directory(&mysim_dir);
        }
        if !self.host.create_image_converter() {
            return count;
        }
        if !self.host.create_output(&self.data_file_path()) {
            self.host.release_image_converter();
            return count;
        }
        self.host.write_output(&MYSIMS_DAT_MAGIC);
        let zodiac: Vec<Vec<u8>> = ZODIAC_INSTANCES
            .iter()
            .map(|&i| make_lower(&self.host.localized_string(ZODIAC_GROUP, i).unwrap_or_default()))
            .collect();
        let install = self.installation();
        for i in 1..=USERDATA_COUNT {
            let ud = userdata_dir(&install, i);
            if !self.host.directory_exists(&ud) {
                continue;
            }
            let export = [ud.as_slice(), b"\\Export\\"].concat();
            for file in self.host.list_files(&export, b"*.pub") {
                count += self.import_family(&ud, &export, &file, &male, &zodiac, &mysim_dir);
            }
        }
        self.host.close_output();
        self.host.release_image_converter();
        if count > 0 && self.loaded.get() {
            self.cache.borrow_mut().clear();
            self.loaded.set(false);
        }
        count
    }

    /// Steps 2–6 for one `F.pub` of `<ud>\Export\`: the records written.
    fn import_family(
        &self,
        ud: &[u8],
        export: &[u8],
        pub_name: &[u8],
        male: &[u8],
        zodiac: &[Vec<u8>],
        mysim_dir: &[u8],
    ) -> u32 {
        // `F.ini` = the Export directory + the entry name minus its last 4
        // bytes (`Left(len − 4)`) + `.ini` (MS12 d). A name shorter than 4
        // bytes keeps the whole name: unsigned Left(len - 4), MS14 c.
        let stem = &pub_name[..pub_name.len().checked_sub(4).unwrap_or(pub_name.len())];
        let ini_path = [export, stem, b".ini"].concat();
        let Some(ini) = self.host.read_ini(&ini_path) else {
            return 0;
        };
        let Some(desc) = ini.get(b"WebPages", b"FamilyDescription") else {
            return 0;
        };
        let desc = desc.strip_suffix(b"/").unwrap_or(desc);
        // `<ud>\Web Pages\<value>\FamilyGFX\` (MS12 c).
        let pictures = [ud, b"\\Web Pages\\", desc, b"\\FamilyGFX\\"].concat();
        // A missing family name: `ReadEntry` writes nothing, the string stays
        // empty (literal reading).
        let family = ini.get(b"FamilyInfo", b"sims_family_name").unwrap_or_default().to_vec();
        // MS14 a: n is initialized to zero for each family.
        let n = ini.read_number(b"FamilyInfo", b"sims_family_nummembers").unwrap_or(0) as u32;
        let mut written = 0u32;
        // MS14: UINT_MAX never terminates; after MAX it wraps through zero.
        // Finite loops can skip missing names because IniFile is immutable.
        // Never exercise the nonterminating bound in a conformance import.
        let members: Box<dyn Iterator<Item = u32>> = if n == u32::MAX {
            Box::new((0..=u32::MAX).cycle().skip(1))
        } else {
            Box::new(member_indices(&ini, n).into_iter())
        };
        for k in members {
            let key = |f: &str| member_key(k, f);
            let field = |f: &str| ini.get(b"FamilyInfo", key(f).as_bytes());
            let Some(first) = field("name") else { continue };
            let Some(gender) = field("gender") else { continue };
            let Some(zodiac_value) = field("zodiac") else { continue };
            let zod = make_lower(zodiac_value);
            let Some(guid) = ini.read_number(b"FamilyInfo", key("guid").as_bytes()) else {
                continue;
            };
            let portrait = [first, b"_", format!("{:x}", guid as u32).as_bytes()].concat();
            let src = [pictures.as_slice(), member_face(k).as_bytes()].concat();
            let dst = [mysim_dir, portrait.as_slice(), PORTRAIT_SUFFIX].concat();
            if !self.host.convert_image(&src, &dst) {
                continue;
            }
            let sign = zodiac.iter().position(|z| *z == zod).unwrap_or(0) as u8;
            let name = [first, b" ", family.as_slice()].concat();
            self.host
                .write_output(&encode_record(&name, is_male_text(gender, male), sign, &portrait));
            written = written.wrapping_add(1);
        }
        self.host.delete_file(&ini_path);
        written
    }
}

/// `Sprintf("%s%sUserData", path, "\")` for `i` = 1, `...UserData%d` else:
/// the `\` is always inserted (MS12 f).
pub fn userdata_dir(path: &[u8], i: u32) -> Vec<u8> {
    let mut dir = [path, b"\\UserData"].concat();
    if i > 1 {
        dir.extend_from_slice(i.to_string().as_bytes());
    }
    dir
}

/// MS14: the unsigned loop index is formatted as signed int32 (`%d`).
pub fn member_key(k: u32, field: &str) -> String {
    format!("sims_familymember{}_{field}", k as i32)
}

/// Source portrait filename, including its extension (§4.12).
pub fn member_face(k: u32) -> String {
    format!("family{}_face.jpg", k as i32)
}

/// The members `k` the unsigned loop `k = 1 … n` (§4.12 step 4) reaches with
/// a `sims_familymember<k>_name` entry, ascending. Every other `k` fails the
/// `_name` read and is skipped without effect, so only these are visited (a
/// negative `n` loops over 2^32 − |n| keys in the original). `k` is printed
/// as `%d` (`k ≥ 2^31` negative; MS14 b).
fn member_indices(ini: &IniFile, n: u32) -> Vec<u32> {
    const PREFIX: &[u8] = b"sims_familymember";
    let mut ks: Vec<u32> = ini
        .entries
        .iter()
        .filter(|(s, _, _)| ini_lower(s) == b"familyinfo")
        .filter_map(|(_, k, _)| {
            let k = ini_lower(k);
            let mid = k.strip_prefix(PREFIX)?.strip_suffix(b"_name")?;
            let v: i32 = std::str::from_utf8(mid).ok()?.parse().ok()?;
            // Only the exact text `%d` prints matches the read key.
            (v.to_string().as_bytes() == mid).then_some(v as u32)
        })
        .filter(|&k| k >= 1 && k <= n)
        .collect();
    ks.sort_unstable();
    ks.dedup();
    ks
}

/// An in-memory [`PoolHost`] (tests, conformance): files, registry, ini
/// files, directory listings; records conversions and deletions.
#[derive(Default)]
pub struct MemoryPoolHost {
    pub keys: RefCell<Option<Vec<TemplateKey>>>,
    /// By key instance.
    pub exemplars: RefCell<HashMap<u32, TemplateExemplar>>,
    pub dir: Vec<u8>,
    pub files: RefCell<HashMap<Vec<u8>, Vec<u8>>>,
    pub strings: RefCell<HashMap<(u32, u32), Vec<u8>>>,
    pub registry: RefCell<HashMap<Vec<u8>, Vec<u8>>>,
    pub dirs: RefCell<Vec<Vec<u8>>>,
    pub converter: Cell<bool>,
    pub output: RefCell<Option<(Vec<u8>, Vec<u8>)>>,
    pub listings: RefCell<HashMap<Vec<u8>, Vec<Vec<u8>>>>,
    pub inis: RefCell<HashMap<Vec<u8>, IniFile>>,
    pub converted: RefCell<Vec<(Vec<u8>, Vec<u8>)>>,
    pub deleted: RefCell<Vec<Vec<u8>>>,
}

impl PoolHost for MemoryPoolHost {
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
        if key != SIMS_REGISTRY_KEY {
            return None;
        }
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
        if let Some((_, b)) = self.output.borrow_mut().as_mut() {
            b.extend_from_slice(bytes);
        }
    }
    fn close_output(&self) {
        let Some((p, b)) = self.output.borrow_mut().take() else {
            return;
        };
        self.files.borrow_mut().insert(p, b);
    }
    fn list_files(&self, dir: &[u8], pattern: &[u8]) -> Vec<Vec<u8>> {
        if pattern != b"*.pub" {
            return Vec::new();
        }
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

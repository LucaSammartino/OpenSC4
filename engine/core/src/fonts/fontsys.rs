// SPDX-License-Identifier: GPL-3.0-or-later
//! `cGZFontSys`, the font catalogue (drawing-fonts.md §12, §12.1).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::font::{FontObj, FontRef, GzFont, PEN_DEFAULT_COLOUR, USER_STYLES};
use super::{lower, CodePageConverter, FontLibrary, FontRenderer};

/// Class ID of `cGZFontSys` (§1).
pub const CLSID_GZ_FONT_SYS: u32 = 0x2067_3960;
/// Its primary interface IDs (slot 0).
pub const IID_GZ_FONT_SYS: u32 = 0x0067_3960;
/// The second interface, asked for at director OnStart.
pub const IID_GZ_FONT_SYS2: u32 = 0x287F_B697;

/// File patterns set by construction (§12).
pub const FILE_PATTERNS: [&str; 4] = ["*.tt?", "*.pfb", "*.mxf", "*.otf"];

/// `GetFontSourceTypeFromFileName` (`0x0089433C`).
pub fn source_type_from_file_name(lib: &dyn FontLibrary, path: &str) -> u32 {
    let lp = lower(path);
    if lp.ends_with(".ttf") {
        return 1;
    }
    if lp.ends_with(".ttc") || lp.ends_with(".otf") {
        return 4;
    }
    if lp.ends_with(".pfb") {
        return 2;
    }
    let head = lib.read_head(path, 32);
    let head = &head[..head.len().min(32)];
    if head.windows(4).any(|w| w == b"%!PS") {
        2
    } else if head.starts_with(&[0, 1, 0, 0]) {
        1
    } else if head.starts_with(b"ttcf") {
        4
    } else {
        0
    }
}

/// Catalogue record (§12).
#[derive(Clone, Default)]
pub struct FontFixedInfo {
    /// +0: cache list of {font, recorded style}; each holds one reference.
    pub cache: Vec<(FontRef, u32)>,
    pub family: String,
    pub style: u32,
    pub kind: u32,
    pub source_type: u32,
}

/// GetFontInfoList record (0x88 bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontInfoRecord {
    pub family: [u8; 0x80],
    pub style: u32,
    pub kind: u32,
}

impl Default for FontInfoRecord {
    fn default() -> Self {
        FontInfoRecord {
            family: [0; 0x80],
            style: 0,
            kind: 0,
        }
    }
}

impl FontInfoRecord {
    pub fn to_bytes(&self) -> [u8; 0x88] {
        let mut b = [0u8; 0x88];
        b[..0x80].copy_from_slice(&self.family);
        b[0x80..0x84].copy_from_slice(&self.style.to_le_bytes());
        b[0x84..0x88].copy_from_slice(&self.kind.to_le_bytes());
        b
    }
}

pub struct GzFontSys {
    lib: Arc<dyn FontLibrary>,
    conv: Arc<dyn CodePageConverter>,
    pub initialised: bool,
    pub ref_count: u32,
    pub default_aa_mode: u32,
    pub mono_min: u32,
    pub mono_max: u32,
    pub code_page: u32,
    renderer: Option<Arc<dyn FontRenderer>>,
    pub families: BTreeSet<String>,
    pub font_names: BTreeMap<u32, String>,
    pub catalogue: BTreeMap<String, FontFixedInfo>,
    pub substitutions: BTreeMap<String, Vec<String>>,
    pub family_ranges: BTreeMap<String, (u32, u32)>,
    pub file_patterns: Vec<String>,
}

fn ieq(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

impl GzFontSys {
    /// Construction (`0x00895268`): `code_page` is the framework code page.
    pub fn new(lib: Arc<dyn FontLibrary>, conv: Arc<dyn CodePageConverter>, code_page: u32) -> GzFontSys {
        GzFontSys {
            lib,
            conv,
            initialised: false,
            ref_count: 0,
            default_aa_mode: 0,
            mono_min: 0,
            mono_max: 0,
            code_page,
            renderer: None,
            families: BTreeSet::new(),
            font_names: BTreeMap::new(),
            catalogue: BTreeMap::new(),
            substitutions: BTreeMap::new(),
            family_ranges: BTreeMap::new(),
            file_patterns: FILE_PATTERNS.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Slot 0: `Some(false)` primary interface, `Some(true)` the second (this + 4), AddRef'd.
    pub fn query_interface(&mut self, id: u32) -> Option<bool> {
        let second = match id {
            1 | IID_GZ_FONT_SYS => false,
            IID_GZ_FONT_SYS2 => true,
            _ => return None,
        };
        self.ref_count += 1;
        Some(second)
    }

    pub fn add_ref(&mut self) -> u32 {
        self.ref_count += 1;
        self.ref_count
    }

    /// Slot 2: > 1 → decrement; else Shutdown and destroy, 0.
    pub fn release(&mut self) -> u32 {
        if self.ref_count > 1 {
            self.ref_count -= 1;
            return self.ref_count;
        }
        self.shutdown();
        self.ref_count = 0;
        0
    }

    /// Slot 3, Init (`0x00895E68`).
    pub fn init(&mut self) -> bool {
        if !self.initialised {
            self.initialised = true;
            self.font_set_name(5, "Times New Roman");
            self.font_set_name(9, "Arial");
            self.font_set_name(6, "Courier New");
            self.font_set_name(10, "Lucida Console");
        }
        true
    }

    /// `FontSetName(id, name)`: insert or replace.
    pub fn font_set_name(&mut self, id: u32, name: &str) {
        self.font_names.insert(id, name.to_owned());
    }

    /// Slot 4, Shutdown.
    pub fn shutdown(&mut self) -> bool {
        if self.initialised {
            for e in self.catalogue.values_mut() {
                for (f, _) in e.cache.drain(..) {
                    f.release();
                }
            }
            self.catalogue.clear();
            self.font_names.clear();
            self.families.clear();
            self.substitutions.clear();
            self.renderer = None;
            self.initialised = false;
        }
        true
    }

    /// Slot 5.
    pub fn set_code_page(&mut self, cp: u32) -> bool {
        self.code_page = cp;
        true
    }
    /// Slot 6 (AddRef new, release old, no same-pointer check: Arc clone/drop).
    pub fn set_renderer(&mut self, r: Option<Arc<dyn FontRenderer>>) {
        self.renderer = r;
    }
    pub fn renderer(&self) -> Option<Arc<dyn FontRenderer>> {
        self.renderer.clone()
    }
    /// Slot 7.
    pub fn set_default_antialiasing(&mut self, mode: u32) {
        self.default_aa_mode = mode;
    }
    /// Slot 8, SetMonochromeSizeRange(min, max, family).
    pub fn set_monochrome_size_range(&mut self, min: u32, max: u32, family: Option<&str>) {
        match family {
            None => {
                self.mono_min = min;
                self.mono_max = max;
            }
            Some(name) => {
                if (min as i32) < (max as i32) {
                    self.family_ranges.entry(name.to_owned()).or_insert((min, max));
                } else {
                    self.family_ranges.remove(name);
                }
            }
        }
    }

    fn new_font(&self) -> GzFont {
        GzFont::new(self.lib.clone(), self.conv.clone())
    }

    /// Slot 10, FontAcquire(path, size, style, cache) (`0x00895A1C`). Returns an AddRef'd font.
    pub fn font_acquire_path(&mut self, path: &str, size: u32, style: u32, cache: bool) -> Option<FontRef> {
        if cache {
            if let Some(e) = self.catalogue.get(path) {
                for (f, st) in &e.cache {
                    if *st == style && f.lock().point_size == size {
                        f.add_ref();
                        return Some(f.clone());
                    }
                }
            }
        }
        let mut font = self.new_font();
        font.set_file_path(path);
        font.set_size(size);
        font.set_style(style, true);
        font.set_code_page(self.code_page);
        font.set_renderer(self.renderer.clone());
        let mut mode = self.default_aa_mode;
        if self.mono_min <= size && size <= self.mono_max {
            mode = 1;
        } else if let Some(&(lo, hi)) = self.family_ranges.get(font.family_name.as_str()) {
            // Keyed by the pre-Init (empty) family name, as §12 requires.
            if lo <= size && size <= hi {
                mode = 1;
            }
        }
        font.set_antialiasing(mode);
        if !font.init() {
            let obj = FontObj::new(font);
            obj.release();
            return None;
        }
        let obj = FontObj::new(font);
        if cache {
            // The spec records the font "in the catalogue entry"; with no entry for this path nothing
            // is recorded (FN14).
            if let Some(e) = self.catalogue.get_mut(path) {
                obj.add_ref();
                e.cache.push((obj.clone(), style));
            }
        }
        Some(obj)
    }

    /// Slot 11, FontAcquire(familyList, kind, size, style, cache) (`0x00894ADF`).
    pub fn font_acquire_family(
        &mut self,
        family_list: &str,
        kind: u32,
        size: u32,
        style: u32,
        cache: bool,
    ) -> Option<FontRef> {
        // Empty names are dropped, so an empty list gives n = 0 (FN14: RZParseDelimitedText's
        // handling of empty fields is not given).
        let mut names: Vec<String> = lower(family_list)
            .split(',')
            .filter(|s| !s.is_empty())
            .take(32)
            .map(str::to_owned)
            .collect();
        for i in (0..names.len()).rev() {
            for (key, aliases) in &self.substitutions {
                for a in aliases {
                    if ieq(&names[i], a) && !self.families.contains(&lower(&names[i])) {
                        names[i] = lower(key);
                        break;
                    }
                }
            }
        }
        let masked = style & !USER_STYLES;
        let mut best: Option<&String> = None;
        let mut best_score = 1000u32;
        for (path, e) in &self.catalogue {
            let mut score = if !names.is_empty() {
                let fam = lower(&e.family);
                match names.iter().position(|n| *n == fam) {
                    Some(0) => 0,
                    Some(_) => 2,
                    None => 0x20,
                }
            } else if e.kind == kind {
                0
            } else {
                4
            };
            score += if e.style == masked {
                0
            } else if e.style & !masked == 0 {
                8
            } else {
                0x10
            };
            if score < best_score {
                best_score = score;
                best = Some(path);
            }
        }
        let best = best?.clone();
        if cache {
            if let Some(e) = self.catalogue.get(&best) {
                for (f, st) in &e.cache {
                    if f.lock().point_size == size && *st == style {
                        f.add_ref();
                        return Some(f.clone());
                    }
                }
            }
        }
        self.font_acquire_path(&best, size, style, cache)
    }

    /// Slot 9, FontAcquireByKind(kind, size, style, colour, cache).
    pub fn font_acquire_by_kind(
        &mut self,
        kind: u32,
        size: u32,
        style: u32,
        colour: u32,
        cache: bool,
    ) -> Option<FontRef> {
        let f = self.font_acquire_family("", kind, size, style, cache)?;
        f.lock().set_font_color(colour, PEN_DEFAULT_COLOUR);
        Some(f)
    }

    /// Slot 12, FontRelease(font).
    pub fn font_release(&mut self, font: &FontRef) -> u32 {
        let mut last = 0;
        for e in self.catalogue.values_mut() {
            if let Some(i) = e.cache.iter().position(|(f, _)| Arc::ptr_eq(f, font)) {
                last = e.cache[i].0.release();
                if last == 1 {
                    last = e.cache[i].0.release();
                    e.cache.remove(i);
                }
            }
        }
        last
    }

    /// Slot 13, AddFont(font).
    pub fn add_font(&mut self, font: Option<&FontRef>) -> bool {
        let Some(font) = font else {
            return false;
        };
        let (key, family, styles, kind, size) = {
            let f = font.lock();
            (
                f.file_path.clone(),
                f.family_name.clone(),
                f.style_flags,
                f.face_kind,
                f.point_size,
            )
        };
        match self.catalogue.get_mut(&key) {
            None => {
                font.add_ref();
                self.catalogue.insert(
                    key,
                    FontFixedInfo {
                        cache: vec![(font.clone(), styles)],
                        family: family.clone(),
                        style: styles,
                        kind,
                        source_type: 0,
                    },
                );
                self.families.insert(lower(&family));
                true
            }
            Some(e) => {
                if e.cache.iter().any(|(f, _)| f.lock().point_size == size) {
                    return false;
                }
                font.add_ref();
                e.cache.push((font.clone(), styles));
                true
            }
        }
    }

    /// Slot 14, AddFont(path).
    pub fn add_font_path(&mut self, path: &str) -> bool {
        let mut p = path.to_owned();
        let relative = !(p.starts_with('\\') || p.starts_with('/') || p.as_bytes().get(1) == Some(&b':'));
        if relative && !self.lib.file_exists(&p) {
            p = format!("{}{}", self.lib.system_fonts_dir(), p);
        }
        if self.catalogue.contains_key(&p) {
            return false;
        }
        let Some(face) = self.lib.open(&p) else {
            return false;
        };
        let family = face.family_name();
        if family.is_empty() {
            return false;
        }
        let info = FontFixedInfo {
            cache: Vec::new(),
            family: family.clone(),
            style: face.style_flags(),
            kind: if face.is_fixed_size() { 2 } else { 1 },
            source_type: source_type_from_file_name(&*self.lib, &p),
        };
        self.catalogue.insert(p, info);
        self.families.insert(lower(&family));
        true
    }

    /// Slot 15, AddAllFontsFromDirectory(dir): count added.
    pub fn add_all_fonts_from_directory(&mut self, dir: &str) -> u32 {
        let dir = if dir.is_empty() {
            self.lib.system_fonts_dir()
        } else {
            dir.to_owned()
        };
        let mut n = 0;
        for pat in self.file_patterns.clone() {
            for f in self.lib.list_files(&dir, &pat) {
                if self.add_font_path(&f) {
                    n += 1;
                }
            }
        }
        n
    }

    /// Slot 16: one ignored argument, false.
    pub fn slot16(&self, _arg: u32) -> bool {
        false
    }

    /// Slot 17, RemoveFontsByFamily(name).
    pub fn remove_fonts_by_family(&mut self, name: &str) -> bool {
        self.catalogue.retain(|_, e| {
            if ieq(&e.family, name) {
                for (f, _) in e.cache.drain(..) {
                    f.release();
                }
                false
            } else {
                true
            }
        });
        false
    }

    /// Slot 18, GetFontInfoList(records, max = out.len()).
    pub fn get_font_info_list(&self, out: &mut [FontInfoRecord]) -> u32 {
        let n = self.catalogue.len();
        if out.is_empty() {
            return n as u32;
        }
        let m = n.min(out.len());
        for (rec, e) in out.iter_mut().zip(self.catalogue.values()).take(m) {
            *rec = FontInfoRecord::default();
            let b = e.family.as_bytes();
            let k = b.len().min(127);
            rec.family[..k].copy_from_slice(&b[..k]);
            rec.style = e.style;
            rec.kind = e.kind;
        }
        m as u32
    }

    /// Slot 19, GetCachedFonts(fonts, max = out.len()); not AddRef'd.
    pub fn get_cached_fonts(&self, out: &mut [Option<FontRef>]) -> u32 {
        let all: Vec<&FontRef> = self
            .catalogue
            .values()
            .flat_map(|e| e.cache.iter().map(|(f, _)| f))
            .collect();
        if out.is_empty() {
            return all.len() as u32;
        }
        let m = all.len().min(out.len());
        for (o, f) in out.iter_mut().zip(all) {
            *o = Some(f.clone());
        }
        m as u32
    }

    /// Slot 20, AddSubstitution(family, alias).
    pub fn add_substitution(&mut self, family: &str, alias: &str) -> bool {
        self.substitutions
            .entry(family.to_owned())
            .or_default()
            .push(alias.to_owned());
        true
    }

    /// Slot 21, GetSubstitutions(family, strings, max = out.len()); assigns only the first m.
    pub fn get_substitutions(&self, family: &str, out: &mut [Option<String>]) -> u32 {
        let Some(a) = self.substitutions.get(family) else {
            return 0;
        };
        let m = a.len().min(out.len());
        for (o, s) in out.iter_mut().zip(a).take(m) {
            if let Some(o) = o {
                *o = s.clone();
            }
        }
        m as u32
    }

    /// Slot 22, IsFamilyAvailable(name, unused, style).
    pub fn is_family_available(&self, name: &str, _unused: u32, style: u32) -> bool {
        let ln = lower(name);
        if !self.families.contains(&ln) {
            return false;
        }
        self.catalogue
            .values()
            .any(|e| e.style == (style & 3) && lower(&e.family) == ln)
    }

    /// Slot 23, GetAvailableSubstitution(family, &out).
    pub fn get_available_substitution(&self, family: &str, out: &mut String) -> bool {
        let Some(a) = self.substitutions.get(family) else {
            return false;
        };
        for s in a {
            if self.is_family_available(s, 0, 0) {
                *out = s.clone();
                return true;
            }
        }
        false
    }
}

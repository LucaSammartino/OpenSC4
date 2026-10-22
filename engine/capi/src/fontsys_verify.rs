// SPDX-License-Identifier: GPL-3.0-or-later
//! Opt-in native verification transport. Persistent production GzFontSys, not a verification
//! implementation of its algorithms. Metadata is supplied once at construction, never resynced.
//! Provider sizes/rasters are deliberately outside this FontSys-only measurement domain.
use fc4_core::fonts::{
    DrawColour, DrawRecord, FontFace, FontInfoRecord, FontLibrary, FontRef, FontRenderer, FontUse, GlyphRaster,
    GreyLevels, GzFontSys, Latin1Converter, ScreenFormat, SizeMetrics, SurfaceId,
};
use std::ffi::{c_char, CStr};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Metadata {
    path: [u8; 260],
    family: [u8; 128],
    style: u32,
    kind: u32,
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(&bytes[..bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len())]).into_owned()
}
struct Library(Vec<Metadata>);
struct Face(Metadata);
impl FontFace for Face {
    fn family_name(&self) -> String {
        text(&self.0.family)
    }
    fn style_flags(&self) -> u32 {
        self.0.style
    }
    fn is_fixed_size(&self) -> bool {
        self.0.kind == 2
    }
    fn char_index(&self, _: u32) -> Option<u32> {
        Some(0)
    }
    fn fixed_advance(&self) -> i32 {
        0
    }
    fn set_char_size(&mut self, _: u32, _: u32, _: u32, _: f32, _: f32) -> Option<SizeMetrics> {
        // Only positive ordinary sizes are used by the constructed FontSys tests. No metrics claim.
        Some(SizeMetrics::default())
    }
    fn load_glyph(&mut self, _: u32, _: GreyLevels, _: u16) -> Option<GlyphRaster> {
        None
    }
}
impl FontLibrary for Library {
    fn file_exists(&self, path: &str) -> bool {
        self.0.iter().any(|m| text(&m.path) == path)
    }
    fn open(&self, path: &str) -> Option<Box<dyn FontFace>> {
        self.0
            .iter()
            .find(|m| text(&m.path) == path)
            .map(|m| Box::new(Face(*m)) as Box<dyn FontFace>)
    }
    fn read_head(&self, _: &str, _: usize) -> Vec<u8> {
        Vec::new()
    }
    fn system_fonts_dir(&self) -> String {
        self.0
            .first()
            .and_then(|m| text(&m.path).rsplit_once('\\').map(|(dir, _)| format!("{dir}\\")))
            .unwrap_or_default()
    }
    fn list_files(&self, dir: &str, pattern: &str) -> Vec<String> {
        if !dir.eq_ignore_ascii_case(&self.system_fonts_dir()) {
            return Vec::new();
        }
        self.0
            .iter()
            .map(|m| text(&m.path))
            .filter(|path| {
                let path = path.to_ascii_lowercase();
                match pattern {
                    "*.tt?" => path
                        .rsplit_once('.')
                        .is_some_and(|(_, ext)| ext.len() == 3 && ext.starts_with("tt")),
                    "*.pfb" => path.ends_with(".pfb"),
                    "*.mxf" => path.ends_with(".mxf"),
                    "*.otf" => path.ends_with(".otf"),
                    _ => false,
                }
            })
            .collect()
    }
    fn screen_format(&self) -> ScreenFormat {
        ScreenFormat::Screen(9)
    }
}

struct LifetimeRenderer;
impl FontRenderer for LifetimeRenderer {
    fn lock_surface(&self, _: SurfaceId, _: FontUse) -> bool {
        false
    }
    fn unlock_surface(&self, _: SurfaceId, _: FontUse) -> bool {
        false
    }
    fn draw(&self, _: SurfaceId, _: &[DrawRecord], _: &[u8], _: &DrawColour) {}
}
pub struct Session {
    sys: GzFontSys,
    fonts: Vec<FontRef>,
    renderer: Arc<LifetimeRenderer>,
}
impl Session {
    fn intern(&mut self, font: Option<FontRef>) -> u32 {
        let Some(font) = font else {
            return 0;
        };
        if let Some(i) = self.fonts.iter().position(|f| Arc::ptr_eq(f, &font)) {
            return i as u32 + 1;
        }
        self.fonts.push(font);
        self.fonts.len() as u32
    }
    #[allow(clippy::too_many_arguments)]
    fn run(&mut self, op: u32, a: u32, b: u32, c: u32, d: u32, s: &str, t: &str, out: &mut [u8]) -> Option<u32> {
        let font = || self.fonts.get(a.checked_sub(1)? as usize).cloned();
        Some(match op {
            0 => u32::from(self.sys.init()),
            1 => u32::from(self.sys.shutdown()),
            2 => u32::from(self.sys.add_font_path(s)),
            3 if a <= 4 => {
                let mut records = vec![FontInfoRecord::default(); a as usize];
                let n = self.sys.get_font_info_list(&mut records);
                for (dst, r) in out.chunks_exact_mut(136).zip(records.iter()).take(n.min(a) as usize) {
                    dst.copy_from_slice(&r.to_bytes());
                }
                n
            }
            4 => u32::from(self.sys.is_family_available(s, b, a)),
            5 => u32::from(self.sys.add_substitution(s, t)),
            6 if a <= 4 => {
                let mut strings: Vec<_> = (0..a)
                    .map(|i| {
                        if b & (1 << i) == 0 {
                            Some("sentinel".to_owned())
                        } else {
                            None
                        }
                    })
                    .collect();
                let n = self.sys.get_substitutions(s, &mut strings);
                for (i, string) in strings.iter().enumerate() {
                    if let Some(string) = string {
                        put_string(&mut out[i * 128..(i + 1) * 128], string);
                    }
                }
                n
            }
            7 => {
                let mut string = "sentinel".to_owned();
                let result = self.sys.get_available_substitution(s, &mut string);
                put_string(&mut out[..128], &string);
                u32::from(result)
            }
            8 => {
                self.sys
                    .set_monochrome_size_range(a, b, if c == 0 { None } else { Some(s) });
                0
            }
            9 => {
                self.sys.set_code_page(a);
                0
            }
            10 => {
                self.sys.set_default_antialiasing(a);
                0
            }
            11 => {
                let f = self.sys.font_acquire_path(s, a, b, c != 0);
                self.intern(f)
            }
            12 => {
                let f = self.sys.font_acquire_family(s, d, a, b, c != 0);
                self.intern(f)
            }
            13 => {
                let f = self.sys.font_acquire_by_kind(d, a, b, 0x12345678, c != 0);
                self.intern(f)
            }
            14 => {
                let f = font()?;
                let v = f.lock();
                put_string(&mut out[..260], &v.file_path);
                put_string(&mut out[260..388], &v.family_name);
                for (dst, value) in out[388..].chunks_exact_mut(4).zip([
                    v.point_size,
                    v.style_flags,
                    v.face_kind,
                    v.aa_mode,
                    v.code_page,
                    v.fg,
                    v.bg,
                    f.ref_count(),
                ]) {
                    dst.copy_from_slice(&value.to_le_bytes());
                }
                1
            }
            15 => {
                let f = font()?;
                self.sys.font_release(&f)
            }
            16 if a <= 8 => {
                let mut fonts = vec![None; a as usize];
                let n = self.sys.get_cached_fonts(&mut fonts);
                for (i, f) in fonts.into_iter().take(n.min(a) as usize).enumerate() {
                    let id = self.intern(f);
                    out[i * 4..i * 4 + 4].copy_from_slice(&id.to_le_bytes());
                }
                n
            }
            17 => u32::from(self.sys.remove_fonts_by_family(s)),
            18 => {
                let f = if a == 0 { None } else { Some(font()?) };
                u32::from(self.sys.add_font(f.as_ref()))
            }
            19 => u32::from(self.sys.slot16(a)),
            20 => self.sys.add_ref(),
            21 => self.sys.release(),
            22 => match self.sys.query_interface(a) {
                Some(second) => {
                    self.sys.release();
                    1 + u32::from(second)
                }
                None => 0,
            },
            23 => {
                self.sys.set_renderer(None);
                0
            }
            // Uncached caller references use the font's own Release, not FontRelease (slot 12).
            24 => font()?.release(),
            25 => self.sys.add_all_fonts_from_directory(s),
            26 => {
                self.sys.set_renderer(Some(self.renderer.clone()));
                0
            }
            27 => Arc::strong_count(&self.renderer) as u32,
            _ => return None,
        })
    }
}
fn put_string(out: &mut [u8], value: &str) {
    out.fill(0);
    let n = value.len().min(out.len() - 1);
    out[..n].copy_from_slice(&value.as_bytes()[..n]);
}

#[no_mangle]
pub unsafe extern "C" fn fc4_fontsys_verify_new(meta: *const Metadata, count: u32) -> *mut Session {
    catch_unwind(AssertUnwindSafe(|| {
        if count > 4096 || (count != 0 && meta.is_null()) {
            return std::ptr::null_mut();
        }
        let data = if count == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(meta, count as usize) }.to_vec()
        };
        let mut sys = GzFontSys::new(Arc::new(Library(data)), Arc::new(Latin1Converter), 0);
        sys.add_ref();
        Box::into_raw(Box::new(Session {
            sys,
            fonts: Vec::new(),
            renderer: Arc::new(LifetimeRenderer),
        }))
    }))
    .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_fontsys_verify_call(
    session: *mut Session,
    op: u32,
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    s: *const c_char,
    t: *const c_char,
    out: *mut u8,
) -> u32 {
    catch_unwind(AssertUnwindSafe(|| {
        let session = unsafe { session.as_mut() }?;
        if out.is_null() {
            return None;
        }
        let decode = |p: *const c_char| {
            if p.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
            }
        };
        let (s, t) = (decode(s), decode(t));
        session.run(op, a, b, c, d, &s, &t, unsafe {
            std::slice::from_raw_parts_mut(out, 1024)
        })
    }))
    .ok()
    .flatten()
    .unwrap_or(u32::MAX)
}

#[no_mangle]
pub unsafe extern "C" fn fc4_fontsys_verify_free(session: *mut Session) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !session.is_null() {
            let mut session = unsafe { Box::from_raw(session) };
            session.sys.shutdown();
        }
    }));
}

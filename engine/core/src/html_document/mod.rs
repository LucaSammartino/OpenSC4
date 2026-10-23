//! Clean-room `cHTMLDocument`: the HTML help/news document parser (docs/specs/html-document.md).
//!
//! The document turns HTML source bytes into a tree of element nodes held in an arena
//! ([`HtmlDocument::nodes`]). Element classes (`cHTMLBODY`, `cHTMLTextSegment`, ...) are other
//! owners: the document records exactly what it passes to them as typed fields, and everything
//! it asks of the graphics system, the downloader, the image loader or the file system goes
//! through the [`HtmlHost`] trait. Strings are byte strings so the source encoding is preserved.

pub mod entities;
mod parse;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

pub use entities::{entity_code, get_string_for_escape};

/// Index of a node in [`HtmlDocument::nodes`].
pub type NodeId = usize;
/// Opaque image / drawing-surface handle owned by the host.
pub type ImageHandle = u64;

/// Token ids (§2.2) used by name in this module; tag ids are those of [`TAGS`].
pub mod tok {
    pub const UNKNOWN: u32 = 0;
    pub const EOF: u32 = 1;
    pub const DOCTYPE: u32 = 2;
    pub const TEXT: u32 = 3;
    pub const HTML: u32 = 4;
    pub const HTML_END: u32 = 5;
    pub const HEAD: u32 = 6;
    pub const HEAD_END: u32 = 7;
    pub const BODY: u32 = 8;
    pub const BODY_END: u32 = 9;
}

/// The tag table of InitTagList (§2.3): upper-case name -> id, 160 entries.
pub const TAGS: [(&str, u32); 160] = [
    ("!DOCTYPE", 2),
    ("HTML", 4),
    ("/HTML", 5),
    ("HEAD", 6),
    ("/HEAD", 7),
    ("BODY", 8),
    ("/BODY", 9),
    ("TITLE", 10),
    ("/TITLE", 11),
    ("BASE", 12),
    ("STYLE", 13),
    ("/STYLE", 14),
    ("META", 15),
    ("LINK", 16),
    ("ISINDEX", 17),
    ("TT", 18),
    ("/TT", 19),
    ("I", 20),
    ("/I", 21),
    ("B", 22),
    ("/B", 23),
    ("STRIKE", 24),
    ("/STRIKE", 25),
    ("BIG", 26),
    ("/BIG", 27),
    ("SMALL", 28),
    ("/SMALL", 29),
    ("U", 30),
    ("/U", 31),
    ("FONT", 32),
    ("/FONT", 33),
    ("EM", 34),
    ("/EM", 35),
    ("STRONG", 36),
    ("/STRONG", 37),
    ("DFN", 38),
    ("/DFN", 39),
    ("CODE", 40),
    ("/CODE", 41),
    ("SAMP", 42),
    ("/SAMP", 43),
    ("KBD", 44),
    ("/KBD", 45),
    ("VAR", 46),
    ("/VAR", 47),
    ("CITE", 48),
    ("/CITE", 49),
    ("ABBR", 50),
    ("/ABBR", 51),
    ("ACRONYM", 52),
    ("/ACRONYM", 53),
    ("A", 54),
    ("/A", 55),
    ("IMG", 56),
    ("OBJECT", 57),
    ("/OBJECT", 58),
    ("BR", 59),
    ("SCRIPT", 60),
    ("/SCRIPT", 61),
    ("MAP", 62),
    ("/MAP", 63),
    ("Q", 64),
    ("/Q", 65),
    ("SUB", 66),
    ("/SUB", 67),
    ("SUP", 68),
    ("/SUP", 69),
    ("SPAN", 70),
    ("/SPAN", 71),
    ("BDO", 72),
    ("/BDO", 73),
    ("BLINK", 74),
    ("/BLINK", 75),
    ("INPUT", 76),
    ("SELECT", 77),
    ("/SELECT", 78),
    ("OPTION", 79),
    ("/OPTION", 80),
    ("OPTGROUP", 81),
    ("/OPTGROUP", 82),
    ("TEXTAREA", 83),
    ("/TEXTAREA", 84),
    ("LABEL", 85),
    ("/LABEL", 86),
    ("BUTTON", 87),
    ("/BUTTON", 88),
    ("H1", 89),
    ("/H1", 90),
    ("H2", 91),
    ("/H2", 92),
    ("H3", 93),
    ("/H3", 94),
    ("H4", 95),
    ("/H4", 96),
    ("H5", 97),
    ("/H5", 98),
    ("H6", 99),
    ("/H6", 100),
    ("P", 101),
    ("/P", 102),
    ("PRE", 103),
    ("/PRE", 104),
    ("UL", 105),
    ("/UL", 106),
    ("OL", 107),
    ("/OL", 108),
    ("DL", 109),
    ("/DL", 110),
    ("DIV", 111),
    ("/DIV", 112),
    ("NOSCRIPT", 113),
    ("/NOSCRIPT", 114),
    ("BLOCKQUOTE", 115),
    ("/BLOCKQUOTE", 116),
    ("FORM", 117),
    ("/FORM", 118),
    ("HR", 119),
    ("TABLE", 120),
    ("/TABLE", 121),
    ("FIELDSET", 122),
    ("/FIELDSET", 123),
    ("ADDRESS", 124),
    ("/ADDRESS", 125),
    ("CENTER", 126),
    ("/CENTER", 127),
    ("DIR", 128),
    ("/DIR", 129),
    ("MENU", 130),
    ("/MENU", 131),
    ("CAPTION", 132),
    ("/CAPTION", 133),
    ("COL", 134),
    ("COLGROUP", 135),
    ("/COLGROUP", 136),
    ("THEAD", 137),
    ("/THEAD", 138),
    ("TBODY", 139),
    ("/TBODY", 140),
    ("TFOOT", 141),
    ("/TFOOT", 142),
    ("TD", 143),
    ("/TD", 144),
    ("TH", 145),
    ("/TH", 146),
    ("TR", 147),
    ("/TR", 148),
    ("LI", 149),
    ("/LI", 150),
    ("DT", 151),
    ("/DT", 152),
    ("DD", 153),
    ("/DD", 154),
    ("AREA", 155),
    ("FRAMESET", 156),
    ("/FRAMESET", 157),
    ("FRAME", 158),
    ("NOFRAMES", 159),
    ("/NOFRAMES", 160),
    ("NOBR", 161),
    ("/NOBR", 162),
];

/// Tag-table lookup of an upper-cased tag name.
pub fn tag_id(name: &[u8]) -> Option<u32> {
    TAGS.iter().find(|(n, _)| n.as_bytes() == name).map(|&(_, id)| id)
}

/// Grammar sets of BuildFirstSets (§4.1).
pub mod sets {
    pub fn fontstyle(t: u32) -> bool {
        matches!(t, 18 | 20 | 22 | 24 | 26 | 28 | 30 | 32)
    }
    pub fn phrase(t: u32) -> bool {
        matches!(t, 34 | 36 | 38 | 40 | 42 | 44 | 46 | 48 | 50 | 52)
    }
    pub fn special(t: u32) -> bool {
        matches!(t, 54 | 56 | 57 | 59 | 60 | 62 | 64 | 66 | 68 | 70 | 72 | 74)
    }
    pub fn formctrl(t: u32) -> bool {
        matches!(t, 76 | 77 | 83 | 85 | 87)
    }
    pub fn inline(t: u32) -> bool {
        t == super::tok::TEXT || fontstyle(t) || phrase(t) || special(t) || formctrl(t)
    }
    pub fn heading(t: u32) -> bool {
        matches!(t, 89 | 91 | 93 | 95 | 97 | 99)
    }
    pub fn list(t: u32) -> bool {
        matches!(t, 105 | 107)
    }
    pub fn block(t: u32) -> bool {
        matches!(t, 101 | 103 | 109 | 111 | 113 | 115 | 117 | 119 | 120 | 122 | 124 | 126) || heading(t) || list(t)
    }
    pub fn flow(t: u32) -> bool {
        inline(t) || block(t)
    }
}

/// Default HTML size table for sizes 1..7 (§1.1).
pub const DEFAULT_FONT_SIZES: [i32; 7] = [8, 10, 12, 14, 18, 24, 36];
/// Heading pixel sizes for H1..H6 (§7.14); the font-size table is bypassed.
pub const HEADING_SIZES: [i32; 6] = [40, 32, 28, 24, 20, 16];
/// Message type of a finished download (§9).
pub const MSG_RESOURCE_DOWNLOADED: u32 = 0x7575_8483;
pub const IID_IGZUNKNOWN: u32 = 1;
pub const IID_HTML_DOCUMENT: u32 = 0xe0ec_a23a;
pub const IID_MESSAGE_TARGET: u32 = 0x4522_94aa;

/// Services the document uses from other owners. Methods with a default body are optional.
pub trait HtmlHost {
    /// Opaque dword of the 32-bit document layout (§7.11). Used only for FONT SIZE
    /// reads inside the object that address foreign pointers, strings or container storage.
    /// Scalar fields owned here are supplied by `font_size_word` instead. No native Rust
    /// address is cast to a 32-bit game pointer. An adapter supplies its compatibility words.
    fn document_layout_word(&self, offset: u32) -> u32;
    /// Init's AddRefs on the owner window and, when present, the extra interface (§1.2).
    fn retain_interfaces(&mut self, _has_extra: bool) {}
    /// QueryInterface's AddRef through the primary base (§1.5). Lifetime is host-owned.
    fn retain_document(&mut self) {}
    /// Shared `HTMLIsSpace(text, i)` (§2.1). Default: the C-locale `isspace` set on single
    /// bytes (literal single-byte reading; a DBCS host must supply its own).
    fn is_space(&self, text: &[u8], i: usize) -> bool {
        matches!(text.get(i), Some(b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c))
    }
    /// Shared `HTMLAdvance(text, i)` (§2.1). Default: one byte.
    fn advance(&self, _text: &[u8], i: usize) -> usize {
        i + 1
    }
    /// Graphics-system colour constructor from r,g,b (GS vtable +0x88).
    fn rgb(&mut self, r: i32, g: i32, b: i32) -> u32;
    /// `cHTMLShared::SetColor(value, colour)`: parse a colour attribute into `color`
    /// (callers pre-load 0xff000000).
    fn set_color(&mut self, value: &[u8], color: &mut u32);
    /// The GS +0x88 conversion the document applies to a `SetColor` result (helpers 009081d2,
    /// 0090824c, 0090820f, 008fe68f).
    fn convert_color(&mut self, parsed: u32) -> u32;
    /// `cHTMLTABLE::SetBGCOLOR(value)`: the resulting table colour (+0x44).
    fn table_bgcolor(&mut self, value: &[u8]) -> u32;
    /// Owner window base-URL forward (window+0xd8, vtable +0x4c).
    fn window_set_base_url(&mut self, _url: &[u8]) {}
    /// Extra interface vtable +0x60: has `url` been visited.
    fn is_visited(&mut self, _url: &[u8]) -> bool {
        false
    }
    /// Downloader vtable +0x44 (url, message target, 3, 0, 1); returns the request value.
    fn request_download(&mut self, _url: &[u8]) -> u32 {
        0
    }
    /// Downloader vtable +0x58 with an entry's request value (Shutdown).
    fn withdraw_download(&mut self, _url: &[u8], _request: u32) {}
    /// Waiter slot 0 with an image (`waiter` = the body or image node, None for no body).
    fn deliver_image(&mut self, _waiter: Option<NodeId>, _image: ImageHandle) -> bool {
        false
    }
    /// Image vtable +8 (Release) at Shutdown.
    fn release_image(&mut self, _image: ImageHandle) {}
    /// GZCOM create of the image loader (class 0xa2ffb5d3, iid 0x02ffbb60).
    fn create_image_loader(&mut self) -> bool {
        false
    }
    /// Loader vtable +0x10 (path, out, 1); `Some` stores into the out image.
    fn load_image(&mut self, _path: &[u8]) -> Option<ImageHandle> {
        None
    }
    /// Release of the image loader, downloader and owner window at Shutdown.
    fn release_interfaces(&mut self) {}
    /// `cRZFile::MakeTempPathName(path, 0)`.
    fn make_temp_path(&mut self) -> Vec<u8>;
    /// Open (write only, create always, exclusive) and write `data`; false on open failure.
    fn write_file(&mut self, path: &[u8], data: &[u8]) -> bool;
    /// `cRZFile::Remove(path)`.
    fn remove_file(&mut self, path: &[u8]);
    /// CreateBullet surface (§7.9 step 3): `width` x `height`, filled with `background`, plus a
    /// filled circle in (0,0)-(d-1,d-1) of `circle = (d, colour)` for style 0. None = failure.
    fn create_bullet_surface(
        &mut self,
        width: u32,
        height: u32,
        background: u32,
        circle: Option<(u32, u32)>,
    ) -> Option<ImageHandle>;
    /// Anchor name test (anchor +0x30, vtable +0x28 with (name, 1)) used by FindNamedAnchor.
    fn anchor_name_matches(&mut self, anchor_name: &[u8], name: &[u8]) -> bool {
        anchor_name == name
    }
}

/// The 0x28-byte text style record (§8.2, §7.11).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Style {
    pub face: Vec<u8>,
    /// 0x144: 1 after Init, 10 fixed-pitch, 9 heading, 2 PRE, 0 use `face`.
    pub selector: i32,
    pub size: i32,
    pub color: u32,
    /// 1 bold, 2 italic, 4 underline, 8 strike-through.
    pub flags: u32,
    /// 0x154: preformatted (§5.4).
    pub pre: bool,
    /// 0x155: the colour is a link colour.
    pub link: bool,
}

pub const BOLD: u32 = 1;
pub const ITALIC: u32 = 2;
pub const UNDERLINE: u32 = 4;
pub const STRIKE: u32 = 8;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Body {
    pub link: u32,
    pub vlink: u32,
    pub alink: u32,
    /// body+0x2c: raw `SetColor` result (not GS-converted).
    pub bgcolor: Option<u32>,
    pub background_url: Vec<u8>,
    pub bgproperties: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Table {
    /// Raw values for `cHTMLShared::SetPixels` / `cHTMLTABLE::SetWIDTH` / `SetLength`.
    pub border: Option<Vec<u8>>,
    pub width: Option<Vec<u8>>,
    pub cellpadding: Option<Vec<u8>>,
    pub cellspacing: Option<Vec<u8>>,
    pub bgcolor: Option<u32>,
    pub rows: Vec<NodeId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tr {
    pub color: Option<u32>,
    pub cells: Vec<NodeId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Td {
    pub color: Option<u32>,
    /// None = constructor default.
    pub valign: Option<u32>,
    /// Raw values for `cHTMLShared::SetLength` / `cHTMLTD::SetNOWRAP`.
    pub width: Option<Vec<u8>>,
    pub nowrap: Option<Vec<u8>>,
    pub align: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hr {
    pub align: Option<u32>,
    pub size: Option<u32>,
    pub width: Option<u32>,
    pub percent: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Img {
    pub url: Vec<u8>,
    pub alt: Option<Vec<u8>>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// +0x34 MakeRelativeToTextAlignmentStyle; None = constructor default.
    pub valign: Option<u32>,
    /// +0x38 the current alignment.
    pub align: u32,
    /// Bullet surface given by CreateBullet.
    pub surface: Option<ImageHandle>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Anchor {
    pub href: Option<Vec<u8>>,
    pub href_absolute: Option<Vec<u8>>,
    pub name: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextSegment {
    pub text: Vec<u8>,
    pub style: Style,
    pub align: u32,
}

/// Element kinds the document creates (constructor kind codes in comments).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Body(Body),
    Text(TextSegment),
    Br,
    Input,
    Select,
    OptionItem,
    Div,
    /// FORM (ctor 008fed5c, exported as `cHTMLP`).
    Form,
    /// BLOCKQUOTE container (ctor 008fed0c).
    Blockquote,
    /// P kind 0x15, DL 8, DT 9, DD 10.
    Container(u32),
    /// Heading container (kind 2), level index 0..5.
    Heading(u8),
    /// UL kind 6 / OL kind 7 with the per-document serial.
    List {
        kind: u32,
        serial: u32,
    },
    ListItem,
    HangingIndent,
    Table(Table),
    Tr(Tr),
    Td(Td),
    Th,
    Hr(Hr),
    Span,
    Blink,
    Anchor(Anchor),
    Img(Img),
    Map {
        name: Option<Vec<u8>>,
    },
    /// SHAPE code; None = constructor default.
    Area {
        shape: Option<u32>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub kind: Kind,
    pub children: Vec<NodeId>,
    pub parent: Option<NodeId>,
    pub deleted: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MetaRecord {
    pub name: Option<Vec<u8>>,
    pub content: Vec<u8>,
    pub http_equiv: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LinkRecord {
    pub title: Option<Vec<u8>>,
    /// byte +0x20: 1 when REV is absent.
    pub forward: bool,
    pub href: Vec<u8>,
    pub href_absolute: Option<Vec<u8>>,
}

/// A download-table entry (§7.14, §9).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Download {
    pub image: Option<ImageHandle>,
    pub waiters: Vec<Option<NodeId>>,
    pub request: u32,
}

/// The download record delivered by DoMessage (§9).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DownloadRecord {
    pub url: Vec<u8>,
    /// +0x2c: `data` is a file path (true) or the downloaded bytes (false).
    pub is_path: bool,
    pub data: Vec<u8>,
}

/// Interface base returned by QueryInterface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Base {
    Primary,
    Secondary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BulletRecord {
    d: u32,
    style: u32,
    color: u32,
    surface: Option<ImageHandle>,
}

/// Process-wide bullet cache (00b62524), never pruned.
static BULLETS: std::sync::Mutex<Vec<BulletRecord>> = std::sync::Mutex::new(Vec::new());
/// §1.1: colour construction happens once per process, like the immutable tag/grammar tables.
static DEFAULT_COLORS: std::sync::OnceLock<[u32; 4]> = std::sync::OnceLock::new();

pub struct HtmlDocument<H: HtmlHost> {
    pub host: H,
    pub nodes: Vec<Node>,
    /// 0x14
    pub root: Option<NodeId>,
    /// 0x18
    pub initialised: bool,
    /// 0x28
    pub pushback: bool,
    /// 0x29 (slot 15)
    pub slot15_byte: u8,
    /// 0x2a: the page contains blinking content.
    pub has_blink: bool,
    /// 0x30 present.
    pub has_extra: bool,
    pub proportional_face: Vec<u8>,
    pub proportional_size: i32,
    pub fixed_face: Vec<u8>,
    pub fixed_size: i32,
    /// 0x98
    pub title: Vec<u8>,
    /// 0xac
    pub metas: Vec<MetaRecord>,
    /// 0xb8
    pub base_url: Option<Vec<u8>>,
    /// 0xbc
    pub links: Vec<LinkRecord>,
    /// 0xc8: anchors in NAME order.
    pub named_anchors: Vec<NodeId>,
    pub tag_text: Vec<u8>,
    pub token: u32,
    pub attr_text: Vec<u8>,
    pub pending: Vec<u8>,
    /// 0x114 (insertion order; first occurrence wins).
    pub attrs: Vec<(Vec<u8>, Vec<u8>)>,
    pub style: Style,
    /// 0x158
    pub align: u32,
    pub style_stack: Vec<Style>,
    pub align_stack: Vec<u32>,
    pub ol_serial: u32,
    pub ul_serial: u32,
    pub source: Vec<u8>,
    pub pos: usize,
    pub downloads: BTreeMap<Vec<u8>, Download>,
    pub image_loader: bool,
    pub font_sizes: [i32; 7],
    /// Global colours of §1.1: text, link, vlink, alink.
    pub default_text: u32,
    pub default_link: u32,
    pub default_vlink: u32,
    pub default_alink: u32,
}

impl<H: HtmlHost> HtmlDocument<H> {
    /// Constructor (§1.1).
    pub fn new(mut host: H) -> Self {
        let [default_text, default_vlink, default_link, default_alink] = *DEFAULT_COLORS.get_or_init(|| {
            [
                host.rgb(0, 0, 0),
                host.rgb(100, 32, 164),
                host.rgb(0, 0, 255),
                host.rgb(192, 0, 255),
            ]
        });
        let mut d = HtmlDocument {
            host,
            nodes: Vec::new(),
            root: None,
            initialised: false,
            pushback: false,
            slot15_byte: 0,
            has_blink: false,
            has_extra: false,
            proportional_face: Vec::new(),
            proportional_size: 0,
            fixed_face: Vec::new(),
            fixed_size: 0,
            title: Vec::new(),
            metas: Vec::new(),
            base_url: None,
            links: Vec::new(),
            named_anchors: Vec::new(),
            tag_text: Vec::new(),
            token: 0,
            attr_text: Vec::new(),
            pending: Vec::new(),
            attrs: Vec::new(),
            style: Style::default(),
            align: 0,
            style_stack: Vec::new(),
            align_stack: Vec::new(),
            ol_serial: 0,
            ul_serial: 0,
            source: Vec::new(),
            pos: 0,
            downloads: BTreeMap::new(),
            image_loader: false,
            font_sizes: [0; 7],
            default_text,
            default_link,
            default_vlink,
            default_alink,
        };
        d.set_font_size_table(&DEFAULT_FONT_SIZES);
        d.proportional_face = b"arial".to_vec();
        d.fixed_face = b"courier new".to_vec();
        d.proportional_size = d.font_sizes[3];
        d.fixed_size = d.font_sizes[3];
        d
    }

    /// Init (§1.2): once per object; always true.
    pub fn init(&mut self, has_extra: bool, base_url: &[u8]) -> bool {
        if self.initialised {
            return true;
        }
        self.host.retain_interfaces(has_extra);
        self.has_extra = has_extra;
        self.set_base_url(base_url);
        self.style.selector = 1;
        self.style.size = self.font_sizes[3];
        self.style.color = self.default_text;
        self.style.flags = 0;
        self.style.pre = false;
        self.style.link = false;
        self.initialised = true;
        true
    }

    /// SetBaseURL (§1.3).
    pub fn set_base_url(&mut self, url: &[u8]) -> bool {
        self.base_url = Some(url.to_vec());
        self.host.window_set_base_url(url);
        true
    }

    /// SetFontSizeTable (§1.4, slot 7).
    pub fn set_font_size_table(&mut self, sizes: &[i32]) {
        let n = sizes.len().min(7);
        self.font_sizes[..n].copy_from_slice(&sizes[..n]);
        self.proportional_size = self.font_sizes[3];
        self.fixed_size = self.font_sizes[3];
    }

    /// GetFontSizeTable (slot 6, §1.6): returns min(count, 7); zero-fills the rest of `out`.
    pub fn get_font_size_table(&self, out: &mut [i32]) -> usize {
        let n = out.len().min(7);
        out[..n].copy_from_slice(&self.font_sizes[..n]);
        for v in &mut out[n..] {
            *v = 0;
        }
        n
    }

    /// Slot 4: the source text.
    pub fn source_text(&self) -> &[u8] {
        &self.source
    }

    /// Slot 15: store the byte at 0x29.
    pub fn set_slot15_byte(&mut self, v: u8) {
        self.slot15_byte = v;
    }

    /// QueryInterface (§1.5).
    pub fn query_interface(iid: u32) -> Option<Base> {
        match iid {
            IID_IGZUNKNOWN | IID_HTML_DOCUMENT => Some(Base::Primary),
            IID_MESSAGE_TARGET => Some(Base::Secondary),
            _ => None,
        }
    }

    /// Primary and secondary QueryInterface share this operation. A miss preserves `out`.
    pub fn query_interface_into(&mut self, iid: u32, out: &mut Option<Base>) -> bool {
        if let Some(base) = Self::query_interface(iid) {
            self.host.retain_document();
            *out = Some(base);
            true
        } else {
            false
        }
    }

    /// §7.11: compute the 32-bit address before deciding whether it is a table entry.
    /// Opaque layout words remain foreign; this core never dereferences arbitrary memory.
    pub fn font_size_word(&self, index: i32) -> i32 {
        let offset = 0x1a8u32.wrapping_add((index as u32).wrapping_mul(4));
        let word = match offset {
            0x1a8..=0x1c0 => return self.font_sizes[((offset - 0x1a8) / 4) as usize],
            // HD7: an out-of-object read is undefined by the original. Provisional zero.
            0x1c4..=u32::MAX => 0,
            0x18 => u32::from(self.initialised),
            0x20 => 1,
            0x28 => u32::from(self.pushback) | (u32::from(self.slot15_byte) << 8) | (u32::from(self.has_blink) << 16),
            0x54 => self.proportional_size as u32,
            0x6c => self.fixed_size as u32,
            0xe8 => self.token,
            0x124 => self.attrs.len() as u32,
            0x128 => self.ol_serial,
            0x12c => self.ul_serial,
            0x144 => self.style.selector as u32,
            0x148 => self.style.size as u32,
            0x14c => self.style.color,
            0x150 => self.style.flags,
            0x154 => u32::from(self.style.pre) | (u32::from(self.style.link) << 8),
            0x158 => self.align,
            0x194 => self.pos as u32,
            0x1a4 if !self.image_loader => 0,
            _ => self.host.document_layout_word(offset),
        };
        word as i32
    }

    /// Shutdown (§1.6); also run by the destructor when initialised.
    pub fn shutdown(&mut self) -> bool {
        if !self.initialised {
            return true;
        }
        self.initialised = false;
        if let Some(r) = self.root.take() {
            self.delete(r);
        }
        self.base_url = None;
        self.metas.clear();
        self.links.clear();
        let downloads = std::mem::take(&mut self.downloads);
        for (url, e) in downloads {
            match e.image {
                None => self.host.withdraw_download(&url, e.request),
                Some(img) => self.host.release_image(img),
            }
        }
        self.image_loader = false;
        self.has_extra = false;
        self.host.release_interfaces();
        true
    }

    /// FindNamedAnchor (slot 13): first live anchor whose name test passes.
    pub fn find_named_anchor(&mut self, name: &[u8]) -> Option<NodeId> {
        for i in 0..self.named_anchors.len() {
            let id = self.named_anchors[i];
            if !self.is_live(id) {
                continue;
            }
            let an = match &self.nodes[id].kind {
                Kind::Anchor(a) => a.name.clone().unwrap_or_default(),
                _ => continue,
            };
            if self.host.anchor_name_matches(&an, name) {
                return Some(id);
            }
        }
        None
    }

    // ---- arena -------------------------------------------------------------------------

    pub(crate) fn create(&mut self, kind: Kind) -> NodeId {
        self.nodes.push(Node {
            kind,
            children: Vec::new(),
            parent: None,
            deleted: false,
        });
        self.nodes.len() - 1
    }

    /// Parent vtable +4: add-child; ownership passes to the parent.
    pub(crate) fn add(&mut self, parent: NodeId, child: NodeId) {
        self.nodes[child].parent = Some(parent);
        self.nodes[parent].children.push(child);
    }

    /// Element vtable 0 with flag 1.
    pub(crate) fn delete(&mut self, id: NodeId) {
        self.nodes[id].deleted = true;
    }

    pub(crate) fn has_children(&self, id: NodeId) -> bool {
        !self.nodes[id].children.is_empty()
    }

    /// Not deleted, and no ancestor deleted.
    pub fn is_live(&self, mut id: NodeId) -> bool {
        loop {
            if self.nodes[id].deleted {
                return false;
            }
            match self.nodes[id].parent {
                Some(p) => id = p,
                None => return true,
            }
        }
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        &self.nodes[id].children
    }

    pub fn kind(&self, id: NodeId) -> &Kind {
        &self.nodes[id].kind
    }

    pub fn body(&self) -> Option<&Body> {
        match self.root.map(|r| &self.nodes[r].kind) {
            Some(Kind::Body(b)) => Some(b),
            _ => None,
        }
    }

    pub(crate) fn body_mut(&mut self) -> Option<&mut Body> {
        let r = self.root?;
        match &mut self.nodes[r].kind {
            Kind::Body(b) => Some(b),
            _ => None,
        }
    }

    pub(crate) fn new_body(&mut self) -> NodeId {
        let b = Body {
            link: self.default_link,
            vlink: self.default_vlink,
            alink: self.default_alink,
            ..Body::default()
        };
        self.create(Kind::Body(b))
    }

    // ---- input and tokens (§2) -----------------------------------------------------------

    /// ParseCDATA (§2.4).
    pub fn parse_cdata(&mut self) -> bool {
        self.pending.clear();
        let len = self.source.len();
        while self.pos < len {
            match find_character(&self.host, &self.source, b"<", self.pos) {
                None => {
                    self.pending.extend_from_slice(&self.source[self.pos..]);
                    self.pos = len;
                    break;
                }
                Some(i) if i > self.pos => {
                    self.pending.extend_from_slice(&self.source[self.pos..i]);
                    self.pos = i;
                }
                Some(_) => {
                    if len - self.pos >= 4 && &self.source[self.pos..self.pos + 4] == b"<!--" {
                        self.pos += 4;
                        self.pos = find_character(&self.host, &self.source, b"-->", self.pos).map_or(len, |j| j + 3);
                    } else {
                        break;
                    }
                }
            }
        }
        !self.pending.is_empty()
    }

    /// GetToken(skipText) (§2.5).
    pub fn get_token(&mut self, skip_text: bool) {
        if self.pushback {
            self.pushback = false;
            if !(skip_text && self.token == tok::TEXT) {
                return;
            }
        }
        self.token = tok::UNKNOWN;
        if self.pos >= self.source.len() {
            self.token = tok::EOF;
            return;
        }
        loop {
            let found = self.parse_cdata();
            if found && !skip_text {
                self.token = tok::TEXT;
                return;
            }
            if !found && self.pos >= self.source.len() {
                self.token = tok::EOF;
                return;
            }
            if self.pos < self.source.len() && self.source[self.pos] == b'<' {
                break;
            }
        }
        self.pos += 1;
        let start = self.pos;
        let mut i = start;
        while i < self.source.len() && self.source[i] != b'>' {
            i = self.host.advance(&self.source, i);
        }
        if i >= self.source.len() {
            return;
        }
        self.tag_text = self.source[start..i].to_vec();
        self.pos = i + 1;
        self.split_tag();
        self.token = tag_id(&self.tag_text).unwrap_or(tok::UNKNOWN);
        self.parse_attribute_list();
    }

    /// SkipUntilToken(id): true on a match, false when the end is reached first.
    pub fn skip_until_token(&mut self, id: u32) -> bool {
        loop {
            self.get_token(false);
            if self.token == id {
                return true;
            }
            if self.pos >= self.source.len() {
                return false;
            }
        }
    }

    /// SkipDocumentWhitespace.
    pub fn skip_document_whitespace(&mut self) {
        while self.pos < self.source.len() && self.host.is_space(&self.source, self.pos) {
            self.pos = self.host.advance(&self.source, self.pos);
        }
    }

    /// StripWhiteSpace (§2.6).
    pub fn strip_white_space(&self, s: &mut Vec<u8>) {
        let mut i = 0;
        while i < s.len() && self.host.is_space(s, i) {
            i = self.host.advance(s, i);
        }
        s.drain(..i.min(s.len()));
        let mut end = 0;
        let mut i = 0;
        while i < s.len() {
            let next = self.host.advance(s, i).min(s.len());
            if !self.host.is_space(s, i) {
                end = next;
            }
            i = next;
        }
        s.truncate(end);
    }

    /// SplitTag (§2.6).
    pub fn split_tag(&mut self) {
        let mut tag = std::mem::take(&mut self.tag_text);
        let mut i = 0;
        let mut ws = None;
        while i < tag.len() {
            if self.host.is_space(&tag, i) {
                ws = Some(i);
                break;
            }
            i = self.host.advance(&tag, i);
        }
        match ws {
            Some(k) => {
                self.attr_text = tag[self.host.advance(&tag, k)..].to_vec();
                tag.truncate(k);
            }
            None => self.attr_text.clear(),
        }
        self.strip_white_space(&mut tag);
        tag.make_ascii_uppercase();
        let mut at = std::mem::take(&mut self.attr_text);
        self.strip_white_space(&mut at);
        self.attr_text = at;
        self.tag_text = tag;
    }

    /// ParseAttributeList (§3).
    pub fn parse_attribute_list(&mut self) {
        if self.token == tok::DOCTYPE {
            return;
        }
        self.attrs.clear();
        let mut t = std::mem::take(&mut self.attr_text);
        self.strip_white_space(&mut t);
        let n = t.len();
        let mut i = 0;
        while i < n {
            if self.host.is_space(&t, i) {
                i = self.host.advance(&t, i);
                continue;
            }
            let ns = i;
            i = self.host.advance(&t, i);
            while i < n && (t[i].is_ascii_alphabetic() || t[i] == b'-' || t[i] == b'/') {
                i = self.host.advance(&t, i);
            }
            let mut name = t[ns..i].to_vec();
            let mut j = i;
            while j < n && self.host.is_space(&t, j) {
                j = self.host.advance(&t, j);
            }
            let value;
            if j < n && t[j] == b'=' {
                j += 1;
                if j < n && (t[j] == b'"' || t[j] == b'\'') {
                    let vs = j + 1;
                    let ve = find_character(&self.host, &t, &t[j..j + 1], vs).unwrap_or(n);
                    value = t[vs..ve].to_vec();
                    i = if ve < n { ve + 1 } else { n };
                } else {
                    let vs = j;
                    let mut k = vs;
                    if k < n {
                        k = self.host.advance(&t, k);
                    }
                    while k < n && !self.host.is_space(&t, k) {
                        k = self.host.advance(&t, k);
                    }
                    value = t[vs..k].to_vec();
                    i = k;
                }
            } else {
                value = Vec::new();
                i = j;
            }
            // Quote clean-up, name only (step 6): delete index 1, then the last character.
            if !name.is_empty() && name[0] == b'"' && name[name.len() - 1] == b'"' {
                if name.len() > 1 {
                    name.remove(1);
                }
                name.pop();
            }
            name.make_ascii_uppercase();
            if !name.is_empty() && !self.attrs.iter().any(|(k, _)| *k == name) {
                self.attrs.push((name, value));
            }
        }
        // Step 8: the attribute text is cleared.
    }

    /// FindAttribute(name, out, remove) (§3).
    pub fn find_attribute(&mut self, name: &str, remove: bool) -> Option<Vec<u8>> {
        let i = self.attrs.iter().position(|(k, _)| k.as_slice() == name.as_bytes())?;
        if remove {
            Some(self.attrs.remove(i).1)
        } else {
            Some(self.attrs[i].1.clone())
        }
    }

    /// DumpUnhandledAttributes: a no-op (§3).
    pub fn dump_unhandled_attributes(&self, _a: &str, _b: &str) {}

    // ---- style and alignment (§8) ----------------------------------------------------------

    pub fn push_style(&mut self) {
        self.style_stack.push(self.style.clone());
    }

    pub fn pop_style(&mut self) {
        if let Some(s) = self.style_stack.pop() {
            self.style = s;
        }
    }

    pub(crate) fn push_align(&mut self, a: u32) {
        self.align_stack.push(self.align);
        self.align = a;
    }

    pub(crate) fn pop_align(&mut self) {
        if let Some(a) = self.align_stack.pop() {
            self.align = a;
        }
    }

    /// InsertTextSegment (§8.2).
    pub fn insert_text_segment(&mut self, text: &[u8], parent: NodeId) {
        let seg = TextSegment {
            text: text.to_vec(),
            style: self.style.clone(),
            align: self.align,
        };
        let id = self.create(Kind::Text(seg));
        self.add(parent, id);
    }

    // ---- downloads (§7.14 slot 16, §9) -----------------------------------------------------

    /// Primary slot 16: request the image at `url` for `waiter`.
    pub fn request_image(&mut self, waiter: Option<NodeId>, url: &[u8]) {
        match self.downloads.get_mut(url) {
            None => {
                self.downloads.insert(
                    url.to_vec(),
                    Download {
                        waiters: vec![waiter],
                        ..Download::default()
                    },
                );
                let r = self.host.request_download(url);
                if let Some(e) = self.downloads.get_mut(url) {
                    e.request = r;
                }
            }
            Some(e) => match e.image {
                None => e.waiters.push(waiter),
                Some(img) => {
                    if self.host.deliver_image(waiter, img) {
                        self.set_slot15_byte(0);
                    }
                }
            },
        }
    }

    /// DoMessage (secondary slot 3): true for the download message type.
    pub fn do_message(&mut self, msg_type: u32, status: u32, record: &DownloadRecord) -> bool {
        if msg_type != MSG_RESOURCE_DOWNLOADED {
            return false;
        }
        if status != 0 {
            self.resource_has_been_downloaded(record);
        }
        true
    }

    /// LoadFromImageFile (§9): None leaves `out` untouched.
    pub fn load_from_image_file(&mut self, path: &[u8], out: &mut Option<ImageHandle>) {
        if !self.image_loader {
            if !self.host.create_image_loader() {
                return;
            }
            self.image_loader = true;
        }
        if let Some(img) = self.host.load_image(path) {
            *out = Some(img);
        }
    }

    /// ResourceHasBeenDownloaded (§9).
    pub fn resource_has_been_downloaded(&mut self, record: &DownloadRecord) {
        let mut path = record.data.clone();
        if !self.downloads.contains_key(&record.url) {
            return;
        }
        if !record.is_path {
            let data = std::mem::take(&mut path);
            path = self.host.make_temp_path();
            // An open failure writes nothing; the steps below still run on that path.
            let _ = self.host.write_file(&path, &data);
        }
        let mut image = self.downloads[&record.url].image;
        self.load_from_image_file(&path, &mut image);
        if !record.is_path {
            self.host.remove_file(&path);
        }
        let entry = self.downloads.get_mut(&record.url).expect("entry");
        entry.image = image;
        if let Some(img) = image {
            let waiters = std::mem::take(&mut entry.waiters);
            for w in waiters {
                self.host.deliver_image(w, img);
            }
        }
    }

    /// CreateBullet(style) (§7.9).
    pub fn create_bullet(&mut self, style: u32) -> Option<NodeId> {
        let d = ((self.style.size as u32) / 2).max(1);
        let color = self.style.color;
        let mut cache = BULLETS.lock().unwrap_or_else(|e| e.into_inner());
        let hit = cache
            .iter()
            .find(|r| r.d == d && r.style == style && r.color == color)
            .and_then(|r| r.surface);
        let surface = match hit {
            Some(s) => s,
            None => {
                let bg = match self.body().and_then(|b| b.bgcolor) {
                    Some(c) => c,
                    None => self.host.rgb(192, 192, 192),
                };
                let circle = if style == 0 { Some((d, color)) } else { None };
                let s = self.host.create_bullet_surface(d + 4, d + 2, bg, circle)?;
                cache.push(BulletRecord {
                    d,
                    style,
                    color,
                    surface: Some(s),
                });
                s
            }
        };
        drop(cache);
        let img = Img {
            align: self.align,
            surface: Some(surface),
            ..Img::default()
        };
        Some(self.create(Kind::Img(img)))
    }

    /// Resolve `value` against the base URL (`cRZURL::ChangeURL` + `GetFullURL`).
    pub(crate) fn resolve(&self, value: &[u8]) -> Option<Vec<u8>> {
        let base = self.base_url.as_ref()?;
        let mut u = crate::rzurl::RzUrl::new();
        u.change_url(value, base);
        Some(u.full_url().to_vec())
    }
}

/// Destructor body (009005ad): Shutdown when the initialised byte is set.
impl<H: HtmlHost> Drop for HtmlDocument<H> {
    fn drop(&mut self) {
        if self.initialised {
            self.shutdown();
        }
    }
}

/// MakeOutputStyle (§8.1): LEFT 0, RIGHT 2, CENTER 1, anything else 0.
pub fn make_output_style(v: &[u8]) -> u32 {
    match v.to_ascii_uppercase().as_slice() {
        b"RIGHT" => 2,
        b"CENTER" => 1,
        _ => 0,
    }
}

/// MakeRelativeToTextAlignmentStyle (§8.1): LEFT 0, RIGHT 1, TOP 2, MIDDLE 3, else 4.
pub fn make_relative_to_text_alignment_style(v: &[u8]) -> u32 {
    match v.to_ascii_uppercase().as_slice() {
        b"LEFT" => 0,
        b"RIGHT" => 1,
        b"TOP" => 2,
        b"MIDDLE" => 3,
        _ => 4,
    }
}

/// `RZConvertToUint32A` (§7.11): detect base before skipping whitespace/sign,
/// then unsigned 32-bit strtoul with saturation followed by wrapping negation.
pub fn convert_to_u32(v: &[u8]) -> u32 {
    let hex = v.starts_with(b"0x")
        || v.starts_with(b"0X")
        || v.iter()
            .skip_while(|c| c.is_ascii_digit())
            .next()
            .is_some_and(|c| matches!(c, b'a'..=b'f' | b'A'..=b'F'));
    let base = if hex { 16 } else { 10 };
    let mut i = 0;
    while i < v.len() && matches!(v[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
        i += 1;
    }
    let negative = v.get(i) == Some(&b'-');
    if matches!(v.get(i), Some(b'+' | b'-')) {
        i += 1;
    }
    if hex && v.get(i) == Some(&b'0') && matches!(v.get(i + 1), Some(b'x' | b'X')) {
        i += 2;
    }
    let mut n = 0u32;
    while let Some(digit) = v.get(i).and_then(|c| (*c as char).to_digit(base)) {
        n = n.saturating_mul(base).saturating_add(digit);
        i += 1;
    }
    if negative {
        n.wrapping_neg()
    } else {
        n
    }
}

/// The text-segment whitespace rule driven by the preformatted flag (§5.4 table).
pub fn normalize_segment_text<H: HtmlHost>(host: &H, text: &[u8], pre: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        if pre {
            match text[i] {
                b'\r' => {
                    out.push(b'\n');
                    i += if text.get(i + 1) == Some(&b'\n') { 2 } else { 1 };
                }
                _ => {
                    let next = host.advance(text, i).min(text.len());
                    out.extend_from_slice(&text[i..next]);
                    i = next;
                }
            }
        } else if host.is_space(text, i) {
            out.push(b' ');
            while i < text.len() && host.is_space(text, i) {
                i = host.advance(text, i);
            }
        } else {
            let next = host.advance(text, i).min(text.len());
            out.extend_from_slice(&text[i..next]);
            i = next;
        }
    }
    out
}

/// Search only at host character boundaries (§2.1), so a DBCS trail byte is never a tag.
fn find_character<H: HtmlHost>(host: &H, s: &[u8], pat: &[u8], mut i: usize) -> Option<usize> {
    while i < s.len() {
        if s[i..].starts_with(pat) {
            return Some(i);
        }
        i = host.advance(s, i);
    }
    None
}

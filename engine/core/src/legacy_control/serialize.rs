// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FragmentCoverage {
    CompleteKnownDomain,
    PartialLc8 { interface: u32 },
}
/// The caller receives the string's one reference even for an explicitly partial
/// fragment. `text` contains only published words; coverage is never inferred
/// from a roundtrip or mistaken for byte equivalence to an unknown writer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fragment {
    pub string: Handle,
    pub text: String,
    pub coverage: FragmentCoverage,
}

const ORDER: [Kind; 20] = [
    Kind::FlatRect,
    Kind::Text,
    Kind::TextEdit,
    Kind::ListBox,
    Kind::LineInput,
    Kind::Combo,
    Kind::Btn,
    Kind::Bmp,
    Kind::Scrollbar,
    Kind::Slider,
    Kind::OptGrp,
    Kind::Spinner,
    Kind::Outline,
    Kind::TextTicker,
    Kind::Gen,
    Kind::FileBrowser,
    Kind::TreeView,
    Kind::Folders,
    Kind::Grid,
    Kind::Custom,
];
const FLAGS: [(&str, u32); 13] = [
    ("winflag_visible", 1),
    ("winflag_enabled", 2),
    ("winflag_moveable", 0x100),
    ("winflag_sizeable", 0x200),
    ("winflag_sortable", 0x800),
    ("winflag_pbuff", 0x10000),
    ("winflag_pbufftrans", 0x20000),
    ("winflag_pbufferase", 0x40000),
    ("winflag_pbuffvid", 0x100000),
    ("winflag_alphablend", 4),
    ("winflag_acceptfocus", 0x8000),
    ("winflag_mousetrans", 0x80000),
    ("winflag_ignoremouse", 0x200000),
];
fn rows() -> Vec<(u32, u32, &'static str)> {
    let mut rows: Vec<_> = ORDER
        .into_iter()
        .map(|kind| {
            let table = vocabulary::TABLES
                .iter()
                .find(|t| t.kind == kind)
                .expect("complete Kind table");
            (
                table.class,
                table.iid,
                if kind == Kind::Gen {
                    "GZWinGen"
                } else {
                    table.words[0].0
                },
            )
        })
        .collect();
    rows.push((0xe2ba00ee, WIN_IID, "GZWin"));
    rows
}

impl LegacyControl {
    /// Fully owns the §7.4 allocation/AddRef/output/failure wrapper. The foreign
    /// serializer is NOT called. §7.6 frame/common are implemented here; unknown
    /// successful per-type writer dispatches produce explicit PartialLc8 output.
    pub fn get_script_fragment(&self, out: &mut Option<Fragment>, f: &mut impl Foreign) -> Result<bool> {
        let Some(h) = self.control else {
            return Ok(false);
        };
        let string = f.new_string();
        f.add_ref(string);
        match serialize(h, f) {
            Ok((text, coverage)) => {
                f.append_string(string, &text);
                *out = Some(Fragment { string, text, coverage });
                Ok(true)
            }
            Err(e) => {
                f.release(string);
                Err(e)
            }
        }
    }
}
fn serialize(h: Handle, f: &mut impl Foreign) -> Result<(String, FragmentCoverage)> {
    let rows = rows();
    let mut cls = match f.read(h, Read::Param(0xe85eebde)) {
        Value::U32(v) => v,
        _ => 0,
    };
    let mut iid = match f.read(h, Read::Param(0x08d6e462)) {
        Value::U32(v) => v,
        _ => 0,
    };
    if cls == 0 || iid == 0 {
        // All 21 QIs run, even once the first answering row filled both IDs.
        for &(c, i, _) in &rows {
            if let Some(interface) = f.query(h, i) {
                f.release(interface);
                if cls == 0 {
                    cls = c;
                }
                if iid == 0 {
                    iid = i;
                }
            }
        }
    }
    if cls == 0 {
        cls = 0xe2ba00ee;
    }
    if iid == 0 {
        iid = WIN_IID;
    }
    let class = rows
        .iter()
        .find(|r| r.0 == cls)
        .map(|r| r.2.to_string())
        .unwrap_or_else(|| format!("0x{cls:08x}"));
    let interface = rows
        .iter()
        .find(|r| r.1 == iid)
        .map(|r| format!("I{}", r.2))
        .unwrap_or_else(|| format!("0x{iid:08x}"));
    let mut text = format!("<LEGACY clsid={class} iid={interface} ");
    let id = word(f.read(h, Read::Id))?;
    if id != 0 {
        let _ = write!(text, "id=0x{id:08x} ");
    }
    let Value::Rect([l, t, r, b]) = f.read(h, Read::Area) else {
        return Err(Unsupported::ForeignResult("fragment area"));
    };
    let _ = write!(text, "area=({l},{t},{r},{b}) ");
    let Value::Colour([r, g, b]) = f.read(h, Read::Fill) else {
        return Err(Unsupported::ForeignResult("fragment fill"));
    };
    // Q9 expressly allows byte values, not native uninitialised upper 24 bits.
    let _ = write!(text, "fillcolor=({r},{g},{b}) ");
    let Value::Text(caption) = f.read(h, Read::Caption) else {
        return Err(Unsupported::ForeignResult("fragment caption"));
    };
    if !caption.is_empty() {
        let _ = write!(text, "caption=\"{caption}\" ");
    }
    if let Value::Key(key) = f.read(h, Read::Param(0x0b31771a)) {
        if key.group != 0 || key.instance != 0 {
            let _ = write!(text, "captionres={{{:08x},{:08x}}} ", key.group, key.instance);
        }
    }
    if let Value::Comments(comments) = f.read(h, Read::Param(0xbaca6879)) {
        // Comments is the typed result only after variant type &0x10/list QI
        // succeed; foreign parameter access must preserve this type distinction.
        for (word, s) in [
            "comments",
            "commentslmd",
            "commentsrmd",
            "commentslmu",
            "commentsrmu",
            "commentsmm",
            "commentsku",
            "commentskd",
        ]
        .into_iter()
        .zip(comments)
        {
            if !s.is_empty() {
                let _ = write!(text, "{word}=\"{s}\" ");
            }
        }
    }
    for (name, mask) in FLAGS {
        let flag = word(f.read(h, Read::Flag(mask)))?;
        let _ = write!(text, "{name}={} ", if flag != 0 { "yes" } else { "no" });
    }
    let mut coverage = FragmentCoverage::CompleteKnownDomain;
    if ORDER.into_iter().any(|k| k != Kind::Custom && k.iid() == iid) {
        if let Some(interface) = f.query(h, iid) {
            f.release(interface);
            coverage = FragmentCoverage::PartialLc8 { interface: iid };
        }
    }
    text.push_str(">\n");
    Ok((text, coverage))
}

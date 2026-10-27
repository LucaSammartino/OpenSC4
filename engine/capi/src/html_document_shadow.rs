// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link, same-invocation cHTMLDocument observation transport.
use fc4_core::html_document::{self, HtmlDocument, HtmlHost, ImageHandle};

#[repr(C)]
#[derive(Clone, Copy)]
struct Bytes {
    data: *const u8,
    len: u32,
}

impl Bytes {
    unsafe fn value(&self) -> &[u8] {
        if self.len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.data, self.len as usize) }
        }
    }
}

#[repr(C)]
struct TextState {
    source: Bytes,
    pending: Bytes,
    tag: Bytes,
    attributes: Bytes,
    position: u32,
}

type Character = unsafe extern "C" fn(u32, *const u8, u32, u32) -> u32;
struct Host(Option<Character>);
impl HtmlHost for Host {
    fn document_layout_word(&self, _: u32) -> u32 {
        0
    }
    fn is_space(&self, text: &[u8], i: usize) -> bool {
        self.0.map_or_else(
            || matches!(text.get(i), Some(b' ' | b'\t' | b'\n' | b'\r' | 11 | 12)),
            |f| unsafe { f(0, text.as_ptr(), text.len() as u32, i as u32) != 0 },
        )
    }
    fn advance(&self, text: &[u8], i: usize) -> usize {
        self.0.map_or(i + 1, |f| unsafe {
            f(1, text.as_ptr(), text.len() as u32, i as u32) as usize
        })
    }
    // These services are never used by the observed methods. Constructor colour
    // defaults are immaterial: none of the methods reads them or initializes COM.
    fn rgb(&mut self, r: i32, g: i32, b: i32) -> u32 {
        ((r << 16) | (g << 8) | b) as u32
    }
    fn set_color(&mut self, _: &[u8], _: &mut u32) {}
    fn convert_color(&mut self, color: u32) -> u32 {
        color
    }
    fn table_bgcolor(&mut self, _: &[u8]) -> u32 {
        0
    }
    fn make_temp_path(&mut self) -> Vec<u8> {
        Vec::new()
    }
    fn write_file(&mut self, _: &[u8], _: &[u8]) -> bool {
        false
    }
    fn remove_file(&mut self, _: &[u8]) {}
    fn create_bullet_surface(&mut self, _: u32, _: u32, _: u32, _: Option<(u32, u32)>) -> Option<ImageHandle> {
        None
    }
}

// Returns a field mismatch mask. Inputs are BEFORE state only; native AFTER
// state is used exclusively on the comparison side, never to seed the core.
// Safety: plugin supplies live disjoint immutable snapshots and a synchronous
// character callback reusing the game's shared HTML helpers (spec section 2.1).
#[no_mangle]
unsafe extern "C" fn fc4_html_document_text_check(
    operation: u32,
    before: *const TextState,
    after: *const TextState,
    native_value: u32,
    character: Option<Character>,
) -> u32 {
    if before.is_null() || after.is_null() {
        return u32::MAX;
    }
    let (b, a) = unsafe { (&*before, &*after) };
    let mut doc = HtmlDocument::new(Host(character));
    doc.source = unsafe { b.source.value() }.to_vec();
    doc.pending = unsafe { b.pending.value() }.to_vec();
    doc.tag_text = unsafe { b.tag.value() }.to_vec();
    doc.attr_text = unsafe { b.attributes.value() }.to_vec();
    doc.pos = b.position as usize;
    let result = match operation {
        0 => Some(u32::from(doc.parse_cdata())),
        1 => {
            doc.skip_document_whitespace();
            None
        }
        2 => {
            let mut text = std::mem::take(&mut doc.tag_text);
            doc.strip_white_space(&mut text);
            doc.tag_text = text;
            None
        }
        3 => {
            doc.split_tag();
            None
        }
        4 | 5 => {
            let value = if operation == 4 {
                html_document::make_output_style(&doc.tag_text)
            } else {
                html_document::make_relative_to_text_alignment_style(&doc.tag_text)
            };
            // The original uppercases its reference argument as well as returning
            // the code. The core helper's value API leaves this adapter that write.
            doc.tag_text.make_ascii_uppercase();
            Some(value)
        }
        6 => {
            return u32::from(doc.source_text() != unsafe { a.source.value() });
        }
        _ => return u32::MAX,
    };
    let mut bad = 0;
    for (i, (ours, native)) in [
        (doc.source.as_slice(), unsafe { a.source.value() }),
        (doc.pending.as_slice(), unsafe { a.pending.value() }),
        (doc.tag_text.as_slice(), unsafe { a.tag.value() }),
        (doc.attr_text.as_slice(), unsafe { a.attributes.value() }),
    ]
    .into_iter()
    .enumerate()
    {
        if ours != native {
            bad |= 1 << i;
        }
    }
    if doc.pos != a.position as usize {
        bad |= 16;
    }
    if result.is_some_and(|v| v != native_value) {
        bad |= 32;
    }
    bad
}

// Words: seven table entries, proportional size, fixed size, slot-15 byte.
// op 0 setter, 1 getter, 2 slot-15 setter. All out[count] words are checked.
// Safety: before/after contain ten words, values/out contain count words.
#[no_mangle]
unsafe extern "C" fn fc4_html_document_scalar_check(
    op: u32,
    before: *const i32,
    after: *const i32,
    values: *const i32,
    count: u32,
    output: *const i32,
    native_value: u32,
) -> u32 {
    if before.is_null() || after.is_null() {
        return u32::MAX;
    }
    let b = unsafe { std::slice::from_raw_parts(before, 10) };
    let a = unsafe { std::slice::from_raw_parts(after, 10) };
    let mut doc = HtmlDocument::new(Host(None));
    doc.font_sizes.copy_from_slice(&b[..7]);
    doc.proportional_size = b[7];
    doc.fixed_size = b[8];
    doc.slot15_byte = b[9] as u8;
    let mut bad = 0;
    match op {
        0 => {
            let n = count.min(7) as usize;
            if n > 0 && values.is_null() {
                return u32::MAX;
            }
            let sizes = if n == 0 {
                &[]
            } else {
                unsafe { std::slice::from_raw_parts(values, n) }
            };
            doc.set_font_size_table(sizes);
        }
        1 => {
            if count > 0 && output.is_null() {
                return u32::MAX;
            }
            let out = if count == 0 {
                &[]
            } else {
                unsafe { std::slice::from_raw_parts(output, count as usize) }
            };
            let mut expected = vec![0x5a5a5a5a; count as usize];
            let n = doc.get_font_size_table(&mut expected);
            bad |= u32::from(out != expected || native_value != n as u32);
        }
        2 => doc.set_slot15_byte(count as u8),
        _ => return u32::MAX,
    }
    bad |= u32::from(
        doc.font_sizes != a[..7]
            || doc.proportional_size != a[7]
            || doc.fixed_size != a[8]
            || i32::from(doc.slot15_byte) != a[9],
    ) << 1;
    bad
}

#[cfg(test)]
mod tests;

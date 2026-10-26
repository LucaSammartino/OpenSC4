// SPDX-License-Identifier: GPL-3.0-or-later
//! win-filebrowser §§8.7.1–8.7.2 and A.1. All returned handles are borrowed.
use super::*;

const STYLES: [u32; 4] = [0x238, 0x25c, 0x280, 0x2a4];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DriveRecord {
    pub name: Vec<u8>,
    pub kind: u32,
    pub icon: Option<Handle>,
    pub depth: u32,
}

pub trait PropertyIo: IconIo {
    fn manager(&mut self, window: Handle) -> Handle;
    fn drives(&mut self) -> Vec<(Vec<u8>, u32)>;
    fn editor_text(&mut self, editor: Handle) -> Vec<u8>;
    fn remove_directory(&mut self, path: &[u8], option: u32) -> bool;
    fn combo_label(&mut self, combo: Handle, text: &[u8], index: u32);
    fn compare_name(&mut self, input: &[u8], stored: &[u8], option: u32) -> bool;
    fn unpack_rgb(&mut self, color: u32) -> [u8; 3];
    fn window_flag(&mut self, window: Handle, flag: u32) -> bool;
    fn complete_modal(&mut self, manager: Handle, id: u32, window: Handle, argument: u32) -> u32;
    fn complete_window(&mut self, window: Handle, target: u32, event: u32, sender: Handle, zero: [u32; 2]) -> u32;
    fn base_send(&mut self, window: Handle, message: Message, argument: u32) -> bool;
}

impl FileBrowser {
    pub fn init_property(&mut self, slot: u32, args: &[u32], f: &mut impl PropertyIo) -> Result<()> {
        match slot {
            93 => {
                self.initialize_drives(f);
            }
            94 => self.initialize_fallback(),
            40 => {
                self.set_column_order(args, f);
            }
            61 | 63 | 65 => {
                self.set_dimension(
                    match slot {
                        61 => 0x1e4,
                        63 => 0x1fc,
                        _ => 0x224,
                    },
                    args[0],
                    f,
                );
            }
            67 | 69 => {
                self.set_optional_word(if slot == 67 { 0x300 } else { 0x304 }, args[0] as u8, args[1], f);
            }
            71 => {
                self.set_style_colors(args[0], args[1], f);
            }
            73 => {
                for (offset, value) in [0x164, 0x168, 0x16c].into_iter().zip(args) {
                    self.store_word(offset, *value);
                }
                self.mark_dirty(f);
            }
            79 => {
                self.set_outline(args[0], f);
            }
            _ => return Err(Unsupported::Precondition("unsupported Init property slot")),
        }
        Ok(())
    }
    /// Indexed style operations address words, without a six-word range check.
    pub fn word(&self, offset: u32) -> u32 {
        for (style, base) in STYLES.iter().enumerate() {
            if offset == *base {
                return self.style_fonts[style].unwrap_or(0) as u32;
            }
            if offset >= base + 4 && offset <= base + 32 && offset % 4 == 0 {
                return self.style_words[style][((offset - base - 4) / 4) as usize];
            }
        }
        self.raw_words.get(&offset).copied().unwrap_or(0)
    }

    pub fn store_word(&mut self, offset: u32, value: u32) {
        for (style, base) in STYLES.iter().enumerate() {
            if offset == *base {
                self.style_fonts[style] = (value != 0).then_some(u64::from(value));
                return;
            }
            if offset >= base + 4 && offset <= base + 32 && offset % 4 == 0 {
                self.style_words[style][((offset - base - 4) / 4) as usize] = value;
                return;
            }
        }
        self.raw_words.insert(offset, value);
    }

    pub fn style_word(&self, style: usize, index: u32) -> u32 {
        self.word(STYLES[style].wrapping_add(4).wrapping_add(index.wrapping_mul(4)))
    }

    pub fn set_style_word(&mut self, style: usize, index: u32, value: u32, f: &mut impl Foreign) -> bool {
        self.store_word(STYLES[style].wrapping_add(4).wrapping_add(index.wrapping_mul(4)), value);
        self.mark_dirty(f);
        true
    }

    pub fn style_byte(&self, style: usize) -> u32 {
        ((self.style_words[style][6] >> 8) as u8 as i8 as i32) as u32
    }

    pub fn set_style_byte(&mut self, style: usize, value: u32, f: &mut impl Foreign) -> bool {
        let word = &mut self.style_words[style][6];
        *word = (*word & !0xff00) | ((value & 0xff) << 8);
        self.mark_dirty(f);
        true
    }

    pub fn set_style_font(&mut self, style: usize, font: Option<Handle>, f: &mut impl Foreign) -> bool {
        if self.style_fonts[style] != font {
            replace_ref(&mut self.style_fonts[style], font, f);
            self.mark_dirty(f);
        }
        true
    }

    pub fn set_common_font(&mut self, font: Option<Handle>, f: &mut impl Foreign) -> bool {
        self.common_font = font;
        self.mark_dirty(f);
        true
    }

    pub fn set_dimension(&mut self, offset: u32, value: u32, f: &mut impl Foreign) -> bool {
        self.store_word(offset, value);
        self.mark_dirty(f);
        true
    }

    pub fn optional_word(&self, offset: u32, value: &mut u32, enabled: &mut u8) -> bool {
        *value = self.word(offset);
        *enabled = u8::from(*value != 0x4000_0000);
        true
    }

    pub fn set_optional_word(&mut self, offset: u32, enabled: u8, value: u32, f: &mut impl Foreign) -> bool {
        self.set_dimension(offset, if enabled == 0 { 0x4000_0000 } else { value }, f)
    }

    pub fn set_style_colors(&mut self, first: u32, second: u32, f: &mut impl Foreign) -> bool {
        for style in &mut self.style_words {
            style[4] = first;
            style[5] = second;
        }
        self.mark_dirty(f);
        true
    }

    pub fn style_colors(&self) -> [u32; 2] {
        [self.style_words[0][4], self.style_words[0][5]]
    }

    pub fn set_column_order(&mut self, ids: &[u32], f: &mut impl Foreign) -> bool {
        for id in 0..6 {
            self.raw_bytes.insert(0x1dc + 12 * id, 0);
            self.store_word(0x1e0 + 12 * id, u32::MAX);
        }
        for (position, &id) in ids.iter().enumerate() {
            if id < 6 {
                self.raw_bytes.insert(0x1dc + 12 * id, 1);
                self.store_word(0x1e0 + 12 * id, position as u32);
            }
        }
        self.store_word(0x228, ids.len() as u32);
        let valid = !ids.is_empty() && self.raw_bytes[&0x1e8] != 0;
        if !valid {
            for id in 0..2 {
                self.raw_bytes.insert(0x1dc + 12 * id, 1);
                self.store_word(0x1e0 + 12 * id, id);
            }
            self.store_word(0x228, 2);
        }
        self.mark_dirty(f);
        valid
    }

    pub fn column_order(&self, capacity: u32) -> Result<Vec<u32>> {
        let count = capacity.min(self.word(0x228));
        let mut output = Vec::new();
        for id in 0..6 {
            if output.len() as u32 == count {
                return Ok(output);
            }
            if self.word(0x1e0 + 12 * id) == output.len() as u32 {
                output.push(id);
            }
        }
        if output.len() as u32 == count {
            Ok(output)
        } else {
            Err(Unsupported::Precondition(
                "column-order getter scans outside six records",
            ))
        }
    }

    pub fn three_columns(&self) -> Result<[u32; 3]> {
        let mut result = [5; 3];
        for (slot, id) in result.iter_mut().zip(self.column_order(3)?) {
            *slot = id;
        }
        Ok(result)
    }

    pub fn set_max_rows(&mut self, value: u32) -> bool {
        self.store_word(0x230, value.max(4));
        true
    }

    pub fn set_parent_label(&mut self, text: &[u8], f: &mut impl Foreign) -> bool {
        self.parent_label = text.to_vec();
        self.mark_dirty(f);
        true
    }

    pub fn set_outline(&mut self, value: u32, f: &mut impl Foreign) -> bool {
        if self.raw_bytes[&0x308] != value as u8 {
            self.raw_bytes.insert(0x308, value as u8);
            self.mark_dirty(f);
        }
        true
    }

    pub fn set_background_color(&mut self, color: u32, f: &mut impl PropertyIo) -> bool {
        let [r, g, b] = f.unpack_rgb(color);
        self.store_word(
            0x30c,
            0xff00_0000 | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b),
        );
        self.raw_bytes.insert(0x309, 1);
        f.invalidate(self.window());
        true
    }

    pub fn background_color(&self, f: &mut impl IconIo) -> u32 {
        let red = (self.word(0x30c) >> 16) as u8;
        f.convert_rgb(self.window(), [red; 3])
    }

    pub fn set_background_buffer(&mut self, buffer: Option<Handle>, f: &mut impl Foreign) -> bool {
        if self.extra_reference != buffer {
            replace_ref(&mut self.extra_reference, buffer, f);
            self.raw_bytes.insert(0x309, 1);
            f.invalidate(self.window());
        }
        true
    }

    pub fn background_disabled(&self) -> bool {
        self.raw_bytes[&0x309] == 0
    }

    pub fn disable_background(&mut self, value: u8, f: &mut impl BrowserForeign) {
        self.raw_bytes.insert(0x309, u8::from(value == 0));
        if value != 0 {
            for flag in [0x20000, 0x40000] {
                f.set_window_flag(self.window(), flag, true);
            }
        }
    }

    pub fn initialize_drives(&mut self, f: &mut impl PropertyIo) -> bool {
        for (name, kind) in f.drives() {
            let (kind, icon) = match kind {
                1 => (0, 3),
                2 => (1, 6),
                3 => (2, 5),
                5 => (3, 4),
                _ => (4, 3),
            };
            if name.len() >= 2 {
                self.drive_records.push(DriveRecord {
                    name: name[..2].to_vec(),
                    kind,
                    icon: self.icons[icon],
                    depth: 0,
                });
            }
            // Native short-name allocation is leaked; no retained record is appended.
        }
        true
    }

    pub fn initialize_fallback(&mut self) {
        self.fallback = self
            .drive_records
            .iter()
            .find(|r| r.kind == 0 && r.name.len() >= 2)
            .map_or_else(|| b"c:\\".to_vec(), |r| [&r.name[..2], b"\\"].concat());
    }

    pub fn validate_current(&mut self, path: &[u8], f: &mut impl BrowserForeign) -> bool {
        let valid = f.directory_exists(path);
        self.current = if valid { path.to_vec() } else { self.fallback.clone() };
        valid
    }

    pub fn set_directory_property(&mut self, path: &[u8], f: &mut impl BrowserForeign) -> bool {
        self.validate_current(path, f);
        self.mark_dirty(f);
        true
    }

    pub fn edited_descriptor(&mut self, f: &mut impl PropertyIo) -> SelectionDescriptor {
        let mut result = self.descriptor();
        if let Some(editor) = self.children[6] {
            let text = f.editor_text(editor);
            if self.selection.kind == 2 || !text.is_empty() {
                self.selection.leaf = text;
                self.selection.full_path = [&self.selection.directory[..], &self.selection.leaf[..]].concat();
                if self.selection.kind != 2 {
                    result.kind = 2;
                }
            }
        }
        result
    }

    fn group_index(&self, index: u32) -> usize {
        if index == u32::MAX {
            self.filter_index as usize
        } else {
            index as usize
        }
    }

    fn grow_groups(&mut self, index: usize) {
        self.custom_groups
            .resize_with(self.custom_groups.len().max(index + 1), Vec::new);
        self.filter_labels
            .resize_with(self.filter_labels.len().max(index + 1), Vec::new);
        // SPEC-GAP: parsed pattern storage is host-supplied; new filters start empty.
        self.filter_patterns.resize_with(self.filter_labels.len(), Vec::new);
    }

    pub fn custom_entry(
        &self,
        entry: u32,
        group: u32,
        name: &mut Vec<u8>,
        buffer: &mut Option<Handle>,
        id: &mut u32,
    ) -> bool {
        let Some(record) = self
            .custom_groups
            .get(self.group_index(group))
            .and_then(|g| g.get(entry as usize))
        else {
            return false;
        };
        name.clone_from(&record.text);
        *buffer = record.interface;
        *id = record.id;
        true
    }

    pub fn add_custom_entry(
        &mut self,
        name: &[u8],
        buffer: Option<Handle>,
        id: u32,
        group: u32,
        f: &mut impl Foreign,
    ) -> bool {
        let index = self.group_index(group);
        self.grow_groups(index);
        if let Some(buffer) = buffer {
            f.add_ref(buffer);
        }
        self.custom_groups[index].push(CustomEntry {
            id,
            interface: buffer,
            text: name.to_vec(),
        });
        self.mark_dirty(f);
        true
    }

    pub fn set_filter_label(&mut self, text: &[u8], group: u32, f: &mut impl PropertyIo) -> bool {
        let index = self.group_index(group);
        self.grow_groups(index);
        self.filter_labels[index] = text.to_vec();
        if let Some(combo) = self.children[7].or(self.filter_combo) {
            f.combo_label(combo, text, index as u32);
        }
        true
    }

    fn remove_custom(&mut self, group: usize, position: Option<usize>, f: &mut impl Foreign) -> bool {
        let Some(position) = position else {
            return false;
        };
        let record = self.custom_groups[group].remove(position);
        if let Some(buffer) = record.interface {
            f.release(buffer);
        }
        self.mark_dirty(f);
        true
    }

    pub fn remove_custom_id(&mut self, id: u32, group: u32, f: &mut impl Foreign) -> bool {
        let group = self.group_index(group);
        let position = self
            .custom_groups
            .get(group)
            .and_then(|g| g.iter().position(|r| r.id == id));
        self.remove_custom(group, position, f)
    }

    pub fn remove_custom_name(&mut self, name: &[u8], group: u32, f: &mut impl PropertyIo) -> bool {
        let group = self.group_index(group);
        let position = self
            .custom_groups
            .get(group)
            .and_then(|g| g.iter().position(|r| f.compare_name(name, &r.text, 1)));
        self.remove_custom(group, position, f)
    }

    pub fn complete(&self, argument: u32, f: &mut impl PropertyIo) -> u32 {
        if f.window_flag(self.window(), 0x1000) {
            let manager = f.manager(self.window());
            f.complete_modal(manager, 15, self.window(), argument)
        } else {
            f.complete_window(self.window(), self.word(0x4c), 1, self.window(), [0, 0])
        }
    }

    /// This handler belongs only to the two older secondary tables (§A.1).
    pub fn alternate_send(
        &mut self,
        message: Message,
        argument: u32,
        descriptor_type: u32,
        f: &mut impl PropertyIo,
    ) -> bool {
        match (message.kind, message.words[1] as u32) {
            (3, 0xbaa5_5d58) => {
                if self.edited_descriptor(f).kind == 2 {
                    self.complete(1, f);
                }
                true
            }
            (3, 0xbaa5_5d59) => {
                self.complete(0, f);
                true
            }
            _ => {
                if message.kind == 0x7abe_7204 && descriptor_type == 2 {
                    self.complete(1, f);
                }
                f.base_send(self.window(), message, argument)
            }
        }
    }
}

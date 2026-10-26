// SPDX-License-Identifier: GPL-3.0-or-later
use super::{file_browser::*, *};

impl PropertyIo for Recording {
    fn drives(&mut self) -> Vec<(Vec<u8>, u32)> { self.fb_drives.clone() }
    fn editor_text(&mut self, _: Handle) -> Vec<u8> { self.fb_editor.clone() }
    fn remove_directory(&mut self, path: &[u8], option: u32) -> bool {
        assert_eq!(option, 1);
        self.paths.push(path.to_vec());
        self.create_success
    }
    fn combo_label(&mut self, _: Handle, text: &[u8], index: u32) {
        self.fb_ops.push((0x6c, vec![index]));
        self.editor_texts.push(text.to_vec());
    }
    fn compare_name(&mut self, input: &[u8], stored: &[u8], option: u32) -> bool {
        assert_eq!(option, 1);
        input.eq_ignore_ascii_case(stored)
    }
    fn unpack_rgb(&mut self, _: u32) -> [u8; 3] { [18, 52, 86] }
    fn window_flag(&mut self, _: Handle, flag: u32) -> bool { self.base_flags & flag != 0 }
    fn complete_modal(&mut self, _: Handle, id: u32, _: Handle, argument: u32) -> u32 {
        self.fb_ops.push((4, vec![id, argument])); 77
    }
    fn complete_window(&mut self, _: Handle, target: u32, argument: u32) -> u32 {
        self.fb_ops.push((4, vec![target, 1, argument, 0, 0])); 88
    }
    fn base_send(&mut self, _: Handle, message: Message, argument: u32) -> bool {
        self.base_messages.push(message);
        self.fb_ops.push((145, vec![argument]));
        false
    }
}

#[test]
fn fbq4_style_widths_references_optional_words_and_common_font() {
    let mut b = FileBrowser::new(80);
    let mut f = Recording::default();
    b.style_fonts[0] = Some(11);
    assert!(b.set_style_font(0, Some(12), &mut f));
    assert_eq!(f.refs, [(true, 12), (false, 11)]);
    assert!(b.dirty);
    let invalidations = f.invalidations;
    b.dirty = false;
    b.set_style_font(0, Some(12), &mut f);
    assert_eq!(f.refs.len(), 2);
    assert_eq!(f.invalidations, invalidations);
    assert!(!b.dirty);
    b.set_style_byte(0, 0x1234_0080, &mut f);
    assert_eq!(b.style_byte(0), 0xffff_ff80);
    assert_eq!(b.style_word(0, 6), 0x8000); // +1D, not constructor-cleared +1C.
    b.set_style_word(0, 9, 0x55, &mut f); // Aliases the next block's first word.
    assert_eq!(b.style_word(1, 0), 0x55);
    b.set_optional_word(0x300, 0, 123, &mut f);
    let (mut value, mut enabled) = (0, 9);
    assert!(b.optional_word(0x300, &mut value, &mut enabled));
    assert_eq!((value, enabled), (0x4000_0000, 0));
    b.set_optional_word(0x300, 1, 0x4000_0000, &mut f);
    b.optional_word(0x300, &mut value, &mut enabled);
    assert_eq!((value, enabled), (0x4000_0000, 0));
    b.set_style_colors(17, 19, &mut f);
    assert!(b.style_words.iter().all(|s| s[4..6] == [17, 19]));
    b.style_words[3][4] = 99;
    assert_eq!(b.style_colors(), [17, 19]);
    f.refs.clear();
    b.set_common_font(Some(20), &mut f);
    b.set_common_font(Some(21), &mut f);
    assert!(f.refs.is_empty());
    b.shutdown(&mut f);
    assert_eq!(f.refs.iter().filter(|(_, h)| *h == 20 || *h == 21).copied().collect::<Vec<_>>(), [(false, 21)]);
}

#[test]
fn fbq4_columns_custom_records_and_unchanged_outputs() {
    let mut b = FileBrowser::new(80);
    let mut f = Recording::default();
    assert!(b.set_column_order(&[0, 1, 5], &mut f));
    assert_eq!(b.word(0x228), 3);
    assert_eq!(b.three_columns(), Ok([0, 1, 5]));
    assert!(!b.set_column_order(&[2], &mut f));
    assert_eq!(b.word(0x228), 2);
    assert_eq!(b.raw_bytes[&0x1f4], 1);
    assert_eq!(b.three_columns(), Ok([0, 1, 5]));
    assert!(b.set_column_order(&[1, 0], &mut f));
    assert!(matches!(b.column_order(2), Err(Unsupported::Precondition(_))));
    b.add_custom_entry(b"alpha", Some(101), 42, 2, &mut f);
    b.add_custom_entry(b"beta", Some(102), 42, 2, &mut f);
    assert_eq!((b.custom_groups.len(), b.filter_labels.len()), (3, 3));
    assert_eq!(f.refs, [(true, 101), (true, 102)]);
    let (mut name, mut buffer, mut id) = (vec![9], Some(9), 9);
    assert!(!b.custom_entry(0, 3, &mut name, &mut buffer, &mut id));
    assert_eq!((&name[..], buffer, id), (&[9][..], Some(9), 9));
    assert!(b.custom_entry(0, 2, &mut name, &mut buffer, &mut id));
    assert_eq!((&name[..], buffer, id), (&b"alpha"[..], Some(101), 42));
    assert_eq!(f.refs.len(), 2); // Getter borrows.
    assert!(b.remove_custom_id(42, 2, &mut f));
    assert_eq!(b.custom_groups[2][0].text, b"beta");
    assert_eq!(f.refs.last(), Some(&(false, 101)));
    b.filter_index = 2;
    assert!(b.remove_custom_name(b"BETA", u32::MAX, &mut f));
    assert!(!b.remove_custom_id(42, 2, &mut f));
    b.children[7] = Some(50);
    b.dirty = false;
    let invalidations = f.invalidations;
    b.set_filter_label(b"cities", 4, &mut f);
    assert_eq!((b.custom_groups.len(), b.filter_labels.len()), (5, 5));
    assert_eq!(f.fb_ops.last(), Some(&(0x6c, vec![4])));
    assert_eq!(f.invalidations, invalidations);
    assert!(!b.dirty);
    b.set_max_rows(3);
    assert_eq!(b.word(0x230), 4);
    b.set_max_rows(u32::MAX);
    assert_eq!(b.word(0x230), u32::MAX);
    assert_eq!(f.invalidations, invalidations);
}

#[test]
fn fbq4_descriptor_background_and_drive_worked_cases() {
    let mut b = FileBrowser::new(80);
    let mut f = Recording { fb_editor: b"new.sc4".to_vec(), ..Recording::default() };
    b.children[6] = Some(20);
    b.selection.kind = 1;
    b.selection.id = 9;
    b.selection.directory = b"C:\\Town\\".to_vec();
    let descriptor = b.edited_descriptor(&mut f);
    assert_eq!((descriptor.kind, descriptor.id), (2, 9));
    assert_eq!(b.borrowed_string(descriptor.strings[0]), b"C:\\Town\\new.sc4");
    assert_eq!(b.borrowed_string(descriptor.strings[2]), b"new.sc4");
    assert_eq!(b.descriptor().kind, 1);
    f.fb_editor.clear();
    b.edited_descriptor(&mut f);
    assert_eq!(b.selection.leaf, b"new.sc4");
    b.selection.kind = 2;
    b.edited_descriptor(&mut f);
    assert!(b.selection.leaf.is_empty());
    b.set_background_color(123, &mut f);
    assert_eq!(b.word(0x30c), 0xff12_3456);
    assert!(!b.dirty);
    b.background_color(&mut f);
    assert_eq!(f.fb_rgb.last(), Some(&[18, 18, 18]));
    b.set_background_buffer(Some(90), &mut f);
    b.disable_background(1, &mut f);
    let invalidations = f.invalidations;
    b.set_background_buffer(Some(90), &mut f);
    assert!(b.background_disabled());
    assert_eq!(f.invalidations, invalidations);
    assert_eq!(f.refs, [(true, 90)]);
    b.disable_background(0, &mut f);
    assert!(!b.background_disabled());
    assert_eq!(f.window_flags, [(0x20000, true), (0x40000, true)]);
    b.initialize_fallback();
    assert_eq!(b.fallback, b"c:\\");
    f.fb_drives = vec![(b"X:".to_vec(), 3), (b"D:long".to_vec(), 1), (b"E:".to_vec(), 1), (b"Z".to_vec(), 1)];
    b.icons[3] = Some(3);
    b.initialize_drives(&mut f);
    assert_eq!(b.drive_records.len(), 3);
    assert_eq!((b.drive_records[1].kind, b.drive_records[1].icon, b.drive_records[1].depth), (0, Some(3), 0));
    b.initialize_fallback();
    assert_eq!(b.fallback, b"D:\\");
    assert!(!b.validate_current(b"missing", &mut f));
    assert_eq!(b.current, b"D:\\");
    assert!(!b.dirty);
}

#[test]
fn fbq6_older_table_dispatch_and_completion_channels() {
    let mut b = FileBrowser::new(80);
    let mut f = Recording::default();
    b.selection.kind = 2;
    let confirm = Message { kind: 3, words: [0, 0xbaa5_5d58_u32 as i32, 0, 0] };
    assert!(b.alternate_send(confirm, 71, 0, &mut f));
    assert_eq!(f.fb_ops, [(4, vec![0, 1, 1, 0, 0])]);
    assert!(f.base_messages.is_empty());
    b.selection.kind = 1;
    assert!(b.alternate_send(confirm, 71, 0, &mut f));
    assert_eq!(f.fb_ops.len(), 1);
    f.base_flags = 0x1000;
    let double_click = Message { kind: 0x7abe_7204, words: [0; 4] };
    assert!(!b.alternate_send(double_click, 72, 2, &mut f));
    assert_eq!(&f.fb_ops[1..], [(4, vec![15, 1]), (145, vec![72])]);
    assert_eq!(f.base_messages, [double_click]);
    assert_eq!(b.complete(0, &mut f), 77);
    f.base_flags = 0;
    assert_eq!(b.complete(1, &mut f), 88);
    assert_eq!(PRIMARY_SLOTS, 107);
}

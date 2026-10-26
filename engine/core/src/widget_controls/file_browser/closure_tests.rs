// SPDX-License-Identifier: GPL-3.0-or-later
use super::file_browser::Entry;
use super::{file_browser::*, *};

impl PopulationIo for Recording {
    fn grid(&mut self, grid: Handle, operation: GridOp) {
        if let GridOp::Text(column, row, text, _, _, _) = &operation {
            self.installed.push((*column, *row, text.clone()));
            self.grid_texts.insert(*row, text.clone());
        }
        self.fb_grids.push((grid, operation));
    }
    fn window_rect(&mut self, _: Handle) -> Rect {
        self.fb_rect
    }
    fn cell_visible(&mut self, _: Handle, _: i32, _: i32) -> bool {
        self.fb_visible
    }
    fn split_components(&mut self, _: &[u8], capacity: usize, component_bytes: usize) -> Vec<Vec<u8>> {
        assert_eq!((capacity, component_bytes), (48, 260));
        self.fb_parts.clone()
    }
    fn focus_grid(&mut self, _: Handle, _: Handle) {
        self.fb_ops.push((0xf0c, vec![]));
    }
    fn background_rect(&mut self, _: Handle, area: Rect, options: [u32; 2]) -> Rect {
        assert_eq!(options, [0, 0]);
        area
    }
}

impl PropertyIo for Recording {
    fn manager(&mut self, _: Handle) -> Handle {
        111
    }
    fn drives(&mut self) -> Vec<(Vec<u8>, u32)> {
        self.fb_drives.clone()
    }
    fn editor_text(&mut self, _: Handle) -> Vec<u8> {
        self.fb_editor.clone()
    }
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
    fn unpack_rgb(&mut self, _: u32) -> [u8; 3] {
        [18, 52, 86]
    }
    fn window_flag(&mut self, _: Handle, flag: u32) -> bool {
        self.base_flags & flag != 0
    }
    fn complete_modal(&mut self, _: Handle, id: u32, _: Handle, argument: u32) -> u32 {
        self.fb_ops.push((4, vec![id, argument]));
        77
    }
    fn complete_window(&mut self, _: Handle, target: u32, event: u32, sender: Handle, zero: [u32; 2]) -> u32 {
        self.fb_ops
            .push((4, vec![target, event, sender as u32, zero[0], zero[1]]));
        88
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
    assert_eq!(
        f.refs
            .iter()
            .filter(|(_, h)| *h == 20 || *h == 21)
            .copied()
            .collect::<Vec<_>>(),
        [(false, 21)]
    );
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
    let mut f = Recording {
        fb_editor: b"new.sc4".to_vec(),
        ..Recording::default()
    };
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
    f.fb_drives = vec![
        (b"X:".to_vec(), 3),
        (b"D:long".to_vec(), 1),
        (b"E:".to_vec(), 1),
        (b"Z".to_vec(), 1),
    ];
    b.icons[3] = Some(3);
    b.initialize_drives(&mut f);
    assert_eq!(b.drive_records.len(), 3);
    assert_eq!(
        (
            b.drive_records[1].kind,
            b.drive_records[1].icon,
            b.drive_records[1].depth
        ),
        (0, Some(3), 0)
    );
    b.initialize_fallback();
    assert_eq!(b.fallback, b"D:\\");
    assert!(!b.validate_current(b"missing", &mut f));
    assert_eq!(b.current, b"D:\\");
    assert!(!b.dirty);
    assert!(b.set_directory_property(b"missing", &mut f));
    assert_eq!(b.current, b"D:\\");
    assert!(b.dirty);
}

#[test]
fn fbq6_older_table_dispatch_and_completion_channels() {
    let mut b = FileBrowser::new(80);
    let mut f = Recording::default();
    b.selection.kind = 2;
    let confirm = Message {
        kind: 3,
        words: [0, 0xbaa5_5d58_u32 as i32, 0, 0],
    };
    assert!(b.alternate_send(confirm, 71, 0, &mut f));
    assert_eq!(f.fb_ops, [(4, vec![0, 1, 84, 0, 0])]);
    assert!(f.base_messages.is_empty());
    b.selection.kind = 1;
    assert!(b.alternate_send(confirm, 71, 0, &mut f));
    assert_eq!(f.fb_ops.len(), 1);
    f.base_flags = 0x1000;
    let double_click = Message {
        kind: 0x7abe_7204,
        words: [0; 4],
    };
    assert!(!b.alternate_send(double_click, 72, 2, &mut f));
    assert_eq!(&f.fb_ops[1..], [(4, vec![15, 1]), (145, vec![72])]);
    assert_eq!(f.base_messages, [double_click]);
    assert_eq!(b.complete(0, &mut f), 77);
    f.base_flags = 0;
    assert_eq!(b.complete(1, &mut f), 88);
    assert_eq!(PRIMARY_SLOTS, 107);
}

impl MessageIo for Recording {
    fn combo_current(&mut self, _: Handle) -> u32 {
        self.bar as u32
    }
    fn current_cell(&mut self, _: Handle) -> (i32, i32) {
        self.fb_ops.push((0xa8, vec![]));
        (0, self.bar)
    }
    fn forward_message(&mut self, _: Handle, message: Message) {
        self.messages.push((Delivery::Send, message));
    }
    fn base_do_message(&mut self, _: Handle, message: Message) -> bool {
        self.base_messages.push(message);
        false
    }
}

#[test]
fn fbq2_c41_c42_c43_message_routes_and_borrowed_notifications() {
    let mut b = FileBrowser::new(80);
    let mut f = Recording {
        exists: true,
        ..Recording::default()
    };
    let receive = |kind, id, sender, payload, key| Received {
        message: Message {
            kind,
            words: [3, 4, 5, 6],
        },
        subtype: 0,
        sender,
        sender_id: id,
        payload,
        key,
    };
    b.children[1] = Some(100);
    assert_eq!(
        b.do_message(
            receive(0x9a15_52d5, FIRST_CHILD_ID + 1, 100, [0; 3], 13),
            &mut f,
            &mut Unsettled
        ),
        Ok(false)
    );
    assert_eq!(f.fb_ops.last(), Some(&(0xa8, vec![])));
    assert!(b.children[1].is_none());
    b.children[1] = Some(101);
    let calls = f.fb_ops.len();
    b.do_message(
        receive(0x9a15_52d5, FIRST_CHILD_ID + 1, 101, [0; 3], 27),
        &mut f,
        &mut Unsettled,
    )
    .unwrap();
    assert_eq!(f.fb_ops.len(), calls);
    assert_eq!(f.refs.last(), Some(&(false, 101)));
    b.current = b"C:\\".to_vec();
    b.flags = 0;
    b.entries = (0..8)
        .map(|_| {
            Some(Entry {
                tag: DIRECTORY,
                name: b"leaf".to_vec(),
                ..Entry::default()
            })
        })
        .collect();
    b.do_message(
        receive(0x9a15_52d4, FIRST_CHILD_ID + 5, 5, [5, 0, 7], 0),
        &mut f,
        &mut Unsettled,
    )
    .unwrap();
    assert_eq!(f.selections.last().unwrap().0, 0x7abe_7204);
    assert_eq!(f.selections.last().unwrap().2.full_path, b"C:\\leaf\\");
    assert_eq!(b.current, b"C:\\leaf\\");
    assert_eq!(f.base_messages.last().unwrap().kind, 0x9a15_52d4);
    b.children[6] = Some(300);
    let count = f.selections.len();
    b.do_message(
        receive(0x9b15_52d8, FIRST_CHILD_ID + 6, 301, [0; 3], 0),
        &mut f,
        &mut Unsettled,
    )
    .unwrap();
    assert_eq!(f.selections.len(), count);
    b.do_message(
        receive(0x9b15_52d8, FIRST_CHILD_ID + 6, 300, [0; 3], 0),
        &mut f,
        &mut Unsettled,
    )
    .unwrap();
    assert_eq!(f.selections.len(), count + 1);
    assert_eq!(f.selections.last().unwrap().0, 0x7abe_7203);
    assert_eq!(f.base_messages.len(), 5);
}

#[test]
fn fbq4_navigation_removal_and_fbq5_live_creation_paint() {
    let mut b = FileBrowser::new(80);
    let mut f = Recording {
        exists: true,
        create_success: true,
        fb_rect: [5, 7, 105, 67],
        ..Recording::default()
    };
    b.flags = 0;
    assert_eq!(b.set_current_live(b"C:\\Town", &mut f, &mut Unsettled), Ok(true));
    let checks = f.existing_checks;
    assert_eq!(b.remove_subdirectory(b"C:\\Town", &mut f, &mut Unsettled), Ok(false));
    assert_eq!(f.existing_checks, checks);
    assert_eq!(
        b.remove_subdirectory(b"c:\\TownOther", &mut f, &mut Unsettled),
        Ok(false)
    );
    assert_eq!(
        b.remove_subdirectory(b"C:\\TownOther", &mut f, &mut Unsettled),
        Ok(true)
    );
    assert_eq!(f.paths[f.paths.len() - 2], b"C:\\TownOther"); // Raw prefix, no separator boundary.
    b.children[5] = Some(200);
    assert_eq!(b.create_directory_live(&mut f, &mut Unsettled), Ok(true));
    assert!(f.paths.iter().any(|p| p.ends_with(b"New Folder (256)\\")));
    assert!(f.fb_grids.contains(&(200, GridOp::Select(0, 0, true))));
    b.raw_bytes.insert(0x309, 1);
    b.dirty = true;
    b.paint_live(&mut f, &mut Unsettled).unwrap();
    assert_eq!(
        f.draws.last(),
        Some(&Draw::Fill {
            rect: [5, 7, 105, 67],
            color: u32::MAX
        })
    );
    assert!(!b.dirty);
    assert!(b.refresh_work.is_empty());
    b.raw_bytes.insert(0x309, 0);
    b.dirty = true;
    let draws = f.draws.len();
    b.paint_live(&mut f, &mut Unsettled).unwrap();
    assert_eq!(f.draws.len(), draws);
    assert!(!b.dirty);
}

#[test]
fn fbq2_c39_c40_control_order_configuration_and_factory_boolean() {
    let mut b = FileBrowser::new(80);
    let mut f = Recording::default();
    b.enabled = [0; 13];
    assert_eq!(b.add_controls(&mut f), Ok(true));
    assert_eq!(f.fb_rgb, [[0, 0, 0], [255, 255, 255]]);
    assert!(f.fb_controls.is_empty());
    b.enabled = [1; 13];
    b.filter_labels = vec![b"cities".to_vec()];
    b.button_buffers = [Some(30), Some(31), Some(32), Some(33)];
    f.fb_factory = Some((false, Some(999))); // Written pointer does not override false.
    assert_eq!(b.add_controls(&mut f), Ok(true));
    let ids = f
        .fb_controls
        .iter()
        .filter_map(|(_, op)| {
            if let ControlOp::Id(id) = op {
                Some(*id - FIRST_CHILD_ID)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(ids, [0, 2, 5, 3, 4, 6, 7, 8, 9, 10, 11, 12]);
    assert!(b.children[1].is_none());
    assert!(f.fb_controls.contains(&(810, ControlOp::Dimensions(2, 0, true))));
    assert!(f.fb_controls.contains(&(810, ControlOp::Font(None))));
    assert!(f.fb_controls.contains(&(810, ControlOp::Append(b"cities".to_vec(), 0))));
    assert!(f.fb_controls.contains(&(810, ControlOp::Select(0, false))));
    assert!(f.fb_controls.contains(&(810, ControlOp::Caption(b"Look in:".to_vec()))));
    assert!(f.fb_controls.contains(&(810, ControlOp::Buffer(32))));
    assert!(f.fb_ops.contains(&(0x231a_1c58, vec![656, 0x11ff, 0xd8])));
    assert!(f.fb_ops.contains(&(0x59b, vec![316, 1, 0xd8])));
    assert_eq!(f.refs.iter().filter(|(add, h)| *add && *h == 810).count(), 12);
    let refs = f.refs.len();
    f.fb_factory = Some((true, None));
    assert!(matches!(obtain_control(0, &mut f), Err(Unsupported::Precondition(_))));
    assert_eq!(f.refs.len(), refs);
    f.fb_factory = Some((true, Some(888)));
    assert_eq!(obtain_control(6, &mut f), Ok(Some(888)));
    assert_eq!(f.refs.len(), refs);
    b.enabled[5] = 0;
    b.enabled[6] = 1;
    f.fb_controls.clear();
    b.add_controls(&mut f).unwrap();
    assert!(!f.fb_controls.contains(&(888, ControlOp::Id(FIRST_CHILD_ID + 5))));
    assert!(f.fb_controls.contains(&(888, ControlOp::Id(FIRST_CHILD_ID + 6))));
}

#[test]
fn fbq5_c28_c29_case_copy_and_unsigned_capacity() {
    let mut b = FileBrowser::new(80);
    let mut f = Recording {
        fb_rect: [0, 0, 200, 60],
        ..Recording::default()
    };
    b.flags = 8;
    b.children[5] = Some(200);
    for (offset, value) in [(0x1e4, 16), (0x220, 20), (0x1fc, 30), (0x314, 12)] {
        b.store_word(offset, value);
    }
    b.raw_bytes.insert(0x1f4, 1);
    f.enumerated[1] = vec![b"a.txt".to_vec(), b"b.txt".to_vec(), b"c.txt".to_vec()];
    b.populate_file_list(&mut f).unwrap();
    assert_eq!(b.word(0x1f0), 132);
    f.enumerated[1].push(b"d.txt".to_vec());
    b.populate_file_list(&mut f).unwrap();
    assert_eq!(b.word(0x1f0), 120);
    b.filter_labels = vec![b"text".to_vec()];
    b.filter_patterns = vec![vec![b"*.TXT".to_vec()]];
    f.enumerated[1] = vec![b"Report.TXT".to_vec()];
    b.populate_file_list(&mut f).unwrap();
    assert_eq!(
        f.fb_matches.last(),
        Some(&(b"*.txt".to_vec(), b"report.txt".to_vec(), 0))
    );
    b.raw_bytes.insert(0x15c, 0x80);
    b.populate_file_list(&mut f).unwrap();
    assert_eq!(
        f.fb_matches.last(),
        Some(&(b"*.TXT".to_vec(), b"Report.TXT".to_vec(), 0x80))
    );
    assert_eq!(b.filter_patterns[0][0], b"*.TXT");
    assert_eq!(f.enumerated[1][0], b"Report.TXT");
    b.store_word(0x224, 0);
    assert!(matches!(
        b.populate_file_list(&mut f),
        Err(Unsupported::Precondition(_))
    ));
}

#[test]
fn fbq5_c30_c31_c38_restore_empty_grid_and_disabled_columns() {
    let mut b = FileBrowser::new(80);
    let mut f = Recording {
        split_count: 2,
        fb_rect: [0, 0, 200, 60],
        ..Recording::default()
    };
    b.children[5] = Some(200);
    f.enumerated = [vec![b"Alpha".to_vec()], vec![b"Beta".to_vec()]];
    b.set_pending(1, b"ALPHA", b"", &mut f);
    b.populate_file_list(&mut f).unwrap();
    assert_eq!(b.selected_row, 1);
    assert_eq!(b.selection.kind, 1);
    assert!(f.fb_grids.contains(&(200, GridOp::Select(0, 1, true))));
    b.set_pending(2, b"missing", b"", &mut f);
    b.populate_file_list(&mut f).unwrap();
    assert_eq!((b.selected_row, b.selection.kind), (0, 0));
    b.flags = 0;
    f.fb_grids.clear();
    let notifications = f.selections.len();
    b.populate_file_list(&mut f).unwrap();
    assert_eq!(f.selections.len(), notifications);
    assert_eq!(
        &f.fb_grids[f.fb_grids.len() - 4..],
        [
            (200, GridOp::FirstVisible(0)),
            (200, GridOp::Select(0, 0, true)),
            (200, GridOp::Update),
            (200, GridOp::BringIntoView(0, 0))
        ]
    );
    b.flags = 8;
    b.raw_bytes.insert(0x1e8, 0);
    b.raw_bytes.insert(0x218, 0);
    b.icons[2] = Some(99);
    f.fb_grids.clear();
    b.populate_file_list(&mut f).unwrap();
    assert!(f.fb_ops.contains(&(0xb17, vec![0, 0, 99])));
    assert!(f.fb_grids.contains(&(200, GridOp::Style(0x280, 1, 0))));
    assert!(f
        .fb_grids
        .contains(&(200, GridOp::Text(2, 0, b"Beta".to_vec(), u32::MAX, true, false))));
    assert!(f.fb_grids.contains(&(200, GridOp::Style(0x238, 2, 0))));
}

#[test]
fn fbq5_c34_c35_c36_c37_directory_and_popup_worked_cases() {
    let mut b = FileBrowser::new(80);
    let mut f = Recording {
        fb_rect: [0, 0, 160, 20],
        fb_parts: [b"C0\\", b"C1\\", b"C2\\", b"C3\\"]
            .iter()
            .map(|s| s.to_vec())
            .collect(),
        ..Recording::default()
    };
    b.current = b"c:\\Town\\".to_vec();
    f.fit_counts = [2; 6].into();
    b.children[0] = Some(300);
    b.common_font = Some(1);
    b.drive_records = vec![
        DriveRecord {
            name: b"C:".to_vec(),
            ..DriveRecord::default()
        },
        DriveRecord {
            name: b"old".to_vec(),
            depth: 1,
            ..DriveRecord::default()
        },
        DriveRecord {
            name: b"D:".to_vec(),
            ..DriveRecord::default()
        },
    ];
    b.populate_directory_list(&mut f).unwrap();
    assert_eq!(
        b.drive_records
            .iter()
            .map(|r| (&r.name[..], r.depth))
            .collect::<Vec<_>>(),
        [
            (&b"C:"[..], 0),
            (&b"C2"[..], 1),
            (&b"C1"[..], 2),
            (&b"C0"[..], 3),
            (&b"D:"[..], 0)
        ]
    );
    assert_eq!((b.word(0x19c), b.word(0x234)), (3, 5));
    assert_eq!(f.installed.last().unwrap().2, b"C0");
    b.store_word(0x230, 3);
    b.store_word(0x1e4, 16);
    b.store_word(0x314, 12);
    f.fit_widths.clear();
    b.open_popup(&mut f).unwrap();
    assert!(f.fb_controls.contains(&(810, ControlOp::Dimensions(13, 1, false))));
    assert!(f.fb_grids.contains(&(810, GridOp::Area([0, 19, 160, 81]))));
    assert!(f.fb_grids.contains(&(810, GridOp::Column(0, 9))));
    assert!(f.fb_grids.contains(&(810, GridOp::RowHeights(5, 20))));
    assert_eq!(f.fit_widths[2], 98);
    assert!(f
        .fb_grids
        .contains(&(810, GridOp::Text(4, 2, b"C1".to_vec(), 2, true, false))));
    b.close_popup(&mut f);
    assert!(b.children[1].is_none());
    assert_eq!(f.refs.last(), Some(&(false, 810)));
    b.current = b"Z:\\".to_vec();
    f.fb_grids.clear();
    b.populate_directory_list(&mut f).unwrap();
    assert_eq!(b.drive_records.len(), 2);
    assert!(f.fb_grids.is_empty());
}

#[test]
fn fbq1_arrow_recipe_failures_and_narrowed_icon_button_boundaries() {
    let b = FileBrowser::new(80);
    let mut f = Recording {
        buffer_ok: true,
        fb_locks: [true, true, false, true, true].into(),
        ..Recording::default()
    };
    let mut output = None;
    assert_eq!(b.create_combo_arrow(&mut output, &mut f), Ok(true));
    assert_eq!(output, Some(700));
    assert_eq!(f.fb_rgb, [[0, 0, 0], [255, 0, 255]]);
    assert_eq!(f.fb_fills, [[0, 0, 64, 15]]);
    let expected = [4, 36, 52]
        .into_iter()
        .flat_map(|x| (0..4).map(move |s| ([x + s, 5 + s], [x + 7 - s, 5 + s])))
        .collect::<Vec<_>>();
    assert_eq!(f.fb_lines, expected);
    assert_eq!(f.icon_calls.iter().filter(|&&s| s == "unlock").count(), 4);
    assert!(f.refs.is_empty());
    f = Recording {
        buffer_ok: true,
        ..Recording::default()
    };
    output = None;
    assert_eq!(b.create_combo_arrow(&mut output, &mut f), Ok(true));
    assert_eq!(f.fb_lock_flags, [0x8001, 0x8004, 0x8004, 0x8004, 0x8004]);
    assert!(f.fb_fills.is_empty() && f.fb_lines.is_empty());
    assert!(!f.icon_calls.contains(&"unlock"));
    f = Recording::default();
    output = None;
    assert_eq!(b.create_combo_arrow(&mut output, &mut f), Ok(false));
    assert_eq!(output, None);
    assert_eq!(f.refs, [(false, 700)]);
    assert!(f.fb_rgb.is_empty() && f.fb_lock_flags.is_empty());
    f = Recording {
        buffer_ok: true,
        ..Recording::default()
    };
    assert_eq!(b.draw_default_icon(7, Some(700), [2, 3, 20, 20], &mut f), Ok(true));
    assert_eq!(f.fb_lock_flags, [0x8001]);
    assert_eq!(f.fb_rgb.len(), 8);
    assert!(f.icon_calls.contains(&"key"));
    f.fb_lock_flags.clear();
    output = None;
    assert_eq!(b.create_button_buffer(4, &mut output, &mut f), Ok(true));
    assert_eq!(f.fb_lock_flags, [0x8001, 0x8004]);
    assert!(!f.icon_calls.contains(&"unlock"));
    assert_eq!(
        Entry::default(),
        Entry {
            icon: None,
            tag: 0,
            id: 0,
            name: vec![]
        }
    );
    assert_eq!(
        CustomEntry::default(),
        CustomEntry {
            id: 0,
            interface: None,
            text: vec![]
        }
    );
    assert_eq!(
        DriveRecord::default(),
        DriveRecord {
            name: vec![],
            kind: 0,
            icon: None,
            depth: 0
        }
    );
}

// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use grid::{Grid, GridHost, Input, LayoutContext, PaintHost, PaintOp, PaintRecord};
use std::collections::BTreeMap;
#[path = "vectors.rs"]
mod vectors;

#[derive(Default)]
pub(super) struct Events {
    pub sizes: BTreeMap<Handle, [i32; 2]>,
    pub areas: Vec<(Handle, Rect)>,
    notifications: Vec<(u32, Vec<i32>)>,
    bars: Vec<(Handle, [i32; 2], [i32; 2], i32)>,
    images: Vec<(Handle, Handle, Rect)>,
    paints: Vec<PaintOp>,
    owners: Vec<(u32, PaintRecord)>,
    focused: Vec<Handle>,
    created: Vec<(u32, u32, [i32; 2])>,
    next: Handle,
    time: u64,
    capture: bool,
    disabled: bool,
    fail_bar: bool,
    fail_editor: bool,
    veto: bool,
}

#[test]
fn owner_final_release_destroys_storage_after_double_shutdown() {
    let mut f = Recording::default();
    let mut g = Grid::new(100, 0);
    g.default_format.font = Some(44);
    let mut owner = grid::GridOwner::new(g);
    assert_eq!(owner.add_ref(), 2);
    assert_eq!(owner.release(&mut f), 1);
    assert!(owner.grid.is_some());
    assert!(f.refs.is_empty());
    assert_eq!(owner.release(&mut f), 0);
    assert!(owner.grid.is_none());
    assert_eq!(f.refs.iter().filter(|r| **r == (false, 44)).count(), 1);
}

impl GridHost for Recording {
    fn grid_rgb(&mut self, _: Handle, rgb: [u8; 3]) -> u32 {
        u32::from_be_bytes([0x7f, rgb[0], rgb[1], rgb[2]])
    }
    fn compare_grid_text(&mut self, a: &[u8], b: &[u8]) -> std::cmp::Ordering {
        let a = grid::Cell {
            value: grid::Value::Text(a.to_vec()),
            ..grid::Cell::default()
        };
        let b = grid::Cell {
            value: grid::Value::Text(b.to_vec()),
            ..grid::Cell::default()
        };
        grid::compare_c_locale(&a, &b)
    }
    fn enabled(&mut self, _: Handle) -> bool {
        !self.owner.disabled
    }
    fn capturing(&mut self, _: Handle) -> bool {
        self.owner.capture
    }
    fn focused(&mut self) -> Option<Handle> {
        Some(88)
    }
    fn focus(&mut self, window: Handle) {
        self.owner.focused.push(window);
    }
    fn base_focus(&mut self, _: Handle) -> bool {
        true
    }
    fn now_ms(&mut self) -> u64 {
        self.owner.time
    }
    fn create_grid_bar(&mut self, class: u32, style: u32, range: [i32; 2]) -> Option<Handle> {
        self.owner.created.push((class, style, range));
        if self.owner.fail_bar {
            return None;
        }
        self.owner.next += 1;
        let bar = 700 + self.owner.next;
        self.owner
            .sizes
            .insert(bar, if style == 2 { [10, 100] } else { [100, 10] });
        Some(bar)
    }
    fn bar_parameters(&mut self, bar: Handle, range: [i32; 2], steps: [i32; 2], position: i32) {
        self.owner.bars.push((bar, range, steps, position));
    }
    fn image_area(&mut self, _: Handle) -> Rect {
        [0, 0, 8, 6]
    }
    fn bar_image(&mut self, bar: Handle, image: Handle, source: Rect) {
        self.owner.images.push((bar, image, source));
    }
    fn point_in_bar(&mut self, _: Handle, point: [i32; 2]) -> Option<[i32; 2]> {
        if point[0] > 200 {
            Some([0, 0])
        } else {
            None
        }
    }
    fn mouse_holder(&mut self, window: Handle) {
        self.owner.focused.push(window);
    }
    fn bar_mouse(&mut self, _: Handle, _: u32, _: [i32; 2], _: u32) -> bool {
        false
    }
    fn notify(&mut self, _: Handle, id: u32, data: &[i32]) -> bool {
        self.owner.notifications.push((id, data.to_vec()));
        true
    }
    fn create_editor(&mut self, style: u32) -> Option<Handle> {
        assert_eq!(style, 0x11ff);
        if self.owner.fail_editor {
            None
        } else {
            Some(601)
        }
    }
    fn editor_text(&mut self, _: Handle, text: Option<&[u8]>, _: i32) {
        self.editor_texts.push(text.unwrap_or_default().to_vec());
    }
    fn window_buffer(&mut self, _: Handle) -> (Option<Handle>, Rect) {
        (Some(41), [2, 3, 4, 5])
    }
    fn base_fill_color(&mut self, _: Handle, color: u32) {
        self.defaults.push(color);
    }
}

impl PaintHost for Recording {
    fn paint_op(&mut self, operation: PaintOp) {
        self.owner.paints.push(operation);
    }
    fn owner_paint(&mut self, _: Handle, id: u32, record: &PaintRecord) -> bool {
        self.owner.owners.push((id, record.clone()));
        !self.owner.veto
    }
    fn wrapped_lines(&mut self, _: Handle, text: &[u8], _: i32) -> Vec<(Vec<u8>, i32)> {
        vec![(text.to_vec(), text.len() as i32)]
    }
}

fn setup(flags: u32) -> (Grid, LayoutContext, Recording) {
    let mut grid = Grid::new(100, flags);
    grid.initialized = true;
    let f = Recording {
        grid_dimensions: [310, 110],
        font_height: 12,
        ..Recording::default()
    };
    (grid, LayoutContext::default(), f)
}

#[test]
fn owner_layout_bars_images_failure_pause_and_second_pass() {
    let (mut g, mut c, mut f) = setup(6 | 0x18000);
    g.limits = [5, 8];
    g.set_bar_image(true, Some(40), &mut f);
    g.recalculate(&mut c, &mut f);
    assert!(g.bars.iter().all(Option::is_some));
    assert_eq!(g.cell_size, [290, 90]);
    assert_eq!(g.visible, [2.9, 4.5]);
    assert_eq!(f.owner.created, [(0x1a7c4511, 2, [0, 100]), (0x1a7c4510, 1, [0, 100])]);
    assert_eq!(f.owner.images[0].2, [0, 0, 8, 6]);
    assert_eq!(f.owner.focused, [88, 88]);
    assert_eq!(c.depth, 0);
    assert!(!g.dirty);
    g.pause = 1;
    g.dirty = true;
    g.recalculate(&mut c, &mut f);
    assert!(!g.dirty);
    g.set_flags(0, &mut c, &mut f);
    assert_eq!(g.bars, [None; 2]);
    assert_eq!(f.child_ops.iter().filter(|(op, _)| *op == "delete").count(), 2);
    f.owner.fail_bar = true;
    g.flags = 6;
    g.add_bars([true; 2], &mut f);
    assert_eq!(g.flags & 6, 0);
}

#[test]
fn owner_hit_headers_inclusive_edges_and_input_notifications() {
    let (mut g, mut c, mut f) = setup(0x18);
    g.cells.insert((2, 4), grid::Cell::default());
    g.recalculate(&mut c, &mut f);
    assert_eq!(g.hit_test([0, 0], &mut f), Some([1, 0, 0]));
    assert_eq!(g.hit_test([10, 10], &mut f), Some([4, 0, 0]));
    assert_eq!(g.hit_test([90, 10], &mut f), Some([3, 0, 0]));
    assert_eq!(g.hit_test([10, 30], &mut f), Some([2, 0, 0]));
    g.input(
        Input::Down {
            point: [90, 30],
            modifiers: 0,
        },
        &mut c,
        &mut f,
    );
    assert_eq!(g.selection, [(0, 0)]);
    assert_eq!(
        f.owner.notifications.iter().map(|v| v.0).collect::<Vec<_>>(),
        [0x9a1552d3, 0x9a1552d2]
    );
    f.owner.time = 499;
    g.input(
        Input::Down {
            point: [90, 30],
            modifiers: 0,
        },
        &mut c,
        &mut f,
    );
    assert_eq!(f.owner.notifications.last().unwrap().0, 0x9a1552d4);
    g.input(
        Input::Move {
            point: [190, 30],
            modifiers: 0,
        },
        &mut c,
        &mut f,
    );
    assert_eq!(f.owner.notifications.last().unwrap().0, 0x9a1552d6);
    g.input(
        Input::Down {
            point: [190, 50],
            modifiers: 1,
        },
        &mut c,
        &mut f,
    );
    assert_eq!(g.selection, [(0, 0), (0, 1)]); // row membership suppresses duplicates
    g.input(
        Input::Down {
            point: [190, 50],
            modifiers: 2,
        },
        &mut c,
        &mut f,
    );
    assert_eq!(g.selection, [(0, 0)]); // default row removal mode
    g.flags &= !8;
    assert_eq!(g.hit_test([10, 30], &mut f), None);
}

#[test]
fn owner_keys_arrows_message_focus_and_dropdown_capture() {
    let (mut g, mut c, mut f) = setup(0);
    g.cells.insert((2, 4), grid::Cell::default());
    g.selection.push((2, 4));
    g.arrow(0x27, 0, &mut c, &mut f);
    assert_eq!(g.selection, [(2, 4)]);
    assert_eq!(f.owner.notifications[0], (0x9a1552d2, vec![2, 4]));
    g.selection.clear();
    g.arrow(0x28, 0, &mut c, &mut f);
    assert_eq!(g.selection, [(0, 2)]); // empty selection starts at (0, 1)
    assert!(!g.input(Input::Character, &mut c, &mut f));
    g.input(
        Input::KeyDown {
            key: 0x2d,
            modifiers: 0,
        },
        &mut c,
        &mut f,
    );
    assert_ne!(g.flags & 0x80, 0);
    assert_eq!(f.owner.notifications.last().unwrap(), &(0x9a1552d5, vec![0x2d]));
    assert!(g.input(
        Input::KeyUp {
            key: 0x1b,
            modifiers: 7
        },
        &mut c,
        &mut f
    ));
    assert_eq!(f.owner.notifications.last().unwrap(), &(6, vec![0x1b, 7, 0]));
    g.set_flags(g.flags | 0x40000, &mut c, &mut f);
    // Existing Foreign capture recorder has no native manager; establish state.
    f.owner.capture = true;
    g.input(
        Input::Down {
            point: [-1, 0],
            modifiers: 0,
        },
        &mut c,
        &mut f,
    );
    assert_eq!(f.owner.notifications.last().unwrap().0, 0x9a1552d7);
    g.bars[0] = Some(700);
    assert!(!g.input(
        Input::Up {
            point: [250, 0],
            modifiers: 0
        },
        &mut c,
        &mut f
    ));
    f.bar = 3;
    g.message(
        Message {
            kind: 3,
            words: [0x887113a3u32 as i32, 0x1a7c4511, 0, 0],
        },
        &mut c,
        &mut f,
    );
    assert_eq!(g.position[1], 3);
    assert_eq!(f.base_messages.len(), 1);
}

#[test]
fn owner_editable_text_reference_balance_init_and_font() {
    let (mut g, mut c, mut f) = setup(0);
    assert_eq!(g.set_editable_text((0, 0), Some(b"edit"), 4, &mut c, &mut f), Ok(true));
    assert_eq!(f.editor_texts, [b"edit".to_vec()]);
    assert!(matches!(
        g.cells[&(0, 0)].value,
        grid::Value::Window { handle: 1601, .. }
    ));
    g.set_cell_font((1, 1), Some(40), &mut c, &mut f);
    g.shutdown(&mut f);
    assert_eq!(f.refs.iter().filter(|&&(add, h)| add && h == 1601).count(), 3);
    assert_eq!(f.refs.iter().filter(|&&(add, h)| !add && h == 1601).count(), 3); // query, temporary setter, retained cell
    assert!(f.refs.contains(&(false, 601))); // created editor
    let (mut g, mut c, mut f) = setup(0);
    g.initialized = false;
    assert!(g.init(&mut c, &mut f));
    g.default_formatting(&mut f);
    assert_eq!(g.default_format.words[6], 0x0005_0201);
    assert_eq!(g.language, 1);
    assert!(g.init(&mut c, &mut f));
}

#[test]
fn owner_paint_layers_clips_selection_owner_veto_and_open_alignment() {
    let (mut g, mut c, mut f) = setup(0x100 | 0x600 | 0x20000);
    g.visible = [1.0; 2];
    g.dirty = false;
    g.pause = 2;
    g.cells.insert(
        (0, 0),
        grid::Cell {
            value: grid::Value::Bitmap(500),
            ..Default::default()
        },
    );
    let mut format = grid::Format {
        font: Some(40),
        words: [0x40000000; 8],
    };
    format.words[1] = 11;
    format.words[3] = 12;
    format.words[4] = 13;
    format.words[5] = 13;
    format.words[6] = (3 << 8) | (6 << 16);
    let mut formats = BTreeMap::from([((0, 0), format.clone())]);
    g.selection.push((0, 0));
    g.owner_draw = Some(55);
    g.paint(&formats, &grid::Format::default(), [10, 20], 99, &mut c, &mut f)
        .unwrap();
    assert_eq!(g.pause, 0);
    assert!(matches!(
        f.owner.paints[1],
        PaintOp::Primitive(Draw::Fill { color: 99, .. })
    ));
    assert!(f
        .owner
        .paints
        .iter()
        .any(|p| matches!(p, PaintOp::Primitive(Draw::Fill { color: 13, .. }))));
    assert!(f
        .owner
        .paints
        .iter()
        .any(|p| matches!(p, PaintOp::Capability(0x8010, true))));
    assert_eq!(f.owner.owners[0].0, 0xfa785cb2);
    assert_eq!(f.owner.owners[0].1.clip, [15, 25, 115, 45]);
    f.owner.paints.clear();
    f.owner.veto = true;
    g.paint(&formats, &grid::Format::default(), [0, 0], 99, &mut c, &mut f)
        .unwrap();
    assert!(!f
        .owner
        .paints
        .iter()
        .any(|p| matches!(p, PaintOp::Primitive(Draw::Blit { .. }))));
    f.owner.veto = false;
    g.cells.get_mut(&(0, 0)).unwrap().value = grid::Value::Text(b"text".to_vec());
    assert_eq!(
        g.paint(&formats, &grid::Format::default(), [0, 0], 99, &mut c, &mut f),
        Ok(())
    );
    formats.get_mut(&(0, 0)).unwrap().words[6] = 0;
    g.paint(&formats, &grid::Format::default(), [0, 0], 99, &mut c, &mut f)
        .unwrap();
    assert!(f
        .owner
        .paints
        .iter()
        .any(|p| matches!(p, PaintOp::Primitive(Draw::Text { color: 13, .. }))));
}

#[test]
fn owner_live_setters_formats_helpers_and_resources() {
    let (mut g, mut c, mut f) = setup(0);
    assert_eq!(g.buffer_to_draw_to(&mut f), (Some(41), [2, 3, 4, 5]));
    g.set_fill_color(17, &mut c, &mut f);
    assert_eq!(f.defaults, [17]);
    let mut out = (99, 99);
    assert!(!g.first_selection(&mut out));
    assert_eq!(out, (99, 99));
    g.mutate(grid::Mutation::DefaultFont(Some(50)), &mut c, &mut f).unwrap();
    assert!(!g.dirty);
    assert!(f.refs.contains(&(true, 50)));
    g.mutate(grid::Mutation::Wrapping(true), &mut c, &mut f).unwrap();
    assert_ne!(g.flags & 0x800, 0);
    g.mutate(
        grid::Mutation::Text {
            cell: (0, 0),
            text: Some(b"hello"),
            count: 5,
            mode: 1,
        },
        &mut c,
        &mut f,
    )
    .unwrap();
    assert!(!g.dirty);
    g.mutate(grid::Mutation::Selection((0, 0)), &mut c, &mut f).unwrap();
    assert!(g.first_selection(&mut out));
    assert_eq!(out, (0, 0));
    g.extra = 77;
    assert!(g.set_cell_extra_object((0, 0), Some(80), &mut f));
    assert_eq!(g.cell_extra((0, 0)), 77); // measured WG4 store quirk
    assert_eq!(g.extra_mode, 2);
    assert_eq!(grid::signed_gcd(-12, 8), -4);
    assert_eq!(grid::signed_gcd(6, 0), 6);
    assert!(grid::equal_record([9, 0, 1], [0, 0, 999]));
    assert!(!grid::equal_record([0, 1, 1], [0, 1, 2]));
    let mut sorted = [(2, 0), (1, 1), (2, 2)];
    grid::stable_sort_values(&mut sorted, false, |a, b| a.0.cmp(&b.0));
    assert_eq!(sorted, [(2, 0), (2, 2), (1, 1)]);
    let mut refs = 2;
    assert_eq!(g.release(&mut refs, &mut f), 1);
    assert_eq!(g.release(&mut refs, &mut f), 0);
    assert!(!g.initialized);
}

#[test]
fn owner_window_set_keeps_identity_order_and_move_coordinates() {
    let (mut g, _, mut f) = setup(0);
    g.set_window((2, 0), Some(20), false, false, 1, &mut f);
    g.set_window((0, 0), Some(30), false, false, 1, &mut f);
    g.copy_cell((2, 0), (4, 0), false, &mut f);
    let mut cell = (-1, -1);
    assert!(g.is_cell_window(Some(20), 0, &mut cell, &mut f));
    assert_eq!(cell, (2, 0)); // native DoCellWindowMove does not relocate the set record
    f.child_ops.clear();
    g.is_cell_window(None, 999, &mut cell, &mut f);
    assert_eq!(f.child_ops, [("id", 20), ("id", 30)]);
    g.clear_cell((4, 0), &mut f);
    assert!(!g.windows.contains_key(&20));
}

#[test]
fn owner_paint_line_order_header_defaults_and_unbounded_bar_quirk() {
    let (mut g, mut c, mut f) = setup(0x18 | 0x60 | 0x100000);
    g.visible = [2.0; 2];
    g.dirty = false;
    g.cells.insert((1, 1), grid::Cell::default());
    g.owner_draw = Some(40);
    g.line_widths = [2, 3];
    let mut format = grid::Format {
        font: Some(50),
        words: [0x40000000; 8],
    };
    format.words[0] = 12;
    format.words[3] = 12;
    format.words[6] = 0;
    let formats = [(0, 0), (0, 1), (1, 0), (1, 1)]
        .into_iter()
        .map(|cell| (cell, format.clone()))
        .collect();
    g.paint(&formats, &format, [0, 0], 99, &mut c, &mut f).unwrap();
    let ids: Vec<_> = f.owner.owners.iter().take(4).map(|e| e.0).collect();
    assert_eq!(ids, [0xfa785cb0, 0xfa785cb1, 0xfa785cb1, 0xfa785cb0]);
    assert!(f
        .owner
        .paints
        .iter()
        .any(|p| matches!(p, PaintOp::Primitive(Draw::Text { text, .. }) if text == b"Column #0")));
    assert!(f
        .owner
        .paints
        .iter()
        .any(|p| matches!(p, PaintOp::Primitive(Draw::Text { text, .. }) if text == b"Row #1")));
    g.flags = 2 | 0x10000;
    g.limits = [1, -1];
    g.visible = [100.0; 2];
    g.test_bars(&mut f);
    assert!(g.bars[1].is_some()); // horizontal always-wanted test reads row limit
    g.flags |= 0x8000;
    g.limits[0] = 0;
    g.bar_values(&mut f);
    assert_eq!(f.owner.bars.last().unwrap().2, [i32::MAX; 2]);
}

#[test]
fn owner_default_format_and_color_slots_share_storage() {
    let (mut g, _, mut f) = setup(0);
    assert!(g.set_default_color(3, 55));
    assert_eq!(g.default_format.words[3], 55);
    let format = grid::Format {
        font: None,
        words: [1, 2, 3, 4, 5, 6, 7, 8],
    };
    assert_eq!(g.set_cell_format_state((-1, -1), &format, &mut f), Ok(true));
    for i in 0..6 {
        let mut color = 99;
        assert!(g.default_color(i, &mut color));
        assert_eq!(color, i + 1);
    }
}

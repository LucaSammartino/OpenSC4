// SPDX-License-Identifier: GPL-3.0-or-later
use super::list_box::*;
use super::*;
#[path = "closure_tests.rs"]
mod closure;
#[path = "settled_tests.rs"]
mod settled;
#[path = "file_browser/closure_tests.rs"]
mod file_browser_closure;

#[derive(Default)]
struct Recording {
    fb_drives: Vec<(Vec<u8>, u32)>,
    fb_editor: Vec<u8>,
    fb_rgb: Vec<[u8; 3]>,
    fb_ops: Vec<(u32, Vec<u32>)>,
    tree_area: Option<Rect>,
    tree_thickness: [u32; 2],
    tree_resizes: Vec<[u32; 2]>,
    measure_override: Option<i32>,
    base_init_ok: bool,
    fail_working_buffer: bool,
    no_graphics: bool,
    no_default_font: bool,
    base_flag_calls: Vec<(Handle, u32, u8)>,
    base_flags: u32,
    skins: Vec<(Handle, Handle, Rect)>,
    compare_calls: usize,
    parent_limit: Option<i32>,
    capture_owner: Option<Handle>,
    dispatch_capture: Option<Handle>,
    bar_events: Vec<(Handle, bool, i32, i32)>,
    control: bool,
    fail_mode: u32,
    fail_image_slot: Option<usize>,
    image_attempts: Vec<usize>,
    access_calls: Vec<(u32, bool)>,
    drag_accept: u8,
    drag_return: bool,
    drag_target: Option<i32>,
    drag_pointer: Option<[i32; 2]>,
    drag_runs: usize,
    owner: grid_owner_tests::Events,
    refs: Vec<(bool, Handle)>,
    messages: Vec<(Delivery, Message)>,
    invalidations: usize,
    draws: Vec<Draw>,
    font_height: i32,
    buffer_height: i32,
    elapsed: u32,
    interval: u32,
    bar: i32,
    defaults: Vec<u32>,
    base_messages: Vec<Message>,
    paths: Vec<Vec<u8>>,
    exists: bool,
    existing_checks: usize,
    create_success: bool,
    split_count: i32,
    parent_path: Vec<u8>,
    window_flags: Vec<(u32, bool)>,
    selections: Vec<(u32, file_browser::SelectionDescriptor, file_browser::Selection)>,
    editor_texts: Vec<Vec<u8>>,
    grid_texts: std::collections::BTreeMap<i32, Vec<u8>>,
    grid_last: u32,
    grid_reads: usize,
    removed: Vec<(Handle, Handle)>,
    button_updates: Vec<(Handle, Option<Handle>)>,
    fit_counts: std::collections::VecDeque<usize>,
    fit_widths: Vec<i32>,
    marker: Option<Vec<u8>>,
    marker_width: i32,
    enumerated: [Vec<Vec<u8>>; 2],
    patterns: Vec<Vec<u8>>,
    installed: Vec<(i32, i32, Vec<u8>)>,
    combo_indices: Vec<u32>,
    buffer_ok: bool,
    lock_ok: bool,
    icon_calls: Vec<&'static str>,
    child_ops: Vec<(&'static str, Handle)>,
    grid_dimensions: [i32; 2],
    area_requests: Vec<(Handle, Rect)>,
}

impl file_browser::BrowserIo for Recording {
    fn enumerate(&mut self, _: &[u8], kind: file_browser::Enumeration) -> Vec<Vec<u8>> {
        self.enumerated[usize::from(kind == file_browser::Enumeration::Files)].clone()
    }
    fn wildcard_match(&mut self, pattern: &[u8], _: &[u8], _: u32) -> bool {
        self.patterns.push(pattern.to_vec());
        pattern == b"second"
    }
    fn clear_grid(&mut self, _: Handle) {
        self.installed.clear();
    }
    fn grid_set_text(&mut self, _: Handle, column: i32, row: i32, text: &[u8]) {
        self.installed.push((column, row, text.to_vec()));
    }
    fn grid_set_bitmap(&mut self, _: Handle, _: i32, _: i32, _: Handle) {}
    fn combo_select(&mut self, _: Handle, index: u32) {
        self.combo_indices.push(index);
    }
    fn base_set_area(&mut self, _: Handle, _: Rect) {
        self.icon_calls.push("area");
    }
}

impl file_browser::BrowserHost for Recording {
    fn set_width(&mut self, _: Handle, width: u32) {
        assert_eq!(width, 396);
        self.icon_calls.push("width396");
    }
    fn set_height(&mut self, _: Handle, height: u32) {
        assert_eq!(height, 236);
        self.icon_calls.push("height236");
    }
    fn base_destroy(&mut self, _: Handle) {
        self.icon_calls.push("destroy");
    }
}

impl file_browser::ControlIo for Recording {
    fn component(&mut self, _: u32, _: u32) -> Option<Handle> {
        None
    }
    fn builtin(&mut self, _: u32, _: u32) -> Option<Handle> {
        Some(810)
    }
}

#[test]
fn browser_control_prefix_and_unsettled_init_do_not_claim_full_creation() {
    use file_browser::*;
    let mut f = Recording {
        buffer_ok: true,
        ..Recording::default()
    };
    assert_eq!(obtain_control(5, &mut f), Ok(Some(810)));
    assert_eq!(f.refs, [(true, 810)]);
    assert_eq!(obtain_control(1, &mut f), Err(Unsupported::Open("FBQ2")));
    let mut b = FileBrowser::new(80);
    assert_eq!(b.init(&mut f, &mut Unsettled), Err(Unsupported::Open("FBQ4")));
    assert!(b.initialized); // Native first-init has no local rollback.
}

#[test]
fn browser_init_paint_refresh_and_final_release_order() {
    use file_browser::*;
    #[derive(Default)]
    struct Overrides {
        slots: Vec<u32>,
        refreshes: Vec<Refresh>,
        controls: bool,
    }
    impl BrowserOverrides<Recording> for Overrides {
        fn property(&mut self, _: &mut FileBrowser, slot: u32, _: &[u32], _: &mut Recording) -> Result<()> {
            self.slots.push(slot);
            Ok(())
        }
        fn add_controls(&mut self, _: &mut FileBrowser, _: &mut Recording) -> Result<bool> {
            Ok(self.controls)
        }
        fn refresh(&mut self, _: &mut FileBrowser, kind: Refresh, _: &mut Recording) -> Result<()> {
            self.refreshes.push(kind);
            Ok(())
        }
    }
    let mut f = Recording {
        buffer_ok: true,
        exists: true,
        ..Recording::default()
    };
    let mut b = FileBrowser::new(80);
    b.fallback = b"fallback".to_vec();
    b.dirty = true;
    let mut overrides = Overrides::default();
    assert_eq!(b.init(&mut f, &mut overrides), Ok(true));
    assert!(!b.dirty);
    assert_eq!(&f.icon_calls[..2], ["width396", "height236"]);
    assert_eq!(overrides.slots, [93, 94, 40, 73, 61, 63, 65, 67, 69, 71, 79]);
    assert_eq!(b.current, b"fallback");
    assert_eq!(b.raw_words[&0x30c], 0xff7c_90ff);
    assert_eq!(b.raw_bytes[&0x29e], 6);
    assert_eq!(b.raw_bytes[&0x2c2], 6);
    let calls = f.icon_calls.len();
    assert_eq!(b.init(&mut f, &mut overrides), Ok(true));
    assert_eq!(f.icon_calls.len(), calls);
    b.dirty = true;
    b.paint(
        Some(Draw::Fill {
            rect: [1, 2, 3, 4],
            color: 5,
        }),
        &mut f,
        &mut overrides,
    )
    .unwrap();
    assert_eq!(overrides.refreshes, [Refresh::DirectoryList, Refresh::FileList]);
    assert!(!b.dirty);
    b.dirty = true;
    assert_eq!(b.paint(None, &mut f, &mut Unsettled), Err(Unsupported::Open("FBQ5")));
    assert!(b.dirty);
    let mut owner = BrowserOwner::new(b);
    assert_eq!(owner.add_ref(), 2);
    assert_eq!(owner.release(&mut f), 1);
    assert_eq!(owner.release(&mut f), 0);
    assert!(owner.browser.is_none());
    assert_eq!(f.icon_calls.last(), Some(&"destroy"));
    f.refs.clear();
    let mut direct = FileBrowser::new(90);
    direct.icons[0] = Some(123);
    direct.destroy(&mut f);
    assert!(f.refs.is_empty()); // Direct destruction is not virtual Shutdown.
}

impl grid::GridForeign for Recording {
    fn language_value(&mut self) -> Option<u32> {
        None
    }
    fn window_size(&mut self, window: Handle) -> [i32; 2] {
        self.owner.sizes.get(&window).copied().unwrap_or(self.grid_dimensions)
    }
    fn set_area(&mut self, window: Handle, rect: Rect) {
        self.owner.areas.push((window, rect));
        self.child_ops.push(("area", window));
    }
    fn child_add(&mut self, _: Handle, child: Handle) {
        self.child_ops.push(("add", child));
    }
    fn child_remove(&mut self, _: Handle, child: Handle) {
        self.child_ops.push(("remove", child));
    }
    fn child_delete(&mut self, _: Handle, child: Handle) {
        self.child_ops.push(("delete", child));
    }
    fn window_flag(&mut self, _: Handle, flag: u32, value: bool) {
        self.window_flags.push((flag, value));
    }
    fn send_to_back(&mut self, window: Handle) {
        self.child_ops.push(("back", window));
    }
    fn move_window(&mut self, window: Handle, _: i32, _: i32) {
        self.child_ops.push(("move", window));
    }
    fn set_width(&mut self, window: Handle, _: i32) {
        self.child_ops.push(("width", window));
    }
    fn set_height(&mut self, window: Handle, _: i32) {
        self.child_ops.push(("height", window));
    }
    fn line_count(&mut self, _: Handle, _: &[u8], _: i32) -> i32 {
        2
    }
    fn grid_notification(&mut self, _: Handle, _: u32, _: &[i32]) {}
    fn sound_instance(&mut self, _: u32, _: [u32; 5]) -> Option<Handle> {
        Some(999)
    }
    fn play_instance(&mut self, _: Handle) -> bool {
        true
    }
}

#[path = "grid/owner_tests.rs"]
mod grid_owner_tests;

impl grid::GridDataForeign for Recording {
    fn object_query(&mut self, object: Handle, _: u32, output: &mut Option<Handle>) -> bool {
        if object == 99 {
            return false;
        }
        *output = Some(object + 1000);
        self.add_ref(object + 1000);
        true
    }
    fn copy_text(&mut self, _: Handle, output: &mut Vec<u8>) -> bool {
        *output = b"window".to_vec();
        true
    }
    fn window_id(&mut self, window: Handle) -> u32 {
        self.child_ops.push(("id", window));
        window as u32 + 7
    }
}

#[test]
fn grid_identity_clear_result_extra_queries_owner_draw_and_window_text() {
    use grid::*;
    let mut g = Grid::new(100, 0);
    let mut f = Recording::default();
    let mut output = Some(777);
    assert!(g.query_interface(IID, &mut output, &mut f));
    assert_eq!(output, Some(100));
    assert_eq!(f.refs, [(true, 100)]);
    assert!(!g.query_interface(0, &mut output, &mut f));
    assert!(g.clear_cell((500, 500), &mut f)); // even when nothing exists
    g.cells.insert(
        (1, 2),
        Cell {
            extra: 12,
            value: Value::Text(b"text".to_vec()),
            ..Cell::default()
        },
    );
    assert_eq!(g.cell_extra((1, 2)), 12);
    assert_eq!(g.cell_extra((0, 0)), 0);
    g.extra_mode = 42;
    assert!(!g.cell_extra_object((0, 0), IID, &mut output, &mut f));
    assert_eq!(g.extra_mode, 2);
    assert_eq!(output, Some(100)); // null data leaves output untouched
    assert!(g.cell_extra_object((1, 2), IID, &mut output, &mut f));
    assert_eq!(output, Some(1012));
    f.release(output.take().unwrap());
    g.extra = 11;
    assert!(g.extra_object(IID, &mut output, &mut f));
    assert_eq!(output, Some(1011));
    f.release(output.take().unwrap());
    g.extra = 0;
    assert!(!g.extra_object(IID, &mut output, &mut f));
    assert!(g.set_owner_draw(Some(8), &mut f));
    assert!(g.set_owner_draw(Some(8), &mut f));
    assert_eq!(&f.refs[f.refs.len() - 3..], [(true, 8), (true, 8), (false, 8)]);
    assert!(g.make_drop_down());
    assert_ne!(g.flags & 0x40000, 0);
    let mut text = b"sentinel".to_vec();
    assert!(g.cell_text_value((1, 2), &mut text, &mut f));
    assert_eq!(text, b"text");
    g.cells.insert(
        (2, 3),
        Cell {
            value: Value::Window {
                handle: 40,
                keep_width: false,
                keep_height: false,
            },
            ..Cell::default()
        },
    );
    assert!(g.cell_text_value((2, 3), &mut text, &mut f));
    assert_eq!(text, b"window");
    assert_eq!(&f.refs[f.refs.len() - 2..], [(true, 1040), (false, 1040)]);
    assert!(!g.cell_text_value((0, 0), &mut text, &mut f));
    assert_eq!(text, b"window");
    g.cells.insert(
        (3, 3),
        Cell {
            value: Value::Window {
                handle: 99,
                keep_width: false,
                keep_height: false,
            },
            ..Cell::default()
        },
    );
    assert!(!g.cell_text_value((3, 3), &mut text, &mut f));
    let mut cell = (-1, -1);
    // Direct cell fixtures also populate the independently keyed window set.
    g.windows.insert(40, ((2, 3), false, false));
    g.windows.insert(99, ((3, 3), false, false));
    assert!(g.is_cell_window(Some(40), 999, &mut cell, &mut f));
    assert_eq!(cell, (2, 3));
    assert!(f.child_ops.is_empty()); // nonnull pointer ignores id
    assert!(g.is_cell_window(None, 106, &mut cell, &mut f));
    assert_eq!(cell, (3, 3));
    assert_eq!(f.child_ops, [("id", 40), ("id", 99)]);
}

#[test]
fn grid_text_search_rectangles_lengths_null_and_order() {
    let mut g = grid::Grid::new(100, 0);
    let mut f = Recording::default();
    g.set_text((2, 0), Some(b"needle"), 0, 1, &mut f).unwrap();
    g.set_text((1, 4), Some(b"a needle b"), 0, 1, &mut f).unwrap();
    g.set_text((0, 9), Some(b"needle"), 0, 1, &mut f).unwrap();
    let mut cell = (-1, -1);
    assert_eq!(g.find_text(None, 0, (0, 0), (99, 99), &mut cell, false), Ok(false));
    assert_eq!(cell, (-1, -1));
    assert_eq!(
        g.find_text(Some(b"needle\0tail"), 0, (0, 0), (i32::MAX, 5), &mut cell, false),
        Ok(true)
    );
    assert_eq!(cell, (1, 4)); // matrix key order, not row-major display order
    assert_eq!(
        g.find_text(Some(b"needle"), -1, (0, 0), (99, 5), &mut cell, true),
        Ok(true)
    );
    assert_eq!(cell, (2, 0));
    assert_eq!(
        g.find_text(Some(b"needle"), 6, (1, 0), (1, 4), &mut cell, false),
        Ok(false)
    );
    assert_eq!(cell, (2, 0)); // failure does not overwrite
    assert_eq!(g.find_text(Some(b""), 0, (0, 0), (99, 99), &mut cell, false), Ok(true));
    assert_eq!(cell, (0, 9));
}

#[test]
fn grid_copy_axis_overlap_holes_headings_extras_and_move_order() {
    use grid::*;
    let mut g = Grid::new(100, 0);
    let mut f = Recording::default();
    for column in 0..4 {
        g.set_text((column, 0), Some(&[b'A' + column as u8]), 1, 1, &mut f)
            .unwrap();
        g.extra_maps[0].insert(column, column as u32 + 10);
        g.set_heading(0, column, &[b'a' + column as u8]);
    }
    g.copy_columns(0, 1, 3, false, &mut f).unwrap();
    let values: Vec<_> = (0..4).map(|c| g.cells[&(c, 0)].value.clone()).collect();
    assert_eq!(values, (*b"AABC").map(|b| Value::Text(vec![b])));
    assert_eq!(
        [g.extra_maps[0][&1], g.extra_maps[0][&2], g.extra_maps[0][&3]],
        [10, 11, 12]
    );
    assert_eq!(g.heading(0, 3), b"c");
    g.copy_columns(1, 0, 3, true, &mut f).unwrap();
    assert_eq!(g.cells[&(0, 0)].value, Value::Text(b"A".to_vec()));
    assert_eq!(g.cells[&(2, 0)].value, Value::Text(b"C".to_vec()));
    assert!(!g.cells.contains_key(&(3, 0)));
    assert!(!g.extra_maps[0].contains_key(&3));
    assert!(!g.headings[0].contains_key(&3));
    // A missing source row between its own min/max clears the destination.
    g.set_text((5, 0), Some(b"top"), 0, 1, &mut f).unwrap();
    g.set_text((5, 2), Some(b"bottom"), 0, 1, &mut f).unwrap();
    g.set_text((6, 1), Some(b"remove"), 0, 1, &mut f).unwrap();
    g.copy_columns(5, 6, 1, false, &mut f).unwrap();
    assert!(!g.cells.contains_key(&(6, 1)));
    // Row variant uses the orthogonal map and resets column caches.
    g.set_heading(1, 0, b"row");
    g.extra_maps[1].insert(0, 77);
    g.copy_rows(0, 4, 1, false, &mut f).unwrap();
    assert_eq!(g.heading(1, 4), b"row");
    assert_eq!(g.extra_maps[1][&4], 77);
    assert_eq!(g.cells[&(0, 4)].value, g.cells[&(0, 0)].value);
}

#[test]
fn grid_delete_axis_modes_and_default_per_cell_format_ownership() {
    use grid::*;
    let mut g = Grid::new(100, 0);
    let mut f = Recording::default();
    for column in 0..4 {
        g.set_text((column, 0), Some(&[b'A' + column as u8]), 1, 1, &mut f)
            .unwrap();
    }
    assert!(!g.delete_columns(0, -1, 0, &mut f));
    assert!(g.delete_columns(1, 0, 0, &mut f));
    assert!(g.delete_columns(1, 1, 0, &mut f));
    assert_eq!(g.cells[&(1, 0)].value, Value::Text(b"C".to_vec()));
    assert_eq!(g.cells[&(2, 0)].value, Value::Text(b"D".to_vec()));
    assert!(g.delete_columns(0, 1, 999, &mut f)); // invalid mode -> no shifting
    assert!(!g.cells.contains_key(&(0, 0)) && g.cells.contains_key(&(1, 0)));
    g.copy_rows(0, 1, 1, false, &mut f).unwrap();
    assert!(g.delete_rows(0, 1, 0, &mut f));
    assert!(g.cells.contains_key(&(1, 0)) && !g.cells.contains_key(&(1, 1)));
    assert!(!g.make_room_for_row(0, 1, 2, &mut f));
    assert!(!g.make_room_for_column(0, 1, 3, &mut f));
    let format = Format {
        font: Some(123),
        words: [7; 8],
    };
    g.dirty = false;
    assert_eq!(g.set_cell_format_state((-1, -1), &format, &mut f), Ok(true));
    assert!(!g.dirty);
    assert_eq!(g.set_cell_format_state((-1, -1), &format, &mut f), Ok(true));
    assert_eq!(&f.refs[f.refs.len() - 2..], [(true, 123), (false, 123)]);
    assert_eq!(g.set_cell_format_state((-1, 0), &format, &mut f), Ok(true));
    assert_eq!(g.set_cell_format_state((5, 5), &format, &mut f), Ok(true));
    assert!(g.dirty);
    assert_eq!(g.cells[&(5, 5)].format, Some(format));
    g.clear_cell((5, 5), &mut f);
    assert_eq!(f.refs.last(), Some(&(false, 123)));
    let no_font = Format {
        font: None,
        words: [2; 8],
    };
    g.set_cell_format_state((1, 0), &no_font, &mut f).unwrap();
    g.dirty = false;
    g.set_cell_format_state((1, 0), &no_font, &mut f).unwrap();
    assert!(!g.dirty); // no font ownership change with already allocated format
}

#[test]
fn grid_vectors_1_to_6_and_literal_selection_modes() {
    let mut g = grid::Grid::new(100, 0);
    let mut f = Recording::default();
    assert!(!g.set_default_color(6, 42));
    assert_eq!(g.default_colors, [0x4000_0000; 6]);
    assert!(g.set_default_color(5, 42));
    g.set_text((0, 99), Some(b"last"), 0, 1, &mut f).unwrap();
    g.scroll_pages(1, 2, &mut f);
    assert_eq!(g.position[1], 8);
    assert!(g.set_selection((1, 2)));
    g.set_selection_rules(2, 10);
    assert!(g.selection.is_empty());
    g.limits = [10, 10];
    g.enable_fixed_ranges(false, true);
    assert_eq!(g.limits, [-1, 10]);
    assert_eq!(
        [
            g.pause_recalculations(true),
            g.pause_recalculations(true),
            g.pause_recalculations(false),
            g.pause_recalculations(false),
            g.pause_recalculations(false)
        ],
        [1, 2, 1, 0, -1]
    );
    assert_eq!(g.get_axis_extra(0, 7), 0);
    g.enable_fixed_ranges(true, true);
    g.limits[0] = 5;
    assert!(!g.set_axis_extra(0, 7, 42));
    g.set_selection_rules(1, 10);
    assert!(g.add_selection((2, 3), true));
    assert!(!g.add_selection((4, 3), true)); // row already selected (WG5-c)
    assert!(g.add_selection((2, 3), false));
    assert!(g.selection.is_empty());
    g.set_selection_rules(0, 10);
    g.add_selection((2, 3), true);
    g.add_selection((2, 8), true);
    assert!(g.add_selection((2, 3), false));
    assert!(g.selection.is_empty());
    g.act_like_listbox(3, 1, false);
    assert_eq!(g.selection, [(0, 0)]);
    assert_eq!(g.flags & (8 | 16 | 0x4000), 0);
}

#[test]
fn grid_append_copy_window_ownership_and_place_asymmetry() {
    let mut g = grid::Grid::new(100, 0);
    let mut f = Recording {
        grid_dimensions: [310, 110],
        ..Recording::default()
    };
    g.initialized = true;
    g.visible = [3.0, 5.0];
    assert!(g.set_window((0, 0), Some(200), true, true, 1, &mut f));
    assert_eq!(f.refs, [(true, 200), (true, 200), (false, 200)]);
    assert!(f.child_ops.contains(&("area", 200)));
    f.child_ops.clear();
    g.place_all_windows(&mut f);
    assert!(!f.child_ops.contains(&("area", 200)));
    assert!(f.child_ops.contains(&("move", 200)));
    assert!(g.copy_cell((0, 0), (1, 0), true, &mut f));
    assert_eq!(g.cells[&(1, 0)].value, grid::Value::Empty);
    assert!(g.set_bitmap((2, 0), Some(300), 1, &mut f));
    assert!(g.copy_cell((2, 0), (3, 0), true, &mut f));
    assert_eq!(f.refs.iter().filter(|&&(add, h)| add && h == 300).count(), 2);
    g.set_text((i32::MAX, 4), Some(b"A"), -1, 1, &mut f).unwrap();
    assert_eq!(g.last_set, (0, 4));
    g.set_text((i32::MAX, 4), Some(b"B"), -1, 1, &mut f).unwrap();
    assert_eq!(g.last_set, (1, 4));
    g.shutdown(&mut f);
    assert_eq!(f.refs.iter().filter(|&&(add, h)| !add && h == 300).count(), 2);
    assert_eq!(
        f.child_ops
            .iter()
            .filter(|&&(op, h)| op == "remove" && h == 200)
            .count(),
        1
    );
}

#[test]
fn grid_size_cache_float_fraction_and_pause_dirty_contract() {
    let mut g = grid::Grid::new(100, 0);
    let mut f = Recording {
        grid_dimensions: [260, 110],
        ..Recording::default()
    };
    g.initialized = true;
    g.set_text((2, 0), Some(b"x"), 1, 1, &mut f).unwrap();
    let mut context = grid::LayoutContext::default();
    g.recalculate_content(&mut context, &mut f);
    assert_eq!(g.visible[0], 2.5);
    f.grid_dimensions[0] = 215;
    g.recalculate_content(&mut context, &mut f);
    assert_eq!(g.visible[0], 2.0);
    g.sizes[0].clear();
    g.set_size(0, 0, 1, 65537);
    assert_eq!(g.column_width(0), 1);
    g.dirty = false;
    g.set_cell_gutters([9; 4]);
    assert!(!g.dirty);
    g.dirty = true;
    g.pause = 1;
    g.visible = [8.0, 9.0];
    g.recalculate_content(&mut context, &mut f);
    assert_eq!(g.visible, [8.0, 9.0]);
    assert!(!g.dirty);
    context.depth = 2;
    g.dirty = true;
    g.recalculate_content(&mut context, &mut f);
    assert!(g.dirty);
    assert_eq!(context.depth, 2);
    g.sizes[0].entry(8).or_default().cached = 10;
    g.reset_cached(0, 100, i32::MAX, -1);
    assert_eq!(g.sizes[0][&8].cached, -1);
}

impl file_browser::IconIo for Recording {
    fn factory_buffer(&mut self, component: u32, iid: u32) -> Option<Handle> {
        assert_eq!((component, iid), (0xc470_d325, 0x2073_2180));
        self.icon_calls.push("factory");
        None
    }
    fn graphics_buffer(&mut self) -> Option<Handle> {
        self.icon_calls.push("graphics");
        Some(700)
    }
    fn init_buffer(&mut self, _: Handle, width: i32, height: i32) -> bool {
        assert!(matches!((width, height), (18, 17) | (84, 22)));
        self.icon_calls.push("init");
        self.buffer_ok
    }
    fn icon_strip(&mut self, id: u32) -> Option<Handle> {
        assert_eq!(id, 0xa896_3eea);
        Some(701)
    }
    fn lock_buffer(&mut self, _: Handle, flags: u32) -> bool {
        assert_eq!(flags, 0x8010);
        self.icon_calls.push("lock");
        self.lock_ok
    }
    fn copy_buffer(&mut self, _: Handle, _: Handle, from: Rect, to: Rect) {
        assert_eq!((from, to), ([36, 0, 54, 17], [0, 0, 18, 17]));
        self.icon_calls.push("copy");
    }
    fn unlock_buffer(&mut self, _: Handle) {
        self.icon_calls.push("unlock");
    }
    fn convert_rgb(&mut self, _: Handle, rgb: [u8; 3]) -> u32 {
        self.fb_rgb.push(rgb);
        self.icon_calls.push("rgb");
        0x1234
    }
    fn set_color_key(&mut self, _: Handle, color: u32) {
        assert_eq!(color, 0x1234);
        self.icon_calls.push("key");
    }
}

#[test]
fn fb_c19_patterns_are_alternatives_and_directories_bypass_them() {
    use file_browser::*;
    let mut b = FileBrowser::new(100);
    let mut f = Recording::default();
    b.flags = 12;
    b.children[5] = Some(200);
    b.filter_labels.push(b"group".to_vec());
    b.filter_patterns.push(vec![b"first".to_vec(), b"second".to_vec()]);
    f.enumerated = [vec![b"Z".to_vec(), b"A".to_vec()], vec![b"Data.TXT".to_vec()]];
    let column = |index| {
        Some(Column {
            index,
            width: 100,
            font: None,
        })
    };
    b.populate_files([None, column(1), column(2)], &[], 0, &mut f).unwrap();
    assert_eq!(f.patterns, [b"first".to_vec(), b"second".to_vec()]);
    assert_eq!(
        b.entries
            .iter()
            .map(|e| e.as_ref().unwrap().name.as_slice())
            .collect::<Vec<_>>(),
        [b"A".as_slice(), b"Z", b"Data.TXT"]
    );
    assert!(f.installed.contains(&(2, 2, b"txt".to_vec())));
    assert_eq!(b.entries[2].as_ref().unwrap().name, b"Data.TXT");
}

#[test]
fn fb_c20_c21_c25_c26_icon_loader_failure_and_ownership() {
    use file_browser::*;
    let mut b = FileBrowser::new(100);
    let mut f = Recording::default();
    assert_eq!(FileBrowser::create_default_icon(None), Ok(true));
    assert_eq!(b.load_default_icon(2, &mut f), Ok(false));
    assert_eq!(f.refs, [(false, 700)]);
    let mut output = Some(99);
    FileBrowser::create_default_button_buffer(&mut output, &mut f).unwrap();
    assert_eq!(output, None);
    assert_eq!(f.refs, [(false, 700), (false, 700)]);
    f.refs.clear();
    f.icon_calls.clear();
    f.buffer_ok = true;
    assert_eq!(b.load_default_icon(2, &mut f), Ok(true));
    assert_eq!(f.icon_calls, ["factory", "graphics", "init", "lock", "rgb", "key"]);
    assert_eq!(f.refs, [(true, 700), (false, 700)]);
    assert_eq!(b.icons[2], Some(700));
    f.lock_ok = true;
    f.icon_calls.clear();
    assert_eq!(b.load_default_icon(2, &mut f), Ok(true));
    assert_eq!(
        f.icon_calls,
        ["factory", "graphics", "init", "lock", "copy", "unlock", "rgb", "key"]
    );
}

#[test]
fn fb_c27_filter_repeats_invalidate_without_population() {
    let mut b = file_browser::FileBrowser::new(100);
    let mut f = Recording::default();
    b.filter_labels.push(b"all".to_vec());
    b.filter_combo = Some(200);
    assert!(b.select_filter(0, &mut f));
    assert!(b.select_filter(0, &mut f));
    assert!(!b.select_filter(1, &mut f));
    assert_eq!(f.combo_indices, [0, 0]);
    assert_eq!(f.invalidations, 2);
    assert!(b.refresh_work.is_empty());
}

impl Foreign for Recording {
    fn add_ref(&mut self, h: Handle) {
        self.refs.push((true, h));
    }
    fn release(&mut self, h: Handle) {
        self.refs.push((false, h));
    }
    fn base_init(&mut self, _: Handle) -> bool {
        self.base_init_ok
    }
    fn base_set_flag(&mut self, window: Handle, mask: u32, on: u8) -> Result<u32> {
        self.base_flag_calls.push((window, mask, on));
        self.base_flags = if on != 0 {
            self.base_flags | mask
        } else {
            self.base_flags & !mask
        };
        Ok(self.base_flags)
    }
    fn set_window_area(&mut self, window: Handle, area: Rect) {
        self.area_requests.push((window, area));
    }
    fn bar_skin(&mut self, bar: Handle, buffer: Handle, rect: Rect) {
        self.skins.push((bar, buffer, rect));
    }
    fn base_shutdown(&mut self, _: Handle) {}
    fn base_query(&mut self, _: Handle, _: u32, _: &mut Option<Handle>) -> bool {
        false
    }
    fn base_message(&mut self, _: Handle, m: Message) -> bool {
        self.base_messages.push(m);
        false
    }
    fn invalidate(&mut self, _: Handle) {
        self.invalidations += 1;
    }
    fn show(&mut self, _: Handle, _: bool) {}
    fn capture(&mut self, h: Handle, take: bool) {
        self.capture_owner = if take { Some(h) } else { None };
    }
    fn convert_payload(&mut self, _: Handle, value: i32) -> i32 {
        value
    }
    fn deliver(&mut self, _: Handle, d: Delivery, m: Message) -> bool {
        self.messages.push((d, m));
        false
    }
    fn drop_complete(&mut self, _: Handle, kind: u32, row: i32) {
        self.messages.push((
            Delivery::Send,
            Message {
                kind,
                words: [row, 0, 0, 0],
            },
        ));
    }
    fn default_interface(&mut self, key: u32) -> Option<Handle> {
        self.defaults.push(key);
        (!self.no_default_font).then_some(100)
    }
    fn default_color(&mut self, key: u32) -> Option<u32> {
        self.defaults.push(key);
        Some(0)
    }
    fn create(&mut self, _: Resource) -> Option<Handle> {
        Some(200)
    }
    fn font_height(&mut self, _: Handle) -> i32 {
        self.font_height
    }
    fn buffer_height(&mut self, _: Handle) -> i32 {
        self.buffer_height
    }
    fn bar_value(&mut self, _: Handle) -> i32 {
        self.bar
    }
    fn bar_range(&mut self, _: Handle, _: i32, _: i32, value: i32) {
        self.bar = value;
    }
    fn place_bar(&mut self, _: Handle, _: Rect) {}
    fn create_bar(&mut self, _: Handle) -> Option<Handle> {
        Some(300)
    }
    fn destroy_bar(&mut self, _: Handle) {}
    fn timer_elapsed(&mut self, _: Handle) -> u32 {
        self.elapsed
    }
    fn restart_timer(&mut self, _: Handle) {
        self.elapsed = 0;
    }
    fn double_click_interval(&mut self) -> u32 {
        self.interval
    }
    fn begins_with(&mut self, text: &[u8], prefix: &[u8], arg: u32) -> bool {
        assert_eq!(arg, 1);
        text.starts_with(prefix)
    }
    fn text_equal(&mut self, left: &[u8], right: &[u8]) -> bool {
        left == right
    }
    fn draw(&mut self, draw: Draw) {
        self.draws.push(draw);
    }
    fn compare_text(&mut self, a: &[u8], b: &[u8]) -> std::cmp::Ordering {
        self.compare_calls += 1;
        a.cmp(b)
    }
    fn fit_bytes(&mut self, _: Handle, text: &[u8], _: i32) -> usize {
        self.fit_counts.pop_front().unwrap_or(text.len())
    }
    fn parent_height(&mut self, _: Handle) -> i32 {
        self.parent_limit.unwrap_or(i32::MAX)
    }
    fn capture_owner(&mut self) -> Option<Handle> {
        self.capture_owner
    }
    fn bar_dispatch(&mut self, bar: Handle, up: bool, x: i32, y: i32) -> bool {
        self.bar_events.push((bar, up, x, y));
        self.capture_owner = self.dispatch_capture;
        true
    }
}

fn list(drop_mode: bool) -> (ListBox, Recording) {
    let mut f = Recording {
        font_height: 10,
        interval: 500,
        ..Recording::default()
    };
    let mut list = ListBox::new(1000, drop_mode, &mut f);
    list.area = [10, 20, 110, 62];
    list.font = Some(100);
    list.recompute_metrics(&mut f);
    for id in [7, 7, 9] {
        list.insert(-2, format!("row{id}").into_bytes(), id, &mut f);
    }
    (list, f)
}

#[test]
fn wc_l01_query_views_and_null_retention() {
    let (list, mut f) = list(false);
    for _view in [list.window, list.window + LIST_OFFSET] {
        let mut output = None;
        assert!(list.query_interface(IID, Some(&mut output), &mut f));
        assert_eq!(output, Some(list.window + LIST_OFFSET));
        f.release(list.window);
    }
    assert!(!list.query_interface(IID, None, &mut f));
    assert_eq!(f.refs, [(true, 1000), (false, 1000), (true, 1000), (false, 1000)]);
    assert_eq!((LIST_SLOTS, WINDOW_SLOTS), (66, 169));
}

#[test]
fn wc_l02_l06_selection_indices_ids_and_insertions() {
    for drop_mode in [false, true] {
        let (mut list, mut f) = list(drop_mode);
        list.select(1, false, &mut f);
        for _ in 0..2 {
            assert!(list.select(99, true, &mut f));
        }
        assert_eq!(list.selected, 2);
        assert_eq!(f.messages.len(), if drop_mode { 0 } else { 2 });
        for (_, m) in &f.messages {
            assert_eq!(
                *m,
                Message {
                    kind: 3,
                    words: [SELECTION as i32, 2, 0, 0]
                }
            );
        }
        assert!(list.select_id(7, false, &mut f));
        assert_eq!(list.selected, 0);
        let invalidations = f.invalidations;
        assert!(!list.select_id(99, true, &mut f));
        assert_eq!(f.invalidations, invalidations);
        assert_eq!(list.selected_id(), 7);
        list.select(1, false, &mut f);
        list.insert(0, b"new".to_vec(), 20, &mut f);
        assert_eq!(list.selected, 2);
        assert!(list.replace_text(-2, b"last".to_vec(), &mut f));
        assert_eq!(list.text(3), Some(b"last".as_slice()));
    }
    let mut f = Recording::default();
    let mut empty = ListBox::new(1, false, &mut f);
    empty.select(-1, true, &mut f);
    assert_eq!((empty.selected, empty.selected_id()), (-1, 0));
    assert!(empty.entries.is_empty());
}

#[test]
fn wc_l07_l09_signed_metrics_hit_geometry() {
    let (mut list, mut f) = list(false);
    assert_eq!((list.row_height, list.whole_rows), (10, 4));
    assert!(list.resize_whole_lines(&mut f));
    assert_eq!(list.height(), 42);
    for n in 3..8 {
        list.insert(-2, vec![], n, &mut f);
    }
    list.first = 2;
    assert_eq!(list.hit_test(-999, 10, false), Ok(2));
    assert_eq!(list.hit_test(999, 11, false), Ok(3));
    assert_eq!(list.hit_test(0, -30, false), Ok(-1));
    list.y_gutter = -1;
    list.recompute_metrics(&mut f);
    assert_eq!(list.whole_rows, 4);
    list.font = None;
    list.recompute_metrics(&mut f);
    assert_eq!((list.row_height, list.whole_rows), (0, 0));
    assert!(!list.resize_whole_lines(&mut f));
    assert!(matches!(list.hit_test(0, 1, false), Err(Unsupported::Precondition(_))));
}

#[test]
fn wc_l10_l13_color_and_reference_quirks() {
    let (mut list, mut f) = list(false);
    let n = f.invalidations;
    assert!(list.set_font_color(1, 17, &mut f));
    assert_eq!(f.invalidations, n);
    assert!(list.set_font_color(1, 17, &mut f));
    assert_eq!(f.invalidations, n + 1);
    assert!(!list.set_font_color(3, 17, &mut f));
    let mut output = 99;
    assert!(!list.font_color(0, &mut output));
    assert_eq!(output, 99);
    assert!(list.font_color(1, &mut output));
    assert_eq!(output, 17);
    list.background = Some(101);
    list.border = Some(102);
    list.set_font(Some(100), &mut f);
    list.set_background(Some(101), &mut f);
    list.set_border(Some(102), &mut f);
    list.set_font(None, &mut f);
    list.set_background(None, &mut f);
    list.set_border(None, &mut f);
    assert_eq!(
        f.refs,
        [
            (true, 100),
            (false, 100),
            (true, 101),
            (false, 101),
            (true, 102),
            (false, 102),
            (false, 100),
            (false, 101),
            (false, 102)
        ]
    );
    list.set_flat(true, &mut f);
    list.set_three_d(true, &mut f);
    assert!(!list.flat && list.three_d);
}

#[test]
fn wc_l14_l15_delivery_and_key_protocols() {
    let (mut list, mut f) = list(false);
    assert!(list.emit(SELECTION, &mut f));
    list.delivery = Delivery::Post;
    assert!(list.emit(SELECTION, &mut f));
    assert_eq!(f.messages[0].0, Delivery::Send);
    assert_eq!(f.messages[1].0, Delivery::Post);
    assert_eq!(f.messages[0].1, f.messages[1].1);
    assert!(!list.key_up(42, 17, &mut f));
    assert_eq!(
        f.messages.last().unwrap().1,
        Message {
            kind: 6,
            words: [42, 17, 0, 0]
        }
    );
    let count = f.messages.len();
    list.drop_mode = true;
    assert!(list.key_up(42, 17, &mut f));
    assert_eq!(f.messages.len(), count);
    assert_eq!(list.key_down(13, 0, &mut f), Ok(true));
    assert_eq!(f.messages.last().unwrap().1.kind, 15);
    assert_eq!(list.key_down(27, 0, &mut f), Ok(true));
    assert_eq!(f.messages.last().unwrap().1.words[0], -1);
}

#[test]
fn wc_f01_storage_removal_is_not_selection() {
    let (mut list, mut f) = list(false);
    list.select(1, false, &mut f);
    assert_eq!(list.entries.len(), 3);
    assert!(list.remove_storage(1));
    assert_eq!(list.entries.len(), 2);
    assert_eq!(list.selected, 1);
    assert_eq!(list.selected_id(), 9);
}

#[test]
fn list_mouse_timer_strictness_and_no_font_background() {
    let (mut list, mut f) = list(false);
    list.captured = true;
    list.previous_click = 0;
    f.elapsed = 500;
    list.mouse_down(0, 1, &mut f).unwrap();
    assert!(!list.pending_double_click);
    f.elapsed = 499;
    list.mouse_down(0, 1, &mut f).unwrap();
    assert!(list.pending_double_click);
    list.mouse_up(&mut f).unwrap();
    assert_eq!(f.messages.last().unwrap().1.words[0], ACTIVATION as i32);
    assert_eq!(list.previous_click, -1);
    list.font = None;
    list.paint(&mut f).unwrap();
    assert_eq!(f.draws.len(), 1);
    assert!(matches!(f.draws[0], Draw::Fill { .. }));
}

impl tree_view::TreeForeign for Recording {
    fn graphics_available(&mut self) -> bool {
        !self.no_graphics
    }
    fn tree_area(&mut self, _: Handle) -> Option<Rect> {
        self.tree_area
    }
    fn tree_bar_thickness(&mut self, _: Handle) -> [u32; 2] {
        self.tree_thickness
    }
    fn resize_tree_buffer(&mut self, _: Handle, size: [u32; 2]) {
        self.tree_resizes.push(size);
    }
    fn access_mode(&mut self, _: Handle, mode: u32, take: bool) -> bool {
        self.access_calls.push((mode, take));
        !take || self.fail_mode != mode
    }
    fn initialize_image(&mut self, _: Handle, slot: usize, _: [u32; 2]) -> bool {
        self.image_attempts.push(slot);
        self.fail_image_slot != Some(slot)
    }
    fn control_down(&mut self) -> bool {
        self.control
    }
    fn drag_request(&mut self, _: Handle, accept: &mut u8) -> bool {
        *accept = self.drag_accept;
        self.drag_return
    }
    fn run_drag(&mut self, t: &mut tree_view::TreeView, session: &tree_view::DragSession) {
        self.drag_runs += 1;
        if let Some([x, y]) = self.drag_pointer {
            t.notify_drag(x, y, &session.nodes, self).unwrap();
        } else if let Some(h) = self.drag_target {
            t.select(h, true, self);
        }
    }
    fn convert_color(&mut self, _: Handle, color: u32) -> u32 {
        color
    }
    fn initialize_buffer(&mut self, _: Handle) -> bool {
        !self.fail_working_buffer
    }
    fn buffer_access(&mut self, _: Handle, _: bool) -> bool {
        true
    }
    fn measure_text(&mut self, _: Handle, text: &[u8]) -> i32 {
        self.measure_override.unwrap_or(text.len() as i32 * 8)
    }
    fn draw_image(&mut self, image: Handle, rect: Rect) {
        self.draws.push(Draw::Blit { source: image, rect });
    }
    fn set_bar_value(&mut self, _: Handle, value: i32) {
        self.bar = value;
    }
}

fn descriptor(parent: i32, text: &[u8]) -> tree_view::Descriptor {
    tree_view::Descriptor {
        parent,
        reference: -2,
        text: text.to_vec(),
        data: 0,
        images: [-1; 4],
        font: None,
        colors: [0, 1, 2, 3],
        expanded: false,
    }
}

fn tree() -> (tree_view::TreeView, Recording, i32, i32) {
    let mut f = Recording {
        font_height: 10,
        ..Recording::default()
    };
    let mut tree = tree_view::TreeView::new(1000);
    assert!(tree.init(&mut f));
    tree.viewport_rows = 4;
    let a = tree.insert_at(descriptor(0, b"A"), 0, &mut f);
    let b = tree.insert_at(descriptor(0, b"B"), 1, &mut f);
    f.messages.clear();
    (tree, f, a, b)
}

#[test]
fn wc_t01_t03_identity_initialization_and_expansion() {
    use tree_view::{COLLAPSE, EXPAND, IID, PRIMARY_SLOTS, WINDOW_SLOTS};
    let (mut tree, mut f, a, _) = tree();
    assert_eq!(tree.window(), tree.primary + 4);
    assert_eq!((PRIMARY_SLOTS, WINDOW_SLOTS), (60, 151));
    assert_eq!(tree.nodes.len(), 3);
    let requests = f.defaults.len();
    assert!(tree.init(&mut f));
    assert_eq!(f.defaults.len(), requests);
    assert_eq!(tree.root, Some(0));
    assert!(tree.nodes[&0].expanded && tree.nodes[&0].force_branch);
    assert!(!tree.nodes[&0].selected);
    let mut output = None;
    assert_eq!(tree.query_interface(IID, Some(&mut output), &mut f), Ok(true));
    assert_eq!(output, Some(tree.primary));
    assert!(matches!(
        tree.query_interface(IID, None, &mut f),
        Err(Unsupported::Precondition(_))
    ));
    assert!(tree.node(999).is_none());
    let n = f.invalidations;
    assert!(!tree.set_expanded(999, true, &mut f));
    assert_eq!(f.invalidations, n);
    assert!(tree.set_expanded(a, true, &mut f));
    assert!(tree.set_expanded(a, true, &mut f));
    assert!(tree.set_expanded(a, false, &mut f));
    assert_eq!(
        f.messages.iter().map(|(_, m)| m.words[0] as u32).collect::<Vec<_>>(),
        [EXPAND, COLLAPSE]
    );
}

#[test]
fn wc_t04_t06_selection_and_ancestor_visibility() {
    let (mut tree, mut f, a, b) = tree();
    assert!(tree.select(a, false, &mut f));
    assert!(tree.select(b, true, &mut f));
    assert!(!tree.deselect(999, &mut f));
    assert_eq!(tree.selection, [b]);
    assert!(!tree.nodes[&a].selected && tree.nodes[&b].selected);
    assert_eq!(f.messages.len(), 2);
    for (_, m) in &f.messages {
        assert_eq!(m.words[..2], [tree_view::SELECTION as i32, 0]);
    }
    tree.clear_selection(&mut f);
    f.messages.clear();
    tree.flags |= 0x10;
    tree.select(a, false, &mut f);
    tree.select(b, false, &mut f);
    assert_eq!(tree.selection, [a, b]);
    assert!(tree.clear_selection(&mut f));
    let n = f.messages.len();
    assert!(tree.clear_selection(&mut f));
    assert_eq!(f.messages.len(), n);
    let child = tree.insert_at(descriptor(a, b"child"), 0, &mut f);
    assert!(!tree.get_visible(child));
    f.messages.clear();
    assert!(tree.make_visible(child, &mut f));
    assert!(tree.nodes[&a].expanded);
    assert!(tree.get_visible(child));
    assert!(tree.selection.is_empty());
    assert_eq!(f.messages[0].1.words[..2], [tree_view::EXPAND as i32, a]);
}

#[test]
fn wc_t07_t09_wheel_sign_zero_and_missing_bar() {
    let (mut tree, mut f, _, _) = tree();
    for i in 0..7 {
        tree.insert_at(descriptor(0, &[b'C' + i]), i as usize + 2, &mut f);
    }
    tree.recalculate();
    assert_eq!(tree.visible.len(), 10);
    tree.first = 2;
    tree.vertical_bar = Some(300);
    assert!(tree.wheel(-1, &mut f));
    assert_eq!(f.bar, 3);
    assert!(tree.wheel(-1000, &mut f));
    assert_eq!(f.bar, 4);
    tree.first = 2;
    assert!(tree.wheel(0, &mut f));
    assert_eq!(f.bar, 1);
    tree.vertical_bar = None;
    assert!(tree.wheel(-100, &mut f));
    assert!(tree.wheel(100, &mut f));
    assert_eq!(f.bar, 1);
}

#[test]
fn wc_t10_activation_sequence_and_branch_toggle() {
    let (mut tree, mut f, a, b) = tree();
    tree.insert_at(descriptor(a, b"child"), 0, &mut f);
    tree.flags |= 0x10;
    tree.select(a, false, &mut f);
    tree.select(b, false, &mut f);
    f.messages.clear();
    assert_eq!(tree.key_down(13, &mut f), Ok(true));
    assert_eq!(
        f.messages
            .iter()
            .map(|(_, m)| (m.words[0] as u32, m.words[1]))
            .collect::<Vec<_>>(),
        [
            (tree_view::ACTIVATION, a),
            (tree_view::EXPAND, a),
            (tree_view::ACTIVATION, b),
        ]
    );
    assert_eq!(tree.selection, [a, b]);
    assert!(!tree.key_up(13, 0));
}

#[test]
fn wc_t11_bar_messages_always_delegate_to_base() {
    let (mut tree, mut f, _, _) = tree();
    tree.horizontal_bar = Some(301);
    tree.vertical_bar = Some(302);
    for (sub, value) in [(0x1756_bac7_u32, 17), (0x1756_bac6, 4), (0, 99)] {
        f.bar = value;
        let m = Message {
            kind: 3,
            words: [0x8871_13a3_u32 as i32, sub as i32, 0, 0],
        };
        assert_eq!(tree.do_message(m, &mut f), Ok(false));
        assert_eq!(*f.base_messages.last().unwrap(), m);
    }
    assert_eq!((tree.horizontal_position, tree.first), (17, 4));
    assert_eq!(f.base_messages.len(), 3);
}

#[test]
fn wc_t12_missing_image_does_not_suppress_text() {
    let (mut tree, mut f, a, _) = tree();
    assert_eq!(tree.nodes[&a].images, [-1; 4]);
    assert_eq!(
        tree.draw_item(a, [10, 0, 50, 16], [0, 0, 9, 16], [0; 4], &mut f),
        Ok(true)
    );
    assert!(!f.draws.iter().any(|draw| matches!(draw, Draw::Blit { .. })));
    assert!(f
        .draws
        .iter()
        .any(|draw| matches!(draw, Draw::Text { text, .. } if text == b"A")));
    assert_eq!(tree.nodes[&a].text_rect, [10, 0, 50, 16]);
}

#[test]
fn tree_font_growth_shutdown_and_settled_ordering() {
    let (mut tree, mut f, a, _) = tree();
    f.font_height = 30;
    f.refs.clear();
    assert!(tree.set_font(a, Some(999), &mut f));
    assert_eq!(tree.row_height, 30);
    assert!(f.refs.is_empty());
    f.font_height = 5;
    tree.set_font(a, None, &mut f);
    assert_eq!(tree.row_height, 30);
    assert_eq!(tree.insert(descriptor(999, b"missing"), &mut f), Ok(-1));
    let count = tree.nodes.len();
    assert!(tree.insert(descriptor(0, b"settled"), &mut f).unwrap() >= 0);
    assert_eq!(tree.nodes.len(), count + 1);
    tree.images = vec![Some(400), None, Some(401)];
    tree.horizontal_bar = Some(301);
    tree.vertical_bar = Some(302);
    tree.shutdown(&mut f);
    assert!(tree.nodes.is_empty() && tree.visible.is_empty() && tree.selection.is_empty());
    assert_eq!(
        f.refs,
        [
            (false, 400),
            (false, 401),
            (false, 100),
            (false, 200),
            (false, 301),
            (false, 302)
        ]
    );
}

impl file_browser::BrowserForeign for Recording {
    fn directory_exists(&mut self, _: &[u8]) -> bool {
        self.existing_checks += 1;
        self.exists
    }
    fn create_directory(&mut self, path: &[u8]) -> bool {
        self.paths.push(path.to_vec());
        self.create_success
    }
    fn split_directory(&mut self, _: &[u8], capacity: usize, bytes: usize) -> (i32, Vec<u8>) {
        assert_eq!((capacity, bytes), (2, 260));
        (self.split_count, self.parent_path.clone())
    }
    fn split_file(&mut self, path: &[u8]) -> [Vec<u8>; 4] {
        let split = path.iter().rposition(|&b| b == b'\\').map_or(0, |i| i + 1);
        [Vec::new(), path[..split].to_vec(), path[split..].to_vec(), Vec::new()]
    }
    fn path_separator(&mut self, byte: u8) -> bool {
        matches!(byte, b'\\' | b'/')
    }
    fn path_extension(&mut self, path: &[u8]) -> Vec<u8> {
        path.iter()
            .rposition(|&b| b == b'.')
            .map_or_else(Vec::new, |i| path[i..].to_vec())
    }
    fn lowercase(&mut self, bytes: &mut [u8]) {
        bytes.make_ascii_lowercase();
    }
    fn right_trim(&mut self, bytes: &mut Vec<u8>) {
        while bytes.last().is_some_and(u8::is_ascii_whitespace) {
            bytes.pop();
        }
    }
    fn window_id(&mut self, _: Handle) -> u32 {
        17
    }
    fn set_window_flag(&mut self, _: Handle, mask: u32, value: bool) {
        self.window_flags.push((mask, value));
    }
    fn notify_path(&mut self, id: u32, win: u32, path: &[u8], fourth: u32) {
        assert_eq!((id, win, fourth), (0x7abe_7205, 17, 0));
        self.paths.push(path.to_vec());
    }
    fn notify_selection(
        &mut self,
        id: u32,
        win: u32,
        descriptor: file_browser::SelectionDescriptor,
        selection: &file_browser::Selection,
        fourth: u32,
    ) {
        assert_eq!((win, fourth), (17, 0));
        self.selections.push((id, descriptor, selection.clone()));
    }
    fn set_editor_text(&mut self, _: Handle, text: &[u8]) {
        self.editor_texts.push(text.to_vec());
    }
    fn grid_last_row(&mut self, _: Handle) -> u32 {
        self.grid_last
    }
    fn grid_text(&mut self, _: Handle, _: i32, row: i32) -> Option<Vec<u8>> {
        self.grid_reads += 1;
        self.grid_texts.get(&row).cloned()
    }
    fn grid_scroll_to(&mut self, _: Handle, row: i32) {
        self.bar = row;
    }
    fn grid_selected(&mut self, _: Handle, _: i32) -> bool {
        false
    }
    fn grid_select(&mut self, _: Handle, row: i32) {
        self.bar = row;
    }
    fn child_window(&mut self, control: Handle, accessor: u32) -> Handle {
        control + u64::from(accessor)
    }
    fn remove_child(&mut self, parent: Handle, child: Handle) {
        self.removed.push((parent, child));
    }
    fn set_button_buffer(&mut self, button: Handle, buffer: Option<Handle>) {
        self.button_updates.push((button, buffer));
    }
    fn font_fit(&mut self, _: Handle, _: &[u8], width: i32, option: u32) -> usize {
        assert_eq!(option, 0);
        self.fit_widths.push(width);
        self.fit_counts.pop_front().expect("fit count fixture")
    }
    fn text_resource(&mut self, id: u32) -> Option<Vec<u8>> {
        assert_eq!(id, 0x5377_be31);
        self.marker.clone()
    }
    fn font_rect(&mut self, _: Handle, _: &[u8]) -> Rect {
        [0, 0, self.marker_width, 10]
    }
}

#[test]
fn fb_c01_c05_identity_failed_navigation_and_ordered_refresh_work() {
    use file_browser::*;
    let mut f = Recording::default();
    let mut browser = FileBrowser::new(1000);
    assert!(browser.children.iter().all(Option::is_none));
    assert_eq!(browser.flags, 0xd);
    assert_eq!((PRIMARY_SLOTS, WINDOW_SLOTS), (107, 151));
    let mut out = None;
    assert!(browser.query_interface(IID, &mut out, &mut f));
    assert_eq!(out, Some(1000));
    assert_eq!(f.refs, [(true, 1000)]);
    browser.current = b"old\\".to_vec();
    assert!(!browser.set_current_directory(b"missing\\", &mut f));
    assert_eq!(browser.current, b"old\\");
    assert!(browser.refresh_work.is_empty());
    f.split_count = 1;
    assert!(!browser.go_to_parent(&mut f));
    let n = f.existing_checks;
    f.split_count = 2;
    f.parent_path = b"parent\\".to_vec();
    assert!(browser.go_to_parent(&mut f));
    assert_eq!(f.existing_checks, n + 1);
    browser.fallback = b"also-missing\\".to_vec();
    let n = f.existing_checks;
    assert!(browser.update_path(&mut f));
    assert_eq!(f.existing_checks, n + 1);
    assert_eq!(f.paths.last().unwrap(), b"also-missing\\");
    assert_eq!(browser.refresh_work, [Refresh::DirectoryList, Refresh::FileList]);
}

#[test]
fn fb_c06_c09_grid_lookup_wraparound_and_ambiguous_zero() {
    use file_browser::*;
    let mut browser = FileBrowser::new(1);
    let mut f = Recording::default();
    assert_eq!(browser.get_row_of_entry_by_name(b"a", &mut f), 0);
    assert_eq!(f.grid_reads, 0);
    browser.children[5] = Some(500);
    assert_eq!(browser.get_row_of_entry_by_name(b"a", &mut f), 0);
    browser.name_column = Some(0);
    f.grid_last = 3;
    f.grid_texts.insert(1, b"a".to_vec());
    f.grid_texts.insert(3, b"a".to_vec());
    assert_eq!(browser.get_row_of_entry_by_name(b"a", &mut f), 1);
    f.grid_texts.insert(0, b"a".to_vec());
    assert_eq!(browser.get_row_of_entry_by_name(b"a", &mut f), 0);
    for last in [u32::MAX - 1, u32::MAX] {
        f.grid_last = last;
        f.grid_reads = 0;
        assert_eq!(browser.get_row_of_entry_by_name(b"absent", &mut f), 0);
        assert_eq!(f.grid_reads, 1024);
    }
    f.grid_last = 0x8000_0000;
    f.grid_reads = 0;
    assert_eq!(browser.get_row_of_entry_by_name(b"a", &mut f), 0);
    assert_eq!(f.grid_reads, 0);
}

#[test]
fn fb_c10_c12_flag_precedence_and_any_bit_add() {
    let mut browser = file_browser::FileBrowser::new(1);
    let mut f = Recording::default();
    browser.flags = 1;
    browser.add_flags(5, &mut f);
    assert_eq!(browser.flags, 1);
    assert_eq!(f.existing_checks, 0);
    browser.add_flags(4, &mut f);
    assert_eq!(browser.flags, 5);
    assert_eq!(f.existing_checks, 1);
    browser.refresh_work.clear();
    browser.set_flags(3, &mut f);
    assert!(browser.refresh_work.is_empty());
    assert_eq!(f.window_flags, [(0x20000, false), (0x40000, false), (0x10000, true)]);
    f.window_flags.clear();
    browser.clear_flags(2, &mut f);
    assert_eq!(f.window_flags, [(0x20000, true), (0x40000, true)]);
    assert_eq!(browser.any_flag(5), 1);
}

#[test]
fn fb_c14_c18_creation_selection_and_double_click() {
    use file_browser::*;
    let mut browser = FileBrowser::new(1);
    let mut f = Recording {
        exists: true,
        ..Recording::default()
    };
    browser.current = b"C:\\".to_vec();
    assert!(!browser.create_new_directory(&mut f));
    assert_eq!(f.existing_checks, 256);
    assert_eq!(f.paths.last().unwrap(), b"C:\\New Folder (256)\\");
    assert!(browser.refresh_work.is_empty());
    browser.entries.push(Some(Entry {
        tag: CUSTOM_FILE,
        id: 42,
        name: b"old.txt".to_vec(),
    }));
    browser.children[6] = Some(400);
    assert_eq!(browser.selection_change(0, &mut f), Ok(true));
    let descriptor = browser.descriptor();
    assert_eq!(descriptor.id, 42);
    assert_eq!(f.selections[0].0, 0x7abe_7203);
    assert_eq!(f.editor_texts, [b"old.txt".to_vec()]);
    browser.selected_row = 7;
    assert_eq!(browser.selection_double_click(0, &mut f), Ok(true));
    assert_eq!(browser.selected_row, 7);
    assert_eq!(f.selections[1].0, 0x7abe_7204);
    assert!(browser.set_selection(b"C:\\missing.txt", &mut f));
    assert_eq!(browser.selection.id, 42);
    assert_eq!(browser.selection.kind, 2);
    assert_eq!(browser.borrowed_string(descriptor.strings[0]), b"C:\\missing.txt");
    assert_eq!(f.selections.len(), 2);
    assert!(browser.file_grid_double_click(0, &mut f));
    assert!(!browser.file_grid_double_click(u32::MAX, &mut f));
}

#[test]
fn fb_c23_c24_child_enable_and_reference_assignment() {
    use file_browser::*;
    let mut browser = FileBrowser::new(1);
    let mut f = Recording::default();
    browser.set_icon(0, Some(10), &mut f);
    browser.set_icon(0, Some(11), &mut f);
    browser.set_icon(0, Some(11), &mut f);
    browser.set_icon(7, Some(12), &mut f);
    assert_eq!(f.refs, [(true, 10), (true, 11), (false, 10)]);
    browser.children[9] = Some(100);
    browser.set_button_buffer(1, Some(20), &mut f);
    browser.set_button_buffer(1, None, &mut f);
    browser.set_button_buffer(1, None, &mut f);
    browser.set_button_buffer(u32::MAX, Some(99), &mut f);
    assert_eq!(f.button_updates, [(100, Some(20)), (100, None)]);
    browser.children[6] = Some(600);
    browser.children[5] = Some(500);
    browser.enable_child(FIRST_CHILD_ID + 6, 0, &mut f);
    assert_eq!(browser.children[6], None);
    assert_eq!(f.removed, [(5, 620)]);
    browser.enable_child(FIRST_CHILD_ID + 6, 2, &mut f);
    assert_eq!(browser.children[6], None);
    assert_eq!(browser.enabled[6], 2);
    browser.enable_child(FIRST_CHILD_ID + 5, 0, &mut f);
    assert_eq!(browser.children[5], Some(500));
}

#[test]
fn browser_compare_full_tags_byte_order_secondary_once_and_fit_boundary() {
    use file_browser::*;
    let mut f = Recording::default();
    let entry = |tag, text: &[u8]| Entry {
        tag,
        id: 0,
        name: text.to_vec(),
    };
    let p = entry(PARENT, b"..");
    assert!(precedes(&p, &p, 0, 0, 0, &mut f));
    assert!(precedes(
        &entry(FILE, b"file10"),
        &entry(FILE, b"file2"),
        0,
        0,
        0,
        &mut f
    ));
    assert!(!precedes(&entry(FILE, b"A"), &entry(FILE, b"a"), 0, 0, 0, &mut f));
    assert!(!precedes(&entry(FILE, b"a"), &entry(FILE, b"b"), 99, 0, 0, &mut f));
    assert!(!precedes(
        &entry(0xaaa8_8503, b"a"),
        &entry(FILE, b"b"),
        0,
        0,
        0,
        &mut f
    ));
    assert!(precedes(&entry(DIRECTORY, b"z"), &entry(FILE, b"a"), 0, 0, 0, &mut f));
    assert!(!precedes(&entry(DIRECTORY, b"z"), &entry(FILE, b"a"), 0, 0, 2, &mut f));
    let mut text = b"some text".to_vec();
    fit_string(&mut text, -10, None, &mut f).unwrap();
    assert_eq!(text, b"some text");
    f.fit_counts.push_back(1);
    f.marker_width = 12;
    fit_string(&mut text, 12, Some(100), &mut f).unwrap();
    assert!(text.is_empty());
    text = b"ab   tail".to_vec();
    f.fit_counts.extend([3, 5]);
    fit_string(&mut text, 13, Some(100), &mut f).unwrap();
    assert_eq!(text, b"ab...");
    assert_eq!(*f.fit_widths.last().unwrap(), 1);
}

#[test]
fn browser_parent_directory_ids_and_shutdown_reference_order() {
    use file_browser::*;
    let mut browser = FileBrowser::new(1);
    let mut f = Recording::default();
    assert_eq!(browser.raw_words[&0x224], 20);
    assert_eq!(browser.raw_words[&0x1f8], u32::MAX);
    assert_eq!(browser.style_words[0][6], 0);
    assert_eq!(browser.raw_bytes[&0x1dc], 1);
    browser.entries = vec![Some(Entry {
        tag: DIRECTORY,
        id: 42,
        name: b"folder".to_vec(),
    })];
    browser.selection_change(0, &mut f).unwrap();
    assert_eq!(browser.selection.id, 0);
    assert_eq!(browser.selection.leaf, b"folder\\");
    browser.entries[0].as_mut().unwrap().tag = PARENT;
    browser.selection_change(0, &mut f).unwrap();
    assert_eq!(browser.selection.kind, 0);
    for (i, child) in browser.children.iter_mut().enumerate() {
        *child = Some(100 + i as u64);
    }
    browser.common_font = Some(200);
    for (i, icon) in browser.icons.iter_mut().enumerate() {
        *icon = Some(300 + i as u64);
    }
    for (i, buffer) in browser.button_buffers.iter_mut().enumerate() {
        *buffer = Some(400 + i as u64);
    }
    browser.extra_reference = Some(500);
    browser.style_fonts = [Some(600), Some(601), Some(602), Some(603)];
    browser.custom_groups = vec![vec![CustomEntry {
        id: 0,
        interface: Some(700),
        text: vec![],
    }]];
    browser.filter_labels = vec![b"All files".to_vec()];
    browser.filter_index = 9;
    browser.shutdown(&mut f);
    let mut released = vec![700];
    released.extend(100..113);
    released.push(200);
    released.extend(300..307);
    released.extend(400..404);
    released.extend([500, 600, 601, 603]);
    assert_eq!(f.refs, released.into_iter().map(|h| (false, h)).collect::<Vec<_>>());
    assert_eq!(browser.style_fonts[2], Some(602));
    assert_eq!(browser.filter_index, 0);
    assert_eq!(browser.filter_labels, [b"All files".to_vec()]);
    assert!(browser.entries.is_empty());
    let n = f.refs.len();
    browser.shutdown(&mut f);
    assert_eq!(f.refs.len(), n);
}

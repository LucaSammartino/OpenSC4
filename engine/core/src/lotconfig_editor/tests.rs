//! Expected values transcribed from lot-configuration-editor Â§Â§1â€“4, 8â€“12, 16â€“17
//! before the implementation. These fixtures never use game files.
use super::*;

#[derive(Default)]
struct Fake {
    events: Vec<Effect>,
    modal: u32,
    absent: Vec<u32>,
    text: String,
    integers: std::collections::BTreeMap<u32, i32>,
    selections: std::collections::BTreeMap<u32, Vec<(u32, u32)>>,
    prompt: u32,
    persist: bool,
    selection: Option<(u32, String)>,
    provider_key_result: Option<[u32; 3]>,
    read_result: Option<Configuration>,
    cells: std::collections::BTreeMap<(u32, u32), String>,
}
impl Host for Fake {
    fn register_configuration(&mut self, _: &Configuration, _: bool) {}
    fn effect(&mut self, e: Effect) {
        self.events.push(e);
    }
    fn create_dialog(&mut self, _: DialogKind) -> Option<u64> {
        Some(10)
    }
    fn has_control(&mut self, _: u64, id: u32) -> bool {
        !self.absent.contains(&id)
    }
    fn run_modal(&mut self, _: u64) -> u32 {
        self.modal
    }
    fn text(&mut self, _: u64, _: u32) -> String {
        self.text.clone()
    }
    fn integer(&mut self, _: u64, id: u32) -> i32 {
        *self.integers.get(&id).unwrap_or(&0)
    }
    fn selected_rows(&mut self, _: u64, id: u32) -> Vec<(u32, u32)> {
        self.selections.get(&id).cloned().unwrap_or_default()
    }
    fn prompt(&mut self, _: &str, _: u32) -> u32 {
        self.prompt
    }
    fn persist(&mut self, _: &Identity, _: &Configuration) -> Result<bool> {
        Ok(self.persist)
    }
    fn parse_hex(&mut self, text: &str) -> Result<u32> {
        Ok(u32::from_str_radix(text, 16).unwrap())
    }
    fn parse_unsigned(&mut self, text: &str) -> Result<u32> {
        Ok(text.parse().unwrap())
    }
    fn data_directory(&mut self) -> Result<String> {
        Ok("C:\\data".into())
    }
    fn browser_selection(&mut self, _: u64) -> Option<(u32, String)> {
        self.selection.clone()
    }
    fn directory_exists(&mut self, path: &str) -> bool {
        path.eq_ignore_ascii_case("C:\\lots")
    }
    fn providers(&mut self) -> Result<Vec<u64>> {
        Ok(vec![20, 21])
    }
    fn query_provider(&mut self, p: u64) -> Option<u64> {
        Some(p + 10)
    }
    fn provider_directory(&mut self, _: u64) -> String {
        "c:\\LOTS".into()
    }
    fn provider_key(&mut self, _: u64, _: &str, _: &str) -> Option<[u32; 3]> {
        self.provider_key_result
    }
    fn read_configuration(&mut self, _: [u32; 3]) -> Result<Option<Configuration>> {
        Ok(self.read_result.take())
    }
    fn grid_text(&mut self, _: u64, _: u32, row: u32, column: u32) -> Result<String> {
        Ok(self.cells.get(&(row, column)).cloned().unwrap_or_default())
    }
    fn grid_interval(&mut self, _: u64, _: u32) -> Option<(u32, u32)> {
        Some((2, 4))
    }
    fn control_parent(&mut self, _: u64, _: u32) -> Option<u64> {
        Some(11)
    }
}
fn object(kind: u8, rect: [f32; 4]) -> LotConfigObject {
    // Editor offset labels deliberately differ from the shared record names.
    LotConfigObject {
        kind,
        offset: [4.0, 9.0, 4.0],
        max_x: rect[0],
        max_z: rect[1],
        min_x: rect[2],
        min_z: rect[3],
        ..Default::default()
    }
}
fn editor() -> (Editor, Fake) {
    let mut e = Editor::default();
    let mut h = Fake::default();
    e.create_new(&mut h);
    h.events.clear();
    (e, h)
}

#[test]
fn separator_rows_and_direct_flags() {
    assert_eq!(edit_action(3), None);
    assert_eq!(edit_action(9), None);
    assert_eq!(edit_action(12), Some(Command::AddProp(true, true)));
    assert_eq!(edit_action(24), Some(Command::Help(0)));
    let (mut e, mut h) = editor();
    let id = e.insert(object(1, [2.0, 2.0, 6.0, 6.0]), false);
    e.record_mut(id).unwrap().flags = 0xFF;
    e.object_action(id, 8, &mut h).unwrap();
    assert_eq!(e.record(id).unwrap().flags, 0x11);
    assert_eq!(e.dirty, 0);
    assert!(h.events.is_empty());
}

#[test]
fn pointer_identity_and_deletion_do_not_normalize_state() {
    let (mut e, mut h) = editor();
    let a = e.insert(object(0, [0.0; 4]), false);
    let b = e.insert(object(1, [0.0; 4]), false);
    e.set_selection(Some(b), &mut h);
    e.drag = true;
    e.capturing = true;
    e.set_selection(Some(b), &mut h);
    assert!(e.drag && e.capturing);
    e.delete(b, false, &mut h).unwrap();
    assert_eq!((e.selected, e.selected_index, e.dirty), (None, 1, 0));
    assert!(e.drag && e.capturing);
    e.set_selection(Some(999), &mut h);
    assert_eq!((e.selected, e.selected_index), (Some(999), 0));
    assert!(!e.drag && !e.capturing);
    assert_eq!(e.record(a).unwrap().kind, 0);
}

#[test]
fn duplicate_candidates_front_selection_and_cleanup_containment() {
    let (mut e, mut h) = editor();
    let a = e.insert(object(1, [10.0, 10.0, 14.0, 14.0]), false);
    let b = e.duplicate(a, &mut h).unwrap().unwrap();
    assert_eq!(bounds(e.record(b).unwrap()), [6.0, 6.0, 10.0, 10.0]);
    assert_eq!(e.record(b).unwrap().offset, [0.0, 9.0, 0.0]);
    assert_eq!(e.selected, Some(b));
    assert_eq!(e.config().unwrap().objects[0].id, b);
    e.insert(object(4, [-2.0, 1.0, 2.0, 3.0]), false);
    e.insert(object(2, [-20.0; 4]), false);
    e.config_mut().unwrap().fences = vec![[0, 0, 255, 255], [0, 0, 2, 0], [0, 0, 1, 1]];
    assert_eq!(e.cleanup(&mut h).unwrap(), 2);
    assert_eq!(e.config().unwrap().fences, vec![[0, 0, 255, 255], [0, 0, 1, 1]]);
}

#[test]
fn fence_order_wrap_and_first_foundation() {
    let (mut e, mut h) = editor();
    e.toggle_fence_pixels(0, -16, &mut h).unwrap();
    assert_eq!(e.config().unwrap().fences, vec![[0, 255, 0, 0]]);
    assert_eq!(h.events, vec![Effect::Draw, Effect::Invalidate, Effect::Title]);
    h.events.clear();
    e.dirty = 0;
    e.toggle_fence([0, 255, 0, 0], &mut h).unwrap();
    assert!(e.config().unwrap().fences.is_empty());
    assert_eq!(e.dirty, 0);
    let first = e.insert(object(3, [0.0; 4]), false);
    let second = e.insert(object(3, [0.0; 4]), false);
    e.record_mut(second).unwrap().foundation = 42;
    assert_eq!(e.foundation(), 0);
    e.add_foundation(7, true).unwrap();
    assert!(e.record(first).is_none());
    assert_eq!(e.foundation(), 42);
}

#[test]
fn name_missing_control_cancel_and_title_twice() {
    let (mut e, mut h) = editor();
    h.modal = ACCEPT;
    h.absent.push(TEXT);
    assert!(e.name_dialog(&mut h).unwrap());
    assert_eq!(e.dirty, 0);
    h.absent.clear();
    h.events.clear();
    assert!(e.name_dialog(&mut h).unwrap());
    assert_eq!(e.config().unwrap().name, "<unnamed>");
    assert_eq!(h.events.iter().filter(|x| **x == Effect::Title).count(), 2);
    h.modal = 0;
    h.text = "changed".into();
    assert!(!e.name_dialog(&mut h).unwrap());
    assert_eq!(e.config().unwrap().name, "<unnamed>");
}

#[test]
fn path_growth_without_dirty_and_slope_limit_precedence() {
    let (mut e, mut h) = editor();
    let id = e.insert(object(7, [0.0; 4]), false);
    e.record_mut(id).unwrap().ids = vec![9, 8];
    h.modal = ACCEPT;
    h.text = "ab".into();
    e.network_path_dialog(id, &mut h).unwrap();
    assert_eq!(e.record(id).unwrap().ids, vec![9, 8, 0, 171]);
    assert_eq!(e.dirty, 0);
    h.events.clear();
    h.text = "100".into();
    e.slope_dialog(&mut h).unwrap();
    assert!(h.events.contains(&Effect::Message(
        "Slope is outside min/max values for this lot".into(),
        0
    )));
    assert!(!h
        .events
        .contains(&Effect::Message("Slope limited to 80 degrees".into(), 0)));
}

#[test]
fn compatibility_clears_hidden_bits_and_truncates_before_clamp() {
    let (mut e, mut h) = editor();
    e.config_mut().unwrap().compatibility = [vec![true; 24], vec![true; 8], vec![true; 12]];
    e.config_mut().unwrap().checkboxes = 255;
    h.modal = ACCEPT;
    h.integers.insert(NUMERIC[3], 258);
    h.selections.insert(GRIDS[0], vec![(2, 17), (2, 1)]);
    e.compatibility_dialog(&mut h).unwrap();
    assert_eq!(e.config().unwrap().compatibility[0].iter().filter(|x| **x).count(), 1);
    assert!(e.config().unwrap().compatibility[0][3]);
    assert_eq!(e.config().unwrap().minimum, 2);
    assert!(!h.events.contains(&Effect::Draw));
    h.absent.push(GRIDS[1]);
    assert_eq!(e.compatibility_dialog(&mut h), Err(Unsupported::MissingAcceptedGrid(1)));
}

#[test]
fn save_failure_and_close_cancel_preserve_state() {
    let (mut e, mut h) = editor();
    e.identity.key[0] = 1;
    e.dirty = 7;
    assert!(!e.save(false, false, &mut h).unwrap());
    assert_eq!(e.dirty, 7);
    assert!(h.events.is_empty());
    h.persist = true;
    assert!(e.save(false, true, &mut h).unwrap());
    assert_eq!(e.dirty, 0);
    assert_eq!(h.events, vec![Effect::Title, Effect::Saved(e.identity.clone())]);
    e.dirty = 1;
    h.prompt = 0x53018147;
    h.events.clear();
    assert!(!e.close(true, true, &mut h).unwrap());
    assert!(h.events.is_empty());
}

#[test]
fn hit_priority_uses_selected_then_vector_then_popup() {
    let (mut e, mut h) = editor();
    let a = e.insert(object(1, [0.0; 4]), false);
    let b = e.insert(object(1, [0.0; 4]), false);
    e.set_selection(Some(b), &mut h);
    let rects = [(a, [0, 0, 9, 9]), (b, [0, 0, 9, 9])];
    assert_eq!(
        e.hit_candidates([2, 2], 0, Some([0, 0, 9, 9]), &rects, None),
        Hit::Object(b, 101)
    );
    e.selected = None;
    assert_eq!(e.hit_candidates([2, 2], 0, None, &rects, None), Hit::Object(a, 100));
    assert_eq!(e.hit_candidates([2, 2], 2, None, &rects, Some(-1)), Hit::Cancelled);
    assert_eq!(e.hit_candidates([2, 2], 2, None, &rects, Some(1)), Hit::Object(b, 101));
}

#[test]
fn duplicate_third_and_untested_final_candidates() {
    let (mut e, mut h) = editor();
    let id = e.insert(object(2, [0.0, 10.0, 14.0, 14.0]), false);
    let new = e.duplicate(id, &mut h).unwrap().unwrap();
    assert_eq!(bounds(e.record(new).unwrap()), [0.0, 6.0, 14.0, 10.0]);
    let id = e.insert(object(2, [0.0, 0.0, 32.0, 32.0]), false);
    let new = e.duplicate(id, &mut h).unwrap().unwrap();
    assert_eq!(bounds(e.record(new).unwrap()), [-8.0, 4.0, 24.0, 36.0]);
    assert_eq!(e.record(new).unwrap().offset[1], 9.0);
}

#[test]
fn unordered_containment_and_independent_mover_tests() {
    let mut o = object(1, [f32::NAN, 1.0, 5.0, 8.0]);
    assert!(contained(&o, [1, 1]));
    o.min_z = 17.0;
    assert!(!contained(&o, [1, 1]));
    o = object(1, [2.0, -4.0, f32::NAN, -1.0]);
    move_within(&mut o, [1, 1], true);
    assert!(o.min_x.is_nan());
    assert_eq!(o.min_z.to_bits(), f32::from_bits(0x3D4CCCCD).to_bits());
    assert_eq!(o.max_z, -2.95);
}

#[test]
fn save_as_copy_restores_identity_but_not_dirty_or_title_and_releases_slots() {
    let (mut e, mut h) = editor();
    e.identity = Identity {
        id: 9,
        key: [1, 2, 9],
        path: "C:\\lots\\old.dat".into(),
        references: [Some(80), Some(80)],
    };
    e.config_mut().unwrap().id = 9;
    let old = e.identity.clone();
    e.dirty = 3;
    e.chooser = true;
    h.modal = ACCEPT;
    h.persist = true;
    h.selection = Some((2, "C:\\lots\\new.dat".into()));
    h.provider_key_result = Some([1, 2, 99]);
    assert!(e.save_as(true, false, true, &mut h).unwrap());
    assert_eq!(
        (e.identity.clone(), e.config().unwrap().id, e.dirty),
        (old.clone(), 9, 0)
    );
    assert_eq!(h.events.iter().filter(|v| **v == Effect::Title).count(), 1);
    assert!(!h.events.contains(&Effect::Release(80)));
    let saved = h.events.iter().position(|e| matches!(e, Effect::Saved(_))).unwrap();
    let gone = h.events.iter().position(|e| *e == Effect::RemoveDialog(10)).unwrap();
    assert!(gone < saved);
    assert!(h.events.contains(&Effect::RefreshChooser));
    h.events.clear();
    e.dirty = 4;
    assert!(e.save_as(false, false, true, &mut h).unwrap());
    assert_eq!(e.identity.id, 99);
    assert_eq!(h.events.iter().filter(|v| **v == Effect::Release(80)).count(), 2);
}

#[test]
fn save_as_failures_rollback_and_cancellation_never_install() {
    let (mut e, mut h) = editor();
    e.identity.key = [1, 2, 7];
    e.identity.id = 7;
    e.config_mut().unwrap().id = 7;
    e.dirty = 6;
    h.modal = ACCEPT;
    h.selection = Some((2, "C:\\lots\\new.dat".into()));
    h.provider_key_result = Some([1, 2, 99]);
    assert!(!e.save_as(false, false, false, &mut h).unwrap());
    assert_eq!((e.identity.id, e.config().unwrap().id, e.dirty), (7, 7, 6));
    assert!(!h.events.contains(&Effect::Title));
    h.events.clear();
    h.modal = 0;
    h.persist = true;
    assert!(!e.save_as(false, true, true, &mut h).unwrap());
    assert_eq!(e.identity.id, 7);
    assert!(!h.events.iter().any(|e| matches!(e, Effect::Saved(_))));
    h.modal = ACCEPT;
    h.selection = Some((1, String::new()));
    assert!(!e.save_as(false, false, false, &mut h).unwrap());
    assert!(h.events.contains(&Effect::Message("You must select a file.".into(), 0)));
}

#[test]
fn read_failure_preserves_current_and_success_stops_at_explicit_dependency() {
    let (mut e, mut h) = editor();
    e.config_mut().unwrap().name = "old".into();
    assert!(!e.read([1, 2, 3], &mut h).unwrap());
    assert_eq!(e.config().unwrap().name, "old");
    h.read_result = Some(Configuration {
        name: "loaded".into(),
        ..Default::default()
    });
    assert!(matches!(e.read([1, 2, 3], &mut h), Err(Unsupported::Dependency(_))));
    assert_eq!(e.config().unwrap().name, "loaded");
    assert!(h.events.is_empty());
    assert!(matches!(e.open([0; 3], &mut h), Err(Unsupported::Dependency(_))));
    assert_eq!(e.config().unwrap().name, "loaded");
}

#[test]
fn selector_duplicate_ids_preselection_current_text_append_and_cancel() {
    let mut h = Fake {
        modal: ACCEPT,
        ..Default::default()
    };
    let raw = vec![
        CatalogRow {
            id: 7,
            family: false,
            name: Some("individual".into()),
            image: None,
        },
        CatalogRow {
            id: 7,
            family: true,
            name: Some("family".into()),
            image: None,
        },
    ];
    let rows = selector_rows(SelectorKind::Building, &raw, [true, true]).unwrap();
    assert_eq!(
        rows,
        vec![
            (7, Cell::Text("Family: family".into())),
            (7, Cell::Text("Bldg: individual".into()))
        ]
    );
    h.selections.insert(0xE9264E71, vec![(1, 9), (1, 0), (0, 0)]);
    h.cells.insert((1, 0), "ab".into());
    h.cells.insert((0, 0), "0".into());
    let mut output = vec![99];
    assert!(selector_dialog(SelectorKind::Building, &rows, Some(&[7]), &mut output, &mut h).unwrap());
    assert_eq!(output, vec![99, 171, 171]);
    assert_eq!(
        h.events
            .iter()
            .filter(|e| matches!(e, Effect::GridSelect(_, _, _)))
            .count(),
        2
    );
    assert!(h.events.contains(&Effect::Sort(0xE9264E71, 0, false, 1, 0)));
    h.modal = 0;
    assert!(!selector_dialog(SelectorKind::Building, &rows, None, &mut output, &mut h).unwrap());
    assert_eq!(output, vec![99, 171, 171]);
}

#[test]
fn callback_sorts_raw_options_and_scrolls_only_prop_first_byte() {
    let mut h = Fake::default();
    assert!(dialog_callback(10, 3, 0x287259F6, 0xE9432DD8, [false, true], &mut h).unwrap());
    assert!(h.events.contains(&Effect::Sort(0xE9264E71, 1, true, 1, 0)));
    h.cells.insert((2, 1), "abc".into());
    h.cells.insert((3, 1), "(A)".into());
    h.cells.insert((4, 1), "Alpha".into());
    assert!(dialog_callback(10, 5, 0x41, 0, [false; 2], &mut h).unwrap());
    assert!(h.events.contains(&Effect::Scroll(0x097CEFB1, 4)));
    assert!(!dialog_callback(10, 5, 0x61, 0, [false; 2], &mut h).unwrap());
}

#[test]
fn capture_keyboard_condition_equality_wrap_and_fence_mode() {
    let (mut e, mut h) = editor();
    e.capturing = true;
    e.drag = true;
    e.mode = 8;
    assert!(e.key_down(0x1B, 0, false, &mut h).unwrap());
    assert!(e.drag);
    assert_eq!(e.mode, 8);
    assert!(h.events.is_empty());
    e.capturing = false;
    e.key_down(0x1B, 0, false, &mut h).unwrap();
    assert!(!e.drag);
    assert_eq!(e.mode, 0);
    e.condition = 7;
    e.cycle_condition(&mut h);
    assert_eq!(e.condition, 8);
    e.condition = 255;
    e.cycle_condition(&mut h);
    assert_eq!(e.condition, 0);
    h.events.clear();
    e.placement_mode(8, &mut h).unwrap();
    e.placement_mode(8, &mut h).unwrap();
    assert_eq!(h.events, vec![Effect::Cursor(6)]);
}

#[test]
fn fence_preserves_extended_product_low_byte_and_rejects_invalid_conversion() {
    let (mut e, mut h) = editor();
    e.scale = f32::from_bits(0x4A800001);
    // (2^31-1)*(2^23+1)/32 = 562950020268031 + 31/32.
    // The signed-64 conversion truncates to an integer whose low byte is 255.
    e.toggle_fence_pixels(0, i32::MAX, &mut h).unwrap();
    assert_eq!(e.config().unwrap().fences, vec![[0, 255, 0, 0]]);
    e.scale = f32::NAN;
    assert_eq!(e.toggle_fence_pixels(0, 0, &mut h), Err(Unsupported::NumericConversion));
}

#[test]
fn redraw_helper_remains_distinct_from_direct_draw_and_invalidate() {
    let (mut e, mut h) = editor();
    let id = e.insert(object(0, [1.0, 1.0, 7.0, 7.0]), false);
    e.record_mut(id).unwrap().foundation = 5;
    e.object_action(id, 2, &mut h).unwrap();
    assert_eq!(h.events, vec![Effect::Title, Effect::RedrawRequired]);
    h.events.clear();
    e.object_action(id, 2, &mut h).unwrap();
    assert!(h.events.is_empty());
    e.set_selection(Some(id), &mut h);
    e.key_down(0x24, 0, false, &mut h).unwrap();
    assert_eq!(h.events, vec![Effect::Title, Effect::RedrawRequired]);
}

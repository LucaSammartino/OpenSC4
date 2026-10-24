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
    directory: Option<Vec<u8>>,
    directories: Vec<Vec<u8>>,
    popup: Option<i32>,
    provider_queries: Vec<(Vec<u8>, Vec<u8>)>,
    messages: Vec<(String, String, (u32, u32, u32))>,
    manager: Option<std::rc::Rc<crate::lot_config_manager::LotConfigManager>>,
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
    fn message_box(&mut self, text: &str, title: &str, options: (u32, u32, u32)) -> u32 {
        self.messages.push((text.into(), title.into(), options));
        self.prompt
    }
    fn configuration_manager(&self) -> Result<&crate::lot_config_manager::LotConfigManager> {
        self.manager
            .as_deref()
            .ok_or(Unsupported::Dependency("configuration manager acquisition"))
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
    fn directory_exists(&mut self, path: &[u8]) -> bool {
        self.directories.push(path.to_vec());
        true
    }
    fn popup(&mut self, _: &[String]) -> i32 {
        self.popup.unwrap_or(-1)
    }
    fn providers(&mut self) -> Result<Vec<u64>> {
        Ok(vec![20, 21])
    }
    fn query_provider(&mut self, p: u64) -> Option<u64> {
        Some(p + 10)
    }
    fn provider_directory(&mut self, _: u64) -> Vec<u8> {
        self.directory.clone().unwrap_or_else(|| b"c:\\LOTS\\".to_vec())
    }
    fn provider_key(&mut self, _: u64, name: &[u8], ext: &[u8]) -> Option<[u32; 3]> {
        self.provider_queries.push((name.into(), ext.into()));
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

// Settled LE6 independent vectors (§24.4), using only portable fixtures.
#[test]
fn lei06_constructor_and_editor_new_defaults() {
    let c = Configuration::default();
    assert_eq!(c.numeric, [90.0, 90.0, 0.0]);
    assert_eq!(c.capacities, [65535, 0]);
    assert_eq!((c.minimum, c.byte42, c.checkboxes), (1, 0, 8));
    assert_eq!(c.size, [0, 0]);
    assert!(c.name.is_empty());
    let (e, _) = editor();
    for c in [&c, e.config().unwrap()] {
        assert_eq!(c.compatibility.each_ref().map(Vec::len), [16, 4, 9]);
        assert!(c.compatibility.iter().flatten().all(|bit| !bit));
        assert_eq!((c.minimum, c.byte42, c.checkboxes), (1, 0, 8));
    }
    assert_eq!(e.config().unwrap().name, "New Lot");
    assert_eq!(e.config().unwrap().numeric, [5.0, 90.0, 0.0]);
    assert_eq!(e.config().unwrap().size, [1, 1]);
}

#[test]
fn lei07_fractional_initial_values_and_ignored_water() {
    let (mut e, mut h) = editor();
    e.config_mut().unwrap().numeric = [5.75, -2.75, 0.75];
    e.config_mut().unwrap().byte42 = 7;
    h.modal = ACCEPT;
    for (id, value) in NUMERIC.into_iter().zip([5, -2, 0, 1]) {
        h.integers.insert(id, value);
    }
    h.integers.insert(0x294053DB, 99);
    assert_eq!(e.compatibility_dialog(&mut h), Ok(true));
    for (id, value) in NUMERIC.into_iter().zip([5, -2, 0, 1]) {
        assert!(h.events.contains(&Effect::Seed(10, id, Value::Integer(value))));
    }
    assert!(h.events.contains(&Effect::Seed(10, 0x294053DB, Value::Integer(7))));
    assert_eq!(e.config().unwrap().numeric, [5.0, -2.0, 0.0]);
    assert_eq!(e.config().unwrap().byte42, 7);
    assert_ne!(e.dirty, 0);
    // Signed 64-bit truncation followed by a low-dword transfer, not i32 clamp.
    h.modal = 0;
    h.events.clear();
    e.config_mut().unwrap().numeric[0] = 4294967296.0;
    assert_eq!(e.compatibility_dialog(&mut h), Ok(false));
    assert!(h.events.contains(&Effect::Seed(10, NUMERIC[0], Value::Integer(0))));
}

#[test]
fn lei08_decoder_bounds_are_not_editor_normalized() {
    let o =
        crate::lot_config_manager::object_from_values(&[1, 0, 0, 0, 0, 0, 4 << 16, 5 << 16, 12 << 16, 13 << 16], None)
            .unwrap();
    assert_eq!([o.min_x, o.min_z, o.max_x, o.max_z], [4.0, 5.0, 12.0, 13.0]);
    assert_eq!(bounds(&o), [12.0, 13.0, 4.0, 5.0]);
}

#[test]
fn lei09_10_provider_original_parts_and_literal_separators() {
    let mut h = Fake {
        provider_key_result: Some([1, 2, 3]),
        ..Default::default()
    };
    assert!(identity_from_path("C:\\Lots\\a.b.SC4Lot", &mut h).unwrap().is_some());
    assert_eq!(h.provider_queries, [(b"a.b".to_vec(), b".SC4Lot".to_vec())]);
    assert_eq!(h.directories, [b"C:\\Lots\\".to_vec()]);
    h.provider_queries.clear();
    // Directory existence may succeed, but slash spelling alone differs.
    h.directory = Some(b"C:/Lots/".to_vec());
    assert!(identity_from_path("C:\\Lots\\a.b.SC4Lot", &mut h).unwrap().is_none());
    assert!(h.provider_queries.is_empty());
    for (path, name, ext) in [(".SC4Lot", "", ".SC4Lot"), ("a.", "a", "."), ("a", "a", "")] {
        let p = split_identity_path(path);
        assert_eq!(p.name, name.as_bytes());
        assert_eq!(p.extension, ext.as_bytes());
    }
    let p = split_identity_path("C:/dir.with.dot/a");
    assert_eq!([p.drive, p.directory].concat(), b"C:/dir.with.dot/");
    assert!(p.extension.is_empty());
    assert_eq!(split_identity_path(&"a".repeat(300)).name.len(), 255);
}

#[test]
fn lei11_c_locale_lowercase_is_byte_equality() {
    assert_eq!(lowercase_directory("AÉ".as_bytes(), true).unwrap(), "aÉ".as_bytes());
    assert_eq!(lowercase_directory("aé".as_bytes(), true).unwrap(), "aé".as_bytes());
    assert_ne!(
        lowercase_directory("AÉ".as_bytes(), true),
        lowercase_directory("aé".as_bytes(), true)
    );
    assert!(lowercase_directory(b"ABC", false).is_err());
    // Framework converter truncates supplementary characters to a single word,
    // and stops at malformed input, unlike Rust's decoder/replacement behavior.
    assert_eq!(
        lowercase_directory("A😀".as_bytes(), true).unwrap(),
        "a\u{f600}".as_bytes()
    );
    assert_eq!(lowercase_directory(b"ABC\xe9", true).unwrap(), b"abc");
}

#[test]
fn lei12_signed_slope_and_padded_path_initial_text() {
    let (mut e, mut h) = editor();
    e.slope = u32::MAX;
    e.slope_dialog(&mut h).unwrap();
    assert!(h.events.contains(&Effect::Seed(10, TEXT, Value::Text("-1".into()))));
    for ids in [vec![0, 0, 0, 0xab], vec![]] {
        h.events.clear();
        let mut o = object(7, [0.0; 4]);
        o.ids = ids.clone();
        let id = e.insert(o, false);
        e.network_path_dialog(id, &mut h).unwrap();
        let text = if ids.is_empty() { "00000000" } else { "000000ab" };
        assert!(h.events.contains(&Effect::Seed(10, TEXT, Value::Text(text.into()))));
        assert_eq!(e.record(id).unwrap().ids, ids);
    }
    assert_eq!(e.slope, u32::MAX);
}

#[test]
fn lei13_verification_message_options_and_no_mutation() {
    let (mut e, mut h) = editor();
    e.insert(object(1, [20.0, 20.0, 22.0, 22.0]), false);
    let before = e.current.clone();
    for (answer, accepted) in [(0x5301814A, true), (0, false), (ACCEPT, false)] {
        h.prompt = answer;
        assert_eq!(e.verify_bounds(false, true, &mut h), Ok(accepted));
        assert_eq!(e.current, before);
        assert_eq!(e.dirty, 0);
    }
    assert!(h.messages.iter().all(|m| m
        == &(
            "One or more lot objects don't fit within the lot bounds. Do you want to continue or cancel?".into(),
            "New Lot".into(),
            (1, 1, 0)
        )));
    assert_eq!(h.messages.len(), 3);
}

#[test]
fn lei14_read_executes_fix_removing_invalid_ids_and_records() {
    let (mut e, mut h) = editor();
    h.manager = Some(super::fix_test_host::manager());
    let mut c = Configuration::default();
    let mut prop = object(1, [4.0, 5.0, 12.0, 13.0]);
    prop.ids = vec![99];
    c.objects.push(Record {
        id: 77,
        data: prop,
        creation_known: true,
    });
    h.read_result = Some(c);
    assert_eq!(e.read([1, 2, 3], &mut h), Ok(true));
    assert!(e.config().unwrap().objects.is_empty());
    assert_eq!(e.dirty, 1);
    assert_eq!(
        h.events,
        [
            Effect::Title,
            Effect::ReconciliationWarning {
                introduction: "This lot has components that have changed since the last edit.",
                reason: "The family has changed.",
            }
        ]
    );
}

#[test]
fn lei15_read_zero_preserves_existing_dirty_and_no_warning() {
    let (mut e, mut h) = editor();
    h.manager = Some(super::fix_test_host::manager());
    e.dirty = 7;
    h.read_result = Some(Configuration::default());
    assert_eq!(e.read([1, 2, 3], &mut h), Ok(true));
    assert_eq!(e.dirty, 7);
    assert!(h.events.is_empty());
}

#[test]
fn settled_fence_placeholder_uses_current_title_without_edits() {
    let (mut e, mut h) = editor();
    h.popup = Some(0);
    let before = e.current.clone();
    e.fence_menu(&mut h).unwrap();
    assert_eq!(h.messages, [(
        "Sorry, the fence options menu isn't implemented yet and won't be implemented until fence functionality is in place.".into(),
        "New Lot".into(), (0, 1, 0))]);
    assert_eq!(e.current, before);
    assert_eq!(e.dirty, 0);
    assert_eq!(h.events, [Effect::Focus]);
}

#[test]
fn read_bridge_retains_ids_applies_manager_bounds_and_first_wall_only() {
    let (mut e, mut h) = editor();
    h.manager = Some(super::fix_test_host::manager());
    let mut c = Configuration {
        size: [2, 2],
        retaining_walls: vec![99, 98, 10],
        ..Default::default()
    };
    let mut prop = object(1, [0.0; 4]);
    prop.ids = vec![10];
    prop.offset = [8.0, 0.0, 10.0];
    prop.orientation = 1;
    c.objects.push(Record {
        id: 55,
        data: prop,
        creation_known: true,
    });
    h.read_result = Some(c);
    assert_eq!(e.read([1, 2, 3], &mut h), Ok(true));
    let r = &e.config().unwrap().objects[0];
    assert_eq!(r.id, 1);
    assert_eq!(r.data.ids, [10]);
    // Manager writes its raw minima/maxima; editor roles remain reversed.
    assert_eq!(
        [r.data.min_x, r.data.min_z, r.data.max_x, r.data.max_z],
        [6.0, 9.0, 10.0, 11.0]
    );
    assert_eq!(bounds(&r.data), [10.0, 11.0, 6.0, 9.0]);
    assert_eq!(e.config().unwrap().retaining_walls, [98, 10]);
    assert!(matches!(
        h.events.last(),
        Some(Effect::ReconciliationWarning {
            reason: "The footprint has changed.",
            ..
        })
    ));
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

#[test]
fn add_texture_appends_selects_old_first_and_preserves_reversed_default_bounds() {
    let (mut e, mut h) = editor();
    e.current.as_mut().unwrap().size = [2; 2];
    let a = e.insert(object(0, [0.0; 4]), false);
    let b = e.insert(object(1, [0.0; 4]), false);
    e.mode = 5;
    e.pending_ids = vec![77];
    let t = e.add_object_at([16.0, 16.0], 123, None, &mut h).unwrap().unwrap();
    assert_eq!(
        e.current
            .as_ref()
            .unwrap()
            .objects
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        vec![a, b, t]
    );
    assert_eq!(e.selected, Some(a));
    let texture = e.record(t).unwrap();
    assert_eq!(bounds(texture), [24.0, 24.0, 8.0, 8.0]);
    assert_eq!(
        (
            texture.kind,
            texture.flags,
            texture.orientation,
            texture.mirror,
            texture.tag
        ),
        (2, 0, 0, 0, 123)
    );
    assert_eq!(texture.ids, vec![77]);
    for mode in [1, 2, 3, 5] {
        e.mode = mode;
        e.pending_ids.clear();
        assert_eq!(e.add_object_at([1.0; 2], 0, None, &mut h), Ok(None));
    }
    e.mode = 4;
    assert!(e.add_object_at([1.0; 2], 0, None, &mut h).is_err());
    for (mode, kind) in [(6, 5), (7, 6)] {
        e.mode = mode;
        let id = e.add_object_at([8.0; 2], 0, None, &mut h).unwrap().unwrap();
        assert_eq!(e.record(id).unwrap().kind, kind);
        assert_eq!(e.selected, Some(id));
    }
}

#[test]
fn selector_commit_cancel_confirmation_and_known_mouse_hit_branches() {
    let (mut e, mut h) = editor();
    let id = e.insert(object(1, [1.0; 4]), false);
    let before = e.record(id).unwrap().clone();
    e.selector_commit(id, None, &mut h).unwrap();
    assert_eq!(e.record(id), Some(&before));
    assert!(h.events.is_empty());
    e.selector_commit(id, Some((vec![3, 3], [1.0, 2.0, 3.0, 4.0])), &mut h)
        .unwrap();
    assert_eq!(e.record(id).unwrap().ids, vec![3, 3]);
    assert_eq!(h.events, vec![Effect::Title, Effect::RedrawRequired]);
    h.prompt = 0x53018149;
    assert_eq!(e.delete_with_name(id, "prop", &mut h), Ok(false));
    assert!(e.record(id).is_some());
    e.set_selection(Some(id), &mut h);
    h.events.clear();
    assert_eq!(e.edit_mouse_down_hit(0, None, [0; 2], &mut h), Ok(true));
    assert_eq!(e.selected, None);
    assert_eq!(h.events, vec![Effect::Draw, Effect::Invalidate]);
    e.set_selection(Some(id), &mut h);
    h.events.clear();
    e.edit_mouse_down_hit(3, None, [0; 2], &mut h).unwrap();
    assert_eq!(e.selected, None);
    assert_eq!(h.events, vec![Effect::RedrawRequired]);
    e.mode = 1;
    h.events.clear();
    e.edit_mouse_down_hit(2, None, [0; 2], &mut h).unwrap();
    assert!(h.events.is_empty());
}

#[test]
fn size_request_missing_controls_wrap_and_cancel() {
    let (mut e, mut h) = editor();
    h.modal = ACCEPT;
    h.absent.push(0x0926ECD9);
    h.integers.insert(0xE926ED2E, 3);
    assert_eq!(e.size_dialog_request(&mut h), Ok(Some([1, 4])));
    h.integers.insert(0xE926ED2E, -1);
    assert_eq!(e.size_dialog_request(&mut h), Ok(Some([1, 0])));
    h.modal = 0;
    assert_eq!(e.size_dialog_request(&mut h), Ok(None));
}

#[test]
fn remaining_finite_movement_style_and_pixel_store_vectors() {
    for (mask, step) in [(0, 0.25), (1, 1.0), (2, 0.5), (3, 2.0), (4, 2.0), (7, 16.0)] {
        let (mut e, mut h) = editor();
        e.current.as_mut().unwrap().size = [10; 2];
        let id = e.insert(object(1, [2.0, 2.0, 6.0, 6.0]), false);
        e.move_key(id, 0x27, mask, false, &mut h).unwrap();
        assert_eq!(e.record(id).unwrap().offset[0], 4.0 + step);
    }
    for (kind, rgb) in [
        (0, [0, 0, 128]),
        (1, [128, 0, 128]),
        (2, [0, 128, 128]),
        (3, [0, 72, 72]),
        (4, [0, 128, 0]),
        (255, [128; 3]),
    ] {
        assert_eq!(drawing_style(kind), (rgb, 0.5));
    }
    // 16,777,217 rounds to 16,777,216 before subtracting the origin.
    assert_eq!(pixel_units([16_777_217, -2], 1.0, [16_777_216.0, 1.0]), [0.0, -3.0]);
}

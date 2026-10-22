//! Expected values transcribed from lot-configuration-editor §§1–4, 8–12, 16–17
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
}
impl Host for Fake {
    fn effect(&mut self, e: Effect) { self.events.push(e); }
    fn create_dialog(&mut self, _: DialogKind) -> Option<u64> { Some(10) }
    fn has_control(&mut self, _: u64, id: u32) -> bool { !self.absent.contains(&id) }
    fn run_modal(&mut self, _: u64) -> u32 { self.modal }
    fn text(&mut self, _: u64, _: u32) -> String { self.text.clone() }
    fn integer(&mut self, _: u64, id: u32) -> i32 { *self.integers.get(&id).unwrap_or(&0) }
    fn selected_rows(&mut self, _: u64, id: u32) -> Vec<(u32, u32)> {
        self.selections.get(&id).cloned().unwrap_or_default()
    }
    fn prompt(&mut self, _: &str, _: u32) -> u32 { self.prompt }
    fn persist(&mut self, _: &Identity, _: &Configuration) -> Result<bool> { Ok(self.persist) }
    fn parse_hex(&mut self, text: &str) -> Result<u32> { Ok(u32::from_str_radix(text, 16).unwrap()) }
    fn parse_unsigned(&mut self, text: &str) -> Result<u32> { Ok(text.parse().unwrap()) }
}
fn object(kind: u8, rect: [f32; 4]) -> LotConfigObject {
    // Editor offset labels deliberately differ from the shared record names.
    LotConfigObject { kind, offset: [4.0, 9.0, 4.0],
        max_x: rect[0], max_z: rect[1], min_x: rect[2], min_z: rect[3], ..Default::default() }
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
    e.drag = true; e.capturing = true;
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
    h.events.clear(); e.dirty = 0;
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
    h.modal = ACCEPT; h.absent.push(TEXT);
    assert!(e.name_dialog(&mut h).unwrap());
    assert_eq!(e.dirty, 0);
    h.absent.clear(); h.events.clear();
    assert!(e.name_dialog(&mut h).unwrap());
    assert_eq!(e.config().unwrap().name, "<unnamed>");
    assert_eq!(h.events.iter().filter(|x| **x == Effect::Title).count(), 2);
    h.modal = 0; h.text = "changed".into();
    assert!(!e.name_dialog(&mut h).unwrap());
    assert_eq!(e.config().unwrap().name, "<unnamed>");
}

#[test]
fn path_growth_without_dirty_and_slope_limit_precedence() {
    let (mut e, mut h) = editor();
    let id = e.insert(object(7, [0.0; 4]), false);
    e.record_mut(id).unwrap().ids = vec![9, 8];
    h.modal = ACCEPT; h.text = "ab".into();
    e.network_path_dialog(id, &mut h).unwrap();
    assert_eq!(e.record(id).unwrap().ids, vec![9, 8, 0, 171]);
    assert_eq!(e.dirty, 0);
    h.events.clear(); h.text = "100".into();
    e.slope_dialog(&mut h).unwrap();
    assert!(h.events.contains(&Effect::Message("Slope is outside min/max values for this lot".into(), 0)));
    assert!(!h.events.contains(&Effect::Message("Slope limited to 80 degrees".into(), 0)));
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
    e.identity.key[0] = 1; e.dirty = 7;
    assert!(!e.save(false, false, &mut h).unwrap());
    assert_eq!(e.dirty, 7);
    assert!(h.events.is_empty());
    h.persist = true;
    assert!(e.save(false, true, &mut h).unwrap());
    assert_eq!(e.dirty, 0);
    assert_eq!(h.events, vec![Effect::Title, Effect::Saved(e.identity.clone())]);
    e.dirty = 1; h.prompt = 0x53018147; h.events.clear();
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
    assert_eq!(e.hit_candidates([2, 2], 0, Some([0, 0, 9, 9]), &rects, None), Hit::Object(b, 101));
    e.selected = None;
    assert_eq!(e.hit_candidates([2, 2], 0, None, &rects, None), Hit::Object(a, 100));
    assert_eq!(e.hit_candidates([2, 2], 2, None, &rects, Some(-1)), Hit::Cancelled);
    assert_eq!(e.hit_candidates([2, 2], 2, None, &rects, Some(1)), Hit::Object(b, 101));
}

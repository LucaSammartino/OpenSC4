// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent LEW01–18 expectations from spec §22.
use super::*;

#[derive(Clone, Debug, PartialEq)]
enum Trace {
    Window(WindowEffect),
    Effect(Effect),
    Call(&'static str),
    Message(String, String, (u32, u32, u32)),
}
#[derive(Default)]
struct Fake {
    trace: Vec<Trace>,
    cell: Option<(u32, u32)>,
    text: String,
    resolved: Option<u32>,
    create: Option<u64>,
    confirm: u32,
    writable: bool,
    deleted: bool,
    type_still_present: bool,
    found: Option<(u32, u32)>,
    types: Vec<(u32, String)>,
    family_reads: usize,
    family_unavailable: bool,
    read_result: Option<Configuration>,
    manager: Option<std::rc::Rc<crate::lot_config_manager::LotConfigManager>>,
    prompt: u32,
}
impl Fake {
    fn windows(&self) -> Vec<WindowEffect> {
        self.trace
            .iter()
            .filter_map(|e| match e {
                Trace::Window(e) => Some(e.clone()),
                _ => None,
            })
            .collect()
    }
    fn releases(&self, pointer: u64) -> usize {
        self.trace
            .iter()
            .filter(|e| **e == Trace::Effect(Effect::Release(pointer)))
            .count()
    }
}
impl Host for Fake {
    fn effect(&mut self, e: Effect) {
        self.trace.push(Trace::Effect(e));
    }
    fn create_dialog(&mut self, _: DialogKind) -> Option<u64> {
        None
    }
    fn has_control(&mut self, _: u64, _: u32) -> bool {
        false
    }
    fn run_modal(&mut self, _: u64) -> u32 {
        0
    }
    fn text(&mut self, _: u64, _: u32) -> String {
        String::new()
    }
    fn integer(&mut self, _: u64, _: u32) -> i32 {
        0
    }
    fn selected_rows(&mut self, _: u64, _: u32) -> Vec<(u32, u32)> {
        vec![]
    }
    fn prompt(&mut self, _: &str, _: u32) -> u32 {
        self.prompt
    }
    fn message_box(&mut self, text: &str, title: &str, options: (u32, u32, u32)) -> u32 {
        self.trace.push(Trace::Message(text.into(), title.into(), options));
        0
    }
    fn read_configuration(&mut self, _: [u32; 3]) -> Result<Option<Configuration>> {
        Ok(self.read_result.take())
    }
    fn configuration_manager(&self) -> Option<std::rc::Rc<crate::lot_config_manager::LotConfigManager>> {
        self.manager.clone()
    }
    fn register_configuration(&mut self, _: &Configuration, _: bool) {}
    fn parse_hex(&mut self, text: &str) -> Result<u32> {
        Ok(u32::from_str_radix(text.trim_start_matches("0x"), 16).unwrap())
    }
    fn configuration_id_for_type(&mut self, _: u32) -> Result<Option<u32>> {
        self.trace.push(Trace::Call("resolve type"));
        Ok(self.resolved)
    }
    fn provider_for_key(&mut self, _: [u32; 3]) -> Result<Option<u64>> {
        Ok(Some(20))
    }
    fn query_provider(&mut self, _: u64) -> Option<u64> {
        Some(30)
    }
    fn provider_path(&mut self, _: u64, _: [u32; 3]) -> String {
        "C:\\lots\\item.dat".into()
    }
    fn create_cursor(&mut self, _: u32, _: u32) -> Result<Option<u64>> {
        Ok(None)
    }
}
impl WindowHost for Fake {
    fn window_effect(&mut self, e: WindowEffect) {
        self.trace.push(Trace::Window(e));
    }
    fn base_message(&mut self, _: u32, _: u32, _: u32) -> bool {
        self.trace.push(Trace::Call("base message"));
        false
    }
    fn acquire_grid(&mut self, child: u32, iid: u32) -> Option<u64> {
        assert_eq!((child, iid), (CHOOSER_GRID, 0xDAA6B9BE));
        Some(10)
    }
    fn acquire_manager(&mut self) -> Option<u64> {
        Some(11)
    }
    fn enumerate_types(&mut self, manager: u64, option: bool) -> Vec<(u32, String)> {
        assert_eq!((manager, option), (11, false));
        self.types.clone()
    }
    fn current_cell(&mut self, _: u64) -> Option<(u32, u32)> {
        self.cell
    }
    fn cell_text(&mut self, _: u64, row: u32, column: u32) -> String {
        assert_eq!(Some(row), self.cell.map(|v| v.0));
        assert_eq!(column, 0);
        self.text.clone()
    }
    fn create_editor(&mut self, key: [u32; 3], id: u32) -> Option<u64> {
        assert_eq!((key, id), ([0, 0x96A006B0, 0x09234182], 0xC9235387));
        self.trace.push(Trace::Call("create editor"));
        self.create
    }
    fn confirm(&mut self, title: &'static str, text: &'static str) -> u32 {
        assert_eq!(title, "Are you sure");
        assert_eq!(text, "Are you sure you want to delete the selected item?");
        self.trace.push(Trace::Call("confirm"));
        self.confirm
    }
    fn file_writable_and_exists(&mut self, _: &str) -> bool {
        self.trace.push(Trace::Call("file"));
        self.writable
    }
    fn delete_item(&mut self, identity: &Identity) -> Result<bool> {
        assert_eq!(identity.id, 9);
        self.trace.push(Trace::Call("delete item"));
        Ok(self.deleted)
    }
    fn global_type(&mut self, ty: u32) -> Option<u64> {
        assert_eq!(ty, 5);
        self.trace.push(Trace::Call("global type"));
        self.type_still_present.then_some(55)
    }
    fn unregister_type(&mut self, configuration: u64) {
        assert_eq!(configuration, 55);
        self.trace.push(Trace::Call("unregister if present"));
    }
    fn search_grid(&mut self, _: u64, search: &GridSearch) -> Option<(u32, u32)> {
        assert_eq!(
            search,
            &GridSearch {
                text: "0x00000009".into(),
                length: 10,
                raw: [0, 1, 0, 0x7FFFFFFF],
                final_option: 1
            }
        );
        self.trace.push(Trace::Call("search"));
        self.found
    }
}
impl EditorWindowHost for Fake {
    fn load_family_file(&mut self) -> Result<()> {
        self.family_reads += 1;
        if self.family_unavailable {
            Err(Unsupported::Dependency("LE5 family path/callbacks"))
        } else {
            Ok(())
        }
    }
    fn acquire_font(&mut self, family: &str, size: u32) -> Option<u64> {
        assert_eq!((family, size), ("Arial", 12));
        Some(40)
    }
    fn network_object(&mut self, ty: u32, option: bool) -> Option<u64> {
        assert!(option);
        Some(u64::from(ty) + 50)
    }
}
fn chooser() -> (Chooser, Fake) {
    (
        Chooser {
            grid: Some(10),
            ..Default::default()
        },
        Fake {
            cell: Some((7, 1)),
            text: "0x00000005".into(),
            resolved: Some(9),
            create: Some(100),
            confirm: CONFIRM_DELETE,
            writable: true,
            deleted: true,
            type_still_present: true,
            found: Some((0, 1)),
            ..Default::default()
        },
    )
}

#[test]
fn lew01_02_init_once_and_message_consumption() {
    let mut c = Chooser::default();
    let mut h = Fake::default();
    assert!(c.controller.is_none() && c.grid.is_none() && c.manager.is_none());
    assert!(c.services.is_empty() && c.editors.is_empty());
    assert_eq!((c.sort_column, c.sort_option), (0, false));
    c.init(&mut h);
    c.init(&mut h);
    assert_eq!(
        h.windows(),
        vec![
            WindowEffect::PrimaryFlag(0x10000, true),
            WindowEffect::AuxiliaryFlag(1, true),
            WindowEffect::AuxiliaryFlag(0x10, true),
            WindowEffect::AuxiliaryFlag(0x100, false),
            WindowEffect::AuxiliaryFlag(4, false),
            WindowEffect::BaseInit,
            WindowEffect::Post(POST_INIT, [0; 3])
        ]
    );
    assert_eq!(c.message(3, 0, 0, false, false, &mut h), ChooserAction::Handled);
    assert_eq!(
        c.message(3, WINDOW_COMMAND, 1, false, false, &mut h),
        ChooserAction::Handled
    );
    assert!(!h.trace.contains(&Trace::Call("base message")));
    assert_eq!(c.message(4, 0, 0, false, false, &mut h), ChooserAction::Base(false));
    assert_eq!(
        c.message(POST_INIT, 0, 0, false, false, &mut h),
        ChooserAction::PostInit
    );
    for (id, action) in [
        (0x091C91BB, ChooserAction::Edit(true)),
        (0x091C91BC, ChooserAction::Edit(false)),
        (0x091C91BD, ChooserAction::Delete),
        (0x42B7C353, ChooserAction::Close),
    ] {
        assert_eq!(c.message(3, WINDOW_COMMAND, id, false, false, &mut h), action);
    }
}
#[test]
fn lew03_04_sort_rebuild_preserves_enumeration_order() {
    let (mut c, mut h) = chooser();
    c.message(3, WINDOW_COMMAND, 0x0942D1CC, false, true, &mut h);
    assert_eq!((c.sort_column, c.sort_option), (1, true));
    assert_eq!(h.windows(), vec![WindowEffect::Sort(10, 1, true, 1, 0)]);
    h.trace.clear();
    c.rebuild(&mut h).unwrap();
    assert_eq!(h.windows(), vec![WindowEffect::ClearGrid(10)]);
    c.manager = Some(11);
    h.types = vec![(9, "Z".into()), (2, "A".into())];
    h.trace.clear();
    c.rebuild(&mut h).unwrap();
    assert_eq!(
        h.windows(),
        vec![
            WindowEffect::ClearGrid(10),
            WindowEffect::Row(10, 0, 0, "0x00000009".into()),
            WindowEffect::Row(10, 0, 1, "Z".into()),
            WindowEffect::Row(10, 1, 0, "0x00000002".into()),
            WindowEffect::Row(10, 1, 1, "A".into()),
            WindowEffect::Sort(10, 1, true, 1, 0)
        ]
    );
    c.sort(1, false, &mut h);
    assert!(!c.sort_option);
    c.grid = None;
    c.sort(0, true, &mut h);
    assert_eq!((c.sort_column, c.sort_option), (1, false));
}
#[test]
fn lew05_06_selection_reads_column_zero_and_preserves_failed_key() {
    let (c, mut h) = chooser();
    h.text = "0x000000ab".into();
    assert_eq!(c.selected_type(&mut h), Ok(0xAB));
    let mut key = [123; 3];
    assert_eq!(c.selected_key(&mut key, &mut h), Ok(true));
    assert_eq!(key, [0x6534284A, 0xA8FBD372, 9]);
    h.resolved = None;
    key = [123; 3];
    assert_eq!(c.selected_key(&mut key, &mut h), Ok(false));
    assert_eq!(key, [123; 3]);
    h.cell = None;
    assert_eq!(c.selected_type(&mut h), Ok(0));
}
#[test]
fn lew07_08_no_selection_or_tracked_type_blocks_work() {
    let (mut c, mut h) = chooser();
    h.cell = None;
    for new in [true, false] {
        assert!(c.begin_edit(new, &mut h).unwrap().is_none());
    }
    c.delete_selected(&mut h).unwrap();
    assert!(h.trace.is_empty());
    h.cell = Some((0, 1));
    c.editors.insert(10, 5);
    assert!(c.begin_edit(true, &mut h).unwrap().is_none());
    c.delete_selected(&mut h).unwrap();
    assert_eq!(
        h.windows(),
        vec![
            WindowEffect::Error(
                "No can do",
                "The lot you are trying to edit is already open for editing."
            ),
            WindowEffect::Error(
                "No can do",
                "The lot you are trying to delete is currently open for editing."
            )
        ]
    );
    assert!(!h.trace.contains(&Trace::Call("resolve type")));
}
#[test]
fn lew09_10_11_edit_creation_cleanup_and_pointer_registration() {
    let (mut c, mut h) = chooser();
    let pending = c.begin_edit(true, &mut h).unwrap().unwrap();
    assert_eq!(pending.key, [0; 3]);
    assert!(!h.trace.contains(&Trace::Call("resolve type")));
    c.finish_edit(pending, false, 0, &mut h);
    assert!(c.editors.is_empty());
    assert_eq!(h.releases(100), 1);
    assert!(h.windows().ends_with(&[
        WindowEffect::Error("No can do", "Unable to open the lot for editing."),
        WindowEffect::RemoveCreated(100),
        WindowEffect::ShutdownEditor(100)
    ]));
    h.trace.clear();
    h.resolved = None;
    assert!(c.begin_edit(false, &mut h).unwrap().is_none());
    assert!(!h.trace.contains(&Trace::Call("create editor")));
    h.resolved = Some(9);
    h.trace.clear();
    let pending = c.begin_edit(false, &mut h).unwrap().unwrap();
    assert_eq!(pending.key, [0x6534284A, 0xA8FBD372, 9]);
    c.finish_edit(pending, true, 9, &mut h);
    assert!(!c.is_type_being_edited(5));
    assert!(c.is_type_being_edited(9));
    c.editor_closed(100, &mut h);
    c.editor_closed(100, &mut h);
    assert_eq!(h.releases(100), 2); // creator reference plus registration
}
#[test]
fn lew12_user_close_rechecks_callbacks_and_preserves_partial_progress() {
    let (mut c, mut h) = chooser();
    c.controller = Some(42);
    c.editors.extend([(10, 1), (20, 2)]);
    let result = c.user_close(&mut h, |c, p, ask, save, h| {
        assert!(ask && save);
        if p == 10 {
            c.editor_closed(p, h);
            Ok(true)
        } else {
            Ok(false)
        }
    });
    assert_eq!(result, Ok(false));
    assert_eq!(c.editors.len(), 1);
    assert!(c.editors.contains_key(&20));
    assert_eq!(h.releases(10), 1);
    assert!(!h.windows().contains(&WindowEffect::ControllerClose(42, false)));
    c.user_close(&mut h, |c, p, _, _, h| {
        c.editor_closed(p, h);
        Ok(true)
    })
    .unwrap();
    assert!(h.windows().contains(&WindowEffect::ControllerClose(42, false)));
}
#[test]
fn lew13_14_lei01_delete_failure_order_row_zero_and_false_return() {
    for failure in 0..4 {
        let (c, mut h) = chooser();
        match failure {
            0 => h.confirm = 0,
            1 => h.writable = false,
            2 => h.deleted = false,
            _ => (),
        }
        c.delete_selected(&mut h).unwrap();
        assert_eq!(h.trace.contains(&Trace::Call("unregister if present")), failure == 3);
        assert_eq!(
            h.windows().contains(&WindowEffect::RemoveRow(10, 0, 1, 0)),
            failure == 3
        );
        assert_eq!(
            h.trace.contains(&Trace::Message(
                "The exemplar you are trying to delete is either non-existant or write-protected.\n\n".into(),
                "No can do".into(),
                (0x10000, 1, 0)
            )),
            failure == 1
        );
        assert_eq!(
            h.windows().contains(&WindowEffect::Error(
                "No can do",
                "Unknown error trying to delete item."
            )),
            failure == 2
        );
        assert_eq!(h.releases(20), 2);
        assert_eq!(h.releases(30), 2);
        if failure == 0 {
            assert!(!h.trace.contains(&Trace::Call("file")));
        }
    }
    let (c, mut h) = chooser();
    assert_eq!(c.remove_row(9, &mut h), Ok(false));
    h.found = None;
    h.trace.clear();
    assert_eq!(c.remove_row(9, &mut h), Ok(false));
    assert!(!h.windows().iter().any(|e| matches!(e, WindowEffect::RemoveRow(..))));
}

#[test]
fn lei02_shutdown_exact_release_clear_order_and_retained_array() {
    let mut w = EditorWindow {
        initialized: true,
        ..Default::default()
    };
    let mut e = Editor::default();
    let mut h = Fake::default();
    for offset in [0x130, 0x134, 0x138, 0x13C, 0x15C, 0x160, 0x190] {
        w.references.insert(offset, u64::from(offset));
    }
    e.current = Some(Configuration::default());
    e.identity = Identity {
        references: [Some(0x188), Some(0x18C)],
        id: 99,
        key: [1, 2, 3],
        path: "keep".into(),
    };
    e.cursors = [Some(1), Some(2), Some(3), Some(4), Some(7), Some(6), Some(5)];
    w.networks = [Some(0x194), Some(0x198)];
    w.owned_references = vec![30, 0, 31, 30];
    w.storage.extend([0x1D0, 0x1C4, 0x1A0, 0x178]);
    w.shutdown(&mut e, &mut h);
    use Trace::{Effect as E, Window as W};
    use WindowEffect::{ClearReference as Clear, NetworkStop, NotifyClosed};
    assert_eq!(
        h.trace,
        [
            W(NotifyClosed(0x15C)),
            E(Effect::Release(0x130)),
            W(Clear(0x130)),
            W(Clear(0x134)),
            E(Effect::Release(0x134)),
            W(Clear(0x138)),
            E(Effect::Release(0x138)),
            W(Clear(0x13C)),
            E(Effect::Release(0x13C)),
            W(Clear(0x15C)),
            E(Effect::Release(0x15C)),
            W(Clear(0x160)),
            E(Effect::Release(0x160)),
            E(Effect::Release(0x188)),
            W(Clear(0x188)),
            E(Effect::Release(0x18C)),
            W(Clear(0x18C)),
            W(NetworkStop(0x194)),
            E(Effect::Release(0x194)),
            W(Clear(0x194)),
            W(NetworkStop(0x198)),
            E(Effect::Release(0x198)),
            W(Clear(0x198)),
            E(Effect::Release(30)),
            E(Effect::Release(31)),
            E(Effect::Release(30)),
            W(WindowEffect::BaseShutdown),
        ]
    );
    assert_eq!(w.owned_references, [30, 0, 31, 30]);
    assert_eq!(w.references.get(&0x190), Some(&0x190));
    assert!(e.current.is_some());
    assert_eq!(
        (e.identity.id, e.identity.key, e.identity.path.as_str()),
        (99, [1, 2, 3], "keep")
    );
    assert!(e.cursors.iter().all(Option::is_some));
    h.trace.clear();
    w.destroy(&mut e, 0, &mut h);
    assert_eq!(
        &h.trace[..5],
        &[
            W(WindowEffect::FreeStorage(0x1D0)),
            W(WindowEffect::FreeStorage(0x1C4)),
            W(WindowEffect::FreeStorage(0x1A0)),
            E(Effect::Release(0x190)),
            W(WindowEffect::FreeStorage(0x178)),
        ]
    );
    assert_eq!(
        &h.trace[5..12],
        &(1..=7).rev().map(|p| E(Effect::Release(p))).collect::<Vec<_>>()
    );
    assert_eq!(h.releases(30), 0);
    assert_eq!(h.releases(31), 0);
}

#[test]
fn lei03_uninitialized_shutdown_leaves_references_for_destruction() {
    let mut w = EditorWindow::default();
    let mut e = Editor::default();
    let mut h = Fake::default();
    w.references.insert(0x190, 9);
    e.current = Some(Configuration::default());
    e.cursors[0] = Some(8);
    w.shutdown(&mut e, &mut h);
    assert_eq!(h.windows(), [WindowEffect::BaseShutdown]);
    assert_eq!(w.references.get(&0x190), Some(&9));
    assert!(e.current.is_some());
    assert_eq!(e.cursors[0], Some(8));
    w.destroy(&mut e, 0, &mut h);
    assert_eq!((h.releases(8), h.releases(9)), (1, 1));
}

#[test]
fn lei04_partial_read_never_registers_or_swallows_unsupported_open() {
    let (mut c, mut h) = chooser();
    h.manager = Some(super::fix_test_host::manager());
    h.read_result = Some(Configuration {
        name: "installed".into(),
        ..Default::default()
    });
    let pending = c.begin_edit(false, &mut h).unwrap().unwrap();
    let mut e = Editor::default();
    // The independently initialized stage reaches the still-open SetLotSize
    // continuation after installing Read's state, rather than fabricating Open.
    let opened = e.open_configuration_stage(pending.key, &mut h);
    assert_eq!(opened, Err(Unsupported::Dependency("LE4 SetLotSize")));
    assert_eq!(c.finish_core_edit(pending, &e, opened.clone(), &mut h), opened);
    assert_eq!(e.config().unwrap().name, "installed");
    assert!(c.editors.is_empty());
    assert_eq!(h.releases(100), 1);
    assert!(h.windows().ends_with(&[
        WindowEffect::Error("No can do", "Unable to open the lot for editing."),
        WindowEffect::RemoveCreated(100),
        WindowEffect::ShutdownEditor(100),
    ]));
}

#[test]
fn lei05_core_close_notification_changes_live_set_before_refusal() {
    let (mut c, mut h) = chooser();
    c.editors.extend([(10, 1), (20, 2)]);
    c.controller = Some(42);
    let mut editors = std::collections::BTreeMap::new();
    for p in [10, 20] {
        let mut e = Editor {
            chooser: true,
            ..Default::default()
        };
        e.create_new(&mut h);
        e.dirty = 1;
        editors.insert(p, e);
    }
    let mut visited = vec![];
    let result = c.user_close(&mut h, |c, p, ask, save, h| {
        visited.push(p);
        h.prompt = if p == 10 { 0x53018149 } else { 0x53018147 };
        let before = h.trace.len();
        let result = editors.get_mut(&p).unwrap().close(ask, save, h);
        if h.trace[before..].contains(&Trace::Effect(Effect::NotifyChooser)) {
            c.editor_closed(p, h);
        }
        result
    });
    assert_eq!(result, Ok(false));
    assert_eq!(visited, [10, 20]);
    assert_eq!(c.editors.keys().copied().collect::<Vec<_>>(), [20]);
    assert!(!h.windows().contains(&WindowEffect::ControllerClose(42, false)));
}
#[test]
fn chooser_forced_shutdown_and_raw_destructor_are_distinct() {
    let (mut c, mut h) = chooser();
    c.controller = Some(1);
    c.manager = Some(11);
    c.services.insert(30);
    c.editors.extend([(100, 5), (200, 6)]);
    c.shutdown(&mut h, |c, p, ask, save, h| {
        assert!(!ask && !save);
        if p == 100 {
            c.editor_closed(p, h);
        }
        Ok(false)
    })
    .unwrap();
    assert_eq!(h.releases(100), 1);
    assert_eq!(h.releases(200), 1);
    assert_eq!(h.releases(30), 1);
    assert!(h.trace.ends_with(&[
        Trace::Effect(Effect::Release(1)),
        Trace::Effect(Effect::Release(10)),
        Trace::Effect(Effect::Release(11)),
        Trace::Window(WindowEffect::BaseShutdown)
    ]));
    c.services.insert(99);
    h.trace.clear();
    c.destroy(3, &mut h);
    assert_eq!(h.releases(99), 0);
    assert_eq!(h.windows().last(), Some(&WindowEffect::FreeObject));
}
#[test]
fn lew17_editor_shutdown_releases_elements_before_destructor_storage() {
    let mut w = EditorWindow {
        initialized: true,
        ..Default::default()
    };
    let mut e = Editor::default();
    let mut h = Fake::default();
    w.references.extend([(0x190, 10), (0x15C, 11)]);
    w.storage.extend([0x1D0, 0x1C4, 0x1A0, 0x178]);
    w.owned_references.push(20);
    w.shutdown(&mut e, &mut h);
    assert_eq!(h.releases(20), 1);
    assert_eq!(h.releases(10), 0);
    w.destroy(&mut e, 1, &mut h);
    assert_eq!(h.releases(10), 1);
    let element = h
        .trace
        .iter()
        .position(|e| *e == Trace::Effect(Effect::Release(20)))
        .unwrap();
    let storage = h
        .trace
        .iter()
        .position(|e| *e == Trace::Window(WindowEffect::FreeStorage(0x1D0)))
        .unwrap();
    assert!(element < storage);
    assert_eq!(h.windows().last(), Some(&WindowEffect::FreeObject));
}
#[test]
fn lew18_static_failure_is_not_retried_and_editor_messages_and_networks() {
    let mut w = EditorWindow::default();
    let mut e = Editor::default();
    let mut resources = EditorStaticResources::default();
    let mut h = Fake {
        family_unavailable: true,
        ..Default::default()
    };
    assert!(w.init(&mut e, &mut resources, &mut h).is_err());
    assert!(resources.attempted && w.initialized);
    assert_eq!(w.init(&mut e, &mut resources, &mut h), Ok(true));
    assert_eq!(h.family_reads, 1);
    assert_eq!(
        h.windows()
            .iter()
            .filter(|e| matches!(e, WindowEffect::StaticSection(_)))
            .count(),
        3
    );
    assert_eq!(
        w.message(3, WINDOW_COMMAND, 0x42B7C353, &mut h),
        EditorMessage::Close(true, true)
    );
    assert_eq!(w.query_interface(0xC6AE7085, 1, 2, &mut h), Some(2));
    assert_eq!(w.query_interface(0, 1, 2, &mut h), None);
    assert!(w.post_init_networks(&mut h).is_err());
    assert!(w.post_init_networks(&mut h).is_err());
    assert_eq!(
        h.windows()
            .iter()
            .filter(|e| matches!(e, WindowEffect::NetworkStart(_)))
            .count(),
        2
    );
}

#[derive(Default)]
struct MouseFake {
    routes: Vec<MouseRoute>,
    conversion: bool,
}
impl MouseHost for MouseFake {
    fn child_coordinates(&mut self, p: [i32; 2]) -> [i32; 2] {
        self.conversion = true;
        [p[0] - 10, p[1] - 20]
    }
    fn window_coordinates(&mut self, p: [i32; 2]) -> [i32; 2] {
        [p[0] + 10, p[1] + 20]
    }
    fn child_size(&mut self) -> [i32; 2] {
        [100, 50]
    }
    fn dispatch_mouse(&mut self, route: MouseRoute) -> Result<bool> {
        self.routes.push(route);
        Ok(false)
    }
}
#[test]
fn lew15_mouse_half_open_child_region_and_e0_bypass() {
    for event in [
        MouseEvent::LeftDown,
        MouseEvent::RightDown,
        MouseEvent::LeftUp,
        MouseEvent::Move,
    ] {
        let mut h = MouseFake::default();
        assert_eq!(mouse_wrapper(event, [109, 69], false, &mut h), Ok(true));
        assert_eq!(h.routes, vec![MouseRoute::Edit(event, [99, 49])]);
        for point in [[110, 69], [9, 20], [10, 70]] {
            h.routes.clear();
            mouse_wrapper(event, point, false, &mut h).unwrap();
            let route = if event == MouseEvent::RightDown {
                MouseRoute::OutsideMenu
            } else {
                MouseRoute::Base(event, point)
            };
            assert_eq!(h.routes, vec![route]);
        }
        if matches!(event, MouseEvent::LeftUp | MouseEvent::Move) {
            h.conversion = false;
            assert_eq!(mouse_wrapper(event, [10, 20], true, &mut h), Ok(false));
            assert!(!h.conversion);
        }
    }
}

#[derive(Debug, PartialEq)]
enum Draw {
    Acquire(u32),
    Color([u8; 3]),
    Lock(u32),
    Fill,
    Unlock(u32),
    Raster(RasterPhase),
    Release,
    Invalidate,
}
struct Renderer {
    trace: Vec<Draw>,
    locks: Vec<bool>,
    acquisition: (bool, Option<u64>),
    unsettled: bool,
}
impl EditorRenderer for Renderer {
    fn acquire(&mut self, iid: u32) -> (bool, Option<u64>) {
        self.trace.push(Draw::Acquire(iid));
        self.acquisition
    }
    fn color(&mut self, rgb: [u8; 3]) -> u32 {
        self.trace.push(Draw::Color(rgb));
        123
    }
    fn lock(&mut self, _: u64, mode: u32) -> bool {
        self.trace.push(Draw::Lock(mode));
        self.locks.remove(0)
    }
    fn fill(&mut self, _: u64, color: u32, option: u32) {
        assert_eq!((color, option), (123, 0));
        self.trace.push(Draw::Fill);
    }
    fn unlock(&mut self, _: u64, mode: u32) {
        self.trace.push(Draw::Unlock(mode));
    }
    fn raster(&mut self, _: u64, phase: RasterPhase) -> Result<()> {
        self.trace.push(Draw::Raster(phase));
        if self.unsettled {
            Err(Unsupported::Dependency("LE5 raster geometry"))
        } else {
            Ok(())
        }
    }
    fn release_buffer(&mut self, _: u64) {
        self.trace.push(Draw::Release);
    }
    fn invalidate(&mut self) {
        self.trace.push(Draw::Invalidate);
    }
}
#[test]
fn lew16_draw_failure_phase_order_release_and_trigger() {
    let mut e = Editor {
        current: Some(Configuration::default()),
        ..Default::default()
    };
    let a = e.insert(LotConfigObject::default(), false);
    let b = e.insert(LotConfigObject::default(), false);
    let mut h = Renderer {
        trace: vec![],
        locks: vec![false, true, false],
        acquisition: (true, Some(1)),
        unsettled: false,
    };
    e.trigger_redraw(&mut h).unwrap();
    assert_eq!(
        h.trace,
        vec![
            Draw::Acquire(0x20732180),
            Draw::Color([192; 3]),
            Draw::Lock(0x8001),
            Draw::Lock(0x8004),
            Draw::Raster(RasterPhase::GridOutline),
            Draw::Unlock(0x8004),
            Draw::Raster(RasterPhase::Ordinary(b)),
            Draw::Raster(RasterPhase::Ordinary(a)),
            Draw::Lock(0x8004),
            Draw::Unlock(0x8004),
            Draw::Release,
            Draw::Invalidate
        ]
    );
    h.trace.clear();
    h.acquisition = (false, Some(1));
    e.draw_current(&mut h).unwrap();
    assert_eq!(h.trace, vec![Draw::Acquire(0x20732180), Draw::Release]);
    h.trace.clear();
    h.acquisition = (false, None);
    assert!(e.trigger_redraw(&mut h).is_err());
    assert!(!h.trace.contains(&Draw::Invalidate));
    h.acquisition = (true, Some(1));
    h.locks = vec![true, true];
    h.unsettled = true;
    assert!(e.trigger_redraw(&mut h).is_err());
    assert_eq!(h.trace.last(), Some(&Draw::Release));
}
#[test]
fn foundation_marker_lock_and_height_label_vectors() {
    let mut e = Editor {
        current: Some(Configuration::default()),
        ..Default::default()
    };
    let id = e.insert(
        LotConfigObject {
            kind: 3,
            ..Default::default()
        },
        false,
    );
    for marker_lock in [false, true] {
        let mut h = Renderer {
            trace: vec![],
            locks: vec![true, false, marker_lock, true],
            acquisition: (true, Some(1)),
            unsettled: false,
        };
        e.draw_current(&mut h).unwrap();
        assert_eq!(
            h.trace.contains(&Draw::Raster(RasterPhase::FoundationMarker(id, 0))),
            marker_lock
        );
        assert_eq!(
            h.trace.contains(&Draw::Raster(RasterPhase::FoundationMarker(id, 1))),
            marker_lock
        );
        assert_eq!(h.trace.contains(&Draw::Unlock(0x8400)), marker_lock);
        assert!(!h.trace.contains(&Draw::Raster(RasterPhase::Ordinary(id))));
    }
    for (height, expected) in [(0.0, None), (-0.0, None), (1.0, Some("+1.0")), (-1.0, Some("--1.0"))] {
        let label = height_label(height, |format, v| {
            assert_eq!(format, "+%.1f");
            format!("+{v:.1}")
        });
        assert_eq!(label.as_deref(), expected);
    }
    assert_eq!(
        height_label(f32::NAN, |_, _| "+CRT_NAN".into()),
        Some("+CRT_NAN".into())
    );
}

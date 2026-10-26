// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
#[derive(Default)]
struct Trace {
    events: Vec<String>,
    custom: bool,
    path: Option<String>,
    checkbox: bool,
    tables: usize,
    ui: Vec<Ui>,
}
impl Host for Trace {
    fn retain(&mut self, p: u64) {
        self.events.push(format!("retain:{p}"));
    }
    fn release(&mut self, p: u64) {
        self.events.push(format!("release:{p}"));
    }
    fn song_table(&mut self, k: usize) -> Option<Vec<Vec<u32>>> {
        self.tables += 1;
        (k == 0).then_some(vec![vec![11, 1, 0], vec![12, 2], vec![], vec![14, 4, 1]])
    }
    fn plugin_directory(&mut self) -> String {
        "C:\\".into()
    }
    fn text(&mut self, id: u32) -> String {
        match id {
            1 => "a.mp3",
            2 => "b.wav",
            4 => "d.xa",
            _ => "title",
        }
        .into()
    }
    fn files(&mut self, _: &str, pattern: &str) -> Vec<String> {
        if pattern == "*.pls" && self.custom {
            vec!["mine.PLS".into()]
        } else if pattern == "*.mp3" {
            vec!["A.MP3".into()]
        } else {
            vec![]
        }
    }
    fn exists(&mut self, _: &str) -> bool {
        false
    }
    fn tuning(&mut self) -> Option<[(u32, u32); 2]> {
        Some([(173, 1024), (450, 1024)])
    }
    fn create_window(&mut self) -> Option<u64> {
        Some(1)
    }
    fn grid(&mut self, _: u64) -> Option<u64> {
        Some(2)
    }
    fn window_view(&mut self, p: u64, _: bool) -> u64 {
        p + 100
    }
    fn window_manager(&mut self, _: u64) -> Option<u64> {
        Some(8)
    }
    fn is_modal(&mut self, _: u64) -> bool {
        true
    }
    fn ui(&mut self, _: u64, op: Ui) {
        self.ui.push(op);
    }
    fn checkbox(&mut self, _: u32) -> Option<bool> {
        Some(self.checkbox)
    }
    fn slider(&mut self, _: u32) -> Option<u32> {
        Some(500)
    }
    fn selected_row(&mut self, _: u64) -> Option<i32> {
        Some(0)
    }
    fn child_enabled(&mut self, _: u64, _: u32) -> bool {
        false
    }
    fn click_sound(&mut self) {
        self.events.push("click".into());
    }
    fn post(&mut self, k: u32, a: u32, b: u32) {
        self.events.push(format!("post:{k:08x}:{a}:{b}"));
    }
    fn post_list(&mut self, k: usize, s: u32, l: &[String], mode: u8) {
        self.events.push(format!("list:{k}:{s}:{mode}:{l:?}"));
    }
    fn master_volume(&mut self, v: u32) {
        self.events.push(format!("master:{v}"));
    }
    fn application(&mut self) -> Option<u64> {
        Some(9)
    }
    fn save_preferences(&mut self, _: u64) {
        self.events.push("save".into());
    }
    fn radio(&mut self) -> Option<u64> {
        Some(3)
    }
    fn current_path(&mut self, _: u64) -> Option<String> {
        self.path.clone()
    }
    fn post_preview_path(&mut self, p: &str) {
        self.events.push(format!("preview:{p}"));
    }
    fn modal(&mut self, _: u64) {}
}
fn setup() -> (Owner, Process, Record, Trace) {
    let mut o = Owner::construct();
    o.refcount = 1;
    let mut p = Process::default();
    let r = Record::default();
    let mut h = Trace::default();
    assert!(o.show(&mut p, &r, &mut h));
    (o, p, r, h)
}
#[test]
fn au43_discovery_inheritance_packed_rows_and_load_once() {
    let (mut o, mut p, r, mut h) = setup();
    assert_eq!(
        p.lists[0].iter().map(|s| (s.id, s.default)).collect::<Vec<_>>(),
        [(BASE[0], 0), (BASE[0] + 1, 0), (BASE[0] + 3, 1)]
    );
    assert!(!p.custom(0, &mut h));
    h.custom = true;
    assert!(p.custom(0, &mut h));
    assert_eq!(p.lists[0].len(), 3);
    let rows: Vec<_> =
        h.ui.iter()
            .filter_map(|op| {
                if let Ui::Song { row, enabled, .. } = op {
                    Some((*row, *enabled))
                } else {
                    None
                }
            })
            .collect();
    assert_eq!(rows, [(0, false), (1, false), (2, false)]);
    assert_eq!(r.songs, [[1; 30]; 2]);
    o.show(&mut p, &r, &mut h);
    assert_eq!(h.tables, 2);
    assert!(p.enabled("C:\\custom.mp3", &r, &mut h));
    let mut saved = 300;
    assert_eq!(volume_slider(&mut saved, (450, 1024)), 0);
    assert_eq!(saved, 0);
    let mut saved = 800;
    assert_eq!(volume_slider(&mut saved, (450, 1024)), 609);
}
#[test]
fn au43_live_rebuild_cancel_snapshot_and_literal_modes() {
    let (mut o, mut p, mut r, mut h) = setup();
    r.songs[0][1] = 0;
    o.saved = r.clone();
    p.rebuild(0, &r, &mut h);
    assert_eq!(p.excluded[0], ["a.mp3", "d.xa"]);
    o.control(BASE[0], 0, &mut p, &mut r, &mut h);
    assert_eq!(p.excluded[0], ["a.mp3", "d.xa"]);
    o.control(0x8A55_0C50, 0, &mut p, &mut r, &mut h);
    assert_eq!(p.excluded[0], ["d.xa"]);
    h.events.clear();
    o.control(0xEA55_09E3, 0, &mut p, &mut r, &mut h);
    assert_eq!(r.songs[0][0], 1);
    assert_eq!(p.excluded[0], ["d.xa"]);
    assert_eq!(
        &h.events[..3],
        ["master:1024", "post:aa650860:1:800", "post:231bbf91:0:0"]
    );
    assert!(!h.events.iter().any(|e| e == "save" || e.starts_with("list:")));
    r.mode = 1;
    p.rebuild(0, &r, &mut h);
    assert_eq!(p.excluded[0], ["b.wav"]);
    r.mode = 2;
    p.rebuild(0, &r, &mut h);
    assert_eq!(p.excluded[0], ["a.mp3", "b.wav", "d.xa"]);
    o.control(BASE[0] + 30, 7, &mut p, &mut r, &mut h);
    assert_eq!(r.mode, 7);
    o.control(BASE[1] + 30, 8, &mut p, &mut r, &mut h);
    assert_eq!(r.songs[0][0], 8);
}
#[test]
fn au43_ok_saves_before_rebuild_skips_disabled_path_and_preview_ends_twice() {
    let (mut o, mut p, mut r, mut h) = setup();
    r.songs[0][1] = 0;
    r.mode = 1;
    h.path = Some(format!("{}b.wav", p.folders[0]));
    h.events.clear();
    o.control(0x8A55_0C58, 0, &mut p, &mut r, &mut h);
    o.control(0x8A55_0C58, 0, &mut p, &mut r, &mut h);
    o.control(0xEA55_09E4, 0, &mut p, &mut r, &mut h);
    o.end(&mut p, &mut h);
    let events = &h.events;
    assert_eq!(events.iter().filter(|e| *e == "post:aa650861:1:0").count(), 1);
    assert_eq!(events.iter().filter(|e| *e == "post:aa650861:0:0").count(), 2);
    let save = events.iter().position(|e| e == "save").unwrap();
    let list = events.iter().position(|e| e.starts_with("list:")).unwrap();
    assert!(save < list);
    let next = events.iter().rposition(|e| e == "post:2a611b51:0:0").unwrap();
    let changed = events.iter().position(|e| e == "post:231bbf91:0:0").unwrap();
    assert!(list < next && next < changed);
    assert_eq!(o.refcount, 1);
    assert!(o.preview);
    assert_eq!(o.window, None);
}
#[test]
fn au43_visibility_notification_matrix_and_checkbox_state() {
    let (mut o, mut p, mut r, mut h) = setup();
    h.events.clear();
    assert!(o.message(101, true, [3, 0x2872_59F6, 0x8A55_0C50, 0], &mut p, &mut r, &mut h));
    assert!(h.events.is_empty());
    assert!(o.message(101, true, [3, 0x2872_59F7, 0x8A55_0C50, 0], &mut p, &mut r, &mut h));
    assert_eq!(r.music_on, 0);
    h.events.clear();
    assert!(o.message(101, true, [3, 99, 0, 0], &mut p, &mut r, &mut h));
    assert!(h.events.is_empty());
    assert!(!o.message(101, false, [3, 0x2872_59F7, 0x8A55_0C50, 1], &mut p, &mut r, &mut h));
    assert!(!o.message(99, true, [3, 0x2872_59F7, 0x8A55_0C50, 1], &mut p, &mut r, &mut h));
    o.message(101, true, [3, 0x2872_59F8, 0x8A55_0C52, 1], &mut p, &mut r, &mut h);
    assert_eq!(r.effects_on, 1);
    o.message(101, true, [3, 0x2872_59F8, 0x8A55_0C52, 0], &mut p, &mut r, &mut h);
    assert_eq!(r.effects_on, 0);
    assert!(!o.message(101, true, [5, 27, 0, 0], &mut p, &mut r, &mut h));
    assert_eq!(r.effects_on, 1);
}

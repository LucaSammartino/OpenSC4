// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::region::City;
use crate::region_view::{tests::Recording, CityEntry};
use std::sync::Arc;

impl regional_city::Lifecycle for Recording {
    fn subscribe(&mut self, message: u32, enable: bool) {
        self.events.push(format!("subscribe:{message:x}:{enable}"));
    }
}
impl Host for Recording {
    fn default_difficulty(&mut self) -> u8 {
        1
    }
    fn localized_string(&mut self, group: u32, instance: u32) -> String {
        assert_eq!((group, instance), (0x6a23_1eaa, 0xaa73_8c4e));
        "New City".into()
    }
    fn publish_city(&mut self, record: CityRecord) -> CityRef {
        assert!(record.initialized);
        let city = City {
            position: record.position,
            size: record.size,
            serial: record.serial,
            path: String::from_utf8(record.file_path.clone()).unwrap(),
            name: String::from_utf8(record.city_name.clone()).unwrap(),
            tutorial_id: record.tutorial_id,
            rearrange: false,
        }
        .counted();
        self.records.insert(record.position, record);
        self.events.push("publish".into());
        city
    }
    fn regional_city(&mut self, city: &CityRef) -> CityRecord {
        let state = city.read().unwrap();
        let mut record = self
            .records
            .get(&state.position)
            .cloned()
            .unwrap_or_else(|| CityRecord::new(0));
        record.city_name = state.name.as_bytes().to_vec();
        record
    }
    fn load_city(&mut self, path: &str, _: &CityRef) -> bool {
        self.events.push(format!("load:{path}"));
        self.load_succeeds
    }
    fn clear_new_city_object(&mut self) {
        self.events.push("clear-new".into());
    }
    fn request_new_city(&mut self, _: &CityRef) -> bool {
        self.events.push("new-city".into());
        self.new_succeeds
    }
    fn region_list(&mut self) -> Vec<(RegionKey, String)> {
        vec![(1, "Current".into()), (2, "Other".into())]
    }
    fn current_region(&mut self) -> RegionKey {
        1
    }
    fn set_current_region(&mut self, key: RegionKey) {
        self.events.push(format!("current:{key}"));
    }
    fn request_region_view(&mut self, argument: bool) -> bool {
        self.events.push(format!("go-region:{argument}"));
        true
    }
    fn preference_region(&mut self, name: &str) {
        self.events.push(format!("pref-region:{name}"));
    }
    fn preference_position(&mut self, x: i32, y: i32) {
        self.events.push(format!("pref-position:{x}:{y}"));
    }
    fn save_preferences(&mut self) {
        self.events.push("save-preferences".into());
    }
    fn request_quit(&mut self) {
        self.events.push("quit".into());
    }
    fn double_click_time(&mut self) -> u32 {
        500
    }
    fn play_sound(&mut self, id: u32) {
        self.events.push(format!("sound:{id:x}"));
    }
    fn close_panels(&mut self) {
        self.events.push("close-panels".into());
    }
    fn restore_signposts(&mut self) {
        self.events.push("restore-signposts".into());
    }
    fn selected_presentation(&mut self, _: CityId, established: bool, instance: u32, window_id: Option<u32>) {
        self.events
            .push(format!("selected:{established}:{instance:x}:{window_id:?}"));
    }
    fn show_dialog(&mut self, dialog: Dialog, selected: Option<CityId>) {
        self.events.push(format!("dialog:{dialog:?}:{selected:?}"));
    }
    fn post_command(&mut self, command: u32) {
        self.events.push(format!("post:{command:x}"));
    }
    fn accelerator_command(&mut self, key: u32, code: u32) -> Option<u32> {
        assert_eq!(key, 0x1b);
        assert!(code == 5 || code == 6);
        Some(123)
    }
    fn preference_visibility(&mut self, post_view_init: bool) {
        self.events.push(format!("visibility:{post_view_init}"));
    }
    fn post_camera_view_init(&mut self) {
        self.events.push("camera-init".into());
    }
    fn reset_snapshots(&mut self, city: CityId) {
        self.events.push(format!("reset:{city}"));
    }
    fn read_thumbnails(&mut self, _: &CityRef) {
        self.events.push("thumbnails".into());
    }
    fn default_snapshots(&mut self, _: &CityRef) {
        self.events.push("defaults".into());
    }
    fn population_text(&mut self, id: u32, value: i64) {
        assert_eq!(id, 0xc9e4_1918);
        self.events.push(format!("population:{value}"));
    }
    fn tuning(&mut self, property: u32) -> Option<Tunable> {
        self.events.push(format!("tuning:{property:x}"));
        match property {
            0xca38_3cab => Some(Tunable::Words(vec![0x1234_5678])),
            0xca38_3cb1 => Some(Tunable::Words(vec![32, 64])),
            _ => None,
        }
    }
    fn cloud_opacity(&mut self, minimum: u8, maximum: u8) {
        self.events.push(format!("opacity:{minimum}:{maximum}"));
    }
}

fn view(host: &mut Recording) -> RegionView {
    let mut view = RegionView::new(1);
    view.region_key = Some(1);
    let image = host.image(1, 1, vec![0xffff_ffff]);
    let mut city = CityEntry::new(7, (3, 4), 2, image);
    city.position = (0.0, 1.0);
    city.hit_runs = vec![0, 1];
    view.cities.push(city);
    view
}

#[test]
fn screen_c2_large_empty_tile_creates_four_by_four_and_requests_new_city() {
    let mut region = Region::default();
    let mut host = Recording {
        new_succeeds: true,
        ..Default::default()
    };
    let mut view = view(&mut host);
    view.selected = Some(7);
    let screen = RegionScreen::new(1, 64.0);
    assert!(screen.play_selected(&mut region, &view, &mut host).unwrap());
    let city = region.boxed_city(region.get_city(3, 4).unwrap()).unwrap();
    let state = city.read().unwrap();
    assert_eq!(state.size, (4, 4));
    assert_eq!(state.position, (3, 4));
    assert_eq!(state.name, "New City");
    assert_eq!(host.records[&(3, 4)].difficulty, 1);
    assert!(!state.rearrange);
    drop(state);
    let same = RegionScreen::get_or_create_city(
        &mut region,
        &mut host,
        Location {
            x: 3,
            z: 4,
            size_exp: 2,
        },
        true,
    )
    .unwrap()
    .unwrap();
    assert!(Arc::ptr_eq(&city, &same));
    assert_eq!(host.events.iter().filter(|e| *e == "publish").count(), 1);
    assert_eq!(&host.events[host.events.len() - 2..], ["clear-new", "new-city"]);
}

#[test]
fn screen_c3_existing_path_load_and_failed_load_fallback() {
    let mut host = Recording {
        load_succeeds: true,
        new_succeeds: true,
        ..Default::default()
    };
    let city = City {
        position: (0, 0),
        size: (1, 1),
        serial: 1,
        path: "city.sc4".into(),
        name: "A".into(),
        tutorial_id: -1,
        rearrange: false,
    }
    .counted();
    let mut screen = RegionScreen::new(1, 64.0);
    screen.position = (-1.9, 2.9);
    assert!(screen.launch_regional_city(&mut host, &city));
    assert_eq!(host.events, ["pref-position:-1:2", "load:city.sc4"]);
    host.load_succeeds = false;
    host.events.clear();
    assert!(screen.launch_regional_city(&mut host, &city));
    assert_eq!(
        host.events,
        ["pref-position:-1:2", "load:city.sc4", "clear-new", "new-city"]
    );
}

#[test]
fn screen_c1_double_click_deadline_and_held_hover() {
    let mut region = Region::default();
    let mut host = Recording {
        new_succeeds: true,
        ..Default::default()
    };
    let mut view = view(&mut host);
    let mut screen = RegionScreen::new(1, 64.0);
    assert!(!screen
        .mouse_down(&mut region, &mut view, &mut host, (0, 0), 100)
        .unwrap());
    screen.mouse_move(&mut view, &mut host, 99, 99).unwrap();
    assert_eq!(view.selected, Some(7));
    assert!(screen
        .mouse_down(&mut region, &mut view, &mut host, (0, 0), 200)
        .unwrap());
    host.events.clear();
    screen.held = None;
    assert!(!screen
        .mouse_down(&mut region, &mut view, &mut host, (0, 0), 1000)
        .unwrap());
    assert!(!screen
        .mouse_down(&mut region, &mut view, &mut host, (0, 0), 1600)
        .unwrap());
    assert!(!host.events.iter().any(|e| e == "new-city"));
    assert_eq!(screen.held_since, 1600);
    screen
        .mouse_down(&mut region, &mut view, &mut host, (99, 99), 1700)
        .unwrap();
    assert_eq!(screen.held, None);
    assert_eq!(view.selected, None);
    assert_eq!(host.events.last().unwrap(), "restore-signposts");
}

#[test]
fn screen_c4_load_current_region_has_no_side_effects() {
    let mut host = Recording::default();
    let screen = RegionScreen::new(1, 64.0);
    assert!(screen.load_region(&mut host, "Current"));
    assert!(host.events.is_empty());
    assert!(screen.load_region(&mut host, "Other"));
    assert_eq!(host.events, ["current:2", "go-region:false"]);
    assert!(!screen.load_region(&mut host, "Missing"));
}

#[test]
fn screen_commands_scroll_messages_preferences_and_notifications() {
    let mut host = Recording::default();
    let mut region = Region::default();
    region.set_name("Current");
    let mut view = view(&mut host);
    let mut screen = RegionScreen::new(1, 64.0);
    for (id, expected) in [
        (0x6a93_5cd8, (-64.0, 0.0)),
        (0x6a93_5ce2, (-64.0, 64.0)),
        (0x2a94_826e, (0.0, 64.0)),
        (0x2a94_8272, (0.0, 0.0)),
    ] {
        assert!(screen
            .process_command(&mut region, &mut view, &mut host, id, &[])
            .unwrap());
        assert_eq!(screen.velocity, expected);
    }
    screen.message(&mut view, &mut host, 0xaaa1_cdf2, 384, -640).unwrap();
    assert_eq!(screen.position, (1.5, -2.5));
    assert_eq!(view.scroll, (1, -2));
    screen.message(&mut view, &mut host, 0x8a4b_ac53, 0, 0).unwrap();
    assert!(view.scrolling);
    screen.message(&mut view, &mut host, 0x8a4b_ac5b, 0, 0).unwrap();
    assert!(!view.scrolling);
    screen.message(&mut view, &mut host, 0x1b, 5, 0).unwrap();
    screen.message(&mut view, &mut host, 0x2b96_b3ea, 0, 0).unwrap();
    assert!(host.events.contains(&"post:7b".to_owned()));
    assert!(host.events.contains(&"camera-init".to_owned()));
    screen
        .process_command(&mut region, &mut view, &mut host, 0x6a93_5e3c, &[])
        .unwrap();
    assert_eq!(
        &host.events[host.events.len() - 3..],
        ["pref-region:Current", "pref-position:1:-2", "save-preferences"]
    );
    assert!(!screen
        .button(&mut region, &mut view, &mut host, 0x2a5b_0000, 0, false)
        .unwrap());
    screen
        .button(&mut region, &mut view, &mut host, 0xea5a_96e6, 0x2872_59f7, false)
        .unwrap();
    assert!(!view.grid);
    for (id, expected) in [(0x4a56_0002, "QR2/QR4"), (0x09eb_f2bd, "QR3")] {
        assert_eq!(
            screen.button(&mut region, &mut view, &mut host, id, 0x2872_59f6, false),
            Err(Unsupported::Open(expected))
        );
    }
    assert_eq!(
        screen.process_command(&mut region, &mut view, &mut host, 0x6a97_57c2, &[]),
        Err(Unsupported::Open("QR3"))
    );
}

#[test]
fn screen_tuning_population_and_snapshot_builder_boundary() {
    let mut host = Recording::default();
    let mut region = Region::default();
    let mut view = view(&mut host);
    let mut screen = RegionScreen::new(1, 64.0);
    assert_eq!(screen.read_tuning(&mut host, &mut view), Err(Unsupported::Open("QR1")));
    assert_eq!(view.grid_colour, 0x1234_5678);
    assert!(!host.events.contains(&"tuning:ca383ca2".to_owned()));
    assert!(host.events.contains(&"opacity:32:64".to_owned()));
    RegionScreen::get_or_create_city(
        &mut region,
        &mut host,
        Location {
            x: 0,
            z: 0,
            size_exp: 2,
        },
        true,
    )
    .unwrap();
    host.records.get_mut(&(0, 0)).unwrap().population = 123;
    assert_eq!(screen.update_population(&region, &mut host), 123);
    assert_eq!(
        screen.put_cities_in_view(&region, &mut host),
        Err(Unsupported::Open("RGV5"))
    );
    assert_eq!(host.events.last().unwrap(), "defaults");
}

#[test]
fn screen_lifetime_is_explicitly_unsupported_at_qr1() {
    let mut host = Recording::default();
    let mut windows = Windows::new(1);
    let window = windows.create_default(&mut host);
    let mut screen = RegionScreen::new(window, 64.0);
    assert_eq!(screen.init(&mut windows, &mut host), Err(Unsupported::Open("QR1")));
    assert!(windows.get(window).flag(0x4000));
    assert_eq!(screen.shutdown(&mut windows, &mut host), Err(Unsupported::Open("QR1")));
    assert!(!windows.get(window).flag(0x4000));
    assert_eq!(
        screen.put_cities_in_view(&Region::default(), &mut host),
        Err(Unsupported::Open("QR5"))
    );
}

#[test]
fn screen_launch_city_filters_name_size_and_empty_flag() {
    let mut host = Recording {
        load_succeeds: true,
        ..Default::default()
    };
    let mut region = Region::default();
    let city = RegionScreen::get_or_create_city(
        &mut region,
        &mut host,
        Location {
            x: 2,
            z: 1,
            size_exp: 1,
        },
        true,
    )
    .unwrap()
    .unwrap();
    city.write().unwrap().path = "medium.sc4".into();
    city.write().unwrap().name = "Named".into();
    host.records.get_mut(&(2, 1)).unwrap().established = true;
    let screen = RegionScreen::new(1, 64.0);
    assert!(!screen
        .launch_city(&mut region, &mut host, Some("Named"), 0, false)
        .unwrap());
    assert!(!screen
        .launch_city(&mut region, &mut host, Some("Named"), 1, true)
        .unwrap());
    assert!(screen
        .launch_city(&mut region, &mut host, Some("Named"), 1, false)
        .unwrap());
    assert!(screen
        .launch_city(&mut region, &mut host, Some("Named"), 3, false)
        .unwrap());
    assert!(!screen
        .launch_city(&mut region, &mut host, Some("Missing"), 3, false)
        .unwrap());
}

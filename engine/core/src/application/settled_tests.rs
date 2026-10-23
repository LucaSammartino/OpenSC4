// SPDX-License-Identifier: GPL-3.0-or-later
use super::preference_application::*;
use super::video::*;
use super::*;
use crate::cmdline::CmdLine;

#[derive(Default)]
struct Managers {
    log: Vec<String>,
    settings: [u32; 16],
    line: CmdLine,
    mode: Option<[u32; 3]>,
    language: bool,
    can_switch: bool,
    interface: bool,
}

impl VideoHost for Managers {
    fn set_setting(&mut self, id: u32, value: u32) {
        self.settings[id as usize] = value;
        self.log.push(format!("set:{id}:{value}"));
    }
    fn setting(&mut self, id: u32) -> u32 {
        self.log.push(format!("get:{id}"));
        self.settings[id as usize]
    }
    fn post_video_changed(&mut self, kind: u32) {
        self.log.push(format!("post:{kind:X}"));
    }
}
impl PreferenceHost for Managers {
    fn language_manager_present(&mut self) -> bool {
        self.language
    }
    fn language_by_name(&mut self, _: &[u8], local: bool) -> Option<u32> {
        self.log.push(format!("name:{local}"));
        Some(9)
    }
    fn installed_language(&mut self, deluxe: bool) -> Option<u32> {
        self.log.push(format!("registry:{deluxe}"));
        None
    }
    fn system_language(&mut self) -> u32 {
        self.log.push("system".into());
        7
    }
    fn can_switch_language(&mut self, id: u32) -> bool {
        self.log.push(format!("can:{id}"));
        self.can_switch
    }
    fn select_language(&mut self, id: u32) {
        self.log.push(format!("language:{id}"));
    }
    fn graphics_mode(&mut self) -> Option<[u32; 3]> {
        self.mode
    }
    fn request_mode(&mut self, dimensions: [u32; 3]) {
        self.log.push(format!("mode:{dimensions:?}"));
    }
    fn request_fullscreen(&mut self, fullscreen: bool) {
        self.log.push(format!("fullscreen:{fullscreen}"));
    }
    fn query_graphics_interface(&mut self) -> Option<Token> {
        self.interface.then_some(123)
    }
    fn select_driver(&mut self, interface: Token, driver: u32) {
        assert_eq!(interface, 123);
        self.log.push(format!("driver:{driver:X}"));
    }
    fn release_graphics_interface(&mut self, interface: Token) {
        assert_eq!(interface, 123);
        self.log.push("release graphics".into());
    }
    fn simulator(&mut self, _: Token) -> Option<Token> {
        Some(4)
    }
    fn set_simulator_speed(&mut self, simulator: Token, speed: u32) {
        assert_eq!(simulator, 4);
        self.log.push(format!("speed:{speed}"));
    }
    fn framework_command_line(&mut self) -> CmdLine {
        CmdLine::copy_of(&self.line)
    }
    fn default_cheat(&mut self, _: Token, value: &[u8]) {
        self.log.push(format!("cheat:{}", String::from_utf8_lossy(value)));
    }
}

fn app() -> Application {
    super::tests::app()
}

#[test]
fn video_slots_preserve_bytes_and_unknown_dimensions_and_notify_last() {
    let mut app = app();
    app.message_server2_available = true;
    let mut h = Managers::default();
    let record = [255; 29];
    app.apply_video_preferences(&record, &mut h);
    assert_eq!(h.log.len(), 13);
    assert_eq!(h.log[4], "set:6:255");
    assert_eq!(h.log[11], "set:15:255");
    assert_eq!(h.log[12], "post:C9F775BB");
    assert_eq!(record, [255; 29]);
    for (resolution, dimensions) in [
        (0, [800u32, 600]),
        (1, [1024, 768]),
        (2, [1280, 1024]),
        (3, [1600, 1200]),
    ] {
        h.settings[11] = resolution;
        h.settings[12] = 1;
        h.settings[0] = 0x1234;
        let mut out = [0xCC; 29];
        app.get_auto_video_preferences(&mut out, &mut h);
        assert_eq!(&out[16..20], dimensions[0].to_le_bytes());
        assert_eq!(&out[20..24], dimensions[1].to_le_bytes());
        assert_eq!(&out[24..28], 32u32.to_le_bytes());
        assert_eq!((out[1], out[11]), (0x34, 1));
    }
    h.settings[11] = 99;
    h.settings[12] = u32::MAX;
    let mut out = [0xCC; 29];
    app.get_auto_video_preferences(&mut out, &mut h);
    assert_eq!(&out[16..28], &[0xCC; 12]);
}

#[test]
fn preference_order_window_driver_language_failure_and_partial_speed() {
    let mut app = app();
    app.preferences.pause_on_focus_loss = true;
    app.preferences_image.0[0x4A4] = 2;
    app.preferences.simulation = 50.0;
    app.preferences.width = 1024;
    app.preferences.height = 768;
    app.preferences.depth = 32;
    app.preferences.fullscreen = true;
    app.preferences.language = b"unknown".to_vec();
    app.preferences_image.0[0x4A8..0x4AC].copy_from_slice(&42u32.to_le_bytes());
    app.preferences_image.0[0x4EB] = 9;
    app.city_state.city = Some(2);
    app.driver_id = 0x12345678;
    let mut h = Managers {
        mode: Some([800, 600, 16]),
        language: true,
        interface: true,
        ..Managers::default()
    };
    assert_eq!(app.do_preferences_changed(false, &mut h), CheatStep::Done);
    assert_eq!(h.log, ["speed:20"]);
    assert!(app.pause_on_focus_loss());
    assert_eq!(app.preferences_image.0[0x4A4], 1);
    assert_eq!(app.preferences_image.0[0x4EB], 9);
    h.log.clear();
    app.do_preferences_changed(true, &mut h);
    assert_eq!(h.log.iter().filter(|s| *s == "can:7").count(), 45);
    assert!(!h.log.iter().any(|s| s == "name:true" || s.starts_with("language:")));
    assert_eq!(app.preferences_image.0[0x4EB], 0);
    assert_eq!(&app.preferences_image.0[0x4A8..0x4AC], 42u32.to_le_bytes());
    assert_eq!(
        &h.log[h.log.len() - 5..],
        [
            "mode:[1024, 768, 32]",
            "fullscreen:false",
            "driver:12345678",
            "release graphics",
            "speed:20"
        ]
    );
    assert_eq!(app.preferences.driver_f13, 255);
}

#[test]
fn speed_conversion_wraps_low_signed_word_and_handles_indefinite() {
    for (input, expected) in [
        (-1.0, 5),
        (50.0, 20),
        (100.0, 80),
        (f32::from_bits(0xD1000000), 80),
        (f32::NAN, 5),
        (f32::INFINITY, 5),
    ] {
        assert_eq!(simulator_speed(input), expected);
    }
}

#[test]
fn cheats_copy_repeat_unrelated_erasure_and_expose_nonprogress() {
    let app = app();
    let mut h = Managers {
        line: CmdLine::new(b"p -x:GP -x:ForceIdle"),
        ..Managers::default()
    };
    let original = h.line.args().to_vec();
    for _ in 0..2 {
        assert_eq!(app.apply_command_line_cheats(Some(2), &mut h), CheatStep::Done);
    }
    assert_eq!(h.line.args(), original);
    assert_eq!(h.log, ["cheat:GP", "cheat:ForceIdle", "cheat:GP", "cheat:ForceIdle"]);
    h.log.clear();
    h.line = CmdLine::new(b"p axjunk -x:GP");
    app.apply_command_line_cheats(Some(2), &mut h);
    assert_eq!(h.log, ["cheat:GP", "cheat:GP"]);
    h.line = CmdLine::new(b"p -X:GP");
    let mut requests = CheatRequests::new(&h.line);
    assert_eq!(requests.step(2, &mut h), CheatStep::NonProgress);
    assert_eq!(requests.step(2, &mut h), CheatStep::NonProgress);
}

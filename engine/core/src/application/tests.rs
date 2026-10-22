// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::cmdline::CmdLine;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Call {
    Effect(Effect), Create(Object, Token), Register(Service, Token), Release(Token),
    Registry, Driver, Check(Token), Exists(Vec<u8>), Running(Token), Watched(Token), InstallFilter,
}

struct TestHost {
    calls: Vec<Call>,
    next: Token,
    registry: Option<Vec<u8>>,
    driver: Option<Token>,
    driver_ok: bool,
    exists: std::collections::VecDeque<bool>,
    fail_create: Option<Object>,
    fail_register: Option<Service>,
}

impl Default for TestHost {
    fn default() -> Self {
        Self { calls: Vec::new(), next: 100, registry: Some(b"C:\\Install\\".to_vec()), driver: Some(7), driver_ok: true, exists: Default::default(), fail_create: None, fail_register: None }
    }
}

impl Host for TestHost {
    fn effect(&mut self, effect: Effect) { self.calls.push(Call::Effect(effect)); }
    fn create(&mut self, object: Object) -> Result<Token, HostFailure> {
        self.next += 1;
        self.calls.push(Call::Create(object, self.next));
        if self.fail_create == Some(object) { Err(HostFailure("create")) } else { Ok(self.next) }
    }
    fn register_service(&mut self, service: Service, object: Token) -> Result<(), HostFailure> {
        self.calls.push(Call::Register(service, object));
        if self.fail_register == Some(service) { Err(HostFailure("register")) } else { Ok(()) }
    }
    fn release_created(&mut self, object: Token) { self.calls.push(Call::Release(object)); }
    fn install_directory(&mut self) -> Option<Vec<u8>> { self.calls.push(Call::Registry); self.registry.clone() }
    fn graphics_driver(&mut self) -> Option<Token> { self.calls.push(Call::Driver); self.driver }
    fn check_graphics_driver(&mut self, driver: Token) -> bool { self.calls.push(Call::Check(driver)); self.driver_ok }
    fn directory_exists(&mut self, path: &[u8]) -> bool {
        self.calls.push(Call::Exists(path.to_vec()));
        self.exists.pop_front().unwrap_or(true)
    }
    fn watcher_running(&mut self, watcher: Token) -> bool { self.calls.push(Call::Running(watcher)); watcher == 11 }
    fn watched_directory(&mut self, watcher: Token) -> Vec<u8> { self.calls.push(Call::Watched(watcher)); b"P:\\".to_vec() }
    fn install_exception_filter(&mut self) -> Token { self.calls.push(Call::InstallFilter); 42 }
}

fn app() -> Application {
    Application::new(1, Preferences {
        ime: false, width: 800, height: 600, depth: 32, fullscreen: false,
        pause_on_focus_loss: false, audio_04: true, audio_69: true, radio: 1,
        language: Vec::new(), simulation: 1.0, rdd: false, driver_f13: 9, driver_f0c: 8,
    }, Directories { plugin_a4: b"P:\\".to_vec(), user_base_11c: b"U:\\".to_vec(), ..Default::default() })
}

#[test]
fn init_registry_failure_is_silent_and_driver_failure_saves_before_warning() {
    let mut app = app();
    let mut host = TestHost { registry: None, ..Default::default() };
    app.begin_init().unwrap();
    assert_eq!(app.run(&mut host), Flow::Complete(false));
    assert_eq!(host.calls, [Call::Effect(Effect::FetchFrameworkManagers), Call::Registry]);
    host.registry = Some(b"I".to_vec());
    host.driver_ok = false;
    app.begin_init().unwrap();
    assert_eq!(app.run(&mut host), Flow::NeedsImplementation(Boundary::SavePreferences));
    assert_eq!((app.preferences.driver_f13, app.preferences.driver_f0c), (0, 1));
    assert!(!host.calls.iter().any(|c| matches!(c, Call::Effect(Effect::Warning { .. }))));
    app.resume_after_implemented(&Boundary::SavePreferences).unwrap();
    assert_eq!(app.run(&mut host), Flow::Complete(false));
    assert_eq!(host.calls.last(), Some(&Call::Effect(Effect::Warning { title: "SimCity 4", text: "SimCity 4 will run with the software driver when next launched." })));
    assert!(!host.calls.contains(&Call::Release(7)));
}

#[test]
fn resolution_keeps_unusual_supported_width_and_uses_area_for_unsupported_height() {
    assert!(is_startup_resolution_ok(1366, 768, false));
    assert!(is_startup_resolution_ok(1024, 600, false));
    let (mut w, mut h) = (1280, 720);
    fix_startup_resolution(&mut w, &mut h, false);
    assert_eq!((w, h), (1152, 864));
    assert!(is_startup_resolution_ok(1, 1, true));
}

#[test]
fn idle_gate_and_focus_tail_preserve_strict_comparisons() {
    let mut app = app();
    let mut host = TestHost::default();
    app.enable_full_game_pause_on_focus_loss(true);
    app.on_app_activate(false, &mut host);
    app.on_app_activate(false, &mut host);
    app.timers.advance(1000);
    app.begin_idle().unwrap();
    assert_eq!(app.run(&mut host), Flow::Complete(false));
    assert_eq!(host.calls, [Call::Effect(Effect::Pause { paused: true, flags: [true; 4] }), Call::Effect(Effect::SleepCurrentThread(250_000))]);
    app.message_server2_available = true;
    app.autosave_period = 2000;
    app.periodic_period = 2000;
    app.timers.advance(1);
    app.begin_idle().unwrap();
    assert_eq!(app.run(&mut host), Flow::Complete(false));
    assert!(host.calls.contains(&Call::Effect(Effect::Message { message: Message { kind: GAME_IDLE, data: MessageData::Word(60) }, delivery: Delivery::Post })));
    assert_eq!(app.timers.input, 0);
}

#[test]
fn command_aliases_preserve_primary_presence_and_language_bytes() {
    let mut app = app();
    let mut host = TestHost::default();
    let line = CmdLine::new(br#"-audio:invalid -sound:off -radio:off -music:on -l:"French" -w -f -dgl -IME:enabled -CPUCount:-3 -CPUPriority:LOW"#);
    assert!(app.read_command_line(&line, &mut host).is_empty());
    assert!(app.preferences.audio_04);
    assert_eq!(app.preferences.radio, 0);
    assert_eq!(app.preferences.language, b"French");
    assert!(!app.preferences.fullscreen);
    assert_eq!(app.driver_id, 0xC455_4841);
    assert!(app.preferences.ime);
    assert_eq!(host.calls, [Call::Effect(Effect::ProcessPriority(0x40)), Call::Effect(Effect::CpuCount(-3))]);
}

#[test]
fn lazy_paths_keep_cache_after_base_change_and_scripts_use_plugin_base() {
    let mut dirs = app().directories;
    assert_eq!(dirs.user_folder(UserFolder::Plugins), b"U:\\Plugins\\");
    dirs.user_base_11c = b"V:".to_vec();
    assert_eq!(dirs.user_folder(UserFolder::Plugins), b"U:\\Plugins\\");
    assert_eq!(dirs.user_folder(UserFolder::MySim), b"V:\\MySim\\");
    assert_eq!(dirs.user_folder(UserFolder::Scripts), b"P:\\Scripts\\");
    assert_eq!(dirs.plugin_directory(0, None), b"P:\\English\\");
    assert_eq!(dirs.plugin_directory(-1, Some(b"French")), b"P:\\");
}

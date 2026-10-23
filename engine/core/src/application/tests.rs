// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::cmdline::CmdLine;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Call {
    Effect(Effect),
    Create(Object, Token),
    Register(Service, Token),
    Release(Token),
    Registry,
    Driver,
    Check(Token),
    Exists(Vec<u8>),
    Running(Token),
    Watched(Token),
    InstallFilter,
    Boundary(Boundary),
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
        Self {
            calls: Vec::new(),
            next: 100,
            registry: Some(b"C:\\Install\\".to_vec()),
            driver: Some(7),
            driver_ok: true,
            exists: Default::default(),
            fail_create: None,
            fail_register: None,
        }
    }
}

impl Host for TestHost {
    fn effect(&mut self, effect: Effect) {
        self.calls.push(Call::Effect(effect));
    }
    fn create(&mut self, object: Object) -> Result<Token, HostFailure> {
        self.next += 1;
        self.calls.push(Call::Create(object, self.next));
        if self.fail_create == Some(object) {
            Err(HostFailure("create"))
        } else {
            Ok(self.next)
        }
    }
    fn register_service(&mut self, service: Service, object: Token) -> Result<(), HostFailure> {
        self.calls.push(Call::Register(service, object));
        if self.fail_register == Some(service) {
            Err(HostFailure("register"))
        } else {
            Ok(())
        }
    }
    fn release_created(&mut self, object: Token) {
        self.calls.push(Call::Release(object));
    }
    fn install_directory(&mut self) -> Option<Vec<u8>> {
        self.calls.push(Call::Registry);
        self.registry.clone()
    }
    fn graphics_driver(&mut self) -> Option<Token> {
        self.calls.push(Call::Driver);
        self.driver
    }
    fn check_graphics_driver(&mut self, driver: Token) -> bool {
        self.calls.push(Call::Check(driver));
        self.driver_ok
    }
    fn directory_exists(&mut self, path: &[u8]) -> bool {
        self.calls.push(Call::Exists(path.to_vec()));
        self.exists.pop_front().unwrap_or(true)
    }
    fn watcher_running(&mut self, watcher: Token) -> bool {
        self.calls.push(Call::Running(watcher));
        watcher == 11
    }
    fn watched_directory(&mut self, watcher: Token) -> Vec<u8> {
        self.calls.push(Call::Watched(watcher));
        b"P:\\".to_vec()
    }
    fn install_exception_filter(&mut self) -> Token {
        self.calls.push(Call::InstallFilter);
        42
    }
}

pub(super) fn app() -> Application {
    let mut directories = Directories::default();
    directories.plugin_a4 = b"P:\\".to_vec();
    directories.user_base_11c = b"U:\\".to_vec();
    Application::new(
        1,
        Preferences {
            ime: false,
            width: 800,
            height: 600,
            depth: 32,
            fullscreen: false,
            pause_on_focus_loss: false,
            audio_04: true,
            audio_69: true,
            radio: 1,
            language: Vec::new(),
            simulation: 1.0,
            rdd: false,
            driver_f13: 9,
            driver_f0c: 8,
        },
        directories,
    )
}

#[test]
fn init_registry_failure_is_silent_and_driver_failure_saves_before_warning() {
    let mut app = app();
    let mut host = TestHost {
        registry: None,
        ..Default::default()
    };
    app.begin_init().unwrap();
    assert_eq!(app.run(&mut host), Flow::Complete(false));
    assert_eq!(
        host.calls,
        [Call::Effect(Effect::FetchFrameworkManagers), Call::Registry]
    );
    host.registry = Some(b"I".to_vec());
    host.driver_ok = false;
    app.begin_init().unwrap();
    assert_eq!(app.run(&mut host), Flow::NeedsImplementation(Boundary::SavePreferences));
    assert_eq!((app.preferences.driver_f13, app.preferences.driver_f0c), (0, 1));
    assert!(!host
        .calls
        .iter()
        .any(|c| matches!(c, Call::Effect(Effect::Warning { .. }))));
    app.resume_after_implemented(&Boundary::SavePreferences).unwrap();
    assert_eq!(app.run(&mut host), Flow::Complete(false));
    assert_eq!(
        host.calls.last(),
        Some(&Call::Effect(Effect::Warning {
            title: "SimCity 4",
            text: "SimCity 4 will run with the software driver when next launched."
        }))
    );
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
    assert_eq!(
        host.calls,
        [
            Call::Effect(Effect::Pause {
                paused: true,
                flags: [true; 4]
            }),
            Call::Effect(Effect::SleepCurrentThread(250_000))
        ]
    );
    app.message_server2_available = true;
    app.autosave_period = 2000;
    app.periodic_period = 2000;
    app.timers.advance(1);
    app.begin_idle().unwrap();
    assert_eq!(app.run(&mut host), Flow::Complete(false));
    assert!(host.calls.contains(&Call::Effect(Effect::Message {
        message: Message {
            kind: GAME_IDLE,
            data: MessageData::Word(60)
        },
        delivery: Delivery::Post
    })));
    assert_eq!(app.timers.input, 0);
}

#[test]
fn command_aliases_preserve_primary_presence_and_language_bytes() {
    let mut app = app();
    let mut host = TestHost::default();
    let line = CmdLine::new(
        br#"-audio:invalid -sound:off -music:off -l:"French" -w -f -dgl -IME:enabled -CPUCount:-3 -CPUPriority:LOW"#,
    );
    assert!(app.read_command_line(&line, &mut host).is_empty());
    assert!(app.preferences.audio_04);
    assert_eq!(app.preferences.radio, 0);
    assert_eq!(app.preferences.language, b"French");
    assert!(!app.preferences.fullscreen);
    assert_eq!(app.driver_id, 0xC455_4841);
    assert!(app.preferences.ime);
    assert_eq!(
        host.calls,
        [
            Call::Effect(Effect::ProcessPriority(0x40)),
            Call::Effect(Effect::CpuCount(-3))
        ]
    );
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

/// Synthetic composition only: record boundaries, then acknowledge them to
/// inspect later published spans. This does not implement the missing methods.
fn trace_spans(app: &mut Application, host: &mut TestHost) -> Flow {
    loop {
        match app.run(host) {
            Flow::NeedsImplementation(boundary) => {
                host.calls.push(Call::Boundary(boundary.clone()));
                assert!(!matches!(boundary, Boundary::FindTestScript(_)));
                app.resume_after_implemented(&boundary).unwrap();
            }
            other => return other,
        }
    }
}

#[test]
fn successful_init_exact_order_includes_every_unresolved_owner_call() {
    let mut app = app();
    app.message_server2_available = true;
    let mut host = TestHost::default();
    app.begin_init().unwrap();
    assert_eq!(trace_spans(&mut app, &mut host), Flow::Complete(true));
    assert_eq!(app.install_directory(), b"C:\\Install\\");
    assert_eq!(
        host.calls,
        [
            Call::Effect(Effect::FetchFrameworkManagers),
            Call::Registry,
            Call::Driver,
            Call::Effect(Effect::SetRendererDriver(7)),
            Call::Check(7),
            Call::Boundary(Boundary::DriverConfiguration),
            Call::Effect(Effect::Message {
                message: Message {
                    kind: 0x28E9_397A,
                    data: MessageData::App(1)
                },
                delivery: Delivery::Send
            }),
            Call::Effect(Effect::RegisterV1 {
                kind: 0xC7AA_61C6,
                target: 1
            }),
            Call::Effect(Effect::RegisterV2 {
                kinds: compatibility::V2_SUBSCRIPTIONS,
                target: 1
            }),
            Call::Create(Object::Helper(Helper::CheatManager), 101),
            Call::Effect(Effect::RegisterCheat {
                manager: 101,
                name: "GP",
                id: 0x2AB5_C431
            }),
            Call::Effect(Effect::RegisterCheat {
                manager: 101,
                name: "CreateException",
                id: 0x2B60_E45B
            }),
            Call::Effect(Effect::RegisterCheat {
                manager: 101,
                name: "ForceIdle",
                id: 0x8B6D_4F8D
            }),
            Call::Create(Object::Helper(Helper::GlyphTextureManager), 102),
            Call::Create(Object::Helper(Helper::StringDetokenizer), 103),
            Call::Boundary(Boundary::SetupResources),
            Call::Boundary(Boundary::LanguageManagerHookup),
            Call::Create(Object::Helper(Helper::PersistResourceManager), 104),
            Call::Boundary(Boundary::RenderPropertyVector),
            Call::Create(Object::Helper(Helper::TooltipManager), 105),
            Call::Boundary(Boundary::SlOnAppInit1),
            Call::Create(Object::Helper(Helper::TutorialRegistry), 106),
            Call::Effect(Effect::InitTutorialRegistry(106)),
            Call::Effect(Effect::Message {
                message: Message {
                    kind: 0x28E9_397B,
                    data: MessageData::App(1)
                },
                delivery: Delivery::Send
            }),
        ]
    );
}

#[test]
fn missing_driver_does_not_check_or_release_a_null_or_borrowed_driver() {
    let mut app = app();
    let mut host = TestHost {
        driver: None,
        ..Default::default()
    };
    app.begin_init().unwrap();
    assert_eq!(app.run(&mut host), Flow::NeedsImplementation(Boundary::SavePreferences));
    assert_eq!(
        host.calls,
        [
            Call::Effect(Effect::FetchFrameworkManagers),
            Call::Registry,
            Call::Driver
        ]
    );
    let before = host.calls.clone();
    assert_eq!(app.run(&mut host), Flow::NeedsImplementation(Boundary::SavePreferences));
    assert_eq!(host.calls, before);
    assert!(app.begin_shutdown().is_err());
    assert!(app.resume_after_implemented(&Boundary::DriverConfiguration).is_err());
}

#[test]
fn all_services_register_in_published_order_and_wrap_two_gz_bases() {
    let mut app = app();
    let mut host = TestHost::default();
    app.begin_add_system_services().unwrap();
    assert_eq!(
        app.run(&mut host),
        Flow::NeedsImplementation(Boundary::FindAndPossiblyFixIniFiles)
    );
    assert_eq!(
        host.calls,
        [
            Call::Create(
                Object::Service {
                    kind: Service::Allocator,
                    base: None
                },
                101
            ),
            Call::Register(Service::Allocator, 101),
            Call::Release(101),
            Call::Create(
                Object::Service {
                    kind: Service::Unnamed1,
                    base: None
                },
                102
            ),
            Call::Register(Service::Unnamed1, 102),
            Call::Release(102),
            Call::Create(
                Object::Service {
                    kind: Service::FileSystem,
                    base: None
                },
                103
            ),
            Call::Register(Service::FileSystem, 103),
            Call::Release(103),
        ]
    );
    app.resume_after_implemented(&Boundary::FindAndPossiblyFixIniFiles)
        .unwrap();
    assert_eq!(app.run(&mut host), Flow::Complete(true));
    let order: Vec<_> = host
        .calls
        .iter()
        .filter_map(|call| match call {
            Call::Register(kind, _) => Some(*kind),
            _ => None,
        })
        .collect();
    assert_eq!(
        order,
        [
            Service::Allocator,
            Service::Unnamed1,
            Service::FileSystem,
            Service::MessageServer1,
            Service::MessageServer2,
            Service::Log,
            Service::Command,
            Service::GameDataRegistry,
            Service::LuaScript,
            Service::Language,
            Service::Unnamed2,
            Service::GraphicsSystem,
            Service::UnnamedBeforeFont,
            Service::Font,
            Service::UnnamedAfterFont,
            Service::WindowControl,
            Service::Clipboard,
            Service::Unnamed3,
            Service::ImeProxy,
            Service::PersistResource,
            Service::Registry,
            Service::TextureBindingFactory,
            Service::UnnamedAfterTexture,
            Service::ScriptResourceFactory,
            Service::Www,
            Service::Unnamed4,
            Service::Unnamed5,
            Service::AudioAems
        ]
    );
    assert_eq!(
        &host.calls[9..19],
        [
            Call::Create(Object::GzMessageServer(1), 104),
            Call::Create(
                Object::Service {
                    kind: Service::MessageServer1,
                    base: Some(104)
                },
                105
            ),
            Call::Release(104),
            Call::Register(Service::MessageServer1, 105),
            Call::Release(105),
            Call::Create(Object::GzMessageServer(2), 106),
            Call::Create(
                Object::Service {
                    kind: Service::MessageServer2,
                    base: Some(106)
                },
                107
            ),
            Call::Release(106),
            Call::Register(Service::MessageServer2, 107),
            Call::Release(107),
        ]
    );
    assert!(app.message_server2_available);
    let created: Vec<_> = host
        .calls
        .iter()
        .filter_map(|c| match c {
            Call::Create(_, t) => Some(*t),
            _ => None,
        })
        .collect();
    let mut released: Vec<_> = host
        .calls
        .iter()
        .filter_map(|c| match c {
            Call::Release(t) => Some(*t),
            _ => None,
        })
        .collect();
    released.sort();
    assert_eq!(created, released);
}

#[test]
fn failed_wrapper_creation_releases_only_created_base_and_never_registers_wrapper() {
    let mut app = app();
    let mut host = TestHost {
        fail_create: Some(Object::Service {
            kind: Service::MessageServer1,
            base: Some(104),
        }),
        ..Default::default()
    };
    app.begin_add_system_services().unwrap();
    assert_eq!(trace_spans(&mut app, &mut host), Flow::Failed(HostFailure("create")));
    assert_eq!(host.calls.last(), Some(&Call::Release(104)));
    assert!(!host.calls.contains(&Call::Release(105)));
    assert!(!host
        .calls
        .iter()
        .any(|c| matches!(c, Call::Register(Service::MessageServer1, _))));
    let before = host.calls.clone();
    assert_eq!(app.run(&mut host), Flow::Failed(HostFailure("create")));
    assert_eq!(host.calls, before);
}

#[test]
fn failed_registration_releases_local_reference_and_stops_before_later_services() {
    let mut app = app();
    let mut host = TestHost {
        fail_register: Some(Service::MessageServer2),
        ..Default::default()
    };
    app.begin_add_system_services().unwrap();
    assert_eq!(trace_spans(&mut app, &mut host), Flow::Failed(HostFailure("register")));
    assert_eq!(host.calls.last(), Some(&Call::Release(107)));
    assert!(!app.message_server2_available);
    assert!(!host.calls.iter().any(|c| matches!(c, Call::Register(Service::Log, _))));
}

#[test]
fn pre_framework_order_keeps_restart_and_plugin_boundaries_and_exact_environment() {
    let mut app = app();
    let mut host = TestHost::default();
    app.begin_pre_framework_init(
        b"C:\\Apps\\SimCity.exe",
        &CmdLine::new(b"-Restart:on"),
        b"L:\\stdout.txt",
    )
    .unwrap();
    assert_eq!(trace_spans(&mut app, &mut host), Flow::Complete(true));
    assert_eq!(app.executable_directory(), b"C:\\Apps\\");
    assert_eq!(app.executable_name(), b"SimCity.exe");
    assert_eq!(
        &host.calls[..8],
        [
            Call::Effect(Effect::ReadStartupOption("DebugLevel")),
            Call::Effect(Effect::ReadStartupOption("logging")),
            Call::Boundary(Boundary::BuildVersion),
            Call::Boundary(Boundary::ResolveUserDataDirectory),
            Call::Create(Object::Helper(Helper::RenderProperties), 101),
            Call::Effect(Effect::ReadStartupOption("AllowMultipleInstances")),
            Call::Effect(Effect::ReadStartupOption("Restart")),
            Call::Boundary(Boundary::RestartWait),
        ]
    );
    assert_eq!(
        &host.calls[8..11],
        [
            Call::Effect(Effect::AddDirector(Director::Sc4)),
            Call::Effect(Effect::AddDirector(Director::Audio)),
            Call::Effect(Effect::AddDirector(Director::GzWww))
        ]
    );
    assert_eq!(
        &host.calls[host.calls.len() - 9..],
        [
            Call::Boundary(Boundary::CursorAndOsSettings),
            Call::Boundary(Boundary::PluginDateChecks),
            Call::Effect(Effect::LaunchExceptionReportUploader),
            Call::Boundary(Boundary::AddCodePlugins(Vec::new())),
            Call::Boundary(Boundary::OpenLogs),
            Call::Effect(Effect::SetEnvironment {
                name: "RZSTD_OUTPUT_FILE_PATH",
                value: b"L:\\stdout.txt".to_vec()
            }),
            Call::Effect(Effect::SetEnvironment {
                name: "RZSTD_OUTPUT_PREFIX",
                value: b"StdOut: ".to_vec()
            }),
            Call::Boundary(Boundary::FindAndRegisterLanguages),
            Call::Boundary(Boundary::PreferencesChanged),
        ]
    );
}

#[test]
fn watcher_creation_retests_failed_create_and_pre_shutdown_stops_only_running() {
    let mut app = app();
    app.watchers = [Some(11), Some(12)];
    let mut host = TestHost {
        exists: [false, false, false, true].into(),
        ..Default::default()
    };
    app.begin_post_framework_init(
        &CmdLine::new(b"-NetCommandGenerator:ENABLED"),
        None,
        [b"A".to_vec(), b"B".to_vec()],
    )
    .unwrap();
    assert_eq!(app.run(&mut host), Flow::NeedsImplementation(Boundary::InitAlphaSystem));
    assert_eq!(
        &host.calls[3..],
        [
            Call::Effect(Effect::NetworkCommandGeneratorEnabled),
            Call::Exists(b"A".to_vec()),
            Call::Effect(Effect::CreateDirectory(b"A".to_vec())),
            Call::Exists(b"A".to_vec()),
            Call::Exists(b"B".to_vec()),
            Call::Effect(Effect::CreateDirectory(b"B".to_vec())),
            Call::Exists(b"B".to_vec()),
            Call::Effect(Effect::StartWatcher {
                watcher: 12,
                path: b"B".to_vec()
            }),
        ]
    );
    app.resume_after_implemented(&Boundary::InitAlphaSystem).unwrap();
    assert_eq!(app.run(&mut host), Flow::Complete(true));
    host.calls.clear();
    app.begin_pre_framework_shutdown().unwrap();
    assert_eq!(app.run(&mut host), Flow::Complete(true));
    assert_eq!(
        host.calls,
        [
            Call::Running(11),
            Call::Effect(Effect::StopWatcher(11)),
            Call::Running(12)
        ]
    );
}

#[test]
fn lua_switch_is_erased_once_and_requires_separate_resolution_result() {
    let mut app = app();
    let mut host = TestHost::default();
    let line = CmdLine::new(b"p -LuaScript:MixedCase.lua -w");
    app.begin_post_framework_init(&line, None, [Vec::new(), Vec::new()])
        .unwrap();
    assert_eq!(app.run(&mut host), Flow::NeedsImplementation(Boundary::InitAlphaSystem));
    app.resume_after_implemented(&Boundary::InitAlphaSystem).unwrap();
    let boundary = Boundary::FindTestScript(b"MixedCase.lua".to_vec());
    assert_eq!(app.run(&mut host), Flow::NeedsImplementation(boundary.clone()));
    assert!(app.resume_after_implemented(&boundary).is_err());
    assert_eq!(
        app.consumed_command_line().unwrap().args(),
        [b"p".to_vec(), b"-w".to_vec()]
    );
    assert_eq!(line.argc(), 3);
    app.resume_after_script_resolution(Some(b"P:\\Scripts\\MixedCase.lua".to_vec()))
        .unwrap();
    assert_eq!(app.run(&mut host), Flow::Complete(true));
    assert_eq!(
        host.calls.last(),
        Some(&Call::Effect(Effect::ExecuteLuaScript(
            b"P:\\Scripts\\MixedCase.lua".to_vec()
        )))
    );
    assert!(app.resume_after_script_resolution(None).is_err());
}

#[test]
fn quit_latch_stays_set_while_missing_service_and_city_owners_block() {
    let mut app = app();
    app.screen_owner = Some(25);
    let mut host = TestHost::default();
    assert_eq!(app.begin_quit(), Ok(true));
    assert!(app.quitting());
    assert_eq!(app.begin_quit(), Ok(false));
    assert_eq!(app.run(&mut host), Flow::NeedsImplementation(Boundary::QuitService));
    assert!(host.calls.is_empty());
    app.resume_after_implemented(&Boundary::QuitService).unwrap();
    assert_eq!(app.run(&mut host), Flow::NeedsImplementation(Boundary::CloseCity));
    assert_eq!(
        host.calls,
        [
            Call::Effect(Effect::QuitServiceSlot44),
            Call::Effect(Effect::QuitServiceSlot44)
        ]
    );
    app.resume_after_implemented(&Boundary::CloseCity).unwrap();
    assert_eq!(app.run(&mut host), Flow::Complete(true));
    assert_eq!(
        &host.calls[2..],
        [
            Call::Effect(Effect::DestroyCurrentScreen(25)),
            Call::Effect(Effect::FrameworkQuit(0))
        ]
    );
    assert_eq!(app.begin_quit(), Ok(false));
}

#[test]
fn shutdown_waits_for_heap_discipline_then_v1_call_before_clear_and_post() {
    let mut app = app();
    let mut host = TestHost::default();
    app.message_server2_available = true;
    app.field_314 = Some(55);
    app.begin_shutdown().unwrap();
    assert_eq!(
        app.run(&mut host),
        Flow::NeedsImplementation(Boundary::ShutdownHelperOrder)
    );
    assert_eq!(app.field_314, Some(55));
    app.resume_after_implemented(&Boundary::ShutdownHelperOrder).unwrap();
    assert_eq!(
        app.run(&mut host),
        Flow::NeedsImplementation(Boundary::ShutdownMessageServer1)
    );
    assert_eq!(app.field_314, Some(55));
    app.resume_after_implemented(&Boundary::ShutdownMessageServer1).unwrap();
    assert_eq!(app.run(&mut host), Flow::Complete(true));
    assert_eq!(app.field_314, None);
    assert_eq!(
        host.calls,
        [
            Call::Effect(Effect::Message {
                message: Message {
                    kind: 0x28E9_397C,
                    data: MessageData::App(1)
                },
                delivery: Delivery::Send
            }),
            Call::Effect(Effect::Message {
                message: Message {
                    kind: 0x28E9_397D,
                    data: MessageData::App(1)
                },
                delivery: Delivery::Send
            }),
        ]
    );
    assert!(app.begin_shutdown().is_err());
}

#[test]
fn idle_autosave_blocks_before_periodic_manager_and_paused_tail() {
    let mut app = app();
    let mut host = TestHost::default();
    app.message_server2_available = true;
    app.resource_manager = Some(9);
    app.timers = Timers {
        gate: 1001,
        activity: 299,
        input: 150,
        autosave: 51,
        periodic: 81,
    };
    app.idle_threshold = 100;
    app.idle_subtract = 20;
    app.autosave_period = 50;
    app.periodic_period = 80;
    app.full_game_pause(true, &mut host);
    host.calls.clear();
    app.begin_idle().unwrap();
    assert_eq!(app.run(&mut host), Flow::NeedsImplementation(Boundary::SavePreferences));
    assert_eq!(app.timers.input, 80);
    assert_eq!(app.timers.activity, 0);
    assert_eq!(app.timers.autosave, 0);
    assert_eq!(app.timers.periodic, 81);
    assert_eq!(
        host.calls,
        [Call::Effect(Effect::Message {
            message: Message {
                kind: GAME_IDLE,
                data: MessageData::Word(150)
            },
            delivery: Delivery::Post
        })]
    );
    app.resume_after_implemented(&Boundary::SavePreferences).unwrap();
    assert_eq!(app.run(&mut host), Flow::Complete(false));
    assert_eq!(
        &host.calls[1..],
        [
            Call::Effect(Effect::ResourceManagerSlotAc(9)),
            Call::Effect(Effect::SleepCurrentThread(250_000))
        ]
    );
}

#[test]
fn idle_equal_thresholds_do_not_fire_and_absent_server_still_restarts_timers() {
    let mut app = app();
    let mut host = TestHost::default();
    app.idle_threshold = 40;
    app.autosave_period = 20;
    app.periodic_period = 30;
    app.timers = Timers {
        gate: 1001,
        activity: 299,
        input: 40,
        autosave: 20,
        periodic: 30,
    };
    app.begin_idle().unwrap();
    assert_eq!(app.run(&mut host), Flow::Complete(false));
    assert!(host.calls.is_empty());
    assert_eq!(app.timers.activity, 299);
    app.timers.gate = 1001;
    app.timers.activity = 300;
    app.begin_idle().unwrap();
    assert_eq!(app.run(&mut host), Flow::Complete(false));
    assert!(host.calls.is_empty());
    assert_eq!((app.timers.input, app.timers.activity), (0, 0));
}

#[test]
fn focus_toggle_does_not_invent_unpause_and_repeated_activation_is_idempotent() {
    let mut app = app();
    let mut host = TestHost::default();
    app.on_app_activate(false, &mut host);
    assert!(host.calls.is_empty());
    app.enable_full_game_pause_on_focus_loss(true);
    assert!(app.preferences.pause_on_focus_loss);
    app.on_app_activate(false, &mut host);
    app.enable_full_game_pause_on_focus_loss(false);
    app.on_app_activate(true, &mut host);
    assert!(app.fully_paused());
    app.enable_full_game_pause_on_focus_loss(true);
    app.on_app_activate(true, &mut host);
    app.on_app_activate(true, &mut host);
    assert_eq!(
        host.calls,
        [
            Call::Effect(Effect::Pause {
                paused: true,
                flags: [true; 4]
            }),
            Call::Effect(Effect::Pause {
                paused: false,
                flags: [true; 4]
            })
        ]
    );
}

#[test]
fn v1_watchers_match_identity_and_post_only_after_plugin_algorithm() {
    let mut app = app();
    let mut host = TestHost::default();
    app.watchers = [Some(11), Some(12)];
    app.message_server2_available = true;
    assert_eq!(app.message_v1(0x8716_3754, 13, b"", &mut host), Ok((true, None)));
    assert!(host.calls.is_empty());
    assert_eq!(app.message_v1(0x8716_3754, 12, b"", &mut host), Ok((true, None)));
    assert_eq!(
        app.run(&mut host),
        Flow::NeedsImplementation(Boundary::AddCodePlugins(b"P:\\".to_vec()))
    );
    assert_eq!(host.calls, [Call::Watched(12)]);
    app.resume_after_implemented(&Boundary::AddCodePlugins(b"P:\\".to_vec()))
        .unwrap();
    assert_eq!(app.run(&mut host), Flow::Complete(true));
    assert_eq!(
        host.calls.last(),
        Some(&Call::Effect(Effect::Message {
            message: Message {
                kind: 0xA423_96C6,
                data: MessageData::App(1)
            },
            delivery: Delivery::Post
        }))
    );
    assert_eq!(
        app.message_v1(QUIT_REQUEST, 0, b"", &mut host),
        Ok((true, Some(Boundary::RequestQuit)))
    );
}

#[test]
fn v2_routes_modes_idle_mouse_and_assigned_owners_without_region_algorithms() {
    let mut app = app();
    let mut host = TestHost::default();
    for (kind, mode) in [(0xCBB5_BB45, 1), (0xEA8A_E29A, 2), (0x26D3_1EC2, 0), (0x8BB5_BB46, 0)] {
        assert_eq!(app.message_v2(kind, &mut host), Ok((true, None)));
        assert_eq!(app.mode, mode);
    }
    assert_eq!(
        app.message_v2(0xEA65_224E, &mut host),
        Ok((true, Some(Boundary::CityObliteration)))
    );
    assert_eq!(
        app.message_v2(0x231B_BF91, &mut host),
        Ok((true, Some(Boundary::PreferencesChanged)))
    );
    for kind in [0xFA0C_6390, 0x7A10_4750] {
        app.timers.input = 999;
        app.message_v2(kind, &mut host).unwrap();
        assert_eq!(app.timers.input, 0);
    }
    app.message_v2(GAME_IDLE, &mut host).unwrap();
    assert!(host.calls.is_empty());
    app.resource_manager = Some(17);
    app.message_v2(GAME_IDLE, &mut host).unwrap();
    app.message_v2(0x0B62_7AD3, &mut host).unwrap();
    app.message_v2(0x49A7_80B2, &mut host).unwrap();
    assert_eq!(
        host.calls,
        [
            Call::Effect(Effect::ResourceManagerSlotAc(17)),
            Call::Effect(Effect::LaunchExceptionReportUploader),
            Call::Effect(Effect::LaunchExceptionReportUploader)
        ]
    );
    assert_eq!(app.message_v2(0x1234, &mut host), Ok((true, None)));
    app.message_v2(0x29A8_9B05, &mut host).unwrap();
    assert!(app.quitting());
}

#[test]
fn cheats_respect_arity_exact_gp_values_and_force_idle_value() {
    let mut app = app();
    let mut host = TestHost::default();
    app.message_server2_available = true;
    assert_eq!(
        app.message_v1(0x230E_27AC, GP_CHEAT as Token, b"ON", &mut host),
        Ok((true, Some(Boundary::PreferencesChanged)))
    );
    assert!(app.pause_on_focus_loss());
    for invalid in [b"off extra".as_slice(), b"disabled", b""] {
        assert_eq!(
            app.message_v1(0x230E_27AC, GP_CHEAT as Token, invalid, &mut host),
            Ok((true, None))
        );
        assert!(app.pause_on_focus_loss());
    }
    app.message_v1(0x230E_27AC, EXCEPTION_CHEAT as Token, b"test", &mut host)
        .unwrap();
    app.message_v1(0x230E_27AC, EXCEPTION_CHEAT as Token, b"test extra", &mut host)
        .unwrap();
    app.message_v1(0x230E_27AC, FORCE_IDLE_CHEAT as Token, b"", &mut host)
        .unwrap();
    assert_eq!(
        host.calls,
        [
            Call::Effect(Effect::RaiseTestException(b"test".to_vec())),
            Call::Effect(Effect::Message {
                message: Message {
                    kind: GAME_IDLE,
                    data: MessageData::Word(60)
                },
                delivery: Delivery::Post
            })
        ]
    );
}

#[test]
fn command_options_keep_numeric_prefixes_bytes_and_explicit_validation_boundaries() {
    let mut app = app();
    let mut host = TestHost::default();
    let line = CmdLine::new(b"-r1280x720x16 -gP -audio:off -music:enabled -sim:2.5junk -RDD:on -UserDir:Z:\\Mixed -AutoZoomChange:disabled -CPUCount:17junk");
    assert_eq!(
        app.read_command_line(&line, &mut host),
        [
            Boundary::ResolutionAcceptance {
                width: 1152,
                height: 864,
                depth: 16
            },
            Boundary::UserDirectoryAcceptance(b"Z:\\Mixed".to_vec())
        ]
    );
    assert_eq!(
        (app.preferences.width, app.preferences.height, app.preferences.depth),
        (800, 600, 32)
    );
    assert_eq!(app.directories.user_base_11c, b"U:\\");
    assert!(app.preferences.pause_on_focus_loss);
    assert!(!app.pause_on_focus_loss());
    assert!(!app.preferences.audio_04 && !app.preferences.audio_69);
    assert_eq!(app.preferences.radio, 1);
    assert_eq!(app.preferences.simulation, 2.5);
    assert!(app.preferences.rdd);
    assert_eq!(
        host.calls,
        [
            Call::Effect(Effect::RenderProperty { key: 0x25, value: 0 }),
            Call::Effect(Effect::CpuCount(17))
        ]
    );
    app.read_command_line(&CmdLine::new(b"-UserDir:OFF"), &mut host);
    assert_eq!(app.directories.user_base_11c, b"P:\\");
}

#[test]
fn exception_filter_is_installed_once_and_previous_filter_restored_once() {
    let mut app = app();
    let mut host = TestHost::default();
    for value in [
        b"-ExceptionHandling:on".as_slice(),
        b"-ExceptionHandling:verbose",
        b"-ExceptionHandling:enabled",
        b"-ExceptionHandling:off",
        b"-ExceptionHandling:disabled",
    ] {
        assert!(app.read_command_line(&CmdLine::new(value), &mut host).is_empty());
    }
    assert_eq!(
        host.calls,
        [
            Call::InstallFilter,
            Call::Effect(Effect::ExceptionOption { index: 6, value: true }),
            Call::Effect(Effect::RestoreExceptionFilter(42))
        ]
    );
    assert!(!app.exception_enabled);
    assert!(app.exception_verbose);
}

#[test]
fn resolution_all_unsigned_area_edges_custom_bypass_and_overflow() {
    let edges = [
        (0, (800, 600)),
        (479_999, (800, 600)),
        (480_000, (1024, 768)),
        (786_431, (1024, 768)),
        (786_432, (1152, 864)),
        (995_327, (1152, 864)),
        (995_328, (1280, 1024)),
        (1_310_719, (1280, 1024)),
        (1_310_720, (1600, 1200)),
        (1_919_999, (1600, 1200)),
        (1_920_000, (1800, 1440)),
        (2_591_999, (1800, 1440)),
        (2_592_000, (2048, 1536)),
        (3_145_727, (2048, 1536)),
        (3_145_728, (1024, 768)),
    ];
    for (area, expected) in edges {
        let (mut w, mut h) = (area, 1);
        fix_startup_resolution(&mut w, &mut h, false);
        assert_eq!((w, h), expected, "area {area}");
    }
    let (mut w, mut h) = (65_536, 65_536);
    fix_startup_resolution(&mut w, &mut h, false);
    assert_eq!((w, h), (800, 600));
    fix_startup_resolution(&mut w, &mut h, true);
    assert_eq!((w, h), (800, 600));
    for (width, height) in [
        (800, 600),
        (1024, 768),
        (1152, 864),
        (1280, 1024),
        (1600, 1200),
        (1800, 1440),
        (2048, 1536),
    ] {
        assert!(is_startup_resolution_ok(width, height, false));
        assert!(!is_startup_resolution_ok(width - 1, height, false));
    }
}

#[test]
fn app_file_names_preserve_empty_output_and_directory_layout_is_literal() {
    let mut app = app();
    let mut name = b"unchanged".to_vec();
    app.app_file_name(b".ini", &mut name);
    assert_eq!(name, b"unchanged");
    app.app_name = b"SC4".to_vec();
    app.app_file_name(b".cfg", &mut name);
    assert_eq!(name, b"SC4.cfg");
    for (folder, expected) in [
        (UserFolder::Regions, b"U:\\Regions\\".as_slice()),
        (UserFolder::Albums, b"U:\\Albums\\"),
        (UserFolder::HttpCache, b"U:\\HTTPCache\\"),
        (UserFolder::Temp, b"U:\\Temp\\"),
        (UserFolder::ExceptionReports, b"U:\\Exception Reports\\"),
    ] {
        assert_eq!(app.directories.user_folder(folder), expected);
    }
}

warn: could not canonicalize path C:\Users\tlori
\\?\C:\Users\tlori\FreeCity4\.claude\worktrees\core-application4\engine\core\src\application\lifecycle.rs:

// SPDX-License-Identifier: GPL-3.0-or-later
//! Ordered continuations for published lifecycle spans; missing owner methods stop execution.

use std::collections::VecDeque;

use crate::cmdline::CmdLine;
use crate::gz_messages::Token;
use crate::paths::helpers::splitpath;

use super::command_line::option;
use super::{
    Application, Boundary, Effect, Host, HostFailure, MessageData, POST_APP_INIT, POST_APP_SHUTDOWN, PRE_APP_INIT,
    PRE_APP_SHUTDOWN, QUIT_REQUEST,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Director {
    Sc4,
    Audio,
    GzWww,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Service {
    Allocator,
    Unnamed1,
    FileSystem,
    MessageServer1,
    MessageServer2,
    Log,
    Command,
    GameDataRegistry,
    LuaScript,
    Language,
    Unnamed2,
    Font,
    WindowControl,
    Clipboard,
    Unnamed3,
    ImeProxy,
    PersistResource,
    Registry,
    TextureBindingFactory,
    ScriptResourceFactory,
    Www,
    Unnamed4,
    Unnamed5,
    AudioAems,
}

pub const SERVICES: [Service; 24] = [
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
    Service::Font,
    Service::WindowControl,
    Service::Clipboard,
    Service::Unnamed3,
    Service::ImeProxy,
    Service::PersistResource,
    Service::Registry,
    Service::TextureBindingFactory,
    Service::ScriptResourceFactory,
    Service::Www,
    Service::Unnamed4,
    Service::Unnamed5,
    Service::AudioAems,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Helper {
    RenderProperties,
    CheatManager,
    GlyphTextureManager,
    StringDetokenizer,
    PersistResourceManager,
    TooltipManager,
    TutorialRegistry,
    WindowCommandSupport,
    ToolCommandSupport,
    MiscCommandSupport,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Object {
    Helper(Helper),
    GzMessageServer(u8),
    /// Wrapper constructor retains the borrowed base. Log receives None (null stream).
    Service {
        kind: Service,
        base: Option<Token>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lifecycle {
    PreFrameworkInit,
    Init,
    PostFrameworkInit,
    Shutdown,
    PreFrameworkShutdown,
    Quit,
    Idle,
    PluginChange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Flow {
    Complete(bool),
    NeedsImplementation(Boundary),
    Failed(HostFailure),
}

#[derive(Debug)]
enum Step {
    Effect(Effect),
    Boundary(Boundary),
    Create(Helper),
    Service(Service),
    SplitExecutable(Vec<u8>),
    Registry,
    Driver,
    Cheats,
    Tutorial,
    Send(u32),
    Post(u32),
    Watch(usize, Vec<u8>),
    StopWatchers,
    Lua { line: CmdLine, configured: Option<Vec<u8>> },
    Clear314,
    FreeHelpers,
    DestroyScreen,
    IdleGate,
    IdlePeriodic,
    IdleTail,
}

#[derive(Debug, Default)]
pub(crate) struct Runner {
    kind: Option<Lifecycle>,
    steps: VecDeque<Step>,
    waiting: Option<Boundary>,
    failed: Option<HostFailure>,
    result: bool,
    helpers: Vec<(Helper, Token)>,
    pub install_directory: Vec<u8>,
    pub executable_directory: Vec<u8>,
    pub executable_name: Vec<u8>,
    consumed_command_line: Option<CmdLine>,
    shutdown: bool,
}

impl Application {
    fn begin(&mut self, kind: Lifecycle, steps: Vec<Step>) -> Result<(), HostFailure> {
        if !self.lifecycle.steps.is_empty() || self.lifecycle.waiting.is_some() || self.lifecycle.failed.is_some() {
            return Err(HostFailure("application continuation already pending or failed"));
        }
        self.lifecycle.kind = Some(kind);
        self.lifecycle.steps = steps.into();
        self.lifecycle.result = true;
        Ok(())
    }

    pub fn lifecycle(&self) -> Option<Lifecycle> {
        self.lifecycle.kind
    }
    pub fn install_directory(&self) -> &[u8] {
        &self.lifecycle.install_directory
    }
    pub fn executable_directory(&self) -> &[u8] {
        &self.lifecycle.executable_directory
    }
    pub fn executable_name(&self) -> &[u8] {
        &self.lifecycle.executable_name
    }
    pub fn helper(&self, kind: Helper) -> Option<Token> {
        self.lifecycle
            .helpers
            .iter()
            .find(|(k, _)| *k == kind)
            .map(|(_, token)| *token)
    }
    pub fn consumed_command_line(&self) -> Option<&CmdLine> {
        self.lifecycle.consumed_command_line.as_ref()
    }

    /// Not a host callback or fallback: integration may resume only after the
    /// named missing owner implementation has actually run. This crate supplies
    /// no implementation of those boundaries and cannot perform a complete boot.
    pub fn resume_after_implemented(&mut self, boundary: &Boundary) -> Result<(), HostFailure> {
        if self.lifecycle.waiting.as_ref() != Some(boundary) {
            return Err(HostFailure("wrong application boundary continuation"));
        }
        if matches!(boundary, Boundary::FindTestScript(_)) {
            return Err(HostFailure("script resolution requires resume_after_script_resolution"));
        }
        self.lifecycle.waiting = None;
        Ok(())
    }

    /// FindTestScript itself is missing. An independently implemented resolver
    /// supplies its result here, never through Host. AP6: a missing result does
    /// not execute a script. The configured/switch input is retained in the boundary.
    pub fn resume_after_script_resolution(&mut self, resolved: Option<Vec<u8>>) -> Result<(), HostFailure> {
        if !matches!(self.lifecycle.waiting, Some(Boundary::FindTestScript(_))) {
            return Err(HostFailure("no script resolution pending"));
        }
        self.lifecycle.waiting = None;
        if let Some(path) = resolved {
            self.lifecycle
                .steps
                .push_front(Step::Effect(Effect::ExecuteLuaScript(path)));
        }
        Ok(())
    }

    pub fn begin_pre_framework_init(
        &mut self,
        executable: &[u8],
        line: &CmdLine,
        stdout_path: &[u8],
    ) -> Result<(), HostFailure> {
        let mut steps = vec![
            Step::Effect(Effect::ReadStartupOption("DebugLevel")),
            Step::Effect(Effect::ReadStartupOption("logging")),
            Step::Boundary(Boundary::BuildVersion),
            Step::SplitExecutable(executable.to_vec()),
            Step::Boundary(Boundary::ResolveUserDataDirectory),
            Step::Create(Helper::RenderProperties),
            Step::Effect(Effect::ReadStartupOption("AllowMultipleInstances")),
            Step::Effect(Effect::ReadStartupOption("Restart")),
        ];
        // AP5: the exact restart test/wait is unpublished. Presence never
        // delegates the wait algorithm or silently skips it.
        if line.is_switch_present(b"Restart") {
            steps.push(Step::Boundary(Boundary::RestartWait));
        }
        for director in [Director::Sc4, Director::Audio, Director::GzWww] {
            steps.push(Step::Effect(Effect::AddDirector(director)));
        }
        append_services(&mut steps);
        steps.extend([
            Step::Boundary(Boundary::CursorAndOsSettings),
            Step::Boundary(Boundary::PluginDateChecks),
            Step::Effect(Effect::LaunchExceptionReportUploader),
            Step::Boundary(Boundary::AddCodePlugins(Vec::new())),
            Step::Boundary(Boundary::OpenLogs),
            Step::Effect(Effect::SetEnvironment {
                name: "RZSTD_OUTPUT_FILE_PATH",
                value: stdout_path.to_vec(),
            }),
            Step::Effect(Effect::SetEnvironment {
                name: "RZSTD_OUTPUT_PREFIX",
                value: b"StdOut: ".to_vec(),
            }),
            Step::Boundary(Boundary::FindAndRegisterLanguages),
            Step::Boundary(Boundary::PreferencesChanged),
        ]);
        self.begin(Lifecycle::PreFrameworkInit, steps)
    }

    /// Also usable independently for framework composition/tests.
    pub fn begin_add_system_services(&mut self) -> Result<(), HostFailure> {
        let mut steps = Vec::new();
        append_services(&mut steps);
        self.begin(Lifecycle::PreFrameworkInit, steps)
    }

    pub fn begin_init(&mut self) -> Result<(), HostFailure> {
        self.begin(
            Lifecycle::Init,
            vec![
                Step::Effect(Effect::FetchFrameworkManagers),
                Step::Registry,
                Step::Driver,
                Step::Boundary(Boundary::DriverConfiguration),
                Step::Send(PRE_APP_INIT),
                Step::Effect(Effect::RegisterV1 {
                    kind: QUIT_REQUEST,
                    target: self.app_token,
                }),
                Step::Create(Helper::CheatManager),
                Step::Cheats,
                Step::Create(Helper::GlyphTextureManager),
                Step::Create(Helper::StringDetokenizer),
                Step::Boundary(Boundary::SetupResources),
                Step::Boundary(Boundary::LanguageManagerHookup),
                Step::Create(Helper::PersistResourceManager),
                Step::Boundary(Boundary::RenderPropertyVector),
                Step::Create(Helper::TooltipManager),
                Step::Boundary(Boundary::SlOnAppInit1),
                Step::Create(Helper::TutorialRegistry),
                Step::Tutorial,
                Step::Send(POST_APP_INIT),
            ],
        )
    }

    /// Watch paths and watcher tokens must already have been selected by a
    /// separately specified owner (AP6). This method only owns test/create/test.
    pub fn begin_post_framework_init(
        &mut self,
        line: &CmdLine,
        lua_option: Option<&[u8]>,
        watch_paths: [Vec<u8>; 2],
    ) -> Result<(), HostFailure> {
        let mut steps = vec![
            Step::Create(Helper::WindowCommandSupport),
            Step::Create(Helper::ToolCommandSupport),
            Step::Create(Helper::MiscCommandSupport),
        ];
        if option(line, b"NetCommandGenerator").is_some_and(|v| v.eq_ignore_ascii_case(b"enabled")) {
            steps.push(Step::Effect(Effect::NetworkCommandGeneratorEnabled));
        }
        for (index, path) in watch_paths.into_iter().enumerate() {
            steps.push(Step::Watch(index, path));
        }
        steps.extend([
            Step::Boundary(Boundary::InitAlphaSystem),
            Step::Lua {
                line: CmdLine::copy_of(line),
                configured: lua_option.map(<[u8]>::to_vec),
            },
        ]);
        self.begin(Lifecycle::PostFrameworkInit, steps)
    }

    pub fn begin_shutdown(&mut self) -> Result<(), HostFailure> {
        if self.lifecycle.shutdown {
            return Err(HostFailure("application already shut down"));
        }
        self.begin(
            Lifecycle::Shutdown,
            vec![
                Step::Send(PRE_APP_SHUTDOWN),
                Step::Boundary(Boundary::ShutdownHelperOrder),
                Step::FreeHelpers,
                Step::Boundary(Boundary::ShutdownMessageServer1),
                Step::Clear314,
                Step::Send(POST_APP_SHUTDOWN),
            ],
        )
    }

    pub fn begin_pre_framework_shutdown(&mut self) -> Result<(), HostFailure> {
        self.begin(Lifecycle::PreFrameworkShutdown, vec![Step::StopWatchers])
    }

    /// First call latches immediately, including if CloseCity is still pending.
    /// AP5 service identity is unresolved, but the two slot calls remain ordered.
    pub fn begin_quit(&mut self) -> Result<bool, HostFailure> {
        if self.quitting {
            return Ok(false);
        }
        self.begin(
            Lifecycle::Quit,
            vec![
                Step::Boundary(Boundary::QuitService),
                Step::Effect(Effect::QuitServiceSlot44),
                Step::Effect(Effect::QuitServiceSlot44),
                Step::Boundary(Boundary::CloseCity),
                Step::DestroyScreen,
                Step::Effect(Effect::FrameworkQuit(0)),
            ],
        )?;
        self.quitting = true;
        Ok(true)
    }

    pub(crate) fn begin_plugin_change(&mut self, path: Vec<u8>) -> Result<(), HostFailure> {
        self.begin(
            Lifecycle::PluginChange,
            vec![Step::Boundary(Boundary::AddCodePlugins(path)), Step::Post(0xA423_96C6)],
        )
    }

    pub(crate) fn start_idle(&mut self) -> Result<(), HostFailure> {
        self.begin(Lifecycle::Idle, vec![Step::IdleGate, Step::IdleTail])
    }

    pub fn run(&mut self, host: &mut impl Host) -> Flow {
        if let Some(error) = &self.lifecycle.failed {
            return Flow::Failed(error.clone());
        }
        if let Some(boundary) = &self.lifecycle.waiting {
            return Flow::NeedsImplementation(boundary.clone());
        }
        while let Some(step) = self.lifecycle.steps.pop_front() {
            if let Step::Boundary(boundary) = step {
                self.lifecycle.waiting = Some(boundary.clone());
                return Flow::NeedsImplementation(boundary);
            }
            if let Err(error) = self.run_step(step, host) {
                self.lifecycle.failed = Some(error.clone());
                return Flow::Failed(error);
            }
        }
        if self.lifecycle.kind == Some(Lifecycle::Shutdown) {
            self.lifecycle.shutdown = true;
        }
        Flow::Complete(self.lifecycle.result)
    }

    fn run_step(&mut self, step: Step, host: &mut impl Host) -> Result<(), HostFailure> {
        match step {
            Step::Effect(effect) => host.effect(effect),
            Step::Boundary(_) => unreachable!("run handles boundaries"),
            Step::Create(kind) => {
                let token = host.create(Object::Helper(kind))?;
                self.lifecycle.helpers.push((kind, token));
            }
            Step::Service(kind) => {
                let generation = match kind {
                    Service::MessageServer1 => Some(1),
                    Service::MessageServer2 => Some(2),
                    _ => None,
                };
                let base = generation
                    .map(|n| host.create(Object::GzMessageServer(n)))
                    .transpose()?;
                let object = host.create(Object::Service { kind, base });
                // AP6 portable ownership: wrapper constructor retained its base.
                if let Some(base) = base {
                    host.release_created(base);
                }
                let object = object?;
                let result = host.register_service(kind, object);
                host.release_created(object);
                result?;
                if kind == Service::MessageServer2 {
                    self.message_server2_available = true;
                }
            }
            Step::SplitExecutable(path) => {
                let parts = splitpath(&path);
                self.lifecycle.executable_directory = [parts.drive, parts.directory].concat();
                self.lifecycle.executable_name = [parts.name, parts.extension].concat();
            }
            Step::Registry => {
                if let Some(path) = host.install_directory() {
                    self.lifecycle.install_directory = path;
                } else {
                    self.lifecycle.steps.clear();
                    self.lifecycle.result = false;
                }
            }
            Step::Driver => {
                let driver = host.graphics_driver();
                if let Some(driver) = driver {
                    host.effect(Effect::SetRendererDriver(driver));
                }
                if !driver.is_some_and(|driver| host.check_graphics_driver(driver)) {
                    self.preferences.driver_f13 = 0;
                    self.preferences.driver_f0c = 1;
                    self.lifecycle.result = false;
                    self.lifecycle.steps = [
                        Step::Boundary(Boundary::SavePreferences),
                        Step::Effect(Effect::Warning {
                            title: "SimCity 4",
                            text: "SimCity 4 will run with the software driver when next launched.",
                        }),
                    ]
                    .into();
                }
            }
            Step::Cheats => {
                let manager = self
                    .helper(Helper::CheatManager)
                    .ok_or(HostFailure("missing cheat manager"))?;
                for (name, id) in [
                    ("GP", super::GP_CHEAT),
                    ("CreateException", super::EXCEPTION_CHEAT),
                    ("ForceIdle", super::FORCE_IDLE_CHEAT),
                ] {
                    host.effect(Effect::RegisterCheat { manager, name, id });
                }
            }
            Step::Tutorial => {
                let token = self
                    .helper(Helper::TutorialRegistry)
                    .ok_or(HostFailure("missing tutorial registry"))?;
                host.effect(Effect::InitTutorialRegistry(token));
            }
            Step::Send(kind) => self.post_sc4_app_message(kind, false, host),
            Step::Post(kind) => self.post_sc4_app_message(kind, true, host),
            Step::Watch(index, path) => {
                if !host.directory_exists(&path) {
                    host.effect(Effect::CreateDirectory(path.clone()));
                    if !host.directory_exists(&path) {
                        return Ok(());
                    }
                }
                if let Some(watcher) = self.watchers[index] {
                    host.effect(Effect::StartWatcher { watcher, path });
                }
            }
            Step::StopWatchers => {
                for watcher in self.watchers.into_iter().flatten() {
                    if host.watcher_running(watcher) {
                        host.effect(Effect::StopWatcher(watcher));
                    }
                }
            }
            Step::Lua { mut line, configured } => {
                let switch = option(&line, b"LuaScript");
                // AP6: option wins when both exist; switch is consumed only
                // when it supplies the script. Resolution itself remains open.
                let used_switch = configured.is_none() && switch.is_some();
                let script = configured.or_else(|| switch.clone());
                if let Some(script) = script {
                    if used_switch {
                        let index = line.args().iter().position(|arg| {
                            let one = CmdLine::from_parts(b"", [arg.as_slice()]);
                            option(&one, b"LuaScript").is_some()
                        });
                        if let Some(index) = index {
                            line.erase_argument(index as i32)
                                .map_err(|_| HostFailure("LuaScript erase"))?;
                        }
                    }
                    self.lifecycle
                        .steps
                        .push_front(Step::Boundary(Boundary::FindTestScript(script)));
                }
                self.lifecycle.consumed_command_line = Some(line);
            }
            Step::Clear314 => self.field_314 = None,
            Step::FreeHelpers => {
                // The actual order/free discipline is an AP5/AP6 boundary,
                // not a guessed reverse-COM-release loop. Integration must
                // destroy Init helpers before acknowledging ShutdownHelperOrder.
                self.lifecycle.helpers.retain(|(kind, _)| {
                    matches!(
                        kind,
                        Helper::RenderProperties
                            | Helper::WindowCommandSupport
                            | Helper::ToolCommandSupport
                            | Helper::MiscCommandSupport
                    )
                });
            }
            Step::DestroyScreen => {
                if let Some(screen) = self.screen_owner {
                    host.effect(Effect::DestroyCurrentScreen(screen));
                }
            }
            Step::IdleGate => {
                self.lifecycle.result = false;
                if self.timers.gate > 1000 {
                    self.timers.gate = 0;
                    let value = if self.timers.activity >= 300 {
                        self.timers.input = 0;
                        Some(60)
                    } else if self.timers.input > self.idle_threshold {
                        let idle = self.timers.input;
                        self.timers.input = self.idle_threshold.wrapping_sub(self.idle_subtract);
                        Some(idle)
                    } else {
                        None
                    };
                    if let Some(value) = value {
                        self.timers.activity = 0;
                        self.deliver(super::GAME_IDLE, MessageData::Word(value), true, host);
                    }
                    self.lifecycle.steps.push_front(Step::IdlePeriodic);
                    if self.timers.autosave > self.autosave_period {
                        self.timers.autosave = 0;
                        self.lifecycle
                            .steps
                            .push_front(Step::Boundary(Boundary::SavePreferences));
                    }
                }
            }
            Step::IdlePeriodic => {
                if self.timers.periodic > self.periodic_period {
                    self.timers.periodic = 0;
                    if let Some(manager) = self.resource_manager {
                        host.effect(Effect::ResourceManagerSlotAc(manager));
                    }
                }
            }
            Step::IdleTail => {
                if self.fully_paused {
                    host.effect(Effect::SleepCurrentThread(250_000));
                }
            }
        }
        Ok(())
    }
}

fn append_services(steps: &mut Vec<Step>) {
    for kind in SERVICES {
        steps.push(Step::Service(kind));
        if kind == Service::FileSystem {
            steps.push(Step::Boundary(Boundary::FindAndPossiblyFixIniFiles));
        }
    }
}

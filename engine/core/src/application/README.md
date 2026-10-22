# Portable application subset

Source: `docs/specs/application.md`, sections 1-10 only. This module is a partial
production owner of published decisions, not a replacement/default application,
native binding, coverage declaration or complete implementation of cSC4App.
The spec publishes about 57 of 322 functions. No C ABI or game integration changes.

`Application` owns mutable state without interior mutability. An integration must
serialize mutations and provide shared synchronized snapshots for simultaneous
native getters. Opaque tokens reuse `gz_messages::Token`; command parsing reuses
`cmdline::CmdLine`, cheat arguments reuse `commands::parse_delimited_text`, and
executable splitting reuses `paths::helpers::splitpath`. No RNG exists in this
published application subset, so no new generator or seeding rule is introduced.
No actual files, environment variables, process settings, UI or COM objects are
touched by the core: Host exposes only primitive effects.

## Ordered continuations

Begin a lifecycle operation, then call `run`. A `NeedsImplementation` result is a
hard boundary: calling `run` again repeats it without advancing. Host cannot
return a fabricated final answer for a missing application method. The separate
`resume_after_implemented` API is solely for composition after an independently
specified owner implementation has actually performed that missing method.
There is no such implementation supplied here. Tests explicitly acknowledge
these boundaries to verify ordering of later published spans; that is not
evidence that the boundaries themselves work. Script resolution has its own
explicit result continuation. A failed external primitive is terminal; no
retry or speculative rollback is inferred (AP6). A second begin while a
continuation is pending returns an integration error without replacing it.

Message handlers return `Ok((true, boundary))` on native routes, including unknown
messages. Busy-continuation failures are Rust adapter errors, not native false
acknowledgments. The caller must run queued Quit/plugin continuations and satisfy
returned direct boundaries such as RequestQuit. Region/city events update only
the published app-mode field; no city/region algorithm is implemented.

## Published method map

| Spec method/surface | Implementation or exact boundary |
|---|---|
| section 1 object fields/faces | `Application` state; no native layout, face identity or vtable construction (AP4) |
| primary 3, v1 DoMessage | `message_v1`; watcher identity check, plugin-loading boundary before posting; GP/exception/idle cheats |
| primary 6 ReadPreferences | section 12 unresolved; no host delegation |
| primary 7 ReadCommandLine | `read_command_line`; all listed option effects; unresolved acceptance returns for `r` and UserDir (AP8) |
| primary 8/10 SetupWinCtrlMgr/SetupFontSet | section 12 unresolved |
| primary 11/12 FullGamePause/OnAppActivate | `full_game_pause`, `on_app_activate`; idempotent state transitions and exact four flags |
| primary 13 NewCity | assigned city owner; excluded |
| primary 14 Quit | `begin_quit`/`run`; latch, AP5 service boundary, two calls, assigned CloseCity boundary, screen destroy, framework quit(0) |
| primary 4/5/9 | untraced/empty per section 1.1; no invented implementation |
| cISC4App 3 OnIdle | `begin_idle`/`run`; strict gate/timers, both activity branches, autosave boundary, periodic manager, unconditional paused tail |
| cISC4App 4/5 pumps | `run_message_server_pump`; external message-server primitive; internals belong to existing gz_messages owners |
| cISC4App 6-16 requests/load/close/save | section 11 externally assigned; all omitted, including RequestQuit and RequestQuitFromRegion |
| cISC4App 17 SavePreferences | section 12 unresolved; blocking continuation on driver failure and idle autosave |
| cISC4App 18 enable pause | `enable_full_game_pause_on_focus_loss`; writes both prefs and app state |
| cISC4App 19/20 video prefs | folded/unpublished, unresolved |
| cISC4App 21-24 debug/popup get/set | plain shared/public `debug_enabled`, `popup_dialogs_enabled` state |
| cISC4App 25/26, 32-34, 37 | folded/unpublished, unresolved |
| cISC4App 27/28/30 app/ini/cfg names | shared `app_name`, `app_file_name` (`.ini`/`.cfg`); empty name preserves output |
| cISC4App 29/31, 69/70 | no behavior published; unresolved |
| cISC4App 35/36 region/regional-city | assigned region getters, excluded; no synthesized region result |
| cISC4App 38 preferences | shared `preferences` value; no native preference-object layout |
| cISC4App 39 new-city specification | assigned city owner, excluded |
| cISC4App 40-44 debug console/Gimex/detokenizer/location saver/render properties | `helper` for constructed detokenizer/render properties; other pointer acquisition/construction unspecified; no fabricated pointers |
| cISC4App 45/48 glyph manager | `helper(GlyphTextureManager)` after Init; native embedded offset conflicts with exception handler (section 8); no native alias claim |
| cISC4App 46/47 unnamed/Lua | pointer-only fields unspecified; no fabricated pointers |
| cISC4App 49 tutorial registry | `helper(TutorialRegistry)`; external object Init primitive |
| cISC4App 50 GetCDAppDirectory | AP3 byte meaning unresolved; deliberately no path getter under that alias |
| cISC4App 51-66 directories | `Directories` stored byte fields, `plugin_directory`, `user_folder`; aliases kept offset-named (AP3) |
| cISC4App 67/68 AddDynamicLibrary | section 12 unresolved |
| section 3 PreFrameWorkInit | ordered runner: option reads, version boundary, executable split, user-data boundary, render properties creation, restart boundary, directors/services, OS/date boundaries, uploader, plugin/log/language/preferences boundaries, exact environment effects |
| section 3.1 Init | registry silent failure vs driver fallback/save/warn failure; synchronous lifecycle messages, v1 quit registration, three cheats, helper construction and published order; AP5 config rule + section 12 resource/language/render/SL boundaries |
| section 3 PostFrameWorkInit | three supports, conditional enabled network generator, test/create/test/watch for both supplied paths, alpha boundary, Lua resolution/consumption; untraced path/precedence choices AP6 |
| section 4 AddCOMDirectors | Sc4, Audio, GzWww effects in published order |
| section 4 AddSystemServices | `SERVICES`/runner: all 24 registrations, INI boundary after filesystem, GZ wrapper bases, log null output stream |
| section 5 Shutdown | pre-message, unresolved exact helper destruction, unresolved v1 call, clear +0x314 without release, post-message |
| section 5 PreFrameWorkShutdown | query each watcher, stop only running watchers; borrowed tokens never released |
| section 6 v2 DoMessage | `message_v2`: mode, preferences boundary, Quit, uploader, assigned obliteration boundary, optional manager, input timer reset; always acknowledges native route |
| section 6 PostSC4AppMessage | app-token data, synchronous Send vs asynchronous Post(false) |
| section 7 Is/FullGamePause | shared state and exact focus transition rules; enable toggle alone does not invent unpause |
| section 8 resolution helpers | published supported height/min-width table, custom bypass, unsigned wrapping area and exact fallback ranges |
| section 9 directory selection | lazy eight-folder cache; Scripts uses plugin base (-1), other user folders use +0x11C; UserDir off copies plugin base without invalidating caches |

## Remaining owner and assignment scope

AP3 aliases/flag, AP4 untraced faces/primary methods, AP5 restart/config-rule/
shutdown/service details, and AP6-AP9 implementation ambiguities remain open.
Section 12 remains open in full: setup resources/fonts/window controls, plugin
and dynamic-library algorithms, preferences persistence/change handling, DB
callbacks, LoadRegistry, save-path/scoped-object/unknown-city slots and residual
322-function inventory. The component owners cWinSC4App, COM directors,
cSC4Preferences, cSC4GameDataRegistry and cSC4AutoGamePause are not implemented.
Section 11 request/load/save/new-city/close/obliteration flows and all region
owners, new-region/import/establish/region-view request flows are excluded by
assignment (region assignment through 2026-11-21). Even a green portable gate
does not close any of these gaps or justify a native/default coverage claim.

Ownership is explicit: create returns an owned temporary; registration retains
a separate framework reference; wrapper construction retains a separate base
reference. The runner releases only its created service/base references, even
on a failed registration/construction. Query drivers, watchers, screen owners
and resource managers are borrowed and never released. Init helper destruction
is blocked pending the exact GlobalHeapFree/embedded-object discipline; it is
never replaced by COM Release or a guessed reverse-order destructor.

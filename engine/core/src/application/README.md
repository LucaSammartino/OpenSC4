# Portable application owner

Source: `docs/specs/application.md` §§1–15; save ordering additionally follows
its explicit references to `foundations.md` §§3.7 and 11.1. No native binding,
plugin/capi wiring, ABI change, simulator or framework implementation is included.

`Application` owns app state and algorithms. `Host`, `CityHost`, `ResourceHost`,
`CompatibilityHost`, `StartupHost`, `EntryHost` and `PumpHost` expose foreign
objects, filesystem/stream/COM primitives and clocks. Mutations need external
serialization. Getters use ordinary shared state, never `RefCell`. Native face
offsets are facts in `FACES`, not Rust pointer adjustments. Returned tokens are
borrowed unless a create/read explicitly transfers one reference. Registration
retains separately. The app retains regional-city arguments before releasing its
old holder and owns the city/segment references it eventually releases.

The existing ordered runner remains available as `run` for composition. Use
`run_owned` to execute the specified preference/plugin/city methods at its
boundaries. It also performs the instance poll, INI override selection and the
published hardware-description correction. Unpublished methods stop at a named
`Flow::NeedsImplementation`; repeated calls do not acknowledge those gaps.
`resume_after_implemented` is only for independently implemented missing owners;
`resume_after_script_resolution` supplies the actual script lookup result.
Framework entry uses `app_start`. The complete bounded message-pump algorithm
is `pump_messages`; legacy `Effect::Pump` and city pump effects must map to that
algorithm with foreign queue/clock primitives. Message entry points acknowledge
native routes and expose direct boundaries for composition; returned RequestQuit
means `request_quit(false, false)`, CityObliteration means `do_city_obliteration`.

File operations run only through foreign traits. Save requires the independently
resolved `city_state.resolved_save_target`: the spec explicitly leaves the naming
helper open. Without it, `SavePathNaming` precedes city/file mutation. Obliteration
also checks that input before destructive work. Scopes balance on success,
failure and unsupported returns. Scoped foreign helpers use the exact published
constructor arguments; their internal algorithms remain foreign. `CityHost`
effects for missing foreign singletons are no-ops. DirectedQuit creates and
queues UnilateralQuit to the v2 app target and releases the local message.

Raw preference storage preserves all 0x1734 bytes outside modeled fields. Saving
changes the live language/IME bytes before deleting and rewriting the one record.
Resources expose only the published summary: explicit INI/packed-option/factory
inputs avoid inventing the tables and paths that §12 leaves open. No complete
boot, video/font/window setup or resource-callback implementation is claimed.

## Owner closure

Every row concerns cSC4App. `tests::` names the existing regression tests;
`owner_tests::` names the new tests. Open entries correspond to explicit
untraced/summary-only material and the questions in application.md's question file.

| Spec section | Code item | Test / remaining open contract |
|---|---|---|
| 1 Object layout | `compatibility::FACES`, `Application`, `CityState` | `restart_waits_only_when_needed_and_times_out_at_thirty_units`; open: AP4 face identity |
| 1.1 Primary table | message/pause/command/new/quit methods | `tests::cheats_respect_arity_exact_gp_values_and_force_idle_value`; open: AP4 slots 4/5, §12 empty/untraced setup bodies |
| 1.2 Fields | app state, timers, preferences image, city state, handles | `tests::idle_gate_and_focus_tail_preserve_strict_comparisons`; open: AP6 native constructor defaults |
| 2 cISC4App | `app_interface_getter`, `app_interface_getter_language`, `app_file_name`, video/request/save/load APIs | getter regression and `settled_directory_and_path_slots_use_distinct_sources_and_literal_concatenation`; open: AP3 semantic aliases only |
| 3 Startup order | `app_start`, `begin_pre_framework_init`, `begin_post_framework_init`, `wait_for_instance` | `entry_and_resource_summary_keep_order_and_reject_missing_ini_directory`, `restart_waits_only_when_needed_and_times_out_at_thirty_units`, existing startup/watcher/Lua tests; open: AP6 version/user-data/OS/date/log/language/alpha/script details |
| 3.1 Init | `begin_init`, `apply_driver_description`, `run_owned`, v2 subscriptions | `tests::successful_init_exact_order_includes_every_unresolved_owner_call`, `owned_runner_executes_preference_failure_autosave_plugins_and_close`, `driver_description_correction_and_subscriptions_are_exact`; open: AP5 config target, AP6 unpublished helper calls, AP13 setup |
| 4 Services | `SERVICES`, director/service steps | `tests::all_services_register_in_published_order_and_wrap_two_gz_bases`; open: AP10 unnamed identities/bodies (foreign) |
| 4.1 Directors | ordered Sc4/Audio/GzWww effects | `tests::pre_framework_order_keeps_restart_and_plugin_boundaries_and_exact_environment` |
| 4.2 Services | corrected 28-service order; graphics interface passed borrowed | service-order and allocation/registration failure tests; open: AP6 failure rollback, AP10 unnamed classes |
| 5 Shutdown | ordered lifecycle methods | existing shutdown/quit tests |
| 5.1 Shutdown/watchers | `begin_shutdown`, `unregister_app_cheats`, `unsubscribe_app_messages`, `begin_pre_framework_shutdown` | `tests::shutdown_waits_for_heap_discipline_then_v1_call_before_clear_and_post`, watcher test, subscription test; open: AP6 exact heap-helper destruction order/free discipline |
| 5.2 Quit | latch, two foreign service calls, `close_city`, screen destroy, framework quit | `owned_runner_executes_preference_failure_autosave_plugins_and_close`; open: AP5 service meaning (calls implemented) |
| 6 Messages | v1/v2 and message posting | existing routing tests |
| 6.1 v1 | watcher identity/filter, plugin reload, cheats, quit request | watcher/cheat tests and owned runner test; open: AP9 GP apply/focus lookup |
| 6.2 v2 | mode changes, idle/input, quit/obliteration routes | `tests::v2_routes_modes_idle_mouse_and_assigned_owners_without_region_algorithms`; open: AP14 DoPreferencesChanged body |
| 6.3 Posting | `post_sc4_app_message`, `city_message` | exact init sequence tests; lifecycle messages synchronous |
| 7 Idle/focus | ordered idle steps and pause methods | existing strict-threshold/focus tests |
| 7.1 OnIdle | timers, idle message, preference autosave, periodic call, paused tail | `owned_runner_executes_preference_failure_autosave_plugins_and_close`, existing equality/branch/tail tests |
| 7.2 Focus | idempotent pause and exact four flags | `tests::focus_toggle_does_not_invent_unpause_and_repeated_activation_is_idempotent` |
| 8 Command line/resolution | existing parser, resolution helpers, exception-filter ownership | command/driver/alias/resolution tests; open: AP8 resolution/UserDir acceptance and numeric edge semantics, AP11 opaque meanings |
| 9 Directories | `Directories`, lazy caches, distinct slots 51–58, signed language directory dispatch | lazy-cache and settled-directory tests; open: AP3/AP7 aliases/append details |
| 10 Implementation notes | synchronous lifecycle sends, distinct failure paths | registry/driver failure tests and owned runner preference test |
| 11 City lifecycle | `city` module | request, load, save and close tests below |
| 11.1 RequestNewCity | `request_new_city` | `requests_new_load_and_region_follow_their_distinct_no_city_paths` |
| 11.2 RequestCloseCity | `request_close_city` | `request_close_no_and_blocked_save_do_not_attempt_file_io` |
| 11.3 Region view | `request_go_to_region_view` | request-path and blocked/cancel dialog tests |
| 11.4 CloseCity/partial | `close_city`, `partial_close_city`, `touches` | `new_and_close_city_order_and_inclusive_region_refresh` (edge, corner and gap examples) |
| 11.5 City segment | register/unregister methods, load reference ownership | `read_load_reject_versions_and_release_half_read_objects`, register failure/cleanup test |
| 11.6 Obliteration | `do_city_obliteration` | `obliteration_keeps_template_only_during_new_city_and_cleans_temp_on_read_failure`, successful obliteration test; open: AP14 naming and AP15 unspecified failure/view defaults |
| 11.7 Init messages | `city_init` | new-city and successful-load exact message sequences |
| 11.8 NewCity | `new_city`, `new_city_inner` | new/close order test; rejects an already open city |
| 11.9 LoadCity/Read/Write | `load_city`, `read_city_root`, `write_city_root` | successful-load and version/half-read cleanup tests |
| 11.10 RequestLoadCity | `request_load_city` | request-path test; open: AP14 conflicting SavedCity summary |
| 11.11 SaveCity(flag) | `save_city_current` | empty/current-path save test |
| 11.12 RequestSaveCity | `request_save_city` | blocked-save, notification/Alt-save tests |
| 11.13 RequestQuit | `request_quit` | `quit_requests_queue_directed_delivery_and_balance_cancelled_pause`, blocked/cancel dialog tests |
| 11.14 Quit from region | `request_quit_from_region` | directed delivery/cancel test; no save |
| 11.15 Terrain picker | `load_height_field`, `terrain_message` | centering/truncation/cancel test; open: AP13 INI path source, AP4 unnamed face identity |
| 11.16 SaveCity(path) | `save_city`, `replace_file`, root/store ordering | save success/rollback/naming tests; open: AP14 target naming, §12 unnamed scoped-helper internals |
| 12 Open | typed boundaries and unsupported getter-slot result | `save_naming_gap_balances_scopes_without_changing_city_or_files`; open: AP3–AP15 as applicable, no invented implementation |
| 13 Preferences | raw image plus typed app fields | preference round-trip/quirk test |
| 13.1 File | `read_preferences`, `save_preferences`, record key/version constants | `preferences_preserve_unmodeled_bytes_and_clear_live_language_and_ime` |
| 13.2 Fields | `Preferences`, `PreferencesImage`, command-line effects | preference and command tests; open: AP11 field meanings, native defaults AP6 |
| 13.3 Reapply/video | `preference_application::{do_preferences_changed,CheatRequests,simulator_speed}`, `video` | `settled_tests` covers ordering, language fallback failure, mode/driver, speed wrap, copied-cheat repetition/nonprogress, byte mappings and unknown-value preservation; runner integration pending |
| 14 Plugins/resources/files | `compatibility` methods | plugin/resource summary tests |
| 14.1 Code plugins | `add_code_plugins`, `add_dynamic_library_with_bypass`, owned watcher continuation | `admin_enabled_uses_only_exact_zero_or_case_insensitive_false_and_bypass`, plugin and owned runner tests |
| 14.2 Resources | `registry_entry`, `load_registry_summary`, `setup_resources_summary` | registry substitution and resource enumeration tests; open: AP13 full callback/font/control/cursor/factory details |
| 14.3 Files | INI existing-directory overrides; terrain INI effects; SavedCity stub | resource/terrain/request tests; open: AP13 path resolution, AP14 conflicting SavedCity summary |
| 15 Pumps | `pump_messages` | `pumps_obey_absence_max_and_strict_minimum_time_bounds` |

## Conformance examples

The supplied application spec has no separate conformance-test section. Its
worked cases are covered: 1366×768 and 1280×720 resolution in the existing
resolution tests; inclusive edge/corner city reload in the close test; successful
renamed save and failed replacement rollback in the save tests; and saved-file
name notification in the request-save test. Foreign city/framework/rendering and
simulator internals remain outside this owner. Open questions prevent claiming
all 322 native functions or an executable replacement app.

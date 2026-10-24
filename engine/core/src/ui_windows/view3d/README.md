# cSC4View3DWin

Implemented column: `engine/core/src/ui_windows/view3d.rs` and the modules in
`engine/core/src/ui_windows/view3d/`. This is one core owner, with foreign camera,
renderer, control, simulator and window services behind traits. No C ABI or plugin
wiring is included. Picking delegates to the renderer; projection delegates to
`StatusHost::project`, rather than inventing a second camera projection.

VW19 is settled and implemented in both record adapters. There are no outstanding
spec questions in this table. This module still **does not claim complete owner
closure**: `remaining` denotes implementation or test work against settled specs,
not an Unsupported contract. The coordinator must not claim the whole owner from
this partial implementation.

## Owner closure

Every row refers to cSC4View3DWin. Tests are in `owner_tests.rs` and its included
`resolved_tests.rs`, `trigger_load_tests.rs` and `closure_tests.rs`, unless qualified. Entries describe core coverage; they do not
claim measurement in the real game.

| Spec section | Code item (implemented column) | Test / closure status |
|---|---|---|
| view3d-window §1 identity | `query_interface`, shared reference count | `vw_t1_qi_exact_interfaces_and_shared_count` |
| §1.1 window overrides | `slots`, `on_command`, typed window handlers | `vw_t25_precity_shutdown_preserves_city_objects_and_noop_command`; no-op slot 143 covered; remaining: unified override adapter |
| §2 construction | `OwnerState`, `Lifecycle`, `Motion`, `ViewState`, `PanelState` defaults | VW-T12/T13/T14, lifecycle tests, `vw_t26_dormant_fields_and_panel_extras` |
| §3 Init | `init`, `registry::COMMANDS`, `LifecycleHost::register_view_command` | `init_orders_registration_keeps_base_failure_and_tolerates_keymap_failure`, `vw_t28_registry_is_literal_and_ordered` |
| §3.1 cheats | `process_cheat`, decimal-prefix `atof`, `CheatHost::decimal_separator` | VW-T5/T6; `vw_t31_sizeof_crt_grammar`, `sizeof_uses_host_locale_decimal_separator` |
| §4 Shutdown | `shutdown`, `shutdown_helpers`, `shutdown_tail` | VW-T2, GUI shutdown tests, `shutdown_releases_each_deferred_queue_reference_once`, `dormant_helpers_stop_but_dormant_reference_only_releases`, VW-T27 overlay vector; remaining: exhaustive teardown audit |
| §5 notifications | `notification`, `pre_city_shutdown`, object-record methods | VW-T3, reference tests, `vw_t25_precity_shutdown_preserves_city_objects_and_noop_command` |
| §5.1 pause / mission | `notification`, borrowed `Notification::attachment`, `status::update_mission_progress` | `notification_view_hidden_pause_mission_and_progress_guards`, `vw_t33_mission_uses_borrowed_attachment_and_active_situation` |
| §5.2 view updates | `notification`, `refresh_background`, temporary hidden pause | notification and paint tests, `vw_t36_background_refresh_latches_mode_and_preserves_it_on_failure`; remaining: fresh mode-0 2D-failure/3D-success vector |
| §6 window messages | `window_message`, `messages::handle_advice_link` | VW-T4, `window_focus_accelerator_button_filter_and_clock_routes`, `vw_t34_advice_destinations_and_literal_envelopes` |
| §6.1 focus loss | `window_message` focus-loss branch | `window_focus_accelerator_button_filter_and_clock_routes` |
| §6.3 filter | `filter_message`, `apply_panel_tab` | `child_filter_uses_existing_tab_state_and_special_type12_branch`; remaining: audit common button follow-up |
| §6a city init | `pre_city_init`, `city_object`, `post_city_init`, `read_tunable_values`, `CityCall::{Construct, QueryGraphics}` | existing city init tests; VW-T27 fallback, graphics-gate, reuse and fatal-allocation vectors in `closure_tests.rs`; remaining: full city-service wiring audit |
| §6b GUI lifetime | `add_gui`, `remove_gui`, `GuiState`, `GuiCall::BaseShutdown` | existing GUI ownership tests, VW-T26/T27 and `required_gui_helpers_and_panel_qi_are_fatal`; remaining: full required main-panel failure and auxiliary setup audit |
| §6c input | `process_key`, `on_character`, `on_mouse`, `edge_scroll` | VW-T7/T8/T9/T10 and traversal/tutorial tests; remaining: exhaustive edge/capture branch vectors |
| §6d fields | `OwnerState`, `Motion`, `ViewState`, `DisplayState` | defaults and input/motion tests, `vw_t26_dormant_fields_and_panel_extras` |
| §6e control stack | host-aware control push/pop/persistent methods | `control_modes_init_failure_and_persistent_lifetime`, capture-release tests; legacy token-only catalog entry points still require adapter integration |
| §6f scroll / zoom / views | `Motion`, `change_view`, `set_zoom`, `set_zoom_limits`, `pick_terrain`, `pick_occupant` | VW-T11/T12/T13, `vw_t21_inverted_limits_minimum_first`, picking and hidden-pause tests |
| §6g commands | `process_command`, `process_motion_panel_command`, typed tool methods, `handle_button_activated` | VW-T11–T15 and command guards, `vw_t28_bus_and_pipe_buttons_dispatch_commands_without_parameters`; remaining: exhaustive registry-to-dispatch audit |
| §6h panels / registry | `PanelState`, panel/filter methods, `COMMANDS` | VW-T14, tab/filter tests, `vw_t28_registry_is_literal_and_ordered`; remaining: mayor pane bookkeeping audit |
| §6i menus / buttons | `menus`, `actions`, catalog methods, optional popup memory | catalog/tool/disaster tests, `vw_t32_landscape_scripts_keep_caller_ids_and_transport_has_no_memory`; remaining: exhaustive menu/catalog identity and common tutorial-follow-up audit |
| §6j status | `status`, `DisplayState`, `StatusHost::cost_visibility` | VW-T17/T18, widget/rating/mission tests, `vw_t22_cursor_priorities_and_in_place_update`, `vw_t23_fade_alpha_motion_and_hidden_reset`, `vw_t24_date_formatter_year_boundaries`, `cost_fade_redraw_does_not_show_hidden_bmp` |
| §6k save / triggers | `references::{save_object_record, load_object_record}`, `View3DWin::{save_record, load_record}`, `state::load_triggers`, `StlHashMap` trigger state | VW-T16, opaque-reference tests, `vw_t30_trigger_wrapping_bucket_heads_and_growth`; VW-T40–T44 in `trigger_load_tests.rs` cover both record adapters |
| §6l helpers | `Coverage`, `CoverageHost`, `refresh_background`, `setup_graph`, `stop_auxiliary`, borrowed getters, empty debug slot, `messages::handle_advice_link`, `status::update_mission_progress` | `vw_t35_borrowed_getters_and_debug_no_effect`, `vw_t37_t38_coverage_transform_fallback_and_stale_cache`, `vw_t39_graph_resource_order_and_conditional_stop`, `coverage_init_shutdown_and_graph_add_gui_are_owner_operations`, VW-T33/T34; remaining: fresh-mode-0 fallback VW-T36 |
| §6m quirks | motion/commands, city construction, GUI lifetime and helper state, `handle_button_activated` terrain posting | VW-T11/T12/T13/T21/T26/T27/T30, `vw_t29_owner_posts_whole_map_requests_without_installing_controls`; remaining: complete required-failure audit. Terrain owns erosion/refresh after the posted request |
| §6n paint completion | `paint`, `paint_deferred`, `MotionHost` | `vw_t19_t20_deferred_tail_and_timer_boundaries`, `vw_t19_safe_but_disabled_modal_drops_one_message_without_repost`, existing ordered paint and decay tests |
| §7 conformance | `owner_tests`, `resolved_tests`, `trigger_load_tests`, `closure_tests`, `closure_round3_tests` | Existing VW-T1–T18 plus T19–T35/T37–T44, including the T29 owner posting boundary; T36 still lacks its fresh-mode-0 fallback branch. Terrain's part of T29 is outside this owner |
| §8 questions | `state::load_triggers` | VW19 settled; `vw_t40_trigger_collision_round_trip`, `vw_t41_trigger_duplicate_overwrite`, `vw_t42_trigger_load_preserves_existing_and_empty_stream`, `vw_t43_trigger_pair_count_does_not_reserve_or_grow_for_duplicate`, `vw_t44_trigger_load_grows_only_when_inserting_new_key`; no older VW question is treated as still open |
| view3d §1 window identity | `View3DWin`, `slots`, save-record constants | QI/shared-count, layout and save-record tests |
| view3d §5.1 paint order | `paint`, `MotionHost` | `paint_gates_preserve_tail_and_write_time_before_draw`, decay/pause tests, VW-T19/T20; cost → scroll → resume → context time → one renderer Draw → tail |
| view3d §5.2 draw fields | `Motion` | paint gating, timing, decay and hidden pause tests |
| view3d §5.3 save | `references::load_object_record`, `state::load_triggers`, four-reference object-record adapter | `save_uses_opaque_identities_subobjects_and_load_interfaces_before_init`; VW-T40–T44 cover trigger load order through both record adapters |
| view3d §10 window quirks | `paint`, view state and save methods | paint and reference tests; renderer-owned quirks are outside this owner |
| view3d §11 window vectors 13–15, 20, 25 | `paint`, hidden pause, save methods | gate/timer/draw ordering, pause expiration and reference tests; vector 19 effects sweep remains a renderer/effects integration obligation |

Other sections of view3d.md assign behavior to cSC43DRender,
cSTETerrainView3D, effect drawing and span renderers, rather than this owner.
Their implementations are not claimed here. The owner issues one renderer draw;
the renderer owns ordering inside that draw.

`Coverage` retains unseen keys with null effects and original cached radii, as
specified. SetCategories clears even for an unchanged mask. Hosts must release
the cache using `clear` before destruction because foreign references cannot be
released without a host. Init recalculates the embedded coverage helper and
Shutdown clears it. `add_gui` invokes `setup_graph` after creating the graphs
procedure, with no resource calls if the graph script was absent.

The logical records expose trigger pairs in bucket/head order, not sorted key
order. Loading assigns counts in stream order into the existing table, preserving
absent keys, existing node positions and capacity. Only new-key insertions can
grow the table. Loading neither pre-sizes from the pair count nor sends trigger
notifications. Byte serialization remains the foreign serializer's responsibility.

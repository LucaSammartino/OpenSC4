# cSC4View3DWin

This extends `ui_windows::view3d`; it does not introduce another city-window
owner. Controls, camera, renderer, simulator and window services are foreign
traits. Tokens are opaque; input callbacks expose reference operations explicitly.
The implementation is in progress. The table distinguishes implemented contracts
from work that remains, rather than treating an unimplemented section as open.

## Owner closure

All rows below refer to cSC4View3DWin. Test names are in `owner_tests.rs` unless
qualified with a module. `pending` means implementation work remains.

| Spec section | Code item | Coverage / remaining work |
|---|---|---|
| view3d-window §1, §1.1 identity/interfaces | `View3DWin::query_interface`, `slots`, `VIEW3D_LAYOUT` | `vw_t1_qi_exact_interfaces_and_shared_count`; window override dispatch pending |
| §2 construction | `OwnerState::default`, `Lifecycle::default`, `constructed`, `ViewState`, `PanelState` | motion defaults VW-T12/T13, panels VW-T14; remembered catalogs and scripting hook; window base stays foreign |
| §3 Init | `init`, `LifecycleHost` | `init_orders_registration_keeps_base_failure_and_tolerates_keymap_failure`; open: VW9 exact registry records |
| §3.1 cheats | `process_cheat`, `CheatHost`, `recorder` | VW-T5/T6, render/recorder/control-allocation tests; camera SizeOf old-value guard belongs to foreign camera |
| §4 Shutdown | `shutdown`, `shutdown_helpers`, `shutdown_tail` | VW-T2, `gui_failed_creations_and_shutdown_progress_order`; open: VW8 unidentified helper creation |
| §5 notifications | `notification` | VW-T3, command-QI priority, 33-entry subscription set; pre-city-shutdown helper, save adapter and widget refresh integration pending |
| §5.1 pause/mission | `notification` | `notification_view_hidden_pause_mission_and_progress_guards`; mission derived-data conversion pending |
| §5.2 view update | `notification` | `notification_view_hidden_pause_mission_and_progress_guards` |
| §6 window messages | `window_message` | VW-T4 and `window_focus_accelerator_button_filter_and_clock_routes`; advice/catalog helper integration pending |
| §6.1 focus loss | `window_message` | `window_focus_accelerator_button_filter_and_clock_routes` |
| §6.3 message filter | `filter_message`, `apply_panel_tab` | `child_filter_uses_existing_tab_state_and_special_type12_branch`; button dispatch integration pending |
| §6a city init | `pre_city_init`, `post_city_init`, `CityHost` | `city_pre_post_initialization_orders_camera_and_gui_then_releases_progress`, `city_allocation_failures_are_null_and_obliteration_hides_panel4`; foreign renderer/camera and GUI failures tolerated |
| §6b GUI lifetime | `add_gui`, `remove_gui`, `GuiHost`, `GuiState` | `gui_lifetime_removes_filters_before_destruction_and_releases_queries`, allocation/shutdown tests; open: VW6 per-panel extras and 2EC/2F0 creation |
| §6c input | `process_key`, `on_character`, `on_mouse`, `edge_scroll` | VW-T7/8/9/10; character bound, tutorial rewriting, exit traversal tests; additional edge/capture branches need tests |
| §6d fields | `OwnerState`, `Motion`, existing layout constants | input/motion fields implemented; other owner state pending; open: VW8 |
| §6e input stack | `set_current_control`, `remove_current_control`, persistent methods | `control_modes_init_failure_and_persistent_lifetime`, `remove_control_sets_default_cursor_and_releases_capture`; catalog legacy token path still needs lifecycle integration |
| §6f scrolling/zoom/views | `Motion`, zoom, filter/level/contour methods, `paint` | VW-T11/12/13, filter reference/message order, contour pairing, edge recalculation, hidden pause and paint tests; terrain selection/picking and tuning pending |
| §6g commands | `process_motion_panel_command`, `query_tool_command` | VW-T11/12/13/14/15 and CW/CCW quirk; remaining zoning/network/lot/dispatch/save/quit/options/animation commands pending; open: VW9 for incomplete registration/ID inventory |
| §6h panels | `PanelState`, panel/minimize methods | VW-T14; tab/filter establishment path, established/obliterated callbacks, budget failure and GUI object integration pending; mayor visibility bookkeeping needs audit |
| §6i buttons/catalogs | existing `catalog` module | existing catalog tests cover script toggle/create/close; new button/catalog-kind routing pending; open: VW7 for external message meaning |
| §6j displays | `DisplayState`, `StatusHost`, status methods | VW-T17/T18, rating/alert/mission/speed tests; date/funds/population/name widgets, ambient light and full callback integration pending; cursor ordering/fade exactness open: VW13 |
| §6k save/triggers | `View3DRecord`, `save_record`, `load_record`, trigger methods | VW-T16 four class references and logical trigger pairs; versions 0–4 accepted, not-loaded cleared; legacy two-reference View3DSave kept compatible; opaque reference identity/resolution adapter pending; byte bucket ordering open: VW12 |
| §6l helpers | pending | owner-called helper interfaces remain foreign |
| §6m quirks | `keyboard_scroll`, `recalc_scroll_velocity`, `set_zoom_limits` | VW-T11/12/13; command-result/rotation/save-parameter quirks pending |
| §7 conformance | `owner_tests` | VW-T1–T18 implemented; all three earlier gaps T2–T4 now exercised |
| §8 open questions | questions file | VW6–VW13 |
| ui-windows §2 window model | existing `win` / `message` modules | existing window-model tests; no new foreign window implementation |
| ui-windows §3.1–§3.3 owner identity/layout/slots | `View3DWin`, `slots` | existing layout/slot tests and VW-T1 |
| ui-windows §3.3a save | `save_record`, `load_record` | four class-bound references and logical trigger pairs; original opaque identity/resolution adapter pending |
| ui-windows §4 catalog menus | existing `catalog` | existing catalog tests; host-aware control teardown still pending |
| ui-windows §7 drawing | `paint`, `MotionHost` | `paint_gates_preserve_tail_and_write_time_before_draw` |
| ui-windows §10 quirks, §11 conformance | existing `catalog`, `view3d::tests`, `owner_tests` | existing catalog/identity tests; game-dependent contracts unmeasured |
| view3d §5.1 paint | `paint` | gate, timer, dt clamp, scrolling, draw-time order tests; post-draw tail details open: VW10 |
| view3d §5.2 fields | `Motion` | paint/hidden-pause tests |
| view3d §5.3 save | `save_record`, `load_record` | four-reference logical model; foreign serialization/resolution adapter pending |
| view3d §10 quirks, §11 conformance (window-owned portions) | `paint`, `do_save` | tests 13–15, 20, 25 covered by paint/hidden-pause/reference tests; effects sweep test 19 requires foreign renderer integration |

The legacy `set_input_control` and `remove_all_view_input_controls` APIs import
or extract token state for the older catalog model. Real lifecycle adapters must
use the host-aware methods; otherwise callbacks and ownership cannot be honored.
No plugin or C ABI wiring is present.

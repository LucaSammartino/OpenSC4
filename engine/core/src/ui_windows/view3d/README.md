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
| §2 construction | `OwnerState::default`, `Motion::default`, `View3DWin::new` | motion defaults exercised by VW-T12/T13; panel/catalog defaults and scripting create hook pending |
| §3 Init, §3.1 cheats | `process_cheat`, `CheatHost`, `recorder` | VW-T5/T6 and render/recorder/control-allocation tests; Init registration/failures pending; camera SizeOf old-value guard belongs to foreign camera |
| §4 Shutdown | control removal primitives | full shutdown pending; open: VW8 for unidentified helpers |
| §5, §5.1, §5.2 notifications | pending | command priority, subscriptions, deferred-message queue and refresh routes pending |
| §6, §6.1, §6.3 messages/filter | pending | tutorial command filter, focus loss, accelerators, URL-result quirk and button routes pending |
| §6a city init | pending | foreign object construction/wiring and post-init sequence pending |
| §6b GUI lifetime | pending | main/tab/auxiliary windows, filter removal and destruction order pending; open: VW6 |
| §6c input | `process_key`, `on_character`, `on_mouse`, `edge_scroll` | VW-T7/8/9/10; character bound, tutorial rewriting, exit traversal tests; additional edge/capture branches need tests |
| §6d fields | `OwnerState`, `Motion`, existing layout constants | input/motion fields implemented; other owner state pending; open: VW8 |
| §6e input stack | `set_current_control`, `remove_current_control`, persistent methods | `control_modes_init_failure_and_persistent_lifetime`, `remove_control_sets_default_cursor_and_releases_capture`; catalog legacy token path still needs lifecycle integration |
| §6f scrolling/zoom/views | `Motion`, zoom, filter/level/contour methods, `paint` | VW-T11/12/13, filter reference/message order, contour pairing, edge recalculation, hidden pause and paint tests; terrain selection/picking and tuning pending |
| §6g commands | `keyboard_scroll`, view-change primitives | command ID dispatch pending; open: VW9 for incomplete registration/ID inventory |
| §6h panels | pending | panel/tab state, establishment and obliteration pending |
| §6i buttons/catalogs | existing `catalog` module | existing catalog tests cover script toggle/create/close; new button/catalog-kind routing pending; open: VW7 for external message meaning |
| §6j displays | `DisplayState`, `StatusHost`, status methods | VW-T17/T18, rating/alert/mission/speed tests; date/funds/population/name widgets, ambient light and full callback integration pending; cursor ordering/fade exactness open: VW13 |
| §6k save/triggers | `do_save`, `do_load`, trigger methods | VW-T16 four class references and logical trigger pairs; versions 0–4 accepted, not-loaded cleared; opaque reference identity/resolution adapter pending; byte bucket ordering open: VW12 |
| §6l helpers | pending | owner-called helper interfaces remain foreign |
| §6m quirks | `keyboard_scroll`, `recalc_scroll_velocity`, `set_zoom_limits` | VW-T11/12/13; command-result/rotation/save-parameter quirks pending |
| §7 conformance | `owner_tests` | VW-T1, T5–T13, T16–T18 implemented; T2–T4 and T14–T15 pending |
| §8 open questions | questions file | VW6–VW13 |
| ui-windows §2 window model | existing `win` / `message` modules | existing window-model tests; no new foreign window implementation |
| ui-windows §3.1–§3.3 owner identity/layout/slots | `View3DWin`, `slots` | existing layout/slot tests and VW-T1 |
| ui-windows §3.3a save | `do_save`, `do_load` | four class-bound references and logical trigger pairs; original opaque identity/resolution adapter pending |
| ui-windows §4 catalog menus | existing `catalog` | existing catalog tests; host-aware control teardown still pending |
| ui-windows §7 drawing | `paint`, `MotionHost` | `paint_gates_preserve_tail_and_write_time_before_draw` |
| ui-windows §10 quirks, §11 conformance | existing `catalog`, `view3d::tests`, `owner_tests` | existing catalog/identity tests; game-dependent contracts unmeasured |
| view3d §5.1 paint | `paint` | gate, timer, dt clamp, scrolling, draw-time order tests; post-draw tail details open: VW10 |
| view3d §5.2 fields | `Motion` | paint/hidden-pause tests |
| view3d §5.3 save | `do_save`, `do_load` | four-reference logical model; foreign serialization/resolution adapter pending |
| view3d §10 quirks, §11 conformance (window-owned portions) | `paint` | draw entry implemented; full spec conformance audit pending |

The legacy `set_input_control` and `remove_all_view_input_controls` APIs import
or extract token state for the older catalog model. Real lifecycle adapters must
use the host-aware methods; otherwise callbacks and ownership cannot be honored.
No plugin or C ABI wiring is present.

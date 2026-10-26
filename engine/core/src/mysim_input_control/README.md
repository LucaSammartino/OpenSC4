# MySim input-control owners

Source: `docs/specs/mysim-tools.md`. Each owner has its own module:
`change_house`, `change_job`, `move_in` (shared slots in `BuildingTool`), `dispatch`.
No C ABI or plugin wiring. Existing `ui_windows::base` owns window state;
portrait and callout operations target existing widgets through traits.

| Spec section | Code item | Worked-case test / boundary |
|---|---|---|
| 1 Tools | owner constructors, `SimView` | `badge_failure_is_success_and_default_owner_kinds_are_distinct` |
| 1.1 Building fields | `BuildingTool` | `building_lifecycle_badges_wealth_and_interface_ownership` |
| 1.2 Shared slots | `on_key_down`, `on_mouse_exit`, `destroy` | `building_lifecycle_badges_wealth_and_interface_ownership` |
| 2 Starting/stopping | `init`, `shutdown`, `on_key_down` | `building_lifecycle_badges_wealth_and_interface_ownership`; badge failure case |
| 2.1 Init | `BuildingTool::init` | `building_lifecycle_badges_wealth_and_interface_ownership`; badge failure case |
| 2.2 Ending | `on_key_down` | Esc ends; other keys return false in lifecycle test |
| 2.3 Shutdown | `shutdown`, `restore_highlight` | lifecycle test: no over-release; all held references reach zero |
| 3 Hovering | `candidate`, `on_mouse_move` | `mt_c1_c2_house_callout_meter_and_rejected_occupied_house`; open MST1/MST2: raw slot getters default to Unsupported |
| 3.1 House/move-in | `candidate`, `badge`, `ensure_callout` | MT-C1/C2 test; `raw_slot_gaps_and_null_lot_stop_without_inventing_eligibility` |
| 3.2 Job | `candidate`, `on_mouse_move` | `mt_c3_c4_job_rejects_house_and_uses_first_group_skin`; open MST1/MST2 as above |
| 3.3 Job skin | `job_skin` | MT-C4: groups 0x1002 and 0x1500 select 0x14416206 |
| 3.4 EQ/HQ | `eq_text`, `hq_text` | `mt_c5_thresholds_nan_and_unsupported_stale_fill` |
| 4 Clicking | `on_mouse_down_l` | `clicks_retest_only_change_house_and_end_job_even_on_failure` |
| 5 Dispatch | `dispatch::Dispatch` | MT-C6/C7 tests below |
| 5.1 Fields/destructor | `new`, `destroy` | `shutdown_restores_pylon_and_reports_the_specified_hover_leak` |
| 5.2 Init/shutdown | `init`, `shutdown` | same test: six subscriptions, repeat-init guard, position restore and explicit one-ref leak |
| 5.3 Picking | `PylonFilter::includes`, `pick_pylon` | `filter_raw_slot_boundary_and_hover_double_holds`; open MST3: raw slot getters default to Unsupported |
| 5.4 Helpers | `set_hover_pylon`, `set_new_sim`, `calculate_drag_offset` | hover test; `picked_other_sim_changes_owner_and_deleted_hover_releases_once`; MT-C7 truncation case |
| 5.5 Clicking | `on_mouse_down_l`, `begin_picked` | `mt_c6_clicks_three_hundred_apart_lock_and_post_double_click`; first-placement/no-view test |
| 5.6 Moving | `on_mouse_move` | `dragging_waits_five_hundred_then_uses_altitude_and_location_test` |
| 5.7 Messages/keys | `do_message`, `on_key_down` | `mt_c7_tool_takeover_restores_drag_and_exempt_tool_does_not_suspend`; deleted-hover test |
| 6 Quirks/cases/inventory | tests for MT-C1–C7; explicit unsupported domains | all seven worked cases above; unused dispatch word/string have no observable operations |

All stored foreign references are retained or created. Shutdown deletes windows
through the shared base and releases their foreign interfaces separately.
Call shutdown before destroy; no implicit Drop can call a discarded foreign host.
MST1/MST2 raw slot reads are supported only when a host explicitly supplies the
published interface values. Default adapters report Unsupported, not guessed
semantic field names. Null unchecked native lot lookups and stale meter fill
also report Unsupported rather than inventing a value or dereferencing null.
MST3 follows the same raw-slot policy. Dispatch intentionally records the spec's
single lost hover reference on shutdown/deletion/destruction; normal hover swaps
release both holds. Tests distinguish that documented leak from over-release.

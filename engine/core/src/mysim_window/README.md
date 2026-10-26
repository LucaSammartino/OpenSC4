# cSC4MySimWinProc

Source: `docs/specs/mysim-windows.md`. Core only; no C ABI or plugin wiring.
Existing `ui_windows::base::Windows` owns the window tree, visibility and root
reference. Existing script widgets are supplied as `Bindings`; their editing,
HTML parsing and drawing are foreign calls, not duplicated controls.

Closure table:

| Spec section | Code item | Worked-case test / boundary |
|---|---|---|
| 0 Summary | `MySimWindow` | `three_sims_main_add_buttons_and_balanced_lifecycle` |
| 1.1 Life cycle | `init`, `shutdown`, `show`, `hide` | `three_sims_main_add_buttons_and_balanced_lifecycle` |
| 1.2 Panels | `panels`, `set_main_window`, `set_mode_window`, `set_dialog_state` | `occupied_slot_mode_boot_and_stat_boundary`; open MW2: `Unsupported::StatisticsLayout` |
| 1.3 Sims | `get_current_sims`, `set_current_sim`, `add_sim`, `boot_sim` | three-Sim case; `empty_slot_dialog_disables_panels_and_releases_created_reference`; open MW3: `Unsupported::TransportButtonImage` |
| 1.4 Messages | `do_message` | `changed_sims_retains_occupied_selection_or_returns_main_only_while_shown`, `text_redraw_keeps_list_and_raw_set_aside_flags_follow_spec`; open MW1/MW2/MW3 as below |
| 1.5 Message text | `get_message_list`, `update_body_text_ctrl`, `visible_advice` | `text_redraw_keeps_list_and_raw_set_aside_flags_follow_spec`; open MW1: visibility interpretation Unsupported; open MW3: HTML construction Unsupported |
| 1.6 Transport | `transport::{vehicle_icons,select_car,select_pedestrian}`, modal and tool factories | `transport_resource_enumeration_posts_pause_and_created_dialog_cleanup`; open MW2: choice effects Unsupported; open MW3: icon-property/widget layout Unsupported |
| 2 Pick-sim dialog | `Host::{create_pick_dialog,run_pick_dialog}` | `empty_slot_dialog_disables_panels_and_releases_created_reference`; separate cSC4PickSimWinProc owner outside this task |
| 3 Quirks | pick-sim service boundary | 21-portrait paging belongs to the separate pick-sim owner |
| 4 Open questions | `Unsupported` | open MW1, MW2 and MW3; never inferred U-Drive-It or statistics layout |
| 5 Conformance cases | window tests above | cases 1 and 4 covered; name validation and paging (cases 2 and 3) belong to cSC4PickSimWinProc |

`shutdown` must run before the owner is discarded. Borrowed Sim handles are
retained before releasing old slots, and a created pick dialog is released once.
The pick-sim owner is outside this task and is reached through its service.
Advice handles are borrowed identities copied from the advisor; the adapter must
keep them valid until refresh/shutdown. No speculative visibility filtering or
HTML is emitted. Redraw invalidates the existing text widget without changing
the raw advice list, including when HTML construction reports Unsupported.

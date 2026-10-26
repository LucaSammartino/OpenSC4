# cSC4MySimWinProc

Source: `docs/specs/mysim-windows.md`. Core only; no C ABI or plugin wiring.
Existing `ui_windows::base::Windows` owns the window tree, visibility and root
reference. Existing script widgets are supplied as `Bindings`; their editing,
HTML parsing and drawing are foreign calls, not duplicated controls.

Closure table (remaining sections are completed in subsequent commits):

| Spec section | Code item | Worked-case test / boundary |
|---|---|---|
| 0 Summary | `MySimWindow` | `three_sims_main_add_buttons_and_balanced_lifecycle` |
| 1.1 Life cycle | `init`, `shutdown`, `show`, `hide` | `three_sims_main_add_buttons_and_balanced_lifecycle` |
| 1.2 Panels | `panels`, `set_main_window`, `set_mode_window`, `set_dialog_state` | `occupied_slot_mode_boot_and_stat_boundary`; open MW2: `Unsupported::StatisticsLayout` |
| 1.3 Sims | `get_current_sims`, `set_current_sim`, `add_sim`, `boot_sim` | three-Sim case; `empty_slot_dialog_disables_panels_and_releases_created_reference`; open MW3: `Unsupported::TransportButtonImage` |

`shutdown` must run before the owner is discarded. Borrowed Sim handles are
retained before releasing old slots, and a created pick dialog is released once.
The pick-sim owner is outside this task and is reached through its service.

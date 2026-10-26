# Video preferences window procedure

Portable `cSC4VideoPreferencesWinProc`, from `docs/specs/video-preferences.md`.
`Owner::new` supplies §9.1 construction; `from_raw` retains the qualified raw-state
test entry. `show_dialog` is the full body; `show`/`show_prefix` are prefix entries.
The caller serializes and carries `DialogGlobals` between owners. There is no
native layout, C ABI, plugin factory, renderer, persistence or modal loop here.

| Spec section | Code item | Worked case / boundary |
|---|---|---|
| 1 Scope and evidence | `Owner`, `Host` | `section9_case4_construction_full_show_modal_and_free`; foreign services remain primitives |
| 2 Raw state and primitives | `Preferences`, `RawState`, `lookup` | `unsupported_domains_and_unknown_work_never_become_successful_noops`; success/null and failure/output are Unsupported |
| 3 Reading options | `readback` | `vp_t4_asymmetric_full_mapping_stores_and_matched_reference_chronology`, VP-T5–T7 |
| 4 Distinguishing traces | `fixtures`, `tests` | VP-T1–T24 portable assertions; no claim of native execution |
| 5 Message admission and lifetime | `message`, `end_dialog`, `query_interface`, `shutdown`, `release` | VP-T8–T11, T20–T24; `end_dialog_parent_paths_have_distinct_al_and_eax_contracts` |
| 6 Redraw and actions | `redraw`, `action`, `Host::preferences` | VP-T12–T19; `auto_mutates_live_preferences_and_redraw_preserves_opening_baselines` |
| 7 Initialization and modal boundary | `show_dialog`, `do_modal` | `section9_case4_construction_full_show_modal_and_free`, `modal_sequence_retains_root_ignores_raw_result_and_conditionally_restores_focus` |
| 8 Coverage ledger | full owner bodies, retained prefix entries | all tests above; app/widget/service implementations remain foreign |
| 9 Completion | `completion`, `BINDINGS` | `section9_case1_first_redraw_captures_only_once`, `section9_case2_unlisted_pair_clears_all_and_captures_zero`, `section9_case3_resolution_radio_and_self_toggling_checkbox`, case4 |

No open entries remain in `docs/specs/questions/video-preferences.md`. The shared
AddRef's unspecified arithmetic uses a marked SPEC-GAP wrapping increment; raw
owners require an explicit count for `release_from_raw_count`. The constructed
owner implements its own reference calls rather than delegating them to the host.
`Object` reuses `widget_controls::Handle` / `ui_windows::win::WinToken`; window
lookup, flags, notification targets and control selections stay host primitives
and do not duplicate base-window or control owners. Host adapters can use
`ui_windows::base::Windows` and existing controls to service these calls.
`Host::preferences` receives the live block for Auto/Apply; no defaults are invented.

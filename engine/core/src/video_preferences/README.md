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
| 3.1 Ordered byte assignments | `readback`, `lookup` | VP-T1–T3, `vp_t4_asymmetric_full_mapping_stores_and_matched_reference_chronology` |
| 3.2 Remaining readback and marker | `readback`, `changed`, `utility` | VP-T4–T7; `independent_u16_baselines_nonzero_al_store_before_marker_and_first_checked` |
| 4.1 Full readback vectors | `fixtures`, `readback` | VP-T4–T7 |
| 4.2 Event/lifetime vectors | `fixtures`, `tests` | VP-T8–T24 |
| 5.1 Message admission | `message`, `admitted` | `vp_t8_t10_message_admission_outchannels_and_unadmitted_gap`, `admission_source_checks_and_null_message_are_ordered` |
| 5.2 EndDialog | `end_dialog` | VP-T11; `end_dialog_parent_paths_have_distinct_al_and_eax_contracts` |
| 5.3 Shared slots | `query_interface`, `shutdown`, `release_from_raw_count` | VP-T20–T21; `destruction_releases_owned_host_but_not_borrowed_preferences_and_forbids_use_after_free` |
| 6.1 Redraw slices | `redraw_first_group`, `redraw_depth_tail` | VP-T18–T19; `depth_tail_has_qualified_incoming_reference_capture_and_cleanup` |
| 6.2 Foreign action primitives | `Host`, `save_apply_notify` | `vp_t12_t14_save_before_apply_even_without_changes_and_despite_marker` |
| 6.3 Action order | `action`, `raw_widget` | VP-T12–T16; `auto_mutates_live_preferences_and_redraw_preserves_opening_baselines` |
| 6.4 Selection event group | `request_group` | `vp_t17_selection_group_repeat_has_exact_calls_and_no_preference_writes` |
| 7.1 ShowDialog | `show_prefix`, `show_dialog`, `show_tail` | VP-T23–T24; `section9_case4_construction_full_show_modal_and_free` |
| 7.2 Modal boundary | `do_modal` | VP-T22; `modal_sequence_retains_root_ignores_raw_result_and_conditionally_restores_focus` |
| 9.1 Construction/baselines | `new`, `globals`, `add_ref`, `release` | §9.7 case4; AddRef precision gap documented below |
| 9.2 Whole redraw | `redraw_complete` | §9.7 cases1–2, `redraw_failed_checkboxes_do_not_capture_but_radio_still_does` |
| 9.3 Every control event | `selection_event`, `action` | §9.7 case3, VP-T17 |
| 9.4 Middle bindings | `show_dialog`, `BINDINGS` | §9.7 case4 (35 admitted bindings) |
| 9.5 Foreign boundaries | `Host::preferences`, `do_modal` | Auto live-block test, modal ordering test; app persistence/drawing remain foreign |
| 9.6 Labels | `Byte`, `Dword`, control IDs in `BINDINGS` | §9.7 case3 uses the raw IDs; inferred captions are not synthesized |
| 9.7 Conformance tests | `completion_tests` | the four named `section9_case*` tests |

No open entries remain in `docs/specs/questions/video-preferences.md`. The shared
AddRef's unspecified arithmetic uses a marked SPEC-GAP wrapping increment; raw
owners require an explicit count for `release_from_raw_count`. The constructed
owner implements its own reference calls rather than delegating them to the host.
`Object` reuses `widget_controls::Handle` / `ui_windows::win::WinToken`; window
lookup, flags, notification targets and control selections stay host primitives
and do not duplicate base-window or control owners. Host adapters can use
`ui_windows::base::Windows` and existing controls to service these calls.
`Host::preferences` receives the live block for Auto/Apply; no defaults are invented.

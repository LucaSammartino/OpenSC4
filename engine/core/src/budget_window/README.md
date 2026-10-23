# Budget window core

`SubBudgetWindow` owns lifecycle, population, selection, scrolling and retained
references. `BudgetWindowHost` / `SubBudgetSources` expose foreign window,
simulator, budget and service interfaces. `BodyPaint` carries core-selected
presentation instructions; it cannot supply counts, maps, selected deals or
computed native-owner results. Nothing in this module is wired to the plugin
or C ABI.

## Owner closure

Scope is **cSC4SubBudgetWindow**. Panel-only sections are included below to
make the boundary explicit; they do not extend this task to cSC4BudgetUIWinProc.
Tests named below are in `tests.rs` and `source_tests.rs`. BW17 is the spec's
explicit unsettled boundary, not a claim of native-owner completion. All its
remaining paths are represented by `Unsupported` or the inventory below.

| Spec section | Owner / code item | Test or open question |
|---|---|---|
| budget-window §1 identities | SubBudgetWindow, ids; native binary layout stays foreign | create_sequence_and_failure; unclassified helper identities open: BW17 |
| §2 opening / nested dialogs | BudgetPanel::do_sub_budget_window; SubBudgetWindow::do_individual_budget_window | opening_while_held_does_nothing; modal_run_pauses_releases_and_refreshes_once; nested_individual_dialog |
| §2.1 selector table | panel_command; BudgetPanel::on_command (existing caller) | panel_command_selector_table; tutorial_filter_swallows; tab_commands_run_show_window |
| §3 CreateSubBudgetWindow | SubBudgetWindow::create | create_sequence_and_failure |
| §4 DoWinMsg, init, shutdown | do_win_msg, on_init, on_shutdown, create_content; raw mode initializers | do_win_msg_returns; on_shutdown_releases_window_once_initialised; content_window_created_in_every_mode; mode bodies below; open: BW17 |
| §4.0 key translation | do_win_msg consumes foreign key/modifier words | alt_enter_and_alt_f4_close; delivery in the real game open: BW11 |
| §4.1 group lists | init_group, header_shown, line_item_shown, build_group_row | group_counts_and_special_headers; bw16_id_boundary_counts_and_built_rows; department_ownership_reinit_shutdown_and_drop; setup open: BW17; temporary provider references open: BW20 |
| §4.4 sliders / item getter | on_slider, slider_fraction, get_window_indiv_item | slider_fraction_is_the_f32_product; slider_reentry_guard_is_shared; occupant_funding_levels; item_property_is_type_checked_released_and_returns_borrowed_object; subsequent refresh open: BW17 |
| §5 buttons | on_button_activated | ordinance_apply_calls_only_changed_rows; keys_close_only_as_specified; individual/nested/camera tests; popup open: BW17 |
| §5.1 restore | restore_start_settings; retain_individual_restore | group_slider_then_cancel_restores_start_fractions; occupant_funding_levels; acquisition values open: BW17 |
| §5.2 loan taken | cSC4BudgetUIWinProc, foreign owner | Outside named owner; no implementation claimed |
| §5.3 scroll / arrows | scroll, create_scroll_arrows; requests bounded refreshes | scroll_steps_arrows_and_refresh; group_initial_arrows; selector_initial_arrows; refresh remainder open: BW17 |
| §5.4 ApplyDealEdits | apply_deal_edits | neighbour_mode_apply_deal_edits |
| §5.5 occupant funding | service_kind, set_occupant_funding_level | occupant_funding_levels |
| §5.6 ordinance rows / popup | row-map edit/apply in on_button_activated | ordinance_apply_calls_only_changed_rows; info_popup_and_close_child explicitly tests Unsupported; popup open: BW17 |
| §5.7 camera | on_button_activated camera branch | camera_jump_in_individual_mode |
| §6 close / destructor | end_dialog, destroy, release_final, Drop | end_dialog_branches; create_failure_underflows_pause_counter; department_ownership_reinit_shutdown_and_drop; explicit_final_release_then_drop_is_idempotent |
| §7 panel messages | BudgetPanel::do_message (existing caller) | panel_refreshes_at_most_once_per_tick; outside named owner |
| §8 open work | Unsupported variants and this inventory | open: BW17; runtime key delivery BW11 |
| §9 conformance | See numbered mapping below | All named-owner cases 1–27 mapped; panel-only 28–32 outside scope |
| budget-window-bodies §1 inventory | Existing dialog.rs plus dialog/bodies.rs | Lifecycle/routing tests; unidentified support helpers open: BW17 |
| §1.1 direct owner boundary | Raw SubBudgetSources; no same-owner body callbacks | Population tests below; setup, popup, widget failures, GetBudgetSegment, property setter and unclassified support helpers open: BW17 |
| §2 ordinance population | init_ordinances; ordinance_totals low64 multiplication boundary | available_ordinances_classify_map_and_observe_twice; offscreen_ordinances_contribute_with_low64_product; scale/formatting open: BW17 |
| §2.1 providers / order | init_ordinances | available_ordinances_classify_map_and_observe_twice |
| §2.2 ordered lists / map / totals | init_ordinances; ordinance_totals | Both ordinance source tests; owner-scale initialization and visible B money formatting open: BW17 |
| §3 neighbour population | init_neighbors; core-built deal/type and active-city/type vectors | neighbor_six_scans_headers_totals_and_hidden_rows; options/reference remainder open: BW17 |
| §3.1 scans / sections | init_neighbors | neighbor_six_scans_headers_totals_and_hidden_rows; temporary-list balance open: BW17 |
| §3.2 combos / references | retain_deal_row, clear_deal_rows, edit_deal_rows, request_neighbor_refresh | combo_rows_edit_all_matches_and_own_separate_references; combo_missing_interface_and_invalid_division_are_safe; metadata/options open: BW17; invalid IDIV divergence open: BW19 |
| §4 panel sheet | cSC4BudgetUIWinProc, foreign owner | Outside named owner; no implementation claimed |
| §4.1 CalculateValues | cSC4BudgetUIWinProc, foreign owner | Outside named owner |
| §4.2 UpdateBalanceSheetInfo | cSC4BudgetUIWinProc, foreign owner | Outside named owner |
| §4.3 formatting | cSC4BudgetUIWinProc, foreign owner | Outside named owner |
| §5 individual population | enumerate_individual; retained snapshot pointers separate from unresolved funding values | individual_reenumerates_viewport_preserves_and_rebuilds_snapshot; known_service_retains_pointers_without_inventing_funding_values; service arithmetic open: BW17 |
| §5.1 list / viewport / accumulation | enumerate_individual; individual_amount | individual_reenumerates_viewport_preserves_and_rebuilds_snapshot; raw service amount/float derivation and rendering open: BW17 |
| §5.2 restore ownership | restore_occupants; retain_individual_restore; clear_restore; shutdown/Drop | Both individual source tests; original list pointers retained even when values unresolved; service floats open: BW17 |
| §6 UpdateValues A traversal | refresh_group; busy guard; raw budget group selection; distinct q/p; header reuse until unresolved item rendering | group_refresh_keeps_distinct_ordinal_and_quota; group header/scale tests below; A item/B/slider/stale cleanup open: BW17; invalid empty provider vector open: BW19 |
| §7 completion boundary | Unsupported enum and this table | open: BW17 (all enumerated remainders preserved) |
| §8 panel construction / Init / periods | cSC4BudgetUIWinProc, foreign owner | Outside named owner; BW18 belongs to panel |
| §9 taxes | cSC4BudgetUIWinProc, foreign owner | Outside named owner |
| §10 loans | cSC4BudgetUIWinProc, foreign owner | Outside named owner; summary details BW18 |

## Conformance cases (budget-window §9)

| Cases | Covering tests |
|---|---|
| 1 | opening_while_held_does_nothing |
| 2, 5, 7 | modal_run_pauses_releases_and_refreshes_once; neighbour_mode_apply_deal_edits; create_failure_underflows_pause_counter |
| 3, 13 | group_slider_then_cancel_restores_start_fractions; slider_fraction_is_the_f32_product |
| 4 | ordinance_apply_calls_only_changed_rows |
| 6, 9, 14 | keys_close_only_as_specified; do_win_msg_returns; alt_enter_and_alt_f4_close |
| 8 | nested_individual_dialog |
| 10 | on_shutdown_releases_window_once_initialised |
| 11 | tab_commands_run_show_window (existing panel caller) |
| 12 | scroll_steps_arrows_and_refresh |
| 15 | content_window_created_in_every_mode |
| 16 | empty_group_keeps_saved_pairs |
| 17–18 | group_initial_arrows; selector_initial_arrows (test-only boundary preconditions, counts are not production host inputs) |
| 19, 25 | failed_content_keeps_pause_and_department_ownership |
| 20–21 | saved_pairs_own_references_even_without_visible_rows |
| 22–24 | department_ownership_reinit_shutdown_and_drop; explicit_final_release_then_drop_is_idempotent |
| 26–27 | bw16_id_boundary_counts_and_built_rows |
| 28–32 | Panel sheet/taxes/loans/period commands: outside named owner |

## Remaining limits

Known-service individual snapshots retain all original occupant pointers and
preserve them on rebuild=false. Their funding bytes and amounts remain
unresolved (BW17); cancel cannot restore unresolved bytes. No native computed
row provider fills them. Unknown-service working values are the specified
positive zero. Temporary QI and list references are independent of retained
snapshot references.

The scale is deliberately absent until BW17 specifies its initialization.
Totals/display that require it are not fabricated. The bounded group A walk
stops assigning later header control IDs after an eligible item whose rendering
is unsettled; its q/p traversal and accessor observations continue. Neighbor
combo metadata/options are unsettled, so initialization emits presentation
intents and does not invent combo IDs. `retain_deal_row` models ownership once
a control exists; each edit traversal schedules exactly one unresolved refresh.

Native held-deal vector erase has no per-deal Release, as explicitly specified.
The unresolved returned-list balance remains visible as BW17; the core does not
silently release unowned handles to compensate. BW19 documents safe handling of
native fault inputs. BW20 asks about the initial department-list references.

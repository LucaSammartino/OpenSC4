# Query tool owner

Portable `cSC4ViewInputControlQuery`, specified by city-tools-input.md §8.
This module reuses `view_input_controls::ControlHost`, `Pixel` and `Vec3`.
It introduces no C ABI or plugin integration, and does not duplicate the shared
input base. Inherited capture/cursor/stack operations remain the shared host
contract. Key, character, right-button and wheel handlers are false stubs.

`World` supplies copied occupant/lot records, primitive simulator values and
foreign locale formatting. `Host` supplies renderer traversal, window operations,
timers and the modal message pump. The predicate, selection, calculations,
formatting choice, recursive traversal and write-back belong to this module;
hosts must never call native query tool/filler/description methods.

Tokens are portable identities, not pointers. Pick/world/tree lookups borrow;
`Acquire`/`Release` explicitly balance persistent and dialog holds. Creation
transfers one created obligation. Native adapters must distinguish created and
acquired references. `shutdown` and `destroy` are explicit because a Rust Drop
cannot issue foreign effects. Getters use shared references, without RefCell.

`Effect::Unsettled(Open)` is the explicit **Unsupported** result for a published
gap. It preserves already completed effects; it is never an invitation for the
host to run the original owner method. No partial output marked open should be
represented as an exact native description. The raw numeric contract is finite,
representable inputs with nonoverflowing budget sums (CT19). The unspecified
global default point is injected, rather than guessed.

## Owner closure

Every row is for `cSC4ViewInputControlQuery`; inventory §8.10 is accounting, not
an additional algorithm. Open portions are listed explicitly alongside supported
prefixes. Tests named below are in `tests.rs` and `supplement_tests.rs`. Every
pre-existing row was rechecked against the revised spec on 2026-10-23; §8.16
supersedes the older neighbour script summary. This is partial owner closure.

| Spec section | Code item | Test / remaining open question |
|---|---|---|
| 8.1 Creation | `Mode::{from_command,id,cursor}`, `QueryTool::new` | `ct6_creation_lifecycle_slots_and_holds` |
| 8.2 Layout | `QueryTool` fields/defaults | `ct6_creation_lifecycle_slots_and_holds`; unused word preserved as 3, open: CT6 (external reader) |
| 8.3 Slot table | lifecycle, mouse/message methods, `unhandled_input`, explicit `destroy`; inherited `ControlHost` | `ct6_creation_lifecycle_slots_and_holds` |
| 8.4 Init/Shutdown | `init`, `shutdown`, explicit holds/subscriptions/callout setup | `ct6_creation_lifecycle_slots_and_holds`, `ct8_hover_restore_exit_and_shutdown_message`; open: CT7 (legend key/position/minimisation; no panes invented) |
| 8.5 Hover | `mouse_move`, `mouse_exit`, `unhighlight` | `ct8_hover_restore_exit_and_shutdown_message`, `hover_property_lot_highlight_and_terrain_fallback` |
| 8.6 Clicks | `mouse_down`, `mouse_up` | `click_sound_script_zone_and_shift_miss`, `click_modifiers_and_route_latch`; open: CT9 (vehicle/stoplight primitive actions), CT15 (route generation) |
| 8.7 Messages | `message`, `build_tip`, `walk` | `messages_callbacks_and_route_open_boundary`, `ct8_hover_restore_exit_and_shutdown_message`; open: CT20 (traffic tips), CT15 (redraw geometry) |
| 8.8 Picks/filters/names | `accepts`, `pick`, `display_name` | `ct7_filter_truth_table_and_names` |
| 8.9 Timer callbacks | `tip_callback`, `coverage_callback` | `messages_callbacks_and_route_open_boundary` |
| 8.10 Inventory | implementations identified in these rows | Accounting only; remainder explicitly tracked below |
| 8.11 Tips | `Tip`, `build_tip`, headline appending | `building_tip_group_priority_utilities_and_headline_limit` |
| 8.11.1 Terrain summary | `terrain_tip` | `ct9_tips_ratio_terrain_network_and_display`, `terrain_zones_and_network_priority` |
| 8.11.2 Network summary | `network_tip` | `ct9_tips_ratio_terrain_network_and_display`, `terrain_zones_and_network_priority` |
| 8.11.3 Tip display | `display_tip`, `set_tip` | `ct9_tips_ratio_terrain_network_and_display`; open: CT13 (native callout interface/12,10,5 argument meanings; semantic image intent only) |
| 8.11.4 Building tip | `building_tip`, `load_ratio` | `building_tip_group_priority_utilities_and_headline_limit`, `building_tip_rows_and_transport`; open: CT14/CT20 (debug writer bodies) |
| 8.11.5 Highlight | `set_highlight_state`, `unhighlight` | `hover_property_lot_highlight_and_terrain_fallback` |
| 8.12 Query click | `describe_pick`, dialog entry points | `click_sound_script_zone_and_shift_miss` |
| 8.12.1 DescribePick | pick exclusion, lot building replacement including terrain miss fallback, sound/dialog ordering, zone fallback, published network formats, reset strings and extra2 structure, `DebugPhase` ordering | `click_sound_script_zone_and_shift_miss`, `network_describe_format`, `qct5_16_cell_writer_order_and_terrain_building_fallback`; open: CT14/CT20 (writer bodies/full debug strings), CT18 (empty-cell sound), CT14 (neighbour row fillers) |
| 8.12.2 DoQueryDialog | `do_query_dialog`, `run_dialog` | `ct10_modal_script_walk_cancel_coverage_and_ownership`, `accepted_dialog_set_pass_and_failures` |
| 8.12.3 Filler table/walk | `filler_index`, `walk`, `fill_control`, `all_controls` | `filler_table_all_slots_and_set_pass`, `ct10_modal_script_walk_cancel_coverage_and_ownership`, `qct5_1_to_3_null_slots_and_funding_lists`; null occupant slots and separate zone dispatch settled by §8.16.1 |
| 8.12.4 Entry contents | `fill_control`, `fill_special`, funding helpers, `developer_rating`, `wealth_geometry`, `goofy_stat`, `airport_lost_luggage` | existing filler/source/formula/edit tests plus `qct5_1_to_3_null_slots_and_funding_lists`, `subjective_fillers_select_and_evaluate_developer_curves`, QCT5-8..12 tests; open: CT14 (MySim), CT19 (noise, curve property construction, missing records and exceptional numeric boundaries), CT21 (ID+2 cost-update reference conflict) |
| 8.13 Dialogs/win proc | `play_occupant_sound`, `do_zone_dialog`, `run_dialog`, `dialog_notification`, `neighbor_plan` | existing modal/notification/sound tests plus `qct5_13_14_neighbor_priority_scripts_counts_and_open_rows`; open: CT14 (neighbour row fillers only) |
| 8.14 Building values | capacity/occupancy/aura helpers, `find_best_developer`, `developer_rating`, `wealth_geometry` | `building_values_edit_toggle_and_cost_sum`, `building_value_thresholds`, QCT5-4..10 tests, `developer_all_zone_orders_and_developed_record_path`, `subjective_fillers_select_and_evaluate_developer_curves`; open: CT19 (aura rect endpoints, missing records and exceptional numeric inputs) |
| 8.15 Routes | `get_occupant_paths`, `remove_paths`, retained rect/redraw prefix | `messages_callbacks_and_route_open_boundary`, `click_modifiers_and_route_latch`; open: CT15 (route geometry, filtering, travel types, zoom thickness), CT7 (legend) |
| 8.16 Supplements | `values.rs`, funding lists, live `World::debug_query_enabled` | QCT5-1..16 tests below; residual CT14/CT18/CT19/CT20 and new CT21 explicitly retained |
| 8.16.1 Dispatch/refresh | `filler_index`, `funding_sources`, `dialog_notification` | `qct5_1_to_3_null_slots_and_funding_lists`; all six related lists, including hospital's integer IDs 4 and 0x3F800000; open: CT21 (additional cost-update reference) |
| 8.16.2 Best developer | `find_best_developer` | `qct5_4_to_7_developer_ranking_and_sentinel`, `developer_all_zone_orders_and_developed_record_path`; signed16 ranking, truncated demand ties, sentinel and developed-record branch; open: CT19 (missing/exceptional inputs) |
| 8.16.3 Wealth geometry | `wealth_geometry`, meter window property/geometry effects | `qct5_8_9_wealth_geometry_refresh_and_independent_fraction`; binary32 k, retained left, right alignment, unavailable meter and full-value fallback |
| 8.16.4 Commute | `developer_rating` rating 6, commute-specific filler labels | `qct5_10_commute_unsigned_zero_substitution_and_thresholds`; byte 0/128/255, truncated thresholds/auxiliary, missing curve medium |
| 8.16.5 Curves/airport | `ResponseCurve::evaluate`, `goofy_stat`, `airport_lost_luggage` | `qct5_11_constructed_curves_clamp_and_use_stored_slopes`, `qct5_12_airports_use_one_complete_resource_key`; open: CT19 (curve construction/malformed data, airport noise, exceptional numeric inputs) |
| 8.16.6 Neighbours | `neighbor_plan`, `NeighborRow`, `do_neighbor_dialog` plan effect | `qct5_13_14_neighbor_priority_scripts_counts_and_open_rows`; last-successful type, every script group/count; open: CT14 (row fillers prevent dialog opening) |
| 8.16.7 Debug enable/order | live application/key conjunction, `describe_pick` phase trace | `qct5_15_debug_setting_requires_all_live_keys`, `qct5_16_cell_writer_order_and_terrain_building_fallback`; open: CT14/CT20 (writer bodies; no invented text or native-owner callback) |
| 8.16.8 Conformance | QCT5-1..16 in `supplement_tests.rs` | All settled primitive contracts covered; QCT5-3 additional ID+2 update open CT21; QCT5-16 validates ordering, complete output text remains CT14/CT20 |
| §6 query conformance 6–10 and re-query bullets | tests with ct6/ct7/ct8/ct9/ct10 names plus live slider/Shift-click | Supported cases verified; legend holds remain open CT7 and complete debug readouts open CT14/CT20 |

## Field coverage

`filler_table_all_slots_and_set_pass` checks all 127 slots, high-bit gate,
null-slot descent and write-pass restrictions. `filler_source_matrix` verifies
each direct-source field and alias with distinct simulator values; formula tests
cover load, usage, recycling, costs and zero divisors. Remaining fields are
tested through editing, funding, building/lot and locale fixtures. Zone-only
landfill workers bypass the null occupant table, as confirmed by §8.16.1.

Constructed curves copy finite binary32 knots and stored slopes from foreign
property services; interpolation and endpoint decisions run here. Property
parsing/slope construction remains CT19, rather than silently deriving slopes.
The luggage lookup uses the complete fixed resource key for every airport and
fails when the resource is unavailable. `World` supplies building-development
and developer records and raw desirability inputs; selection and ratings run
here. Meter geometry retains window property `0x8ADC29D4` through `MeterWindow`.

`NeighborPlan` contains the settled rows/kind/resource choice. The owner emits
that plan and Unsupported CT14 instead of opening an unfilled dialog. Likewise
`DebugPhase` is a trace of known sequencing, **not a host writer callback**;
its bodies and final population/demand text remain explicitly Unsupported.

Unsupported portions are **not owner-closed**: 8.4 legend, 8.6 modifier side effects,
8.7 route traffic tips, 8.11.3 native callout mapping, 8.11.4 debug descriptions,
8.12.1 complete debug readouts/neighbour dialog, parts of 8.12.4/8.14, neighbour
dialog in 8.13, and route drawing in 8.15. All known spec-silent choices are
commented and tracked by the retained questions rather than inferred from
implementation sources. CT21 additionally records the supplement's ID+2 cost
reference conflicting with the explicit ID+0x100 helper/notification text.

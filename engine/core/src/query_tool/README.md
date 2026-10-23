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
prefixes. Tests named below are in `tests.rs`.

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
| 8.12.1 DescribePick | pick exclusion, lot building replacement, sound/dialog ordering, zone fallback, published network formats, reset strings and extra2 structure | `click_sound_script_zone_and_shift_miss`, `network_describe_format`; open: CT14/CT20 (unspecified description writers, full debug strings), CT18 (empty-cell sound conflict), CT14 (neighbour dialog) |
| 8.12.2 DoQueryDialog | `do_query_dialog`, `run_dialog` | `ct10_modal_script_walk_cancel_coverage_and_ownership`, `accepted_dialog_set_pass_and_failures` |
| 8.12.3 Filler table/walk | `filler_index`, `walk`, `fill_control`, `all_controls` | `filler_table_all_slots_and_set_pass`, `ct10_modal_script_walk_cancel_coverage_and_ownership`; open: CT18 (landfill dispatch contradiction) |
| 8.12.4 Entry contents | `fill_control`, `fill_special`, funding helpers, value/rating helpers | `filler_numeric_formulas_and_zero_divisors`, `filler_source_matrix`, `conformance_rating_and_live_funding_slider`, `building_values_edit_toggle_and_cost_sum`; open: CT14 (MySim), CT18 (unpublished slider related IDs), CT19 (airport noise/luggage, best-developer selection, wealth resize, multipoint goofy curve, numeric boundary) |
| 8.13 Dialogs/win proc | `play_occupant_sound`, `do_zone_dialog`, `run_dialog`, `dialog_notification` | `click_sound_script_zone_and_shift_miss`, `conformance_rating_and_live_funding_slider`, `dialog_notifications_and_condition_sounds`; open: CT14 (neighbour kind mapping/window lookup/row fillers) |
| 8.14 Building values | `occupant_capacity`, `current_occupancy`, `mayor_percent`, `subjective_level`, wealth fraction | `building_values_edit_toggle_and_cost_sum`, `building_value_thresholds`; open: CT19 (developer selection, resize k/geometry, commute sampling, rect convention) |
| 8.15 Routes | `get_occupant_paths`, `remove_paths`, retained rect/redraw prefix | `messages_callbacks_and_route_open_boundary`, `click_modifiers_and_route_latch`; open: CT15 (route geometry, filtering, travel types, zoom thickness), CT7 (legend) |
| §6 query conformance 6–10 and re-query bullets | tests with ct6/ct7/ct8/ct9/ct10 names plus live slider/Shift-click | Supported cases verified; legend holds remain open CT7 and complete debug readouts open CT14/CT20 |

## Field coverage

`filler_table_all_slots_and_set_pass` checks all 127 slots, high-bit gate,
null-slot descent and write-pass restrictions. `filler_source_matrix` verifies
each direct-source field and alias with distinct simulator values; formula tests
cover load, usage, recycling, costs and zero divisors. Remaining fields are
tested through editing, funding, building/lot and locale fixtures. Zone-only
landfill workers bypass the null occupant table, as documented under CT18.

Unsupported portions are **not owner-closed**: 8.4 legend, 8.6 modifier side effects,
8.7 route traffic tips, 8.11.3 native callout mapping, 8.11.4 debug descriptions,
8.12.1 complete debug readouts/neighbour dialog, parts of 8.12.4/8.14, neighbour
dialog in 8.13, and route drawing in 8.15. All known spec-silent choices are
commented and filed in CT18–CT20 rather than inferred from implementation sources.

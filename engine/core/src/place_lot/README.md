# Place-lot owner

Portable `cSC4ViewInputControlPlaceLot` from city-tools-input.md §3. Core only;
no plugin, C ABI, game shadow or simulator replacement. `ControlHost`, `Pixel`
and `Vec3` reuse the existing input-control contract, as in `query_tool`.
Foreign traits supply configuration/building records, renderer and UI primitives,
terrain, lot developer, budget, demolition and zoning services. They must never
call native place-lot owner methods. Selection, validation, facing, price,
placement ordering and input/message state transitions belong here.

Owner algorithms now include toll selection and independent depth-two network
search, preview occupant selection/centering, coverage categories, rendering
attributes/transforms, footprint/arrow lifecycle and constraint tiles. Renderer
primitives apply the supplied attribute fields to every model instance in mode
5, preserving the renderer's highlight palettes; uninterpreted attribute bytes
are not extra owner state. Texture artwork/extent remain foreign services.
Getters use shared references; there is no RefCell. Identities are portable
tokens, not pointers. Persistent acquired services/cursors and created preview
effects have separate release effects. Shutdown is explicit because Drop cannot
access a foreign host. The unchanged shared input-control contract provides
capture/stack and sound transport; this module introduces no second base class.
Native layout/vtable addresses and allocation size are not portable state.

Integer/float operations follow the spec literally on finite representable city
coordinates, positive lot dimensions/cell width and nonoverflowing costs. The
unsigned world rectangle stores the constructor's -1 sentinel as `u32::MAX` and
compares its signed interpretation, preserving the empty initial rectangle.
The cost indicator displays the low dword. A zero plop price skips development
cost but demolition cost is still added, per §3.10's separate ordered steps.

## Owner closure

Each row is for `cSC4ViewInputControlPlaceLot`. Test names are in `tests.rs`.
All previously open CT2/CT16/CT17/CT22–CT24 rows now have code and tests.
Two remaining spec conflicts are recorded as CT25 (section 3.7 never sets its
visibility flag true on a hit) and CT26 (section 4.5's validation gate disagrees
with detailed section 3.7). The detailed section 3.7 literal reading is retained
with comments; mouse-driven visibility and failed-facing validation remain open.
The independent `feedback(valid, visible, point)` algorithm is fully testable.
No settled helper delegates to the native owner or remains Unsupported.
Existing tests are in `tests.rs`; new closure tests are in `closure_tests.rs`.

| Spec section | Code item | Test / open portion |
|---|---|---|
| 3.1 Identity, creation, defaults, inherited input | `PlaceLot::new`, `ID`, `CURSOR`, `should_stack`, `unhandled_input`; inherited shared `ControlHost` | `construction_and_ct3_lifecycle` |
| 3.2 Init, pickers, filters, price, preview view, holds | `init`, `pick`, `resolved_building`, `build_preview`, `Host::create_ring` | `activation_pickers_filters_and_failure_paths`, `construction_and_ct3_lifecycle`, `ct17_preview_categories_order_centering_props_and_selection_failures` |
| 3.3 Shutdown | `shutdown`, `release_configuration_effects`, `stop_previews` | `construction_and_ct3_lifecycle`, `ct17_preview_categories_order_centering_props_and_selection_failures` |
| 3.4 Messages | `message`, `stop_previews`, `feedback` | `messages_stop_only_running_release_both_and_open_boundaries`, `ct22_finished_clears_then_recalculates_without_other_mutations`, `ct17_message_rebuild_and_absent_resources_do_not_substitute_visuals` |
| 3.5 Rotation keys | `key_down` | `ct4_rotation_keys_top_translation_and_manual_path` |
| 3.6 Left click, world exclusion, count, sound | `mouse_down` | `ct5_click_sound_repeat_rectangle_and_plop_limit` |
| 3.7 Cursor move, validation, auto/manual/toll paths, cost/text/delta feedback, queue flush | `mouse_move`, `can_place`, `automatic_facing`, `pick_toll`, `change_configuration` | `mouse_feedback_text_cost_exit_and_failed_coverage`, `ct4_rotation_keys_top_translation_and_manual_path`, `ct2_flushes_after_processing_on_miss_repeat_and_null`, `ct16_move_commit_changes_configuration_and_keeps_foreign_services`; open: CT25 (visibility), CT26 (failed-facing validation gate) |
| 3.8 Exit | `mouse_exit`, `feedback` hidden branch | `mouse_feedback_text_cost_exit_and_failed_coverage`, `ct17_attributes_validity_visibility_and_hidden_cleanup` |
| 3.9 Cell geometry | `cell_coverage`, `coverage` | `conformance_odd_even_rotated_cells_and_edges` |
| 3.10 Validation, precedence, codes, price, demolition | `can_place`, `demolition`, `CREATION_MESSAGES` (zones.md §8.4.1) | `validation_precedence_price_codes_and_demolition_flags` |
| 3.11 Automatic facing and frontage | `automatic_facing`, `street_count`, `choose_facing` | `conformance_frontage_hysteresis_ties_and_unplaceable_rotations` (CT24 equality included) |
| 3.12 Commit, tutorial/funds, clearing gates, explicit zoning flags, lot/building, refund, aura, connections, effects | `place_lot`, `demolish_accepts`, `Zone`, `Demolition`, `change_configuration` | `commit_order_money_aura_connections_and_effects`, `conformance_builder_failure_deposits_without_withdrawal_and_keeps_lot`, `commit_guards_unknown_cost_free_zones_and_sizes`, `ct23_clearing_gates_commit_without_effects_and_exact_zone_flags`, `ct16_move_commit_changes_configuration_and_keeps_foreign_services` |
| 3.13 Toll defaults, type/side checks, aligned footprints, depth-two search | `pick_toll`, `seek_compatible` | `ct16_picker_shapes_flags_and_failure_outputs`, `ct16_depth_two_independent_branches_and_cycles` |
| 3.14 Feedback | `build_preview`, `feedback`, `constraint_feedback` | CT17 coverage in the following three rows |
| 3.14.1 Preview candidates, centering, categories/radius/names/views, accepted props | `build_preview`, `prepare_preview` | `ct17_preview_categories_order_centering_props_and_selection_failures`, `ct17_all_categories_preview_views_and_configuration_release` |
| 3.14.2 Grid/ring, footprint/arrow, model/prop attributes/transforms, cursor, hiding | `feedback`, `draw`, `attributes` | `ct17_attributes_validity_visibility_and_hidden_cleanup`, `ct17_centered_model_and_prop_transforms`, `ct17_arrow_precision_restart_footprint_texture_and_grid_modes`, `ct17_message_rebuild_and_absent_resources_do_not_substitute_visuals` |
| 3.14.3 Constraint types, rotations, cell parameters, reuse/surplus/cleanup | `constraint_feedback`, `rotate_constraint`, `clear_constraints` | `ct17_constraint_rotations_cells_colours_reuse_and_surplus`, `ct17_attributes_validity_visibility_and_hidden_cleanup` |
| 4 Place-lot quirks 1–5, 9–11 | view guards, stack removal, plop limit, `world_rectangle` extended product/low dword, literal move gate, facing hysteresis, failure deposit | lifecycle/click/validation/facing/builder tests above, `click_world_rectangle_preserves_extended_product_and_low_dword`; open: CT26 (4.5 versus 3.7) |
| 6 Place-lot conformance 3–5 | lifecycle, keys, click | `construction_and_ct3_lifecycle`, `ct4_rotation_keys_top_translation_and_manual_path`, `ct5_click_sound_repeat_rectangle_and_plop_limit` |
| 6 re-plop conformance six bullets | odd/even snapping, frontage, hysteresis, exclusion, failed-builder deposit | `conformance_odd_even_rotated_cells_and_edges`, `conformance_frontage_hysteresis_ties_and_unplaceable_rotations`, `ct5_click_sound_repeat_rectangle_and_plop_limit`, `conformance_builder_failure_deposits_without_withdrawal_and_keeps_lot` |
| 6 CT2-a/b, CT16-a–e | queue flush, toll selection and depth-two traversal | `ct2_flushes_after_processing_on_miss_repeat_and_null`, `ct16_picker_shapes_flags_and_failure_outputs`, `ct16_depth_two_independent_branches_and_cycles` |
| 6 CT17-a–h | drawing, hiding, transforms, constraints, category priority, view IDs, prop acceptance, arrow | the seven `ct17_*` closure tests above |
| 6 CT22-a/b, CT23-a–c, CT24-a | ring completion, clearing flags/gates, equality hysteresis | `ct22_finished_clears_then_recalculates_without_other_mutations`, `ct23_clearing_gates_commit_without_effects_and_exact_zone_flags`, `commit_guards_unknown_cost_free_zones_and_sizes`, `conformance_frontage_hysteresis_ties_and_unplaceable_rotations` |

Section 6 tests 1–2 concern the shared base owner, and tests 6–10/re-query concern
`query_tool`; they are not additional place-lot algorithms. This module checks
the base lifecycle behavior it exercises (idempotent init/shutdown, cursor ID
preservation, unknown cursor no-op and ordered releases) without duplicating its
COM QueryInterface/reference-counter implementation.

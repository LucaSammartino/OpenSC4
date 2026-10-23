# Terrain ripples

Portable `cSTEWaterRipple` behavior from
[water-ripples.md](../../../../../docs/specs/water-ripples.md).
`WaterRipple` owns tracing, strip traversal, preparation, contact removal,
scheduling and draw ordering. Terrain, renderer and texture traits provide
foreign primitives; elapsed clock words arrive as messages. No plugin/C ABI
wiring or wall-time conversion is included.

## Owner closure

Tests below are in `tests_owner.rs` unless prefixed `wr8::` (`tests_wr8.rs`)
or `tests::` (`tests.rs`). Coverage means portable assertions, not native
execution or byte identity. `open:` names the residual within that row.
Selection-only §§2.1–2.3 and selection contracts in §§2.5–2.7 are outside the
assigned owner. Every normal ripple algorithm is implemented; the open native
membership inventory prevents a complete native-owner claim.

| Owner | Spec section | Code item | Test / open contract |
|---|---|---|---|
| cSTEWaterRipple | §0 summary | `WaterRipple`; `shore`, `mesh`, `render` | `first_update_draw_gates_frustum_tokens_and_process_state_cache` |
| cSTEWaterRipple | §1.1 tuning | `RetainedTuning::read`, `RippleProcess::eligible_init`, `Parameters::from_retained` | `wr8::sixth_element_and_source_bits_survive_repeated_absence_transform`, `wr8::raw_array_copy_has_no_tag_conversion_and_copies_bytes`; open: WR11 stored turn-factor bits |
| cSTEWaterRipple | §1.2 shores | `WaterRipple::detect_shores`, `shore::{detect,trace}` | `directional_trace_backward_seed_barriers_and_global_visit_clear`, `trace_singleton_equal_heights_strict_minimum_and_append_detection` |
| cSTEWaterRipple | §1.3 traversal | `mesh::traverse`, `StripWalk::{candidate,start,finish}`, `spline`, `candidate_pair` | `spline_domain_no_padding_phase_shared_and_only_inner_height_raised`, `partial_outer_and_inner_come_from_different_candidates`, `owner_spacing_turn_limit_edges_and_short_strip_reuse`; open: WR13 exact x87 intermediate order |
| cSTEWaterRipple | §1.3 preparation/resources | `WaterRipple::create_ripples`, `mesh::fade`, `Animation::prepare` | `odd_outer_turn_failure_is_not_rolled_back_and_fade_uses_configured_count`, `resource_failure_retry_preserves_old_strip_and_missing_rng_is_explicit` |
| cSTEWaterRipple | §1.4 drawing/bounds | `render::contact_rectangles`, `Rect::{union,render_box}`, `WaterRipple::update_with_renderer` | `grouped_bounds_do_not_bridge_gaps_and_contact_is_inclusive`, `first_update_draw_gates_frustum_tokens_and_process_state_cache`, `shared_random_and_state_survive_owners_reset_and_empty_animation` |
| cSTEWaterRipple | §1.5 RNG/animation | `NativeRandom::{raw,bounded,range}`, `Animation::{prepare,tick}`, `WaterRipple::do_message` | `native_rng_high_product_zero_bound_and_one_advance_overshoot_wrap`, `shared_random_and_state_survive_owners_reset_and_empty_animation`; open: WR12 initial shared seed, WR1 units opaque |
| cSTEWaterRipple | §1.5 edit/scheduling/messages | `RebuildState`, `WaterRipple::{terrain_edit_packed,rebuild_now,do_message,create_ripples}` | `edit_removes_all_contacts_rearms_clock_and_delayed_tick_animates_new_strips`, `edit_wrapping_thresholds_dirty_persistence_and_zoom_pending_redetection`; open: WR14 delayed pending flag; literal sequence redetects twice |
| cSTEWaterRipple | §2.4 lifecycle/shared tuning | `WaterRipple::{init,shutdown}`, `lifecycle::References`, `RippleProcess::{eligible_init,reset}` | `owner_reference_pick_shutdown_reinit_keeps_collection_and_first_update`, `wr8::process_shared_between_instances_and_reset_obeys_initialized_gate`, `wr8::absent_minimum_clamps_and_missing_whole_exemplar_does_nothing`; open: WR8 invalid-service/foreign-exception domains |
| cSTEWaterRipple | §2.5 entry points | `WaterRipple::{query_interface,add_ref,release,remove_ref,references,pick}`, methods above | `owner_reference_pick_shutdown_reinit_keeps_collection_and_first_update`, `wr8::logical_reference_deletion_does_not_manufacture_shutdown`; open: WR8 remaining signatures, Pick semantics and 65/69 inventory |
| cSTEWaterRipple | §2.6 conversion/retention | `RetainedTuning::read`, `retained::numeric`, `RawTuningSource::foreign_scalar` | `wr8::scalar_literals_not_float_nonzero_or_saturating_unsigned`, `wr8::all_numeric_tags_have_the_bound_source_gates`, `wr8::temporary_destroyed_on_success_and_foreign_false_output_retained`, `wr8::present_empty_and_wrong_width_are_unsupported_after_earlier_mutations`; open: WR8 foreign/controlword domains, WR11 |
| cSTEWaterRipple | §2.7 owned storage | `storage::{Buffer,Strip,StripList,Shore,assign_shores}` explicit ledger | `wr8::deep_copy_is_independent_through_edit_and_source_destruction`, `wr8::list_assignment_reuses_prefix_and_erases_rectangle_vertex_node_order`, `wr8::vertex_assignment_reuses_shrinks_grows_before_free_and_self_is_noop`, `wr8::insertion_erase_iterator_clear_and_nonempty_pop_contracts`, `wr8::shore_range_assignment_advances_destination_without_replacing_outer_storage`; open: WR8 allocator/alias/exception cases |
| cSTEWaterRipple | §3 users | Foreign integration boundary | No additional owner behavior; plugin integration excluded |
| cSTEWaterRipple | §4 quirks | Literal flags, step, minimum, raw time | `tests::literal_flags_all_low_byte_combinations`, `trace_singleton_equal_heights_strict_minimum_and_append_detection`; WR1/WR2 semantic interpretation absent |
| cSTEWaterRipple | §5 open questions | Named residuals above; typed unsafe-boundary errors | open: WR1, WR2, WR8, WR11–WR14; WR3 is selection-only |
| cSTEWaterRipple | §6 conformance | Owner and retained-storage fixtures below | All ripple-applicable vectors mapped; no native oracle claim |

## Conformance vectors (§6)

| Vector | Fixture |
|---|---|
| 1 spline | `tests::vector1_spline` |
| 2 zoom-4 width | `tests::vector2_width_zoom4`, `spline_domain_no_padding_phase_shared_and_only_inner_height_raised` |
| 3 spacing, 4 turn | `owner_spacing_turn_limit_edges_and_short_strip_reuse` |
| 5 short shore | `tests::vector5_short_shore`, `trace_singleton_equal_heights_strict_minimum_and_append_detection` |
| 6 exact frame boundary | `native_rng_high_product_zero_bound_and_one_advance_overshoot_wrap` |
| 9 partial pairs | `partial_outer_and_inner_come_from_different_candidates` |
| 10 UV/1002-pair limit | `partial_outer_and_inner_come_from_different_candidates`, `owner_spacing_turn_limit_edges_and_short_strip_reuse` |
| 11 shortened fade | `odd_outer_turn_failure_is_not_rolled_back_and_fade_uses_configured_count` |
| 12 singleton/equality/global clear | `trace_singleton_equal_heights_strict_minimum_and_append_detection`, `directional_trace_backward_seed_barriers_and_global_visit_clear` |
| 13 overshoot/RNG/period bounds | `native_rng_high_product_zero_bound_and_one_advance_overshoot_wrap`, `tests::tuning_property_routes_and_raw_thresholds` |
| 15 thresholds/delay/wrap | `tests::terrain_area_and_dirty_rectangle_boundaries`, `edit_wrapping_thresholds_dirty_persistence_and_zoom_pending_redetection`, `edit_removes_all_contacts_rearms_clock_and_delayed_tick_animates_new_strips` |
| 19 reference/QI | `wr8::logical_reference_deletion_does_not_manufacture_shutdown` |
| 20 process reread | `wr8::process_shared_between_instances_and_reset_obeys_initialized_gate`, `wr8::sixth_element_and_source_bits_survive_repeated_absence_transform` |
| 21 invalid resources/backing | `resource_failure_retry_preserves_old_strip_and_missing_rng_is_explicit`, `wr8::present_empty_and_wrong_width_are_unsupported_after_earlier_mutations` |
| 22 scalar conversions | `wr8::scalar_literals_not_float_nonzero_or_saturating_unsigned`, `wr8::all_numeric_tags_have_the_bound_source_gates` |
| 23 independent field ledger | `wr8::sixth_element_and_source_bits_survive_repeated_absence_transform`, `wr8::absent_minimum_clamps_and_missing_whole_exemplar_does_nothing` |
| 24 nested ownership | `wr8::deep_copy_is_independent_through_edit_and_source_destruction`, `wr8::list_assignment_reuses_prefix_and_erases_rectangle_vertex_node_order`, `wr8::vertex_assignment_reuses_shrinks_grows_before_free_and_self_is_noop` |

Vectors 7–8, 14, 16–18 and 25 are selection-only and receive no ripple credit.
Legacy `PairFilter`, `StripGeometry`, `AnimatedStrip`, `FrameAnimation` and
`RandomSource` APIs remain isolated diagnostic helpers for compatibility. Their
whole-pair rejection and overshoot errors are not owner behavior; the owner
uses `mesh::StripWalk`, `mesh::Animation` and the shared `NativeRandom`.

## Limits

Unchecked null services, invalid raw backing and preparation without a binding
have no defined successful native result. Rust returns typed errors rather
than emulating memory faults. It does not fill missing arrays, invent seeds or
replace zero derivatives with artificial tangents. Retained tuning supports
the published normal numeric mode and exact period products through period
2^24; wider magic-conversion/controlword behavior remains unclaimed (WR8).
Logical native destruction/refcounts use explicit events and storage ledgers,
independent of Rust's memory-safe `Arc`/`Vec` destruction. No claim is made that
Rust Drop leaks foreign references like direct native deleting destruction.

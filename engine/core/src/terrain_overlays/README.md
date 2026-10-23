# Terrain overlay manager

Core implementation of `cSTEOverlayManager`, from
[overlay-manager.md](../../../../docs/specs/overlay-manager.md). No plugin registration or C ABI change.
`OverlayManager::draw_frame` owns visible-row selection, pass control, transforms, state operations,
rectangle routing and submission counters. `DrawingProducer` supplies the post-reset context matrix,
raw terrain-view rows and rectangle consumers; `ShadowLighting` supplies lighting slots 14/15.
`ModelLighting`, `TerrainProvider` and `TextureFactory` supply their named foreign services.
No producer supplies completed owner transforms, selected IDs, bounds for ordinary placement or counter deltas.

The implementation is portable Rust, with `Arc` references and externally synchronized mutation.
Redisplay invokes terrain slot 68 through `TerrainProvider::redisplay` and also retains a diagnostic queue.
Headless clients can consume that queue using the default callback. Index storage is sparse with shift zero;
the resulting cell-list semantics, insertion order, removal order, collection order and stamp wrap match the spec.
No native allocation, vtable layout, unsafe invalid-pointer or exceptional allocation proof is claimed.

## Owner closure

Each row names `cSTEOverlayManager`; summary/container sections refer to the concrete items below.
Tests live in `tests.rs` and `drawing_tests.rs`. “Open” limits the stated numerical/foreign boundary,
not the implemented owner control flow. The companion native inventory's storage-only S entries have
no independent behavior contract and are not counted as additional implemented native bodies.

| Owner / spec section | Core item | Covering test / residual |
|---|---|---|
| cSTEOverlayManager §0 summary | `OverlayManager`, `draw_frame` | `manager_stats_count_submissions_and_preserve_skipped_passes` |
| cSTEOverlayManager §1 object | `IID`, `init`, `shutdown`, producer references | `lifecycle_holds_acquired_references_and_unsubscribes_once_before_release`; COM layout/registration outside core |
| cSTEOverlayManager §1.1 init/repeat/defaults | `new`, `init`, overlay constructors | `public_repeat_init_preserves_selection_and_shutdown_retains_groups_index_binding_stats`, `unsupported_initialization_services_preserve_published_prefix`; exceptional terrain numeric domain open: OV10 |
| cSTEOverlayManager §1.2 shutdown/destruction | `shutdown`, `Drop`, `Arena::clear`, `Overlay::drop` | `destruction_does_not_unsubscribe_and_shutdown_supports_reinitialization`, `public_removed_overlay_range_releases_source_before_binding` |
| cSTEOverlayManager §2 records | `records` | §2.1–2.3 tests below |
| cSTEOverlayManager §2.1 tables/groups | `Overlay`, `HandleRecord`, `Target`, `indices`, `single` | `conformance_group_set_shown_hides_all_eight_and_invalidates`, `group_color_draw_state_alpha_endpoints_and_scale_fan_out` |
| cSTEOverlayManager §2.2 cell index/change | `index_and_mark`, `unindex`, `mark`, `collect_visible_rows` | `public_collection_encounter_order_and_zero_stamp_wrap`, `redisplay_callback_observes_order_and_empty_dynamic_suppression`, `public_position_tolerance_boundary_and_same_rectangle_keep_prepend_order` |
| cSTEOverlayManager §2.3 allocation/reuse/getters | `Arena`, `remove`, scalar/placement getters, `edit` | `removal_retains_payload_until_fifo_reuse_and_unlinks_cells`, `group_removal_cleans_eight_parts_group_and_indirect_record_then_reuses_slots`, `conformance_null_handle_is_ignored_by_every_handle_method` |
| cSTEOverlayManager §3 interface | `add_*`, `set_*`, getters, `stats_string` | concrete slot tests in §3.1–3.7 and §9; no ABI adapter |
| cSTEOverlayManager §3.1 round decals | `add_round_decal`, `round::partition` | `public_ov9_literal_outline_and_binary32_height20_splits`, `public_ov9_add_retains_empty_parts_and_full_texture_matrix`; exceptional conversion open: OV10 |
| cSTEOverlayManager §3.2 placement | `placement::calculate`, `set_position`, `set_placement`, `get_placement` | `public_placement_paper_matrix_and_tolerance`, `placement_getter_copies_nine_floats_and_failed_conversion_never_mutates`; exceptional conversion open: OV10 |
| cSTEOverlayManager §3.2.1 rotated order | ordered coefficients in `placement::calculate` | `rotated_coefficients_expand_footprint_only_without_repeats`; native transcendental/ambient FP precision open: OV10 |
| cSTEOverlayManager §3.3 alpha | `set_alpha` | `public_ov6_binary64_threshold_and_accumulation`, `static_alpha_uses_accepted_value_and_dynamic_alpha_skips_redisplay` |
| cSTEOverlayManager §3.4 replacement scale | `set_texture_scale` | `conformance_texture_scale_sets_flag_and_semantic_coefficients`, `rotated_coefficients_expand_footprint_only_without_repeats` (zero/Inf/NaN); group calls explicitly unsafe/unsupported per spec |
| cSTEOverlayManager §3.5 keyed shadows | `add_shadow_key`, `add_shadow_group`, `model::keyed_secondary` | `keyed_flip_byte_and_placement_unchanged`, `conformance_shadow_quality_zero_returns_none_before_factory_lookup` |
| cSTEOverlayManager §3.6 rectangle/city | `add_overlay_for_rect`, `add_city_decal` | `public_ov2_rectangle_copies_every_word_without_clipping_or_handle`, `public_ov2_rectangle_keeps_signed_and_inverted_words`, `public_ov2_city_matrix_uses_only_signed_nx` |
| cSTEOverlayManager §3.7 model shadows | `add_model_shadow`, `model::{fallback,extrema,secondary}` | `public_model_fallback_and_secondary_literal_quarter_mapping`, `public_model_cached_live_quality_and_null_binding_query_order`; foreign slot17/18 production open: OV1, exceptional cell conversion open: OV10 |
| cSTEOverlayManager §4 draw entry | `draw_frame`, `draw_selected` | `rough_pass_alpha_minimum_flag7_and_snapshot_shadow_mask`, `disabled_index_scans_records_and_empty_selection_still_cleans_context`; exact numerical evaluation open: OV7 |
| cSTEOverlayManager §4.1 states | `DRAW_STATES` | `draw_state_table_covers_all_five_public_states` |
| cSTEOverlayManager §4.2 decals | `draw_selected` decal pass | `decal_source_shade_wrap_dispatch_and_dual_masks`; exact composition open: OV7; conflicting wrap flags open: OV11 |
| cSTEOverlayManager §4.3 shadow passes | `draw_selected`, `draw_shadows`, `ShadowLighting` | `rough_pass_alpha_minimum_flag7_and_snapshot_shadow_mask`, `full_shadow_normal_debug_gate_and_midpoint`; foreign lighting open: OV8 |
| cSTEOverlayManager §4.4 stats/skips | pass counters, `stats_string` | `manager_stats_count_submissions_and_preserve_skipped_passes`, `decal_source_shade_wrap_dispatch_and_dual_masks` |
| cSTEOverlayManager §4.5 visible walk | `collect_visible_rows`, `collect_cells_at_stamp`, arena scan | `public_collection_encounter_order_and_zero_stamp_wrap`, `disabled_index_scans_records_and_empty_selection_still_cleans_context` |
| cSTEOverlayManager §4.6 submission/state exits | `submit`, `draw_frame`, shadow pass exits | `decal_source_shade_wrap_dispatch_and_dual_masks`, `rough_pass_alpha_minimum_flag7_and_snapshot_shadow_mask`, `water_full_routes_use_original_rectangle_and_signed_comparison`; terrain consumer internals open: OV8 |
| cSTEOverlayManager §4.7 camera/composition | `inverse`, `compose`, `draw_frame_exact` | `camera_order_reset_and_singular_propagation`; exact cofactor/spill/all-entry dot orders open: OV7 |
| cSTEOverlayManager §4.8 stages/strength/water | `draw_shadows`, `submit` | `full_shadow_normal_debug_gate_and_midpoint`, `water_full_routes_use_original_rectangle_and_signed_comparison`; lighting/slot53 internals open: OV8 |
| cSTEOverlayManager §5 messages | `on_message`, `handle_settings_message` | `public_ov3_settings_message_is_quiet_and_copies_raw_values`, `disabled_index_scans_records_and_empty_selection_still_cleans_context` |
| cSTEOverlayManager §6 users | public add/mutation API | caller integration outside this owner; core-only task |
| cSTEOverlayManager §7 quirks | group recursion, signed masks, last-pass counters | `conformance_slot20_corrected_or_only_low_bit_no_redraw`, §4.4 tests |
| cSTEOverlayManager §8 open questions | traits, `draw_frame_exact`, safe conversion errors | open: OV1/OV7/OV8 as explicitly parked by spec; new OV10/OV11 qualified above |
| cSTEOverlayManager §9 conformance | tests 1–11 below | all eleven paper vectors covered; native precision qualified |
| cSTEOverlayManager §9 slot20 review | `enable_self_lighting` | `conformance_slot20_corrected_or_only_low_bit_no_redraw`, `conformance_slot20_indirect_group_visits_every_part_and_preserves_other_flags` |

Conformance mapping: (1) null-handle test; (2) group shown test; (3) quality-zero test;
(4) texture-scale test; (5) dynamic snapshot test and producer snapshot test;
(6) `manager_stats_count_submissions_and_preserve_skipped_passes` submits 3/2;
(7–8) literal outline/height20 test; (9) keyed flip test (non-square texture upper edge z=4);
(10) encounter-order/stamp-wrap test uses literal lists `[9,2]`, `[2,7]`;
(11) camera-order test gets U=1.5, while singular input still submits propagating NaNs.

## Precision and evidence

`draw_frame` evaluates the settled equations using f64 intermediates and explicit f32 stores.
It implements C×M and the published row-3 U/V accumulation orders. It is a portable execution mode,
not a native-bit-exact claim. `draw_frame_exact` explicitly returns OV7 except for the early dynamic
snapshot return. `select_frame` is a headless diagnostic; actual submission uses `draw_frame`.
No complete GPU save/restore is invented: exits perform only the named resets, clear stage 0,
restore depth offset and release an acquired extension. Destruction does not implicitly unsubscribe.

No real-game equivalence is claimed: this task has no plugin/capi wiring. Unit producers verify owner
calls and results; the outside-owner algorithms are neither reimplemented nor replaced by guessed data.

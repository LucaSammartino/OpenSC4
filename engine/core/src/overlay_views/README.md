# Overlay view owners

Pure Rust from overlay-views.md (OV) and network-overlay-views.md (NO). This
replaces the previous partial implementation. Native owner names are re-exported
as aliases. No C ABI, plugin, replacement, native or shadow verification is claimed.

Hosts supply foreign world/resource primitives. Geometry, traversal, sorting,
state changes and batching belong to this module. Renderer commands carry exact
primitive/format IDs, vertices and indices. Resource handles are identities, never
dereferenced pointers. Returned bindings are owned; returned models are temporary
acquired references. Model assignment retains before releasing the previous model
and temporary. Providers own any internal service references and must initialize
their Option outputs; uncertified native failure-output contents are not invented.

Share exactly one `OverlayProcess` across views: vertex helper, pulse and direction
table are process state. Default pulse is unknown; use an observed precondition
through `Pulse::with_observed_value`. Advancing unknown pulse is Unsupported(NOV2).
Unwritten repeat/outline/binding slots and scratch alpha are explicit Options.
Unwritten repeat leaves UV unknown; known position and colour remain available.

A nonnull train marker produces `UnresolvedMarker`: its established model,
translation and first-route direction, **not a rendered model**. Update completes
known quads then returns Unsupported(NOV3). No guessed rotation or same-owner host
callback fills that gap. Raw fields support fixtures, not invented native setters.
There is no invented independent outline-disable operation (OVV4).

`View<T>` implements the four shared reference/deletion chains: initial count zero,
ID-1 QI returning primary+4, wrapping AddRef, count-zero/one Release deletion,
deleting flags, and Pick output preservation. Use explicit lifetime calls; Rust
Drop does not invoke external services. Train deletion releases model 1 then 0
without Shutdown or binding releases; explicit Shutdown releases all six bindings.
PathInfo and the process vertex helper do not have invented COM constructors.

`Undefined` diagnoses unsafe native caller boundaries in safe Rust: stale selected
path, invalid storage/index, empty regular path, missing unguarded service,
released interface, or oversized copy. These errors do not assert native rejects
or successful no-ops. Setters retain unvalidated floats and zero-length helpers
retain IEEE division. Exceptional x87/NaN payload and allocation-failure behavior
remain uncertified where the spec says so. Finite geometric tests use tolerances;
bit-exact x87 arithmetic is not claimed.

## Owner closure

Each normative owner section is mapped here. Summary, users and quirks add no
separate operations; open-question sections retain the listed qualifications.
Test names below are in tests.rs. Shared inventory qualifications do not hide
established methods behind blanket stubs.

| Owner | Spec section | Code item | Test or open portion |
|---|---|---|---|
| cSC4PathView | OV 1 | `PathView` defaults/mutations/update | `path_spec_order_strip_duplicate_and_defaults` |
| cSC4PathView | OV 1.1 | `draw_path`, `draw_arrowhead`, render state | `ov_l3_l4_offsets_uv_and_reversal`, `ov_l5_l6_arrows_sequence_and_stored_points` |
| cSC4PathView | OV 1.2 | selected records, repeat, geometry, cleanup | `ov_l1_l2_order_colour_selection_and_unordered_priority`, `path_texture_lifetime_state_and_empty_update`; open: OVV6 exceptional arithmetic/allocation/additional public surface |
| PathInfo | OV 1 | `PathInfo` | `ov_l1_l2_order_colour_selection_and_unordered_priority` |
| PathInfo | OV 1.2 | fields and binding-before-deque cleanup | `deleting_cleanup_of_each_owner_train_does_not_shutdown` |
| SC4MarkedCellView | OV 2 | defaults, init, passes | `cells_batch_split_colours_outline_state_and_bounds` |
| SC4MarkedCellView | OV 2.1 | `draw_cells` | `ov_l9_water_frustum_lot_order_hidden_and_raised` |
| SC4MarkedCellView | OV 2.2 | colour conversion, lower vertex, mutations | `ov_l7_colour_conversion_mutations_and_unvalidated_sizes`, `ov_l8_other_size_clamp_floor_and_no_lots`; open: OVV4 outline-disable API |
| cSTEVertBufExtn | OV 3 | `VertexBufferExtension` and `IndexSource` | `vertex_buffer_acquire_gate_format_capacity_no_initial_lock` |
| cSTEVertBufExtn | OV 3.1 | copy, rebase, flush/release/lock | `ov_l10_l11_vertex_compaction_source_offsets_and_word_wrap`, `ov_l12_l13_flush_relock_failure_and_failed_create_counts`, `vertex_empty_range_growth_flush_and_release_are_distinct` |
| cSC4PathView | OV 3.2 | all published path methods | path tests above; open: OVV6 historical inventory/additional methods |
| PathInfo | OV 3.2 | owned deque record | cleanup test; open: OVV6 standalone ABI not established |
| SC4MarkedCellView | OV 3.2 | inclusive x-outer region scan, lifecycle | `ov_l7_colour_conversion_mutations_and_unvalidated_sizes`; open: OVV6 allocation-failure path |
| cSTEVertBufExtn | OV 3.2 | all seven static methods | vertex tests above; open: OVV6 allocation/invalid caller sequences |
| SC4PathInfoView | OV 3.2 | diagnostic methods | network tests below; open: NOV2 |
| SC4TrainJunctionView | OV 3.2 | resource/quad/frame methods | train tests below; open: NOV3 |
| cSC4PathView | OV 3.3 | published finite path contracts | path tests above; open: OVV6 exceptional arithmetic/allocation |
| PathInfo | OV 3.3 | `PathInfo::cleanup` | cleanup test; open: OVV6 independent record ABI |
| SC4MarkedCellView | OV 3.3 | terrain/colour/region contracts | cell tests above; open: OVV4, OVV6 |
| cSTEVertBufExtn | OV 3.3 | borrowed source and static batching | vertex tests above; open: OVV6 allocation/unsafe caller sequences |
| SC4PathInfoView | OV 3.3 | colours/scratch/pulse step | network tests below; open: NOV2 static initializer/setters |
| SC4TrainJunctionView | OV 3.3 | quads/state/resource lifetime | train tests below; open: NOV3 marker state/setters/provider failure outputs |
| cSC4PathView | OV 3.4 | `View<PathView>` and cleanup | `ov_l14_shared_query_reference_wrap_zero_release_and_pick`, `deleting_cleanup_of_each_owner_train_does_not_shutdown` |
| SC4MarkedCellView | OV 3.4 | `View<MarkedCellView>` and cleanup | same shared/owner cleanup tests |
| SC4PathInfoView | OV 3.4 | `View<PathInfoView>` and cleanup | same shared/owner cleanup tests |
| SC4TrainJunctionView | OV 3.4 | `View<TrainJunctionView>` and cleanup | same shared/owner cleanup tests; no implicit Shutdown |
| SC4PathInfoView | NO 1 | `PathInfoView::update` | `network_traversal_filter_state_idempotence_and_empty_rectangle` |
| SC4PathInfoView | NO 1.1 | `draw_network_path`, colour/pulse | `no_l1_colours_and_process_pulse_no_manager_or_constructor_reset`, `no_l2_sloped_arrow_uses_stored_height_and_3d_chord`, `no_l3_l4_full_vector_arrows_bars_precedence_and_scratch_alpha`, `no_l5_cross_axes_height_queries_and_opaque_alpha`, `network_arrow_backward_search_vertical_threshold_and_degenerate_boundaries` |
| SC4PathInfoView | NO 1.2 | defaults and init tuple | `network_traversal_filter_state_idempotence_and_empty_rectangle`, `init_without_render_service_and_short_unsafe_sources_are_explicit`; open: NOV2 pulse initializer/filter/depth setters |
| SC4TrainJunctionView | NO 2 | `TrainJunctionView`, direction table | `train_resources_lifetime_state_missing_services_and_null_bindings` |
| SC4TrainJunctionView | NO 2.1 | classifier, quad, order, frame offset, Shutdown | `no_l6_l7_train_horizontal_chord_classification`, `no_l8_quad_world_space_height_winding_uv_and_degenerate_direction`, `no_l9_train_order_skips_frame_stacking_transparency_and_marker_boundary`; open: NOV3 marker rotation/model state, cell setters/identities, caller Shutdown discipline |
| cSC4PathView | OV 7 | order/strip/duplicate; JSON adapter | `path_spec_order_strip_duplicate_and_defaults` |
| SC4MarkedCellView | OV 7 | batch counts; JSON adapter | `cells_batch_split_colours_outline_state_and_bounds` |
| cSTEVertBufExtn | OV 7 | bool-25 gate; JSON adapter | `vertex_buffer_acquire_gate_format_capacity_no_initial_lock` |
| cSC4PathView, PathInfo | OV 7.1 L1–L6 | priority/colour/geometry | tests prefixed `ov_l1_l2`, `ov_l3_l4`, `ov_l5_l6` |
| SC4MarkedCellView | OV 7.1 L7–L9 | packing/grid/lot order | tests prefixed `ov_l7`, `ov_l8`, `ov_l9` |
| cSTEVertBufExtn | OV 7.1 L10–L13 | compaction/failure | tests prefixed `ov_l10_l11`, `ov_l12_l13` |
| Four views | OV 7.1 L14 | shared lifetime | `ov_l14_shared_query_reference_wrap_zero_release_and_pick` |
| SC4PathInfoView | NO 6 | colour/25-point cap; JSON adapter | tests prefixed `no_l1`, `no_l3_l4` |
| SC4TrainJunctionView | NO 6 | classifier/transparency; JSON adapter | tests prefixed `no_l6_l7`, `no_l9` |
| SC4PathInfoView | NO 6.1 L1–L5 | diagnostic literal vectors | tests prefixed `no_l1` through `no_l5` |
| SC4TrainJunctionView | NO 6.1 L6–L9 | train literal vectors | tests prefixed `no_l6_l7`, `no_l8`, `no_l9` |

OVV1/NOV1 retain lot flag/key semantic names and external cheat activation; raw
bits are implemented without guessed names. Existing open questions cover all
remaining spec qualifications; no new question is filed. The nine existing JSON
fixtures remain, with the provisional `[choice OVV3]` strip vector corrected to
OV 1.2's now-established normal direction and longitudinal/side UV order, and the
old stable-priority fixture corrected to the same section's newest-first tie rule.

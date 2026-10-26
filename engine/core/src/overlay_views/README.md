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
table are process state. The pulse starts at loader-zero, advances before manager
lookup, and is never reset by an owner's construction, Init, Shutdown or deletion.
Unwritten repeat/outline/binding slots and scratch alpha are explicit Options.
Unwritten repeat leaves UV unknown; known position and colour remain available.

A nonnull train marker emits the specified row-grouped basis/state, model
transparency and draw commands, between temporary model retain/release calls.
Hover and pressed-cell mutations model the caller's inline stores, not extra
native interface slots. There is no independent outline-disable API (OVV4).

`View<T>` implements the four shared reference/deletion chains: initial count zero,
ID-1 QI returning primary+4, wrapping AddRef, count-zero/one Release deletion,
deleting flags, secondary RemoveRef without deletion, count getter and Pick output
preservation. Use explicit lifetime calls; Rust
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

| Owner | Implementation credit | Open IDs |
|---|---|---|
| cSC4PathView | implemented | none (OVV6 applied) |
| SC4TrainJunctionView | implemented | none (OVV6, NOV3 applied) |
| SC4MarkedCellView | partial | OVV4: external inline outline reset observation, OV 2.3 |
| SC4PathInfoView | implemented | none (OVV6, NOV2 applied) |

Each normative owner section is mapped here. Summary, users and quirks add no
separate operations; open-question sections retain the listed qualifications.
Test names below are in tests.rs. Shared inventory qualifications do not hide
established methods behind blanket stubs.

| Owner | Spec section | Code item | Test or open portion |
|---|---|---|---|
| cSC4PathView | OV 1 | `PathView` defaults/mutations/update | `path_spec_order_strip_duplicate_and_defaults` |
| cSC4PathView | OV 1.1 | `draw_path`, `draw_arrowhead`, render state | `ov_l3_l4_offsets_uv_and_reversal`, `ov_l5_l6_arrows_sequence_and_stored_points` |
| cSC4PathView | OV 1.2 | selected records, repeat, geometry, cleanup | `ov_l1_l2_order_colour_selection_and_unordered_priority`, `path_texture_lifetime_state_and_empty_update`; OVV6: OV 3.3 limits successful inputs to valid storage/allocation and the finite geometry contract |
| PathInfo | OV 1 | `PathInfo` | `ov_l1_l2_order_colour_selection_and_unordered_priority` |
| PathInfo | OV 1.2 | fields and binding-before-deque cleanup | `deleting_cleanup_of_each_owner_train_does_not_shutdown` |
| SC4MarkedCellView | OV 2 | defaults, init, passes | `cells_batch_split_colours_outline_state_and_bounds` |
| SC4MarkedCellView | OV 2.1 | `draw_cells` | `ov_l9_water_frustum_lot_order_hidden_and_raised` |
| SC4MarkedCellView | OV 2.2–2.3 | colour conversion, lower vertex, `copy_colours`, mutations/passes | `ov_l7_colour_conversion_mutations_and_unvalidated_sizes`, `ov_l8_other_size_clamp_floor_and_no_lots`, `ovv4_worked_mutations_preserve_additive_outline_until_opaque_override`; open: OVV4 external inline reset observation |
| cSTEVertBufExtn | OV 3 | `VertexBufferExtension` and `IndexSource` | `vertex_buffer_acquire_gate_format_capacity_no_initial_lock` |
| cSTEVertBufExtn | OV 3.1 | copy, rebase, flush/release/lock | `ov_l10_l11_vertex_compaction_source_offsets_and_word_wrap`, `ov_l12_l13_flush_relock_failure_and_failed_create_counts`, `vertex_empty_range_growth_flush_and_release_are_distinct` |
| cSC4PathView | OV 3.2, 3.5 | all published path methods; `View::remove_ref`, `count` | path tests above; `ovv6_secondary_remove_ref_never_deletes_any_owner` (OVV6) |
| PathInfo | OV 3.2, 3.5 | owned deque record, no standalone COM surface | `deleting_cleanup_of_each_owner_train_does_not_shutdown` (OVV6) |
| SC4MarkedCellView | OV 3.2 | inclusive x-outer region scan, lifecycle | `ov_l7_colour_conversion_mutations_and_unvalidated_sizes`, `ovv6_secondary_remove_ref_never_deletes_any_owner`; OVV6 successful allocation/container preconditions apply |
| cSTEVertBufExtn | OV 3.2 | all seven static methods | vertex tests above; OVV6 identifies no additional COM surface, invalid allocation/caller sequences are outside the successful contract |
| SC4PathInfoView | OV 3.2, 3.5 | diagnostic methods and folded/inline mutations | `nov2_worked_state_filter_and_fresh_process_pulse`, `ovv6_secondary_remove_ref_never_deletes_any_owner` |
| SC4TrainJunctionView | OV 3.2, 3.5 | resource/quad/frame methods, shared secondary methods | `nov3_worked_hover_press_marker_basis_state_and_submission_order`, `ovv6_worked_lifetime_shutdown_then_release_has_no_double_model_release` |
| cSC4PathView | OV 3.3 | published finite path contracts | path tests above; `ovv6_secondary_remove_ref_never_deletes_any_owner`; exceptional arithmetic/failed allocation outside successful-input scope |
| PathInfo | OV 3.3 | `PathInfo::cleanup`, parent-owned record | `deleting_cleanup_of_each_owner_train_does_not_shutdown` (OVV6: no independent record interface) |
| SC4MarkedCellView | OV 3.3 | terrain/colour/region contracts | cell tests above, `ovv4_worked_mutations_preserve_additive_outline_until_opaque_override`; open: OVV4 external inline reset observation |
| cSTEVertBufExtn | OV 3.3 | borrowed source and static batching | vertex tests above; OVV6 successful-input/caller sequence qualification applies |
| SC4PathInfoView | OV 3.3 | `Pulse::default`, depth/filter setters, Shutdown | `nov2_worked_state_filter_and_fresh_process_pulse` (NOV2) |
| SC4TrainJunctionView | OV 3.3 | marker basis/state, hover/pressed stores, resource lifetime | `nov3_worked_hover_press_marker_basis_state_and_submission_order`, `ovv6_worked_lifetime_shutdown_then_release_has_no_double_model_release`; NOV3 failed-provider outputs remain caller-unsafe, not guaranteed null |
| cSC4PathView | OV 3.4 | `View<PathView>` and cleanup | `ov_l14_shared_query_reference_wrap_zero_release_and_pick`, `deleting_cleanup_of_each_owner_train_does_not_shutdown` |
| SC4MarkedCellView | OV 3.4 | `View<MarkedCellView>` and cleanup | same shared/owner cleanup tests |
| SC4PathInfoView | OV 3.4 | `View<PathInfoView>` and cleanup | same shared/owner cleanup tests |
| SC4TrainJunctionView | OV 3.4 | `View<TrainJunctionView>` and cleanup | same shared/owner cleanup tests; no implicit Shutdown |
| SC4PathInfoView | NO 1 | `PathInfoView::update` | `network_traversal_filter_state_idempotence_and_empty_rectangle` |
| SC4PathInfoView | NO 1.1 | `draw_network_path`, colour/pulse | `no_l1_colours_and_process_pulse_no_manager_or_constructor_reset`, `no_l2_sloped_arrow_uses_stored_height_and_3d_chord`, `no_l3_l4_full_vector_arrows_bars_precedence_and_scratch_alpha`, `no_l5_cross_axes_height_queries_and_opaque_alpha`, `network_arrow_backward_search_vertical_threshold_and_degenerate_boundaries` |
| SC4PathInfoView | NO 1.2 | defaults, Init tuple, folded depth/Shutdown, inline full-u32 filter | `network_traversal_filter_state_idempotence_and_empty_rectangle`, `nov2_worked_state_filter_and_fresh_process_pulse`, `init_without_render_service_and_short_unsafe_sources_are_explicit` (NOV2) |
| SC4TrainJunctionView | NO 2 | `TrainJunctionView`, direction table | `train_resources_lifetime_state_missing_services_and_null_bindings` |
| SC4TrainJunctionView | NO 2.1–2.2 | classifier, quad, frame offset, `marker_state`, hover/pressed stores, Shutdown | `no_l6_l7_train_horizontal_chord_classification`, `no_l8_quad_world_space_height_winding_uv_and_degenerate_direction`, `no_l9_train_order_skips_frame_stacking_transparency_and_marker`, `nov3_worked_hover_press_marker_basis_state_and_submission_order`, `ovv6_worked_lifetime_shutdown_then_release_has_no_double_model_release` (NOV3) |
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
remaining spec qualifications; no new question is filed. OVV4 alone remains open
for these four owners: capture a nonconstruction/nondeletion 1→0 write at live
P+0x44, writer PC/stack and triggering arguments, then a normal-only Update with
surviving nonempty cells and kPreferOpaque=false (OV 2.3). Hiding cells or opaque
override is not that observation. The nine existing JSON
fixtures remain, with the provisional `[choice OVV3]` strip vector corrected to
OV 1.2's now-established normal direction and longitudinal/side UV order, and the
old stable-priority fixture corrected to the same section's newest-first tie rule.

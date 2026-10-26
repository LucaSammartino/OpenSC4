# Model maker (partial engine implementation)

Portable `cSC4ModelMaker`, from `docs/specs/model-maker.md` only. No C ABI or
plugin wiring. Handles are identities, not addresses. Each trait documents
borrowed versus acquired results. Mutations require exclusive access and shared
getters contain no `RefCell`. Native final release is explicit (`release(host)`);
`destroy(host)` exposes the separate destructor contract.

This round closes model construction; **the owner is not implemented in full**.
PostCityInit, message dispatch, occupant handlers and AddModels type dispatch
remain pending. A pending row does not claim coverage or act as an invented fallback.
The five texture slots are nulled by Shutdown as specified in §3.3; the explicit
destructor releases bindings still present, in reverse order (§3.4).

| Spec section | Code item | Spec case / test and boundary |
|---|---|---|
| 1 Identity and lifetime | `query_interface`, `add_ref`, `release`, `destroy` | M1 `m1_query_interfaces_leave_failed_output_untouched`; M2 `m2_final_release_runs_shutdown_init_is_idempotent_and_keeps_saved_refs` |
| 2 State | `ModelMaker`, `SavedState`, `DeferredEntry` defaults | `constructor_state_and_thread_safe_borrows` |
| 3 Lifecycle | `init`, `shutdown`, `destroy` | M2 above; `destructor_releases_only_remaining_textures_in_reverse_order`; **pending §3.2 PostCityInit**, depends on §§8/12/14 |
| 4 Save record | `persistence::{ReadRecord,WriteRecord}`, `read`, `write` | M3 `m3_bad_version_marks_read_and_consumes_nothing_else`; M4 `m4_record_order_iids_round_trip_and_errors` |
| 5 Messages | Pending | M5/M11/M12 pending; no message dispatch exported |
| 6 Occupant filters | §6.1 acceptance lists passed to `LifecycleHost::create_filter` | Filter construction arguments checked by M2; **pending §§6.2–6.4**, including M17 |
| 7 Radius, rects, unloading | `geometry`, `LoadRectHost`, `UnloadHost` | M13/M14 `m13_memory_radii_and_m14_odd_integer_rounding`; M15/M16 `m15_load_rect_and_m16_city_exclusions`; `low_memory_unload_gate_full_masks_and_rotation` |
| 8 Occupant/deferred handlers | Pending (empty queue state and cleanup only) | M6–M10 pending |
| 9 State changes | Pending | M18/M20/M21 pending |
| 10 View changes | Pending | M19/M23/M27 pending |
| 11 Lighting/dynamic textures | `dynamic_textures`, `TextureHost`, `Kernel` | M22 `m22_light_threshold_rebuild_order_and_existing_binding_identity`; `raw_kernels_round_alpha_and_keep_the_16_bit_row_stride_quirk`; `dxt_endpoint_remapping_and_dxt35_keeps_unswapped_endpoints` |
| 12 Model loading | `model_loading`, `materials`, `transform`, `entries` | Subsection closures below; foreign model/resource/utility ports, no native ABI mapping |
| 12.1 RemoveModels | `remove_models` | `remove_models_runs_hiding_before_clear_and_bulk_reload_skips_hiding` |
| 12.2 Prop/shadow wrappers | `load_prop_models`, `load_models_with_shadow` | `prop_style_gate_and_shadow_wrapper_keep_the_common_result` |
| 12.3 Small helpers | `materials::assign_counted`, `reset_lot_base_texture` | `counted_assignment_keeps_identity_and_acquires_before_release`; base reset in §12.5 test below |
| 12.4 Material walks | `change_models`, `restore_static_textures`, `apply_texture_to_models`, `swap_textures_by_offset` | `change_models_default_mode_checks_registration_and_retains_holder`, `material_apply_reads_frame_once_and_restore_changes_only_swapped_materials`, `offset_swap_leaves_missing_textures_static_and_counts_the_loaded_reference` |
| 12.5 Lookup/geometry/containers | `lot_at_occupant`, `construct_interface`, `transform::rotate_about_axis`, `entries`; foundation pool/setters through `ModelHost` | `lot_lookup_uses_signed_inclusive_city_bounds_and_interface_constructor_does_not_assign`, `rotate_axis_preserves_third_component_and_asymmetric_rounding`, `counted_vectors_grow_prefix_copies_suffix_then_release_old_buffer`; Rust owns backing buffers, no native pool layout |
| 12.6 Network/underground | `load_network_models`, `load_underground_models` | `network_and_underground_fill_sources_and_owned_list_cleanup_follow_type` |
| 12.7 Common loading | `load_models_common` | `common_result_comes_only_from_first_id_and_prop_flags_and_bytes_apply_to_later_models`, `common_building_first_model_notifies_development_and_flora_sets_flags`, `common_other_types_leave_instance_flags_untouched` |
| 12.8 Foundations | `load_foundation_models` | `foundation_copies_optional_arrays_defaults_and_box_or_uses_common_with_y_offset` |
| 12.9 Placeholder | `create_placeholder` | `placeholder_uses_cell_differences_all_size_classes_and_c_remainder` |
| 12.10 Transform | `transform::{find_model_transform,Transform}` | `transform_offset_quirk_negative_zeros_and_counter_wrap`, `tilt_normalises_objects_limits_only_positive_axes_and_prefers_objects` |
| 12.11 AddModel | `add_model` | `add_model_fallback_mutates_key_and_size_cut_does_not_make_a_placeholder`, `animated_class_forces_flip_sets_mirrored_view_and_keeps_only_flag_one`, `ignore_missing_data_is_overridden_by_exemplar_and_missing_holder_releases_all_refs` |
| 12.12 Shadows | `create_occupant_shadow`, `Transform::rotate_shadow`, `Bounds::transformed` | `shadow_prop_gates_then_searches_lower_zoom_and_filters_materials_and_overlay_ids`, `shadow_size_cut_stops_search_and_detail_five_uses_transformed_model_bounds`, `shadow_rotation_uses_binary32_angle_and_changes_columns_once` |
| 13 Terrain hiding | `hide_occupant_terrain`, `hide_terrain_cells`, `TerrainHost` | §13.2 type worked rules: `network_base_texture_and_pipe_bits_preserve_high_flags`; §13.1 clipping/mode: `foundation_mode_rebuilds_flags_and_refreshes_end_exclusive_clipped_rect` |
| 14 AddModels | Pending | M28 pending |
| 15 Background loader | Separate `model_background_loader` module | See its README; **open MM1** is an explicit Unsupported boundary |
| 19 Conformance cases | Tests named M1–M4/M13–M16/M22 and independent type rules above | Other cases are pending, not credited |
| 20 Function inventory | Semantic Rust entry points above | Native addresses/layouts and unimplemented inventory entries are not exported |

Invalid zooms are rejected/ignored with SPEC-GAP comments; malformed image
descriptors return `TintError`. The required host image/spiral/timer services
remain interfaces, without drawing, window, simulator or operating-system calls.
Model construction reports `LoadError::MissingHolder` after cleanup where the
native code dereferences an absent holder. Returned AddModel handles borrow from
the holder; resource, interface and animation temporaries carry explicit counts.
§12.12's written exclusion gate excludes flora before its orientation fallback.

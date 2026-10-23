# Lighting core

Clean-room implementation of [lighting.md](../../../../docs/specs/lighting.md),
covering `cSC4LightingManager`, `cImageARGB8` and the attributed helpers. There is
no plugin or C ABI integration. `Terrain`, `Resources`/`TexturePixels`,
`PropertyHolder` and `EventSink` carry foreign inputs and notifications only;
all lighting calculations stay here. `LightingStatics` shares persistent
properties across managers using an `RwLock` (getters are `Send + Sync`).

## Owner closure

Code names below are in this directory. Test names without a module prefix are
in `tests.rs`; `contracts::` refers to `contracts.rs`. Rows cover all numbered
behavior sections, including the individual inventory in §12. Sections that
only describe users, interfaces or outstanding interpretation are identified
as such rather than treated as unimplemented algorithms.

| Owner | Spec section | Code item | Test / remaining limit |
|---|---|---|---|
| cSC4LightingManager | §0 summary | `LightingManager`; `colors`, `shadows`, `projection` modules | §11 mapping below |
| cSC4LightingManager | §1.1 identity | `CLASS_ID`, `query_interface`, `add_ref`, `release`, `new` | `lifecycle_subscription_order_identity_and_owned_references` |
| cSC4LightingManager | §1.2 state / constructor | `new`, `query_interface`; explicit safe empty geometry/maps for native unwritten storage | `contracts::native_initial_state_shared_defaults_and_query_counts`, `contracts::constructor_zero_map_rotation_and_shutdown_retained_matrix` |
| cSC4LightingManager | §1.3 Init / Shutdown / destruction | `init`, `shutdown`, `destroy`, `deleting_destructor`, `Drop` | `lifecycle_subscription_order_identity_and_owned_references`, `deleting_destructor_always_drops_resources_but_frees_storage_only_for_bit_zero` |
| cSC4LightingManager | §1.4 properties | `Parameters::initial/read`, `LightingStatics`, `read_properties` | `persistent_properties_override_every_time_and_rebuild_keys`, `contracts::native_initial_state_shared_defaults_and_query_counts` |
| cSC4LightingManager | §1.5 maps and lifetime | `init`, `shutdown`, `destroy`, `create_color_map`, `ImageArgb8::copy_from/make_test_pattern` | `resource_copy_lifetime_and_partial_failure_are_owned`, `contracts::vector14_missing_maps_load_once_and_shutdown_is_explicit` |
| cImageARGB8 | §1.5 storage / minimum / helpers | `new`, `find_min_red_component_value`, `copy_from`, `make_test_pattern`, `create_color_map` | `image_sampling_stride_row_spill_minimum_and_test_pattern`, `contracts::vector11_minimum_scans_vector_end_not_capacity_or_active_width`, `resource_copy_lifetime_and_partial_failure_are_owned` |
| cSC4LightingManager | §2 time | `get_time_of_day`, `is_night`, `set_time_of_day` | `conformance_1_day_keys_and_wrap_month`, `conformance_2_threshold_and_untinted_night_announcements` |
| cSC4LightingManager | §2.1 time-dependent values | `set_time_dependent_lighting_values`, `scalar_key`, `color_key`, `key_position` | `conformance_1_day_keys_and_wrap_month`, `time_maps_use_month_center_and_no_terrain_never_announces_or_changes_night`, `suppressed_fade_still_tints_and_does_not_post_dawn_dusk` |
| cImageARGB8 | §2.1 row interpolation | `sample`, `color_linear` | `image_sampling_stride_row_spill_minimum_and_test_pattern` |
| cSC4LightingManager | §2.2 time / RGB discriminator / fractions | `set_time_of_day`, `register::Number` | `conformance_2_threshold_and_untinted_night_announcements`, `conformance_3_day_night_fractions`, `contracts::lt6_length_equality_nan_and_sub_binary32_distance`; open: LT12 for unproved inherited modes |
| cSC4LightingManager | §2.3 tint | `set_lighting_tint`, `reset_lighting_tint` | `tint_reset_recomputes_light_and_redraws_only_above_announced_threshold` |
| cSC4LightingManager | §3 view | `do_zoom_and_rotation_change`, `rotate`, `grid_index`, `map_normal` | `all_grid_rotations_copy_world_heights_and_rotate_changed_rectangles`, `fade_gradient_stays_at_view_change_and_vertices_skip_it_without_shadows`, `conformance_6_vertex_down_is_up_and_strides_are_respected` |
| cSC4LightingManager | §4.1 surface colour and six overloads | `get_color`, `get_color_up`, `get_color_bytes`, `get_color_up_bytes`, `get_color_at`, `get_color_at_bytes` | `conformance_4_lambert_and_rotations`, `point_terrain_and_vertex_color_paths_keep_distinct_fade_shadow_rules`, `surface_map_size_drives_model_map_coordinates_and_rotation` |
| cSC4LightingManager | §4.2 model colour | `get_model_color`, `ImageArgb8::normal_pixel/pixel_color` | `surface_map_size_drives_model_map_coordinates_and_rotation` |
| cSC4LightingManager | §4.3 model light | `get_model_light`, `Occupant`, `Bounds::center` | `conformance_5_model_weights_and_network_priority`, `model_normal_uses_center_or_four_corners_without_renormalizing_average` |
| cSC4LightingManager | §4.4 model vertex colours | `get_vertex_colors`, `read_normal`, `check_stream` | `conformance_6_vertex_down_is_up_and_strides_are_respected`, `known_upper_clamp_and_invalid_buffers_have_checked_boundaries` |
| cSC4LightingManager | §4.5 terrain colours | `get_terrain_lighting`, `get_terrain_color`, `get_terrain_color_floor`, `get_terrain_vertex_colors`, `fade_term` | `point_terrain_and_vertex_color_paths_keep_distinct_fade_shadow_rules`, `fade_gradient_stays_at_view_change_and_vertices_skip_it_without_shadows` |
| cSC4LightingManager | §4.6 getters | `get_global_light`, `get_shadow_direction`, `get_time_of_day`, `is_night` | `lifecycle_subscription_order_identity_and_owned_references`, `conformance_2_threshold_and_untinted_night_announcements`, `all_grid_rotations_copy_world_heights_and_rotate_changed_rectangles` |
| cSC4LightingManager | §4.7 bytes | `color_byte`, `color_bytes` | `conformance_9_bytes_and_ties` |
| cSC4LightingManager | §5 hill shadows | `shadows`, `projection` modules | §5.1–§5.8.4 rows below |
| cSC4LightingManager | §5.1 grids | `do_message(CityStarted)`, `grid_index`, `location_index`, `copy_heights` | `all_grid_rotations_copy_world_heights_and_rotate_changed_rectangles`, `contracts::vector15_rectangular_sweep_counts_do_not_change_with_rotation` |
| cSC4LightingManager | §5.2 sweep | `find_hill_shadows` | `conformance_8_peak_sweep_and_skipped_rect`, `sweep_drift_both_directions_uses_ordered_in_place_interpolation`, `contracts::vector15_rectangular_sweep_counts_do_not_change_with_rotation` |
| cSC4LightingManager | §5.3 depth and bent normal | `shadow_depth`, `height_above_land`, `bend_shadowed_normal`, `shadowed_normal` | `conformance_8_peak_sweep_and_skipped_rect`, `point_terrain_and_vertex_color_paths_keep_distinct_fade_shadow_rules` |
| cSC4LightingManager | §5.4 strength | `get_shadow_strength_from_location` | `conformance_7_shadow_strength` |
| cSC4LightingManager | §5.5 terrain normal | `terrain_normal` | `contracts::vector12_upper_epsilon_is_stored_only_at_forwarding_boundary` |
| cSC4LightingManager | §5.6 parameters and projections | `get_shadow_params`, `get_decal_shadow_params`, `calc_shadow_terrain_rect`, `calc_shadow_projection` | `decal_holder_overrides_only_present_properties`, §5.7–§5.8.4 tests below |
| cSC4LightingManager | §5.7 exact terrain rectangle | `calc_shadow_terrain_rect` ordered probe loop, `native_endpoint` | `contracts::vector17_terrain_probes_keep_stopped_vertex`, `contracts::terrain_probe_signs_integer_ties_and_zero_component_nan_stop`; open: LT12 for unproved inherited modes |
| cSC4LightingManager | §5.8 projection and basis | `calc_shadow_projection`, `basis`, `fit` | `native_fit_cutoff_and_empty_projection_preserve_output`, `contracts::vector18_vertical_fallback_is_fixed_even_for_downward_and_zero_direction`, `contracts::nonfinite_determinant_proceeds_zero_scale_and_literal_output_row`; open: LT12, LT13 |
| cSC4LightingManager | §5.8.1 direction | `model_basis`, `calc_shadow_projection` conditional transpose | `contracts::vector19_direction_transpose_negation_no_normalization_or_scale` |
| cSC4LightingManager | §5.8.2 arithmetic / plane coordinates | `register::Number`, `basis`, `calc_shadow_projection` | `contracts::vector19_direction_transpose_negation_no_normalization_or_scale`, `contracts::vector21_native_product_spill_rejects_noncollinear_triangle`; open: LT12, LT13 |
| cSC4LightingManager | §5.8.3 mixed fit | `fit`, `fit_shadow_plane` | `contracts::vector21_native_product_spill_rejects_noncollinear_triangle`, `contracts::vector22_u_v_translation_spills_differ_with_identical_uv_columns`; open: LT12, LT13 |
| cSC4LightingManager | §5.8.4 inverse / composition / layout | `calc_shadow_projection` | `contracts::vector20_whole_byte_flags_inverse_translation_and_scale`, `contracts::independent_noncommuting_shears_preserve_composition_order`, `contracts::vector22_u_v_translation_spills_differ_with_identical_uv_columns`; open: LT12, LT13 |
| cSC4LightingManager | §6 messages | `LightingMessage`, `do_message`, `update_terrain`, `changed_world_rect` | `long_operations_accumulate_dirty_vertices_then_sweep_and_redisplay`, `render_change_updates_threshold_and_only_changed_shadow_setting_emits`, `contracts::vector16_copy_vertices_then_transformed_callbacks_and_no_change_silence` |
| cSC4LightingManager | §7 slot table / message thunks | Typed methods in the preceding rows; both interfaces use one manager and reference count | `lifecycle_subscription_order_identity_and_owned_references`; physical pointer adjustment belongs to future glue, outside the portable core |
| cSC4LightingManager | §8 users | Foreign consumers only; no additional owner algorithm | No core wiring requested |
| cSC4LightingManager | §9 quirks | `sample`, view cache, `get_model_color`, fade paths, render-change handler, pattern and sweep | Corresponding §1.5, §2.1, §3, §4.2, §4.5, §5.2 and §6 tests above |
| cSC4LightingManager | §10 open questions | Raw `suppress_fade`, `network_flag_40000`, fade and decal values preserve specified behavior | LT1–LT4 concern names/foreign consumers; no missing owner calculation. Numeric limits: open LT12, LT13 |
| cSC4LightingManager | §11 conformance | Owner methods above | Explicit vector mapping below; LT12/LT13 limits retained |
| cImageARGB8 | §11 conformance | `sample`, `find_min_red_component_value`, map helpers | Vectors 11 and 14 below; row-spill test |
| cSC4LightingManager | §12 individual inventory / attributed helpers | All 43 entries map to methods above; `destroy`/`Drop` and `deleting_destructor` cover both destructors, typed interfaces share QI/Release; `changed_world_rect`, image helpers, `rotate`, `basis` cover attributed helpers | Section tests above; no same-owner fallback; exact numeric closure remains open only under LT12/LT13 |
| cImageARGB8 | §12 individual inventory | `sample` and `find_min_red_component_value` cover both bodies | Image section tests above |

### Conformance tests (§11)

| Vector | Test |
|---|---|
| 1–9 | `tests::conformance_1_day_keys_and_wrap_month` through `tests::conformance_9_bytes_and_ties` (one named test per vector) |
| 10 | `contracts::lt6_length_equality_nan_and_sub_binary32_distance` |
| 11 | `contracts::vector11_minimum_scans_vector_end_not_capacity_or_active_width` |
| 12 | `contracts::vector12_upper_epsilon_is_stored_only_at_forwarding_boundary` |
| 13 | `contracts::native_initial_state_shared_defaults_and_query_counts`, `contracts::constructor_zero_map_rotation_and_shutdown_retained_matrix`; native unwritten storage and pointer offsets are represented by safe empty values and typed interfaces |
| 14 | `contracts::vector14_missing_maps_load_once_and_shutdown_is_explicit`, `resource_copy_lifetime_and_partial_failure_are_owned` |
| 15 | `contracts::vector15_rectangular_sweep_counts_do_not_change_with_rotation` |
| 16 | `contracts::vector16_copy_vertices_then_transformed_callbacks_and_no_change_silence` |
| 17 | `contracts::vector17_terrain_probes_keep_stopped_vertex`, `contracts::terrain_probe_signs_integer_ties_and_zero_component_nan_stop` |
| 18 | `native_fit_cutoff_and_empty_projection_preserve_output`, `contracts::vector18_vertical_fallback_is_fixed_even_for_downward_and_zero_direction` |
| 19 | `contracts::vector19_direction_transpose_negation_no_normalization_or_scale` |
| 20 | `contracts::vector20_whole_byte_flags_inverse_translation_and_scale` |
| 21 | `contracts::vector21_native_product_spill_rejects_noncollinear_triangle` |
| 22 | `contracts::vector22_u_v_translation_spills_differ_with_identical_uv_columns` |

### Open exactness limits and safe boundaries

- **LT12:** inherited rounding mode/control word on actual game entry is unsettled.
  `LightingManager::new` requires explicit 24/53/64-bit `RegisterPrecision`;
  the register arithmetic models nearest-even and masked exceptions. Other
  rounding modes and exception signaling are unsupported. No default game mode
  is inferred. This keeps exact closure open for §2.2, §5.7 and §5.8.2–§5.8.4.
- **LT13:** NaN payload/sign and signaling exception encoding is unsettled.
  The unordered determinant proceeds and propagates quiet NaNs, but exact NaN
  output bits are unsupported. This keeps bit-exact closure open for §5.8.
- LT1–LT4 do not block the specified computations: the raw setting/network
  answers are inputs; fade/decal values are returned without inferred meanings.
- Invalid foreign image output latches interrupted Init rather than inventing
  recovery. Missing images are successful null-map loads. Shutdown is explicit;
  destruction intentionally does not free maps or unsubscribe, matching §1.5.
- Checked errors for invalid storage, strides, time/month and grid indices are
  portable safety policies, not additional native error contracts. Native
  unwritten constructor fields are not reproduced with unsafe/uninitialized
  Rust values. Valid linear row spill and mismatched model-map addressing are
  preserved whenever storage exists.

The questions are tracked in [questions/lighting.md](../../../../docs/specs/questions/lighting.md).
No new spec ambiguity was introduced by this audit. No game measurement or
replacement coverage is claimed by these core tests.

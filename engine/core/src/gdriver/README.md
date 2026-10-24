# DirectX 7 driver core

Pure Rust implementation against `renderer-driver.md`. `backend::Device` extends
the existing director `Host`, preserving its 32-bit identity, memory and reference
contracts. The director continues to dispatch its foreign DX7 factory; no plugin,
C ABI, Windows API or registration is changed here. Caller adapters may associate
the returned foreign identity with `Driver`.

Test fixtures supply primitive foreign-operation results, not converter or capture
algorithms. All settled owner contracts, including RD6/RD7/RD12, execute in this
module. RD1/RD3/RD4 retain only the external proof boundaries identified in the spec.
Matrix projection uses a portable 64-bit significand accumulator to preserve the
specified x87 rounding; no host floating-point rounding mode is changed.

## Owner closure

Unqualified owner names below belong to `nSGLDX7`. "Implemented" means the
specified core contract is covered; it does not claim native backend integration.

| Owner | Spec section | Code item | Test / proof boundary |
|---|---|---|---|
| cGDriver | 1 scope | Driver, Device: Host | dx7_01_02_04_noops_and_identity |
| cGDriver | 2 construction | Driver::new, IDENTITY | implemented: dx7_25_26_constructor_and_default_reset_are_distinct |
| cTextureManager | 2 construction | TextureManager::default/destroy | implemented: dx7_07_08_follow_parameters_and_lifo, manager_destructors_do_not_submit |
| cPrimitiveManager | 2 construction | PrimitiveManager::default/destroy | implemented: primitive_copy_merge_immediate_overtake_and_failure_paths, manager_destructors_do_not_submit |
| cGDriver | 3 error | Driver::hr/get_error | dx7_17_sticky_hresult_channel |
| cGDriver | 4.1 init/shutdown/destruction | Driver::init/shutdown/build_info/file_version/release, DRIVER_INFO | implemented: lifecycle_initialization_info_noop_and_known_light_fields, dx7_27_shutdown_does_not_assign_error_from_mode_list_end, dx7_31_final_reference_manager_ownership_without_submission, adapter_version_search_and_shared_information |
| cGDriver | 4.2 mode lifecycle | Driver::set_video_mode | dx7_18_failed_setup_writes_fourth_word_after_cleanup, null_setup_failure_keeps_native_hazard_and_cleans_window; RD1 caller intent remains external |
| cGDriver | 4.3 setup | Driver::setup/bind_owner_surfaces, Context::create | implemented: dx7_15_modes_and_descriptor_enrichment, dx7_28_device_ranking_sentinel_and_selectors, dx7_29_30_first_depth_format_and_terminal_failures, dx7_61_owner_surfaces_setup_loss_recovery_order |
| cGDriver | 4.4 scenes/loss | Driver::begin_scene/flush, Context::restore/rebuild | implemented: dx7_03_09_11_loss_restore_metadata_and_defaults, context_rebuild_releases_previous_surfaces, dx7_61_owner_surfaces_setup_loss_recovery_order, dx7_62_aliased_fallback_failed_recovery_and_cleanup |
| cGDriver | 4.5 teardown | Driver::destroy_viewport | initialization_failures_and_shutdown_preserve_specified_cleanup, loss_placeholder_is_owned_and_released_on_restore |
| cGDriver | 5.1 enumerations | compare/blend/stencil, Driver::state | immediate_states_capabilities_fog_and_multipliers |
| cGDriver | 5.2 states | Driver::state/clear_depth | immediate_states_capabilities_fog_and_multipliers, defaults_clear_quantization_and_stencil_failures |
| cGDriver | 5.3 clear | Driver::clear | defaults_clear_quantization_and_stencil_failures |
| cGDriver | 5.4 capabilities | Driver::capability/is_enabled | enable_preserves_error_across_failed_queued_draw_and_device_state, disable_replaces_error_with_state_result_after_failed_queued_draw, capability_deferred_and_ignored_paths_preserve_queue_and_error |
| cGDriver | 5.5 colour | backend::quantize/color | defaults_clear_quantization_and_stencil_failures |
| cGDriver | 6.1 mode list | Driver::count_modes | dx7_15_modes_and_descriptor_enrichment, initialization_failures_and_shutdown_preserve_specified_cleanup |
| cGDriver | 6.2 mode records | Mode, Driver::count_modes/setup | dx7_15_modes_and_descriptor_enrichment; RD3 unknown field meanings remain external |
| cGDriver | 6.3 mode getters | Driver::mode_info/current_mode | dx7_15_modes_and_descriptor_enrichment, lifecycle_initialization_info_noop_and_known_light_fields |
| cGDriver | 7 defaults | Driver::default_state | defaults_clear_quantization_and_stencil_failures |
| cGDriver | 8.1 arrays | Driver::interleaved_arrays | primitive_copy_merge_immediate_overtake_and_failure_paths |
| cGDriver | 8.2 drawing | Driver::draw_arrays/draw_elements, primitive::expanded_writes | implemented: primitive_copy_merge_immediate_overtake_and_failure_paths, dx7_22_23_incomplete_quad_writes_and_declared_counts |
| cPrimitiveManager | 8.3 batching | PrimitiveManager | implemented: dx7_05_deferred_binding_and_presentation_order, dx7_19_20_compact_writes_preserve_absent_uvs, dx7_21_flush_returns_last_submission_and_resets_scratch_when_stopped |
| cGDriver | 8.4 multipliers/fog | Driver::color_multiplier/alpha_multiplier/enable_vertex_colors/fog_float/fog_int | immediate_states_capabilities_fog_and_multipliers |
| cGDriver | 8.5 vertex formats | vertex module | matrices_viewport_formats_and_draw_patterns |
| cGDriver | 8.6 matrices | Driver::matrix_mode/load_matrix/load_identity, extended::projection | implemented portable finite contract: dx7_24_projection_extended_discriminator, rational_x87_discriminator; no explicit x87-environment mode exposed |
| cTextureManager | 9.1 names | TextureManager::new_name/free/priority | dx7_07_08_follow_parameters_and_lifo |
| cTextureManager | 9.2 loss/lifetime | TextureManager::init/lose/init_device/shutdown | implemented: dx7_03_09_11_loss_restore_metadata_and_defaults, texture_initialization_uploads_white_pixel, dx7_50_argb4_all_ordinary_destinations |
| cTextureManager | 9.3 surfaces | TextureManager::create | dx7_07_08_follow_parameters_and_lifo |
| cTextureManager | 9.4 uploads | TextureManager::load, texture::convert | implemented: dx7_10_texture_upload_flush_boundary, dx7_50_argb4_all_ordinary_destinations, dx7_56_dxt_whole_blocks_ignore_source_stride |
| cGDriver | 9.5 texture slots | Driver texture methods | dx7_07_08_follow_parameters_and_lifo, dx7_10_texture_upload_flush_boundary |
| cTextureManager | 9.6 selection/availability/copy | texture::select_format/converter_available/convert/decode_blocks, Format::alpha_bits | implemented: DX7-39–44 (closure_tests.rs), DX7-50–56 (settled_tests.rs), raw_converter_advances_rows_with_independent_strides |
| cGDriver | 10.1 stage record | Stage::default/reset | implemented: dx7_25_26_constructor_and_default_reset_are_distinct |
| cGDriver | 10.2 stage setters | Driver stage methods, Stage::packed | dx7_06_combiner_cache_and_min_filter_fallthrough, dx7_07_08_follow_parameters_and_lifo, stage_coordinates_matrices_environments_and_capability_gate |
| cGDriver | 10.3 apply | Stage::apply | dx7_06_combiner_cache_and_min_filter_fallthrough, stage_coordinates_matrices_environments_and_capability_gate |
| cGDriver | 11 viewport/offset/blits | Driver::set_viewport/get_viewport/state(67)/blit_unsupported | matrix_modes_unchecked_failures_viewport_reset_and_unsupported_blits, matrices_viewport_formats_and_draw_patterns, dx7_17_sticky_hresult_channel |
| cGDriver | 11.1 scene end | Driver::end_scene | dx7_05_deferred_binding_and_presentation_order |
| cGDriver | 12.1 auxiliary interfaces/light | D3DLight::default | implemented: lifecycle_initialization_info_noop_and_known_light_fields, dx7_32_38_auxiliary_lighting_and_material_vectors |
| cGDriver | 12.4 auxiliary lighting/materials | auxiliary.rs Driver lighting/material methods, D3DLight::bytes | implemented: dx7_32_38_auxiliary_lighting_and_material_vectors |
| cGDriver | 12.5 region handles/transfers | Driver::new_region/delete_region/is_region/transfer_region, Region, capture.rs bindings | implemented: dx7_45_46_region_handles_retry_and_no_error_assignment, dx7_47_48_region_direction_retries_and_restore_guards, dx7_49_partial_save_clears_but_returns_false_after_recovery, dx7_61_owner_surfaces_setup_loss_recovery_order |
| cGDriver | 12.6 CopyColorBuffer | capture.rs Driver::copy_color_buffer/copy_locked_color, copy_pixels/color16 | implemented: DX7-57–60 (settled_tests.rs), covering cached probe, factory/borrowed ownership, clipping, formats and failure exits |
| cGDriver | 12.2 identity/getters/tail | Driver identity/getter methods | dx7_01_02_04_noops_and_identity, lifecycle_initialization_info_noop_and_known_light_fields |
| cGDriver | 12.3 graphics-slot checklist | Driver, vertex module, preceding slot groups | DX7-01–18 plus capability, matrix, stage and immediate-state tests named in this table; no plugin registration |
| cGDriver | 13 order/cache | submit/apply_stages | dx7_05_deferred_binding_and_presentation_order, dx7_06_combiner_cache_and_min_filter_fallthrough, matrix_modes_unchecked_failures_viewport_reset_and_unsupported_blits |
| cPrimitiveManager | 13 order/cache | PrimitiveManager::queue/flush | primitive_copy_merge_immediate_overtake_and_failure_paths, dx7_05_deferred_binding_and_presentation_order |
| cTextureManager | 13 order/cache | TextureManager::lose/init_device/load | dx7_03_09_11_loss_restore_metadata_and_defaults, dx7_10_texture_upload_flush_boundary |
| cPrimitiveManager | 14.1 buffers | VertexBuffer | dx7_16_buffer_reservation_and_reference_order |
| cGDriver | 14.2 context | Context::create/rebuild/restore, context::select_device, capture.rs Driver::bind_owner_surfaces | implemented: dx7_12_flip_fallback, dx7_28_device_ranking_sentinel_and_selectors, dx7_29_30_first_depth_format_and_terminal_failures, context_rebuild_releases_previous_surfaces, DX7-61–62 (settled_tests.rs) |
| cGDriver | 14.3 presentation | Context::present | dx7_13_14_present_retry_and_empty_rect |
| cGDriver | 14.4 descriptor | Driver::setup | dx7_15_modes_and_descriptor_enrichment; RD3 unknown meanings/format outcomes remain external |
| cGDriver | 15 vectors | tests.rs, closure_tests.rs, settled_tests.rs | implemented: DX7-01–62 represented by named tests, including separate scratch writes/declared counts |
| cPrimitiveManager | 15 vectors | tests.rs | dx7_05_deferred_binding_and_presentation_order, dx7_16_buffer_reservation_and_reference_order |
| cTextureManager | 15 vectors | tests.rs, closure_tests.rs, settled_tests.rs | DX7-07–11, DX7-39–44, DX7-50–56 |
| cGDriver | 16 proof boundaries | Driver::copy_color_buffer, bind_owner_surfaces | implemented: DX7-57–62; RD1/RD3/RD4 retain external caller/meaning/handler questions without blocking the specified owner contracts |
| cPrimitiveManager | 16 proof boundaries | PrimitiveManager::queue/flush/destroy, primitive::expanded_writes | implemented; unchecked native out-of-range reads/writes represented by InvalidInput or explicit oversized scratch vectors |
| cTextureManager | 16 proof boundaries | TextureManager::init/load/destroy, texture::convert/decode_blocks | implemented: DX7-50–56; unchecked native invalid pointer/overflow inputs represented by InvalidInput/foreign Fault |
| cTextureManager | 9.6 compressed raw-copy traversal | texture::convert | implemented: raw_converter_advances_rows_with_independent_strides (forward traversal, independent strides, padding) |
| cPrimitive | 8.3 queued records, 13 lifetime/order | primitive::Entry, PrimitiveManager::queue/flush | implemented: primitive_copy_merge_immediate_overtake_and_failure_paths, dx7_21_flush_returns_last_submission_and_resets_scratch_when_stopped |
| cVertexBuffer | 14.1 reservation/reference/loss | primitive::VertexBuffer | implemented: dx7_16_buffer_reservation_and_reference_order, loss_placeholder_is_owned_and_released_on_restore |
| cTextureStageData | 7 defaults, 10.1 record/reset, 10.2 setters, 10.3 application, 13 caches | stage::Stage | implemented: dx7_06_combiner_cache_and_min_filter_fallthrough, dx7_25_26_constructor_and_default_reset_are_distinct, stage_coordinates_matrices_environments_and_capability_gate |
| D3DLight | 12.1 construction, 12.4 fields/submission | D3DLight, auxiliary.rs | implemented: dx7_32_38_auxiliary_lighting_and_material_vectors |
| Presentation context | 14.2 construction/resources/rebuild | context.rs Context, select_device | implemented: dx7_12_flip_fallback, dx7_28_device_ranking_sentinel_and_selectors, dx7_29_30_first_depth_format_and_terminal_failures, context_rebuild_releases_previous_surfaces, DX7-61–62 |
| Presentation context | 14.3 presentation | context.rs Context::present | implemented: dx7_13_14_present_retry_and_empty_rect |
| Placeholder device | 4.4 loss sink | mod.rs Driver::flush, backend.rs Call::PlaceholderDevice | implemented owner contract: loss_placeholder_is_owned_and_released_on_restore; foreign primitive supplies the inactive sink identity |
| Shared vertex-format helpers/copiers | 8.1 arrays, 8.3 copied records, 8.5 vertex formats | vertex.rs, primitive.rs | implemented: matrices_viewport_formats_and_draw_patterns, dx7_19_20_compact_writes_preserve_absent_uvs |

## Remaining boundaries

RD6–RD12 are implemented under the merged spec. Converter arithmetic (§9.6),
CopyColorBuffer (§12.6), and setup/recovery surface ownership (§§4.3–4.4, 14.2)
are covered by DX7-50–62. The externally supplied slot +234 remains unpopulated
by setup/recovery, as specified.
RD1's null-write hazard is represented by `Fault::NullWrite`. RD3's unknown
descriptor meanings and RD4's external loss-message handler do not prevent the
specified descriptor writes and notifications. The portable matrix contract uses
64-bit significands, nearest/even and masked exceptions; no explicit ambient x87
compatibility mode is exposed, as allowed by §8.6.

Owner status: cPrimitiveManager/cPrimitive, cVertexBuffer, cTextureStageData,
D3DLight and the presentation context implement all specified behavior.
cTextureManager and cGDriver implement every settled contract, including RD6,
RD7 and RD12. No plugin/C ABI registration is added.

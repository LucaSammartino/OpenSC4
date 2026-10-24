# DirectX 7 driver core

Pure Rust implementation against `renderer-driver.md`. `backend::Device` extends
the existing director `Host`, preserving its 32-bit identity, memory and reference
contracts. The director continues to dispatch its foreign DX7 factory; no plugin,
C ABI, Windows API or registration is changed here. Caller adapters may associate
the returned foreign identity with `Driver`.

Test fixtures supply explicit answers at open boundaries; they are not evidence
for native behavior. `Unsupported` identifies the relevant question instead of
inventing a native HRESULT. Open sections are listed below. The backend must
provide its own answers for unsettled converter cells and CopyColorBuffer.
Matrix projection uses a portable 64-bit significand accumulator to preserve the
specified x87 rounding; no host floating-point rounding mode is changed.

## Owner closure

Unqualified owner names below belong to `nSGLDX7`. "Implemented" means the
specified core contract is covered; it does not claim native backend integration.

| Owner | Spec section | Code item | Test / open boundary |
|---|---|---|---|
| cGDriver | 1 scope | Driver, Device: Host | dx7_01_02_04_noops_and_identity |
| cGDriver | 2 construction | Driver::new, IDENTITY | implemented: dx7_25_26_constructor_and_default_reset_are_distinct |
| cTextureManager | 2 construction | TextureManager::default/destroy | implemented: dx7_07_08_follow_parameters_and_lifo, manager_destructors_do_not_submit |
| cPrimitiveManager | 2 construction | PrimitiveManager::default/destroy | implemented: primitive_copy_merge_immediate_overtake_and_failure_paths, manager_destructors_do_not_submit |
| cGDriver | 3 error | Driver::hr/get_error | dx7_17_sticky_hresult_channel |
| cGDriver | 4.1 init/shutdown/destruction | Driver::init/shutdown/build_info/file_version/release, DRIVER_INFO | implemented: lifecycle_initialization_info_noop_and_known_light_fields, dx7_27_shutdown_does_not_assign_error_from_mode_list_end, dx7_31_final_reference_manager_ownership_without_submission, adapter_version_search_and_shared_information |
| cGDriver | 4.2 mode lifecycle | Driver::set_video_mode | dx7_18_failed_setup_writes_fourth_word_after_cleanup, null_setup_failure_keeps_native_hazard_and_cleans_window; RD1 caller intent remains external |
| cGDriver | 4.3 setup | Driver::setup, Context::create | implemented: dx7_15_modes_and_descriptor_enrichment, dx7_28_device_ranking_sentinel_and_selectors, dx7_29_30_first_depth_format_and_terminal_failures; open: RD12 auxiliary surface aliases |
| cGDriver | 4.4 scenes/loss | Driver::begin_scene/flush, Context::restore/rebuild | implemented: dx7_03_09_11_loss_restore_metadata_and_defaults, context_rebuild_releases_previous_surfaces; open: RD12 auxiliary surface replacement |
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
| cTextureManager | 9.2 loss/lifetime | TextureManager::init/lose/init_device/shutdown | implemented: dx7_03_09_11_loss_restore_metadata_and_defaults, texture_initialization_uploads_white_pixel; open: RD6 only when selected white-pixel converter arithmetic is unsettled |
| cTextureManager | 9.3 surfaces | TextureManager::create | dx7_07_08_follow_parameters_and_lifo |
| cTextureManager | 9.4 uploads | TextureManager::load | dx7_10_texture_upload_flush_boundary; open: RD6 arithmetic |
| cGDriver | 9.5 texture slots | Driver texture methods | dx7_07_08_follow_parameters_and_lifo, dx7_10_texture_upload_flush_boundary |
| cTextureManager | 9.6 selection/availability/copy | texture::select_format/converter_available/convert, Format::alpha_bits | implemented: dx7_39_40_ordinary_format_ranks_and_stable_ties, dx7_41_42_empty_formats_dxt5_fallback_and_all_availability_cells, dx7_43_pixel_expansions_and_independent_padding, dx7_44_linear_copy_ignores_dimensions_and_nonzero_offset_copy_result; open: RD6 other non-copy arithmetic |
| cGDriver | 10.1 stage record | Stage::default/reset | implemented: dx7_25_26_constructor_and_default_reset_are_distinct |
| cGDriver | 10.2 stage setters | Driver stage methods, Stage::packed | dx7_06_combiner_cache_and_min_filter_fallthrough, dx7_07_08_follow_parameters_and_lifo, stage_coordinates_matrices_environments_and_capability_gate |
| cGDriver | 10.3 apply | Stage::apply | dx7_06_combiner_cache_and_min_filter_fallthrough, stage_coordinates_matrices_environments_and_capability_gate |
| cGDriver | 11 viewport/offset/blits | Driver::set_viewport/get_viewport/state(67)/blit_unsupported | matrix_modes_unchecked_failures_viewport_reset_and_unsupported_blits, matrices_viewport_formats_and_draw_patterns, dx7_17_sticky_hresult_channel |
| cGDriver | 11.1 scene end | Driver::end_scene | dx7_05_deferred_binding_and_presentation_order |
| cGDriver | 12.1 auxiliary interfaces/light | D3DLight::default | implemented: lifecycle_initialization_info_noop_and_known_light_fields, dx7_32_38_auxiliary_lighting_and_material_vectors |
| cGDriver | 12.4 auxiliary lighting/materials | auxiliary.rs Driver lighting/material methods, D3DLight::bytes | implemented: dx7_32_38_auxiliary_lighting_and_material_vectors |
| cGDriver | 12.5 region handles/transfers | Driver::new_region/delete_region/is_region/transfer_region, Region | implemented against exact owner slots: dx7_45_46_region_handles_retry_and_no_error_assignment, dx7_47_48_region_direction_retries_and_restore_guards, dx7_49_partial_save_clears_but_returns_false_after_recovery; open: RD12 context alias population |
| cGDriver | 12.5 CopyColorBuffer | Driver::copy_color_buffer | open: RD7; Unsupported |
| cGDriver | 12.2 identity/getters/tail | Driver identity/getter methods | dx7_01_02_04_noops_and_identity, lifecycle_initialization_info_noop_and_known_light_fields |
| cGDriver | 12.3 graphics-slot checklist | Driver, vertex module, preceding slot groups | DX7-01–18 plus capability, matrix, stage and immediate-state tests named in this table; no plugin registration |
| cGDriver | 13 order/cache | submit/apply_stages | dx7_05_deferred_binding_and_presentation_order, dx7_06_combiner_cache_and_min_filter_fallthrough, matrix_modes_unchecked_failures_viewport_reset_and_unsupported_blits |
| cPrimitiveManager | 13 order/cache | PrimitiveManager::queue/flush | primitive_copy_merge_immediate_overtake_and_failure_paths, dx7_05_deferred_binding_and_presentation_order |
| cTextureManager | 13 order/cache | TextureManager::lose/init_device/load | dx7_03_09_11_loss_restore_metadata_and_defaults, dx7_10_texture_upload_flush_boundary |
| cPrimitiveManager | 14.1 buffers | VertexBuffer | dx7_16_buffer_reservation_and_reference_order |
| cGDriver | 14.2 context | Context::create/rebuild/restore, context::select_device | implemented: dx7_12_flip_fallback, dx7_28_device_ranking_sentinel_and_selectors, dx7_29_30_first_depth_format_and_terminal_failures, context_rebuild_releases_previous_surfaces |
| cGDriver | 14.3 presentation | Context::present | dx7_13_14_present_retry_and_empty_rect |
| cGDriver | 14.4 descriptor | Driver::setup | dx7_15_modes_and_descriptor_enrichment; RD3 unknown meanings/format outcomes remain external |
| cGDriver | 15 vectors | tests.rs, closure_tests.rs | implemented: DX7-01–49 represented by named tests, including separate scratch writes/declared counts |
| cPrimitiveManager | 15 vectors | tests.rs | dx7_05_deferred_binding_and_presentation_order, dx7_16_buffer_reservation_and_reference_order |
| cTextureManager | 15 vectors | tests.rs | dx7_07_08_follow_parameters_and_lifo, dx7_03_09_11_loss_restore_metadata_and_defaults, dx7_10_texture_upload_flush_boundary |
| cGDriver | 16 proof boundaries | Driver::copy_color_buffer, auxiliary_surfaces | open: RD7 CopyColorBuffer, RD12 context-to-owner aliases; RD1/RD3/RD4 retain external caller/meaning/handler questions |
| cPrimitiveManager | 16 proof boundaries | PrimitiveManager::queue/flush/destroy, primitive::expanded_writes | implemented; unchecked native out-of-range reads/writes represented by InvalidInput or explicit oversized scratch vectors |
| cTextureManager | 16 proof boundaries | TextureManager::init/load/destroy, texture::convert | open: RD6 remaining non-copy arithmetic only |
| cTextureManager | 9.6 compressed raw-copy traversal | texture::convert | implemented: raw_converter_advances_rows_with_independent_strides (forward traversal, independent strides, padding) |
| cPrimitive | 8.3 queued records, 13 lifetime/order | primitive::Entry, PrimitiveManager::queue/flush | implemented: primitive_copy_merge_immediate_overtake_and_failure_paths, dx7_21_flush_returns_last_submission_and_resets_scratch_when_stopped |
| cVertexBuffer | 14.1 reservation/reference/loss | primitive::VertexBuffer | implemented: dx7_16_buffer_reservation_and_reference_order, loss_placeholder_is_owned_and_released_on_restore |
| cTextureStageData | 10.1 record/reset, 10.2 setters, 10.3 application, 13 caches | stage::Stage | implemented: dx7_06_combiner_cache_and_min_filter_fallthrough, dx7_25_26_constructor_and_default_reset_are_distinct, stage_coordinates_matrices_environments_and_capability_gate |
| D3DLight | 12.1 construction, 12.4 fields/submission | D3DLight, auxiliary.rs | implemented: dx7_32_38_auxiliary_lighting_and_material_vectors |

## Remaining boundaries

RD8–RD11 are implemented under the merged spec. RD6 retains only the explicitly
unsettled non-copy arithmetic (§9.6); RD7 retains only CopyColorBuffer (§12.5).
RD12 asks for the ownership/alias mapping between the context's surfaces and
the three exact driver slots used by region methods. Those slots are explicit
inputs to this core; setup/restore does not guess their aliases.
RD1's null-write hazard is represented by `Fault::NullWrite`. RD3's unknown
descriptor meanings and RD4's external loss-message handler do not prevent the
specified descriptor writes and notifications. The portable matrix contract uses
64-bit significands, nearest/even and masked exceptions; no explicit ambient x87
compatibility mode is exposed, as allowed by §8.6.

Owner status: cPrimitiveManager/cPrimitive, cVertexBuffer, cTextureStageData,
D3DLight and the presentation context implement all specified behavior.
cTextureManager implements every settled contract, with RD6 explicitly open.
cGDriver implements every settled graphics/auxiliary body, with RD7 and RD12
explicitly open. No plugin/C ABI registration is added.

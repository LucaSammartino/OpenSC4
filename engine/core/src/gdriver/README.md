# DirectX 7 driver core

Pure Rust implementation against `renderer-driver.md`. `backend::Device` extends
the existing director `Host`, preserving its 32-bit identity, memory and reference
contracts. The director continues to dispatch its foreign DX7 factory; no plugin,
C ABI, Windows API or registration is changed here. Caller adapters may associate
the returned foreign identity with `Driver`.

Test fixtures supply explicit answers at open boundaries; they are not evidence
for native behavior. `Unsupported` identifies the relevant question instead of
inventing a native HRESULT. Open sections are listed below. The backend must
provide its own answers to those boundaries before production mode setup can run.
Matrix projection uses a portable 64-bit significand accumulator to preserve the
specified x87 rounding; no host floating-point rounding mode is changed.

## Owner closure

| Owner | Spec section | Code item | Test / open boundary |
|---|---|---|---|
| cGDriver | 1 scope | Driver, Device: Host | dx7_01_02_04_noops_and_identity |
| cGDriver | 2 construction | Driver::new, IDENTITY | dx7_01_02_04_noops_and_identity; open: RD9 initial material/clear values and final destruction |
| cTextureManager | 2 construction | TextureManager::default | dx7_07_08_follow_parameters_and_lifo; open: RD9 final destruction |
| cPrimitiveManager | 2 construction | PrimitiveManager::default | primitive_copy_merge_immediate_overtake_and_failure_paths; open: RD9 final destruction |
| cGDriver | 3 error | Driver::hr/get_error | dx7_17_sticky_hresult_channel |
| cGDriver | 4.1 init/shutdown | Driver::init/shutdown/build_info, init_word | lifecycle_initialization_info_noop_and_known_light_fields, initialization_failures_and_shutdown_preserve_specified_cleanup; open: RD9 unnamed helper and info fallback; RD2 word purpose |
| cGDriver | 4.2 mode lifecycle | Driver::set_video_mode | dx7_18_failed_setup_writes_fourth_word_after_cleanup, null_setup_failure_keeps_native_hazard_and_cleans_window; RD1 caller intent remains external |
| cGDriver | 4.3 setup | Driver::setup | dx7_15_modes_and_descriptor_enrichment; open: RD8 selection |
| cGDriver | 4.4 scenes/loss | Driver::begin_scene/flush | dx7_03_09_11_loss_restore_metadata_and_defaults; open: RD9 stage defaults |
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
| cGDriver | 8.2 drawing | Driver::draw_arrays/draw_elements, primitive::expanded | primitive_copy_merge_immediate_overtake_and_failure_paths, matrices_viewport_formats_and_draw_patterns; open: RD10 incomplete quads |
| cPrimitiveManager | 8.3 batching | PrimitiveManager | dx7_05_deferred_binding_and_presentation_order; open: RD10 copier defaults |
| cGDriver | 8.4 multipliers/fog | Driver::color_multiplier/alpha_multiplier/enable_vertex_colors/fog_float/fog_int | immediate_states_capabilities_fog_and_multipliers |
| cGDriver | 8.5 vertex formats | vertex module | matrices_viewport_formats_and_draw_patterns |
| cGDriver | 8.6 matrices | Driver::matrix_mode/load_matrix/load_identity, extended::projection | matrix_modes_unchecked_failures_viewport_reset_and_unsupported_blits, rational_x87_discriminator; open: RD11 exceptional FPU behavior |
| cTextureManager | 9.1 names | TextureManager::new_name/free/priority | dx7_07_08_follow_parameters_and_lifo |
| cTextureManager | 9.2 loss/lifetime | TextureManager::init/lose/init_device/shutdown | dx7_03_09_11_loss_restore_metadata_and_defaults; open: RD6 conversion |
| cTextureManager | 9.3 surfaces | TextureManager::create | dx7_07_08_follow_parameters_and_lifo |
| cTextureManager | 9.4 uploads | TextureManager::load | dx7_10_texture_upload_flush_boundary; open: RD6 arithmetic |
| cGDriver | 9.5 texture slots | Driver texture methods | dx7_07_08_follow_parameters_and_lifo, dx7_10_texture_upload_flush_boundary |
| cTextureManager | 9.6 selection | texture::select_format | compressed_descriptor_preference_and_ordinary_fallback; open: RD6 ordinary ranking/conversion |
| cGDriver | 10.1 stage record | Stage::reset | dx7_03_09_11_loss_restore_metadata_and_defaults; open: RD9 constructor/reset defaults |
| cGDriver | 10.2 stage setters | Driver stage methods, Stage::packed | dx7_06_combiner_cache_and_min_filter_fallthrough, dx7_07_08_follow_parameters_and_lifo, stage_coordinates_matrices_environments_and_capability_gate |
| cGDriver | 10.3 apply | Stage::apply | dx7_06_combiner_cache_and_min_filter_fallthrough, stage_coordinates_matrices_environments_and_capability_gate |
| cGDriver | 11 viewport/offset/blits | Driver::set_viewport/get_viewport/state(67)/blit_unsupported | matrix_modes_unchecked_failures_viewport_reset_and_unsupported_blits, matrices_viewport_formats_and_draw_patterns, dx7_17_sticky_hresult_channel |
| cGDriver | 11.1 scene end | Driver::end_scene | dx7_05_deferred_binding_and_presentation_order |
| cGDriver | 12.1 auxiliary interfaces/light | D3DLight::default, Driver::auxiliary_operation | lifecycle_initialization_info_noop_and_known_light_fields; open: RD7 auxiliary setters and range |
| cGDriver | 12.2 identity/getters/tail | Driver identity/getter methods | dx7_01_02_04_noops_and_identity, lifecycle_initialization_info_noop_and_known_light_fields |
| cGDriver | 12.3 graphics-slot checklist | Driver, vertex module, preceding slot groups | DX7-01–18 plus capability, matrix, stage and immediate-state tests named in this table; no plugin registration |
| cGDriver | 13 order/cache | submit/apply_stages | dx7_05_deferred_binding_and_presentation_order, dx7_06_combiner_cache_and_min_filter_fallthrough, matrix_modes_unchecked_failures_viewport_reset_and_unsupported_blits |
| cPrimitiveManager | 13 order/cache | PrimitiveManager::queue/flush | primitive_copy_merge_immediate_overtake_and_failure_paths, dx7_05_deferred_binding_and_presentation_order |
| cTextureManager | 13 order/cache | TextureManager::lose/init_device/load | dx7_03_09_11_loss_restore_metadata_and_defaults, dx7_10_texture_upload_flush_boundary |
| cPrimitiveManager | 14.1 buffers | VertexBuffer | dx7_16_buffer_reservation_and_reference_order |
| cGDriver | 14.2 context | Context::create | dx7_12_flip_fallback; open: RD8 |
| cGDriver | 14.3 presentation | Context::present | dx7_13_14_present_retry_and_empty_rect |
| cGDriver | 14.4 descriptor | Driver::setup | dx7_15_modes_and_descriptor_enrichment; RD3 unknown meanings/format outcomes remain external |
| cGDriver | 15 vectors | tests.rs | DX7-01–18 represented by named tests |
| cPrimitiveManager | 15 vectors | tests.rs | dx7_05_deferred_binding_and_presentation_order, dx7_16_buffer_reservation_and_reference_order |
| cTextureManager | 15 vectors | tests.rs | dx7_07_08_follow_parameters_and_lifo, dx7_03_09_11_loss_restore_metadata_and_defaults, dx7_10_texture_upload_flush_boundary |
| cGDriver | 16 proof boundaries | Device::unresolved, auxiliary_operation | open: RD7/RD8/RD9/RD11; RD1/RD2/RD3/RD4 retain caller/meaning/handler questions |
| cPrimitiveManager | 16 proof boundaries | PrimitiveManager::queue/flush, primitive::expanded | open: RD10; RD7 independently exposed buffer APIs; RD9 final destruction |
| cTextureManager | 16 proof boundaries | TextureManager::init/load, select_format | open: RD6; RD9 final destruction |

## Remaining boundaries

RD9–RD11 remain open in `docs/specs/questions/renderer-driver.md`. RD9 covers
constructor/reset values, final-release destruction, adapter fallback and the
Shutdown helper (§§2, 4.1, 4.4, 10.1). Unknown stage values and lifecycle helpers
return `Unsupported` unless supplied explicitly by a backend. `Default` values
for Rust records are storage initialization, not asserted native constructor values.
RD10 covers absent vertex elements, incomplete quads and failure aggregation
(§§8.2–8.3); code comments state the literal policy used. RD11 covers FPU mode,
NaN payloads and exceptional arithmetic (§8.6); finite projection uses a software
64-bit significand with round-to-nearest-even, including the rational discriminator.

The spec's existing RD6 (ranking/converters, §§9.2, 9.4, 9.6), RD7 (auxiliary
interfaces/range, §§12.1, 14.1) and RD8 (context/depth selection, §§4.3, 14.2)
also remain explicit boundaries. RD1's null-write hazard is represented by
`Fault::NullWrite`; RD2's known Init write of 7 is retained without inferring its
purpose. RD3's unknown descriptor meanings and RD4's external loss-message
handler do not prevent the specified descriptor writes and notifications.
No fixture at an open boundary is counted as proof of native behavior.

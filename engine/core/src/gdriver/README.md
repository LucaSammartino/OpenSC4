# DirectX 7 driver core

Pure Rust implementation against `renderer-driver.md`. `backend::Device` extends
the existing director `Host`, preserving its 32-bit identity, memory and reference
contracts. The director continues to dispatch its foreign DX7 factory; no plugin,
C ABI, Windows API or registration is changed here. Caller adapters may associate
the returned foreign identity with `Driver`.

This implementation is under construction. Test fixtures supply explicit answers
at open boundaries; they are not evidence for the native behavior. `Unsupported`
identifies the relevant question instead of inventing a native HRESULT.

## Owner closure

| Owner | Spec section | Code item | Test / open boundary |
|---|---|---|---|
| cGDriver | 1–2 scope/construction | Driver::new | dx7_01_02_04_noops_and_identity |
| cGDriver | 3 error | Driver::hr/get_error | dx7_17_sticky_hresult_channel |
| cGDriver | 4.1 init/shutdown | Driver::init/shutdown | open: RD9 (unnamed helper and info fallback) |
| cGDriver | 4.2 mode lifecycle | Driver::set_video_mode | dx7_18_failed_setup_writes_fourth_word_after_cleanup; RD1 caller intent |
| cGDriver | 4.3 setup | Driver::setup | dx7_15_modes_and_descriptor_enrichment; open: RD8 selection |
| cGDriver | 4.4 scenes/loss | Driver::begin_scene/flush | dx7_03_09_11_loss_restore_metadata_and_defaults; open: RD9 stage defaults |
| cGDriver | 4.5 teardown | Driver::destroy_viewport | dx7_18_failed_setup_writes_fourth_word_after_cleanup |
| cGDriver | 5.1–5.3 states/clear | Driver::state/clear | defaults_clear_quantization_and_stencil_failures |
| cGDriver | 5.4 capabilities | Driver::capability/is_enabled | dx7_17_sticky_hresult_channel (remaining branches need tests) |
| cGDriver | 5.5 colour | backend::quantize/color | defaults_clear_quantization_and_stencil_failures |
| cGDriver | 6 modes | Mode, Driver::count_modes/mode_info | dx7_15_modes_and_descriptor_enrichment; open: RD3 bytes |
| cGDriver | 7 defaults | Driver::default_state | defaults_clear_quantization_and_stencil_failures |
| cGDriver | 8.1–8.2 drawing | Driver::interleaved_arrays/draw_arrays/draw_elements | matrices_viewport_formats_and_draw_patterns; more failure tests needed |
| cPrimitiveManager | 8.3 batching | PrimitiveManager | dx7_05_deferred_binding_and_presentation_order; open: RD10 copier defaults |
| cGDriver | 8.4 multipliers/fog | Driver::color_multiplier/alpha_multiplier/enable_vertex_colors/fog_float/fog_int | pending focused tests |
| cGDriver | 8.5 vertex formats | vertex module | matrices_viewport_formats_and_draw_patterns |
| cGDriver | 8.6 matrices | Driver::load_matrix/load_identity | matrices_viewport_formats_and_draw_patterns; open: RD11 precision |
| cTextureManager | 9.1 names | TextureManager::new_name/free/priority | dx7_07_08_follow_parameters_and_lifo |
| cTextureManager | 9.2 loss/lifetime | TextureManager::init/lose/init_device/shutdown | dx7_03_09_11_loss_restore_metadata_and_defaults; open: RD6 conversion |
| cTextureManager | 9.3 surfaces | TextureManager::create | dx7_07_08_follow_parameters_and_lifo |
| cTextureManager | 9.4 uploads | TextureManager::load | dx7_10_texture_upload_flush_boundary; open: RD6 arithmetic |
| cGDriver | 9.5 texture slots | Driver texture methods | dx7_07_08_follow_parameters_and_lifo, dx7_10_texture_upload_flush_boundary |
| cTextureManager | 9.6 selection | Device::unresolved | open: RD6 |
| cGDriver | 10.1–10.2 stages | Stage, Driver stage methods | dx7_06_combiner_cache_and_min_filter_fallthrough, dx7_07_08_follow_parameters_and_lifo |
| cGDriver | 10.3 apply | Stage::apply | dx7_06_combiner_cache_and_min_filter_fallthrough; more environment branches need tests |
| cGDriver | 11 viewport/blits | Driver::set_viewport/get_viewport/blit_unsupported | matrices_viewport_formats_and_draw_patterns |
| cGDriver | 11.1 scene end | Driver::end_scene | dx7_05_deferred_binding_and_presentation_order |
| cGDriver | 12.1 auxiliary interfaces/light | Driver::auxiliary_operation | open: RD7 |
| cGDriver | 12.2–12.3 identity/getters | Driver identity/getter methods | dx7_01_02_04_noops_and_identity |
| cGDriver/cPrimitiveManager/cTextureManager | 13 order/cache | submit/apply_stages, managers | dx7_05_deferred_binding_and_presentation_order, dx7_06_combiner_cache_and_min_filter_fallthrough, dx7_10_texture_upload_flush_boundary |
| cPrimitiveManager | 14.1 buffers | VertexBuffer | dx7_16_buffer_reservation_and_reference_order |
| cGDriver | 14.2 context | Context::create | dx7_12_flip_fallback; open: RD8 |
| cGDriver | 14.3 presentation | Context::present | dx7_13_14_present_retry_and_empty_rect |
| cGDriver | 14.4 descriptor | Driver::setup | dx7_15_modes_and_descriptor_enrichment; open: RD3 |
| all three | 15 vectors | tests.rs | DX7-01–18 represented by named tests |
| all three | 16 proof boundaries | Device::unresolved, auxiliary_operation | open: RD1/RD3/RD4/RD6/RD7/RD8, new RD9–11 |

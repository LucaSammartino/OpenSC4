# Draw context core

Clean-room implementation of `docs/specs/draw-context.md`. `DrawContext<D>` owns
the context decisions and caches; `PackedState` implements cSC4PackedRenderState.
Driver, camera, lighting, view, instance and texture identities cross foreign
traits only. There is no plugin or C ABI integration and no game shadow key.

Numeric inputs carry explicit `NumericControl`: nearest/even with 64 significand
bits, or `X87(Arithmetic)` selecting 24/53/64 bits and any of the four rounding
directions. The adapter must supply the ambient control; `Unqualified` is a
missing-input error, not a guessed game control word. Arithmetic models masked
exceptions, gradual binary32 underflow, overflow, signed zero and nonfinite
results. Each published arithmetic operation and binary32 spill stays separate.
NaN payloads and floating status flags are not an owner output contract.
`PackedState::pack` and `Transform::followed_by` use the conformance vectors'
nearest/even64 control; their `*_with_control` variants accept explicit control.
Packed replay retains its portable nearest/even default without qualification;
an explicit X87 selection also controls reconstructed alpha-reference stores.

Bounds, enum, address and missing-input checks are portable wrapper policy
(section 7.4), not native safe-error claims. Driver `NormalReturn` registers are
ignored. `Error::Provider` denotes a boundary that did not return normally, not
a failure-valued ordinary driver return. No exception cleanup is invented.
Arc storage keeps portable tokens/providers alive; it does not acquire or release
native game references. Shutdown retains storage; Rust destruction frees owned
storage, without a foreign release call.

## Owner closure

Tests below are in `tests.rs`. Sections 0/10 describe context and callers rather
than additional methods; section 11 repeats the covered quirks. Each assigned
operation is mapped below, including the attributed body inventory in 1.5.

| Owner | Spec section | Code item | Test / open boundary |
|---|---|---|---|
| SC4DrawContext | 0, 1 | `DrawContext`, `ViewState`, provider snapshots | `constructor_init_once_and_dc1_bytes` |
| SC4DrawContext | 1.1 Init | `init`, `HIGHLIGHT_COLORS` | `constructor_init_once_and_dc1_bytes`; `init_failure_is_retryable_and_does_not_publish_state`; open: DC1 (foreign byte meanings only) |
| SC4DrawContext | 1.2 combiner records | constructor/default/highlight combiner constants | `settled_complete_constructor_init_and_stage_records` |
| SC4DrawContext | 1.3 lifecycle | `new`, `init`, `shutdown`, owned storage | `settled_returned_load_failures_retry_and_lifecycle`; `closure_fog_constructor_and_indexed_highlight_write` |
| SC4DrawContext | 1.4 stage defaults | `Stage::default_record`, `model_view_matrix` | `settled_complete_constructor_init_and_stage_records`; `settled_returned_load_failures_retry_and_lifecycle` |
| SC4DrawContext | 1.5 body inventory | state/texture/transform/draw methods below; `set_highlight_color`, `turn_on_fog` | `closure_fog_constructor_and_indexed_highlight_write`; fog tests below; open: DC3 (native stack mutation attribution) |
| SC4DrawContext | 1.5.1 fog (DC9) | `turn_on_fog`; borrowed `Driver::fog_color`, `Driver::fog_scalar` | `fog_dc_v27_through_v32_and_v35_order_cache_and_normal_failures`; `fog_dc_v33_v34_float_comparisons_preserve_payloads`; `fog_nonreturning_boundary_preserves_completed_prefix_and_retries` |
| SC4DrawContext | 2 word | `state_word`, `cache.rs` | `packed_dispatch_order_and_mandatory_resets`; `packed_masks_and_published_default_value` |
| SC4DrawContext | 3 setters | `enable_*`, `set_depth_func`, `set_alpha_func`, `set_blend_func`, transparency forwards, invalidation | `cache_and_force_switches_and_keep_sentinels`; `texture_dependencies_suppressed_stage_and_higher_stage_behavior`; `transforms_compose_numerically_and_restore_plain_view` |
| SC4DrawContext | 4 overrides | property snapshots, filter/wrap policy, model branches | `texture_overrides_and_transform_clear_cache`; `model_culling_highlight_skip_and_draw_pass_order`; `debug_boxes_precede_culling_and_do_not_draw_model` |
| SC4DrawContext | 5.2 packed replay | `set_render_state`, default/top replay, unilateral replay, indexed highlight replay | `packed_dispatch_order_and_mandatory_resets`; `partial_packed_mask_does_not_apply_force_switch_to_excluded_bits`; `packed_masks_and_published_default_value` |
| SC4DrawContext | 6 materials | `set_material_state`, `set_highlight_material_state` | `material_fade_grey_depth_bits_and_texture_matrix`; `settled_shade_checked_fallback_grey_and_full_material_matrix`; open: DC2 (foreign draw-flag meanings only) |
| SC4DrawContext | 7 stage setters | `textures.rs`, `set_tex_filter` | `texture_overrides_and_transform_clear_cache`; `settled_transform_duplicate_selection_and_suppressed_clear` |
| SC4DrawContext | 7.1 emulation | `prepare_second_pass`, model second mesh draw | `suppressed_stage_records_all_resources_and_second_pass_replays_them`; `remaining_second_pass_modes_and_hardware_stage` |
| SC4DrawContext | 7.2 stage/cache ordering | `textures.rs` ordered mutations and gates | `settled_literal_cache_order_suppression_and_normal_returns`; `settled_transform_duplicate_selection_and_suppressed_clear` |
| SC4DrawContext | 7.3 replay ordering | packed/unilateral replay, emulated pass | `packed_dispatch_order_and_mandatory_resets`; `settled_literal_cache_order_suppression_and_normal_returns` |
| SC4DrawContext | 7.4 foreign returns/retries | `emit`, `NormalReturn`, checked model light | `settled_literal_cache_order_suppression_and_normal_returns`; `per_field_device_failure_preserves_completed_prefix_and_retry`; `model_failure_preserves_prefix_without_exception_cleanup` |
| SC4DrawContext | 8 transforms/shading/depth | `transforms.rs` | `transforms_compose_numerically_and_restore_plain_view`; `shade_sources_tint_and_reset_once` |
| SC4DrawContext | 8.1 shade persistence | `turn_on_lighting`, shade methods, grey/fade handling | `settled_shade_checked_fallback_grey_and_full_material_matrix` |
| SC4DrawContext | 8.2 packing/material matrix | `Arithmetic`, alpha cache, material matrix construction | `settled_numeric_qualification_and_original_alpha_reference`; `closure_texture_reciprocals_and_x87_controls` |
| SC4DrawContext | 8.3 compact branches/spills | `set_model_transform`, `Transform::followed_by_with_control`, `qualified_matrix` | `settled_compact_whole_flags_counters_spills_and_raw_scale`; `closure_dc_v17_v18_v20_v23_v24`; `settled_returned_load_failures_retry_and_lifecycle` |
| SC4DrawContext | 8.4 raw matrix | `set_model_matrix` | `closure_dc_v17_v18_v20_v23_v24`; `settled_returned_load_failures_retry_and_lifecycle` |
| SC4DrawContext | 9.1 primitives/mesh | vertex/index setters, draw overloads, `render_mesh` | `primitive_mesh_order_and_index_arithmetic` |
| SC4DrawContext | 9.2 model | `render_model_instance` | `model_culling_highlight_skip_and_draw_pass_order`; `debug_boxes_precede_culling_and_do_not_draw_model`; `model_failure_preserves_prefix_without_exception_cleanup` |
| SC4DrawContext | 9.3 box | `draw_bounding_box`, `bounding_box_vertices` | `bounding_box_geometry_face_order_wrapping_and_draw_state` |
| SC4DrawContext | 10 users, 11 quirks | public methods above; no additional caller implementation assigned | material fade, model culling and second-pass tests above |
| SC4DrawContext | 12 open questions | opaque driver bytes/flags; core stack storage helpers | open: DC1/DC2/DC3; Begin/End owner requirement was withdrawn in 1.3; legacy methods explicitly reject it |
| SC4DrawContext | 13, 13.1, 13.2, 13.3 conformance | all methods above | complete vector mapping below |
| cSC4PackedRenderState | 2 field layout; 5.1 packing | `PackInput`, `PackedState::pack_with_control`, `apply_to` | `packed_masks_and_published_default_value`; `closure_texture_reciprocals_and_x87_controls` |
| cSC4PackedRenderState | 8.2 alpha arithmetic; 13 conformance | `quantize_with_control`, `Arithmetic` | `alpha_is_unrounded_on_input_and_quantized_on_replay`; `settled_numeric_qualification_and_original_alpha_reference` |

cSC4PackedRenderState is not assigned lifecycle, driver, texture or draw methods
in the other sections. Its mask/value are eight portable bytes; stack-top replay
does not remove them. The public push/pop helpers manipulate core storage only;
they are not claims of attributed native methods (DC3).

## Conformance coverage

| Spec tests / vectors | Test |
|---|---|
| 13.1 cache/force | `cache_and_force_switches_and_keep_sentinels` |
| 13.3 alpha | `alpha_is_unrounded_on_input_and_quantized_on_replay` |
| 13.4 mask; 13.5 default | `packed_masks_and_published_default_value` |
| 13.6 composition; 13.7 depth offset | `transforms_compose_numerically_and_restore_plain_view` |
| 13.8 second pass | `suppressed_stage_records_all_resources_and_second_pass_replays_them` |
| 13.9 culling | `model_culling_highlight_skip_and_draw_pass_order` |
| DC-V01 | `settled_complete_constructor_init_and_stage_records` |
| DC-V02, DC-V03, DC-V06, DC-V08 | `settled_literal_cache_order_suppression_and_normal_returns` |
| DC-V04, DC-V05 | `settled_transform_duplicate_selection_and_suppressed_clear` |
| DC-V07 | `packed_dispatch_order_and_mandatory_resets` |
| DC-V09, DC-V10, DC-V12 | `settled_shade_checked_fallback_grey_and_full_material_matrix` |
| DC-V11 | `settled_numeric_qualification_and_original_alpha_reference` |
| DC-V13 | `packed_masks_and_published_default_value` |
| DC-V14, DC-V25, DC-V26 | `settled_returned_load_failures_retry_and_lifecycle` |
| DC-V15, DC-V16, DC-V19, DC-V21, DC-V22 | `settled_compact_whole_flags_counters_spills_and_raw_scale` |
| DC-V17, DC-V18, DC-V20, DC-V23, DC-V24 | `closure_dc_v17_v18_v20_v23_v24` |
| DC-V27–DC-V32, DC-V35 | `fog_dc_v27_through_v32_and_v35_order_cache_and_normal_failures` |
| DC-V33, DC-V34 | `fog_dc_v33_v34_float_comparisons_preserve_payloads` |

These are core conformance results, not measurements of the game. Tests also
cover general texture reciprocals, zero scale, unbounded alpha packing, all
precision/rounding selections, gradual underflow and directed overflow.

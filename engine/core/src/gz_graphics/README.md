# GZ graphics core

Source: `docs/specs/gz-graphics.md` sections 1–9. This sibling of
`draw_context` owns the widget-facing GZ context; the latter owns the distinct
SC4 3D helper. Neither its slots nor its packed render state are reused.
Existing `span_render8`, `span_render1555`, `span_render565`, `span_render4444`
and `span_render8888` kernels supply buffer solid/alpha writes without copying
their blend formulas. No plugin, C ABI, native object layout or replacement
claim is included.

`Target`, `Buffer`, `EngineFactory`, `HardwareDevice`, `GraphicsHost`,
`WindowHost`, `AlphaDelegate` and `AlphaFileIo` describe foreign interfaces.
Arc ownership models acquired references, pointer equality models identity,
and borrowed getters do not acquire references. Query failure preserves the
output. State is ordinary owned Rust data; owners are Send + Sync and getters
do not use RefCell. Mutable operations still require exclusive access.

The native context does not initialize colours. `DrawContext::new` requires
caller-supplied colours. Detached coordinates and unspecified hardware fields
have safe placeholders, not asserted native defaults. The fallible attachment
API uses the spec's explicitly allowed detached-on-failure choice. Invalid
indices/nonfinite thickness return errors rather than reproducing memory
corruption or undefined numeric conversions. Native buffer lock quirks and
success-with-null-renderer allocation failure are represented, including final
release without unlocking. Context destruction calls Shutdown before freeing
temporary vectors and releasing the engine and target (§2.1 / V45).

The generic implementation includes the settled ordered polygon chains, direct
triangle fan, ellipse-to-cubic and filled-ellipse routes, directional horizontal
integer lines and adaptive cubic fixed-point subdivision (including overshoot).
Fractional diamond-boundary ties, thin clips and x87 trig last bits remain the
explicit GZG1 observation set in §3.2.7; AA pixel edges, source sampling
and device subdivision remain GZG2 (§4.4). `GeometryRoute` and `DrawPlan` expose the
specified dispatch/grouping decisions. The real buffer AA execution returns
GZG2 when it would need unresolved pixels; its plan still reports the native
result and complete vertex groups. Recording test engines acknowledge the
routes without pretending to be a rasterizer. Settled generic spans reach the
five existing buffer kernels, including gradient/alpha-gradient writes. The
gradient cache implements the binary32 stores and nearest-even conversion in
the default FP environment. Device callbacks receive the specified pending-state order;
their raw geometry submissions are an explicit GZG2 boundary, not a claim that
unclipped input is native device vertex data. SimGL delegates non-specialized
calls to the base; specialized device output remains GZG2.

Graphics-host calls expose successful lifecycle ordering. The §5.4 driver/INI
preference, folded control slots and ordered UI state/projection are implemented;
partial-init rollback, IO and presentation retain their native observation sets.
The canvas implements §7.4 realization, listeners and event translation through
foreign OS callbacks. Its unnamed folded slots 25–28 still have no behavior
contract in §7.1. Alpha's specialized operations retain the typed delegate
boundary; the ordinary §6.2.1 16-bit kernel has its own lock/cleanup and five-bit
blend, including the type-5 green-bit loss. Source/destination aliasing and
out-of-domain alpha/high bits are explicitly unsupported. Ordinary/stretch
clipping and the legacy indexed loaders are implemented separately.
Loaders preserve raw mask indices and two-division
alpha quantization, fixed offsets, bottom-up rows and actual destination stride.
Open failure retains the native created-reference leak. Lock failure records the
invalid one-past-end free through the IO adapter and reports `Foreign`, without
performing an invalid free or asserting native normal return. Malformed inputs
return safe errors; the spec promises no deterministic native outcome for them.

## Owner closure

Tests are in `tests.rs`, `closure_tests.rs`, `answered_tests.rs`, `canvas_tests.rs`
and the V46 lifetime fixture beside `DrawContext`. V01–90 now have applicable
fixtures. GZG7 is applied and closed. GZG1/2/3/5 retain only the explicit native
observation sets; the canvas inventory retains its unnamed-slot gap. A row
marked implemented has both code and a named test; owner-wide native-exact
credit is not inferred from passing those fixtures.

| Owner | Spec section | Code item | Test / open contract |
|---|---|---|---|
| All GZ owners | 1 | `Rect`, module boundary, foreign traits | `gzg_v02_empty_intersection`, `owner_getters_can_be_shared_across_threads` |
| cGZDrawContext | 2.1 | `query`, `CONTEXT_IIDS`, getters, `Drop` | implemented: `gzg_v01_query_and_reference_lifetimes`, `gzg_v45_v46_context_replacement_failure_stacks_and_shutdown_destruction`, `gzg_v46_null_engine_still_releases_retained_target` |
| cGZDrawContext | 2.2 | `DrawContext::new`, `Attributes`, `reset_defaults` | implemented: `gzg_v03_v04_attachment_and_same_pointer`, `gzg_v09_v10_state_stack_and_narrow_reset` |
| cGZDrawContext | 2.3 | colour/mode/thickness setters, `gradient` forwarding | implemented: `gzg_v11_v12_setter_suppression_and_thickness`, `gzg_v03_v04_attachment_and_same_pointer`; invalid numeric inputs safely rejected |
| cGZDrawContext | 3.1 | offsets, clipping, transform/state stacks | implemented: `gzg_v05_v06_v07_v08_transforms_and_clip`, `gzg_v09_v10_state_stack_and_narrow_reset` |
| cGZDrawContext | 3.2 | `draw`, `draw_no_transform`, point/line/fill/clear | input/dispatch implemented: `gzg_v13_v14_v15_integer_geometry_and_clear`, `gzg_v43_v44_filled_result_switches`; open: GZG1 (unspecified generic point/rectangle bodies and §3.2.7 fractional/thin-clip spans) |
| cGZDrawContext | 3.2.1 | `ellipse`, `filled_ellipse`, `geometry::execute` | implemented: `gzg_v47_ellipse_convenience_origin_once`, `gzg_v49_v50_v51_filled_ellipse_raster_and_repeated_first_pair` |
| cGZDrawContext | 3.2.2, 3.2.6–7 | `geometry::line`, `stroke_quad`, Horner curve grouping | implemented GZG7 and settled GZG1 phase: `gzg7_stroke_corners_and_constant_cubic_worked_cases`, `gzg1_integer_line_phase_worked_cases`, `gzg_v48_curve_groups_and_sixteen_stroke_quads`; open: GZG1 (§3.2.7 fractional diamond ties and thin clips) |
| cGZDrawContext | 3.2.3 | `polygon_spans`, direct fan | implemented: `gzg_v81_triangle_fan_no_second_fallback`, `gzg_v82_v83_v84_ordered_polygon_chains`, `settled_polygon_spans_reach_real_buffer_kernels` |
| cGZDrawContext | 3.2.4, 3.2.7 | `ellipse_cubics`, `outline_ellipse`, kind-2 resubmission | implemented topology: `gzg_v85_outline_ellipse_cubics_use_each_pair`, `gzg1_non_aa_outline_ellipse_topology_worked_case`; open: GZG1 (§3.2.7 native trig last-bit capture) |
| cGZDrawContext | 3.2.5 | `cubic_segments`, `geometry::line` | subdivision and centered-line phase implemented: `gzg_v86_cubic_forward_difference_overshoot`, `gzg1_integer_line_phase_worked_cases`; open: GZG1 (fractional subdivision endpoints/thin clips) |
| cGZDrawContext | 3.2.6 | `stroke_quad`, binary32 Horner coefficients/stores | implemented GZG7: `gzg7_stroke_corners_and_constant_cubic_worked_cases`, `gzg7_non_aa_directional_caps_and_aa_length_gate` |
| cGZDrawContext | 3.3 | blit binding, tiling, convenience blits, suspension, engine forwards | implemented: `gzg_v16_suspension_is_nested_but_every_suspend_calls_engine`, `gzg_v17_blit_effective_clip_is_separate`, `gzg_v52_v53_v54_convenience_blits_and_empty_noop` |
| cGZDrawContext | 3.4 | `set_target`, independent retained engine/target | implemented: `gzg_v03_v04_attachment_and_same_pointer`, `gzg_v45_v46_context_replacement_failure_stacks_and_shutdown_destruction` |
| cGZDrawContextRenderTo* | 4.1, 4.4 | `RenderEngine`, `ENGINE_IIDS`, `query`, SimGL extension hooks | `gzg_v01_query_and_reference_lifetimes`, `gzg2_extension_forwarding_shutdown_and_base_fallback`; open: GZG2 (§4.4 slot-30/31 payload and concrete interface-extent captures) |
| cGZDrawContextRenderToBuffer | 4.2 | `BufferEngine`, `SpanFormat`, record cache, state, `plan_draw`, `GradientCoefficients`, `spans`, five gradient kernels | `gzg_v18_v19_buffer_query_and_lock_failures`, `gzg_v20_v21_v22_buffer_lifetime_format_and_allocation`, `gzg_v23_v24_v25_buffer_aa_dispatch_plan_and_open_coverage`, `buffer_span_kernel_reuse_and_suspend_record_caching`, `gzg_v55_gradient_conversion_cache_and_kernel_writes`; open: GZG2 (source conversion and AA coverage) |
| cGZDrawContextRenderToHardware | 4.3–4.4 | `HardwareEngine`, `EngineState`, device shutdown and extension hooks | `gzg_v26_v27_hardware_failure_and_alpha_predicates`, `gzg2_extension_forwarding_shutdown_and_base_fallback`; open: GZG2 (§4.4 clipped source/color/UV vertex captures) |
| cGZDrawContextRenderToSimGL | 4.3–4.4 | `SimGlEngine::draw`, `ImageBinding::set_image`, extension forwards | implemented fallback and cache-failure ownership: `gzg2_extension_forwarding_shutdown_and_base_fallback`, `gzg2_image_cache_failure_and_null_source_worked_cases`, `gzg2_image_allocation_failure_releases_old_binding_in_order`; open: GZG2 (§4.4 specialized vertices/result-3 and slot-30/31 captures) |
| cGZGraphicSystem | 5.1 | identities, construction, driver/control state | `gzg_v01_query_and_reference_lifetimes`, `graphics_lifecycle_listeners_flags_and_driver_identity`; open: GZG3 (unnamed inventories, IO) |
| cGZGraphicSystem | 5.2 | `init`, `shutdown`, `do_message`, `canvas_event`, `on_tick`, `load_all_drivers` | `gzg_v28_graphics_dispatch_and_service_release`, `graphics_lifecycle_listeners_flags_and_driver_identity`, `gzg3_driver_preference_worked_cases_and_empty_section`; open: GZG3 (§5.4 factory-failure cleanup/second-Init capture) |
| cGZGraphicSystem | 5.3 | driver identity/borrowed getters, availability/downgrade, flags | `gzg_v61_driver_getters_are_borrowed_exact_and_null_on_missing`, `gzg_v29_translucency_is_the_byte_not_device_capability`, `gzg_v30_v31_v32_screen_downgrade_reduces_before_testing`; open: GZG3 (§5.4 screen allocation defaults and RenderBufferToScreen copy/present/failure captures) |
| cGZGraphicSystem | 5.4 | `load_all_drivers`, `control_slot17/20`, `apply_ui_rendering_mode` | implemented: `gzg3_driver_preference_worked_cases_and_empty_section`, `gzg3_ui_order_and_cached_projection_worked_case` |
| cGZAlphaBlt | 6.1 | identities, `AlphaOperation`, `AlphaBlt::call`, count | `gzg_v33_v34_v35_alpha_stateless_forwarding_and_open_failure`; open: GZG5 (remaining standalone signatures/pixel kernels) |
| cGZAlphaBlt | 6.2 | `surface_from_file`, `alpha_surface_from_file`, `mask_surface_from_file` | implemented: `gzg_v58_v59_indexed_loader_levels_raw_mask_and_row_stride`, `gzg_v35_v60_loader_open_and_lock_failure_ownership` |
| cGZAlphaBlt | 6.2.1 | `alpha_blt_no_clip`, `AlphaSource`, five-bit interpolation | implemented ordinary domain: `gzg5_ordinary_alpha_worked_pixels_and_lock_failures`, `gzg5_ordinary_keyed_type6_and_unsupported_source_depth`; specialized operations/alias/high-bit observations remain GZG5 (§6.2.1) |
| cGZAlphaBlt | 6.3 | `generic_buffer_clip`, `clipped_alpha_rectangles`, `alpha_blt` | implemented: `gzg_v56_v57_ordinary_clip_and_wrapper_local_copies`, ordinary-kernel tests above |
| cGZAlphaBlt | 6.3.1 | `generic_buffer_clip_stretch` | implemented: `gzg_v87_v88_v89_v90_stretch_clip` |
| cGZAlphaBlt | 6.3.2 | `alpha::load_surface`, `AlphaFileIo` | implemented: `gzg_v58_v59_indexed_loader_levels_raw_mask_and_row_stride`, `gzg_v35_v60_loader_open_and_lock_failure_ownership`; undefined native inputs safely rejected |
| cGZCanvasW32 | 7.1 | identities, `configure_bounds`, primary/service API separation, HWND getters | `gzg_v01_query_and_reference_lifetimes`, `gzg4_v65_v66_v77_listener_order_ownership_and_hwnd_getters`; open: §7.1 folded slots 25–28 have no behavior contract (SPEC-GAP); GZG4 listener/adjustment question otherwise applied |
| cGZCanvasW32 | 7.2, 7.4 | `initialize_native`, `realize_native`, suspend/resume/delete, service lifecycle | implemented GZG4: `canvas_lifecycle_failures_focus_and_counter_balance`, `gzg4_v62_v63_v64_realization_failure_release_and_repaint`, `gzg4_native_mode_matching_center_suspend_resume_and_failure`, `gzg4_v76_v78_default_route_and_unconditional_delete` |
| cGZCanvasW32 | 7.3–7.4 | bounds/title/show/post/input, `native_listener`, `native_message`, `filter_keyboard_queue` | implemented GZG4: existing V36–42 tests, `gzg4_v65_v66_v77_listener_order_ownership_and_hwnd_getters`, `gzg4_v67_through_v74_mouse_key_wheel_and_signed_text`, `gzg4_v75_queue_boundary_and_discard_count`, `gzg4_v76_v78_default_route_and_unconditional_delete` |
| Requested owners | 8, 8.1 | applicable V01–90 fixtures | grouped `gzg_v*` / `gzg1/2/3/4/5/7_*` tests; real AA execution explicitly remains GZG2 |
| Requested owners | 9 | `Error::Unsupported(question_id)` boundaries and inventory gap | open: §3.2.7 GZG1, §4.4 GZG2, §5.4 GZG3, §6.2.1 GZG5 observation sets, plus §7.1 unnamed slots; GZG7 closed |

These core fixtures do not expose a plugin replacement or graphics shadow.
A plugin smoke run checks integration only; no graphics-native comparison or
unobserved owner-wide compatibility is claimed.

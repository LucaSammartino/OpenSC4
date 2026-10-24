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
Unsettled line ties/outline coverage remain GZG1; AA pixel edges, source sampling
and device subdivision remain GZG2. `GeometryRoute` and `DrawPlan` expose the
specified dispatch/grouping decisions. The real buffer AA execution returns
GZG2 when it would need unresolved pixels; its plan still reports the native
result and complete vertex groups. Recording test engines acknowledge the
routes without pretending to be a rasterizer. Settled generic spans reach the
five existing buffer kernels, including gradient/alpha-gradient writes. The
gradient cache implements the binary32 stores and nearest-even conversion in
the default FP environment. Device callbacks receive the specified pending-state order;
their raw geometry submissions are an explicit GZG2 boundary, not a claim that
unclipped input is native device vertex data. SimGL fallback remains GZG2.

Graphics-host calls expose successful lifecycle ordering, not a guessed driver
INI resolver or partial-init rollback. Window realization and event translation
stay foreign; the canvas implements the settled saved state, counter, posting,
title and service contracts. Alpha's 26 operation identities forward unchanged
arguments through a typed delegate envelope because native widths and return
channels are GZG5. Ordinary/stretch clipping and the legacy indexed loaders are
implemented separately. Loaders preserve raw mask indices and two-division
alpha quantization, fixed offsets, bottom-up rows and actual destination stride.
Open failure retains the native created-reference leak. Lock failure records the
invalid one-past-end free through the IO adapter and reports `Foreign`, without
performing an invalid free or asserting native normal return. Malformed inputs
return safe errors; the spec promises no deterministic native outcome for them.

## Owner closure

Tests are in `tests.rs`, its child `closure_tests.rs`, and the V46 lifetime
fixture beside `DrawContext`. Open cells identify
remaining spec limits, not successful rendering. V01–61 and V79–90 concern
this lane's owners (V36–42 concern canvas and retain existing coverage).
V62–78 concern the out-of-scope native canvas extension and are not claimed.
All four requested owner groups still have explicit open spec contracts;
none is claimed fully native-exact. Settled bodies are mapped individually below.

| Owner | Spec section | Code item | Test / open contract |
|---|---|---|---|
| All GZ owners | 1 | `Rect`, module boundary, foreign traits | `gzg_v02_empty_intersection`, `owner_getters_can_be_shared_across_threads` |
| cGZDrawContext | 2.1 | `query`, `CONTEXT_IIDS`, getters, `Drop` | implemented: `gzg_v01_query_and_reference_lifetimes`, `gzg_v45_v46_context_replacement_failure_stacks_and_shutdown_destruction`, `gzg_v46_null_engine_still_releases_retained_target` |
| cGZDrawContext | 2.2 | `DrawContext::new`, `Attributes`, `reset_defaults` | `gzg_v03_v04_attachment_and_same_pointer`, `gzg_v09_v10_state_stack_and_narrow_reset` |
| cGZDrawContext | 2.3 | colour/mode/thickness setters, `gradient` forwarding | implemented: `gzg_v11_v12_setter_suppression_and_thickness`, `gzg_v03_v04_attachment_and_same_pointer`; invalid numeric inputs safely rejected |
| cGZDrawContext | 3.1 | offsets, clipping, transform/state stacks | `gzg_v05_v06_v07_v08_transforms_and_clip`, `gzg_v09_v10_state_stack_and_narrow_reset` |
| cGZDrawContext | 3.2 | `draw`, `draw_no_transform`, point/line/fill/clear | input/dispatch implemented: `gzg_v13_v14_v15_integer_geometry_and_clear`, `gzg_v43_v44_filled_result_switches`; open: GZG1 (remaining generic point/rectangle/line coverage) |
| cGZDrawContext | 3.2.1 | `ellipse`, `filled_ellipse`, `geometry::execute` | implemented: `gzg_v47_ellipse_convenience_origin_once`, `gzg_v49_v50_v51_filled_ellipse_raster_and_repeated_first_pair` |
| cGZDrawContext | 3.2.2 | curve grouping, fixed conversion, horizontal lines, polygon grouping | `gzg_v48_curve_groups_and_sixteen_stroke_quads`, `gzg_v79_v80_horizontal_directional_lines`; open: GZG1 (general integer lines), GZG7 (stroke corners/rounding) |
| cGZDrawContext | 3.2.3 | `polygon_spans`, direct fan | implemented: `gzg_v81_triangle_fan_no_second_fallback`, `gzg_v82_v83_v84_ordered_polygon_chains`, `settled_polygon_spans_reach_real_buffer_kernels` |
| cGZDrawContext | 3.2.4 | `ellipse_cubics`, kind-2 resubmission | `gzg_v85_outline_ellipse_cubics_use_each_pair`; open: GZG1 (non-AA result-1 outline) |
| cGZDrawContext | 3.2.5 | `cubic_segments` | subdivision implemented: `gzg_v86_cubic_forward_difference_overshoot`; general integer-line execution remains GZG1 |
| cGZDrawContext | 3.3 | blit binding, tiling, convenience blits, suspension, engine forwards | implemented: `gzg_v16_suspension_is_nested_but_every_suspend_calls_engine`, `gzg_v17_blit_effective_clip_is_separate`, `gzg_v52_v53_v54_convenience_blits_and_empty_noop` |
| cGZDrawContext | 3.4 | `set_target`, independent retained engine/target | implemented: `gzg_v03_v04_attachment_and_same_pointer`, `gzg_v45_v46_context_replacement_failure_stacks_and_shutdown_destruction` |
| cGZDrawContextRenderTo* | 4.1 | `RenderEngine`, `ENGINE_IIDS`, `query` | `gzg_v01_query_and_reference_lifetimes`; open: GZG2 (extension slots) |
| cGZDrawContextRenderToBuffer | 4.2 | `BufferEngine`, `SpanFormat`, record cache, state, `plan_draw`, `GradientCoefficients`, `spans`, five gradient kernels | `gzg_v18_v19_buffer_query_and_lock_failures`, `gzg_v20_v21_v22_buffer_lifetime_format_and_allocation`, `gzg_v23_v24_v25_buffer_aa_dispatch_plan_and_open_coverage`, `buffer_span_kernel_reuse_and_suspend_record_caching`, `gzg_v55_gradient_conversion_cache_and_kernel_writes`; open: GZG2 (source conversion and AA coverage) |
| cGZDrawContextRenderToHardware | 4.3 | `HardwareEngine`, `EngineState`, device hooks | `gzg_v26_v27_hardware_failure_and_alpha_predicates`, `hardware_state_order_and_simgl_open_routes`; open: GZG2 (Shutdown details, source/device subdivision, extension slots) |
| cGZDrawContextRenderToSimGL | 4.3 | `SimGlEngine`, construction, target probe, pair-count gate | `hardware_state_order_and_simgl_open_routes`; open: GZG2 (specialized subdivision and fallback) |
| cGZGraphicSystem | 5.1 | identities, construction, driver/control state | `gzg_v01_query_and_reference_lifetimes`, `graphics_lifecycle_listeners_flags_and_driver_identity`; open: GZG3 (unnamed inventories, IO) |
| cGZGraphicSystem | 5.2 | `init`, `shutdown`, `do_message`, `canvas_event`, `on_tick` | `gzg_v28_graphics_dispatch_and_service_release`, `graphics_lifecycle_listeners_flags_and_driver_identity`; open: GZG3 (discovery, rollback), GZG4 (Windows translation) |
| cGZGraphicSystem | 5.3 | driver identity/borrowed getters, stored preference/path setters, availability/downgrade, flags | `gzg_v61_driver_getters_are_borrowed_exact_and_null_on_missing`, `gzg_v29_translucency_is_the_byte_not_device_capability`, `gzg_v30_v31_v32_screen_downgrade_reduces_before_testing`, `graphics_lifecycle_listeners_flags_and_driver_identity`; open: GZG3 (presentation, projection, screen format/defaults, folded slots) |
| cGZAlphaBlt | 6.1 | identities, `AlphaOperation`, `AlphaBlt::call`, count | `gzg_v33_v34_v35_alpha_stateless_forwarding_and_open_failure`; open: GZG5 (remaining standalone signatures/pixel kernels) |
| cGZAlphaBlt | 6.2 | `surface_from_file`, `alpha_surface_from_file`, `mask_surface_from_file` | implemented: `gzg_v58_v59_indexed_loader_levels_raw_mask_and_row_stride`, `gzg_v35_v60_loader_open_and_lock_failure_ownership` |
| cGZAlphaBlt | 6.3 | `generic_buffer_clip`, `clipped_alpha_rectangles` | implemented: `gzg_v56_v57_ordinary_clip_and_wrapper_local_copies`; final NoClip pixels remain delegated/GZG5 |
| cGZAlphaBlt | 6.3.1 | `generic_buffer_clip_stretch` | implemented: `gzg_v87_v88_v89_v90_stretch_clip` |
| cGZAlphaBlt | 6.3.2 | `alpha::load_surface`, `AlphaFileIo` | implemented: `gzg_v58_v59_indexed_loader_levels_raw_mask_and_row_stride`, `gzg_v35_v60_loader_open_and_lock_failure_ownership`; undefined native inputs safely rejected |
| cGZCanvasW32 | 7.1 | identities, `configure_bounds`, primary/service API separation | `gzg_v01_query_and_reference_lifetimes`, `canvas_lifecycle_failures_focus_and_counter_balance`; open: GZG4 (unnamed slots, listener plumbing) |
| cGZCanvasW32 | 7.2 | construction, code page, services/realization, wheel ID, lifecycle | `canvas_lifecycle_failures_focus_and_counter_balance`; open: GZG4 (native window ownership, event 8/9 conditions) |
| cGZCanvasW32 | 7.3 | bounds/title/show/post/input/message methods | `gzg_v36_v37_canvas_bounds_and_title_quirk`, `gzg_v38_v39_nested_channels_and_selector`, `gzg_v40_v41_v42_packages_and_messages`, `canvas_lifecycle_failures_focus_and_counter_balance`; open: GZG4 (coordinate names and event translation) |
| Requested owners | 8, 8.1 | applicable V01–61, V79–90 fixtures | grouped `gzg_v*` tests; V24 checks grouping/result plan while real AA execution explicitly remains GZG2 |
| Requested owners | 9 | `Error::Unsupported(question_id)` boundaries | AA/SimGL/general-line tests check explicit gaps; open: GZG1/GZG2/GZG3/GZG5/GZG7 |

No real-game request is applicable to this core-only module. These are
specified-fixture results, not game measurements or shadow equality.

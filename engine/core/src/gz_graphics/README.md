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
release without unlocking. Context destruction follows the literal release-only
reading of section 2.1; GZG6 asks whether an additional Shutdown is required.

Generic geometry coverage, AA pixel edges, source/gradient conversion and
device subdivision are not approximated. `GeometryRoute` and `DrawPlan` expose
the specified dispatch/grouping decisions. The real buffer AA execution returns
GZG2 when it would need unresolved pixels; its plan still reports the native
result and complete vertex groups. Recording test engines acknowledge the
routes without pretending to be a rasterizer. Actual engines reject generic
coverage with GZG1. Device callbacks receive the specified pending-state order;
their raw geometry submissions are an explicit GZG2 boundary, not a claim that
unclipped input is native device vertex data. SimGL fallback remains GZG2.

Graphics-host calls expose successful lifecycle ordering, not a guessed driver
INI resolver or partial-init rollback. Window realization and event translation
stay foreign; the canvas implements the settled saved state, counter, posting,
title and service contracts. Alpha's 26 operation identities forward unchanged
arguments through a typed delegate envelope because native widths and return
channels are GZG5. A loader open failure returns null; an opened file returns
GZG5 rather than inventing alpha/mask quantization or truncated-file handling.

## Owner closure

Each test name below is in `tests.rs`; grouped vector tests name every covered
ID. Open cells identify the spec's existing explicit limits, not successful
rendering. Section 8 contains 44 declarative vectors, covered by 21 grouped
tests, plus six lifecycle/kernel/thread tests.

| Owner | Spec section | Code item | Test / open contract |
|---|---|---|---|
| All GZ owners | 1 | `Rect`, module boundary, foreign traits | `gzg_v02_empty_intersection`, `owner_getters_can_be_shared_across_threads` |
| cGZDrawContext | 2.1 | `query`, `CONTEXT_IIDS`, getters, `Drop` | `gzg_v01_query_and_reference_lifetimes`, `context_replacement_failure_stacks_and_release_only_destruction`; open: GZG6 (destructor Shutdown) |
| cGZDrawContext | 2.2 | `DrawContext::new`, `Attributes`, `reset_defaults` | `gzg_v03_v04_attachment_and_same_pointer`, `gzg_v09_v10_state_stack_and_narrow_reset` |
| cGZDrawContext | 2.3 | colour/mode/thickness setters, `gradient` forwarding | `gzg_v11_v12_setter_suppression_and_thickness`, `gzg_v03_v04_attachment_and_same_pointer`; open: GZG1 (gradient conversion, nonfinite contract) |
| cGZDrawContext | 3.1 | offsets, clipping, transform/state stacks | `gzg_v05_v06_v07_v08_transforms_and_clip`, `gzg_v09_v10_state_stack_and_narrow_reset` |
| cGZDrawContext | 3.2 | `draw`, `draw_no_transform`, point/line/fill/clear | `gzg_v13_v14_v15_integer_geometry_and_clear`, `gzg_v43_v44_filled_result_switches`; open: GZG1 (geometry coverage, tessellation, slots 36–37) |
| cGZDrawContext | 3.3 | blit binding, tiling, suspension, engine forwards | `gzg_v16_suspension_is_nested_but_every_suspend_calls_engine`, `gzg_v17_blit_effective_clip_is_separate`; open: GZG1 (Blt/BltTiled convenience semantics) |
| cGZDrawContext | 3.4 | `set_target`, independent retained engine/target | `gzg_v03_v04_attachment_and_same_pointer`, `context_replacement_failure_stacks_and_release_only_destruction` |
| cGZDrawContextRenderTo* | 4.1 | `RenderEngine`, `ENGINE_IIDS`, `query` | `gzg_v01_query_and_reference_lifetimes`; open: GZG2 (extension slots) |
| cGZDrawContextRenderToBuffer | 4.2 | `BufferEngine`, `SpanFormat`, record cache, state, `plan_draw`, `span_route`, `draw_preclipped_spans` | `gzg_v18_v19_buffer_query_and_lock_failures`, `gzg_v20_v21_v22_buffer_lifetime_format_and_allocation`, `gzg_v23_v24_v25_buffer_aa_dispatch_plan_and_open_coverage`, `buffer_span_kernel_reuse_and_suspend_record_caching`; open: GZG1/GZG2 (gradient/source conversion and AA coverage) |
| cGZDrawContextRenderToHardware | 4.3 | `HardwareEngine`, `EngineState`, device hooks | `gzg_v26_v27_hardware_failure_and_alpha_predicates`, `hardware_state_order_and_simgl_open_routes`; open: GZG2 (Shutdown details, source/device subdivision, extension slots) |
| cGZDrawContextRenderToSimGL | 4.3 | `SimGlEngine`, construction, target probe, pair-count gate | `hardware_state_order_and_simgl_open_routes`; open: GZG2 (specialized subdivision and fallback) |
| cGZGraphicSystem | 5.1 | identities, construction, driver/control state | `gzg_v01_query_and_reference_lifetimes`, `graphics_lifecycle_listeners_flags_and_driver_identity`; open: GZG3 (unnamed inventories, IO) |
| cGZGraphicSystem | 5.2 | `init`, `shutdown`, `do_message`, `canvas_event`, `on_tick` | `gzg_v28_graphics_dispatch_and_service_release`, `graphics_lifecycle_listeners_flags_and_driver_identity`; open: GZG3 (discovery, rollback), GZG4 (Windows translation) |
| cGZGraphicSystem | 5.3 | driver identity/borrowed getters, availability/downgrade, flags | `gzg_v29_translucency_is_the_byte_not_device_capability`, `gzg_v30_v31_v32_screen_downgrade_reduces_before_testing`, `graphics_lifecycle_listeners_flags_and_driver_identity`; open: GZG3 (presentation, projection, screen format/defaults, folded slots) |
| cGZAlphaBlt | 6.1 | identities, `AlphaOperation`, `AlphaBlt::call`, count | `gzg_v33_v34_v35_alpha_stateless_forwarding_and_open_failure`; open: GZG5 (standalone signatures, GenericBufferClip mutation, pixel kernels) |
| cGZAlphaBlt | 6.2 | `surface_from_file`, foreign IO open failure | `gzg_v33_v34_v35_alpha_stateless_forwarding_and_open_failure`; open: GZG5 (successful legacy loading/quantization and malformed inputs) |
| cGZCanvasW32 | 7.1 | identities, `configure_bounds`, primary/service API separation | `gzg_v01_query_and_reference_lifetimes`, `canvas_lifecycle_failures_focus_and_counter_balance`; open: GZG4 (unnamed slots, listener plumbing) |
| cGZCanvasW32 | 7.2 | construction, code page, services/realization, wheel ID, lifecycle | `canvas_lifecycle_failures_focus_and_counter_balance`; open: GZG4 (native window ownership, event 8/9 conditions) |
| cGZCanvasW32 | 7.3 | bounds/title/show/post/input/message methods | `gzg_v36_v37_canvas_bounds_and_title_quirk`, `gzg_v38_v39_nested_channels_and_selector`, `gzg_v40_v41_v42_packages_and_messages`, `canvas_lifecycle_failures_focus_and_counter_balance`; open: GZG4 (coordinate names and event translation) |
| All owners | 8 | 44 `GZG-V01`–`GZG-V44` fixtures | 21 grouped `gzg_v*` tests; V24 checks grouping/result plan while real AA execution explicitly remains GZG2 |
| All owners | 9 | `Error::Unsupported(question_id)` boundaries | AA/SimGL/loader tests check explicit gaps; open: GZG1–GZG5, plus GZG6 |

No real-game request is applicable to this core-only module. These are
specified-fixture results, not game measurements or shadow equality.

# Model maker (partial engine implementation)

Portable `cSC4ModelMaker`, from `docs/specs/model-maker.md` only. No C ABI or
plugin wiring. Handles are identities, not addresses. Each trait documents
borrowed versus acquired results. Mutations require exclusive access and shared
getters contain no `RefCell`. Native final release is explicit (`release(host)`);
`destroy(host)` exposes the separate destructor contract.

This round closes independent sections; **the owner is not implemented in full**.
PostCityInit, message dispatch, occupant handlers and model construction remain
pending. A pending row does not claim coverage or act as an invented fallback.
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
| 12 Model loading | Pending | §§12.1–12.12 pending, no substitute foreign implementation of this owner's logic |
| 13 Terrain hiding | `hide_occupant_terrain`, `hide_terrain_cells`, `TerrainHost` | §13.2 type worked rules: `network_base_texture_and_pipe_bits_preserve_high_flags`; §13.1 clipping/mode: `foundation_mode_rebuilds_flags_and_refreshes_end_exclusive_clipped_rect` |
| 14 AddModels | Pending | M28 pending |
| 15 Background loader | Separate `model_background_loader` module | See its README; **open MM1** is an explicit Unsupported boundary |
| 19 Conformance cases | Tests named M1–M4/M13–M16/M22 and independent type rules above | Other cases are pending, not credited |
| 20 Function inventory | Semantic Rust entry points above | Native addresses/layouts and unimplemented inventory entries are not exported |

Invalid zooms are rejected/ignored with SPEC-GAP comments; malformed image
descriptors return `TintError`. The required host image/spiral/timer services
remain interfaces, without drawing, window, simulator or operating-system calls.

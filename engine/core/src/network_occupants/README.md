# Network occupants

## Owner closure

This inventory distinguishes complete specified owner behavior from partial
helpers. `cSC4NetworkOccupantModelInstance` is closed for the published contract:
its only Unsupported path is the six-plane mode-0 cut expressly excluded by
network-occupants §10.3 (NO29). Bind constants are mandatory foreign inputs
(NO27); `ProcessorLighting` is the external `cS3DModelProcessor::LightModel`
primitive (NO28), not a replacement network-owner implementation. It must be
supplied, has no default, and runs before ReleaseModel. Tests use fixture
constants and a processor double, and do not establish native lighting colours.
NO31 asks about unpublished native constructor/destructor effects; no effects
beyond the specified resource/mesh/processor lifetimes are claimed.

### cSC4NetworkOccupantModelInstance

All section numbers below refer to network-occupants.md. Tests live in
`tests_model.rs`. §10.1/10.2 caller behavior is included to make the lifetime and
pass-order boundary reviewable; it does not close the prebuilt occupant owner.

| Spec section | Code item | Covering test or explicit open remainder |
|---|---|---|
| §10 identity, ordinary members, processor and resource ownership | `NetworkModelInstance`, `create`, Arc-owned resource/arrays | `bind_private_copy_colour_bounds_source_lifetime_and_rejection`, `model_state_is_safe_for_cross_thread_getters` |
| §10.1 SetModel: LOD 0, meshes, bounds, model reference | `NetworkModelInstance::create` | `cache_preserved_on_load_failure_cleared_on_set_model_failure`, `fresh_instances_caller_plus_cache_refs_and_no_accumulated_deformation` |
| §10.1 transforms and alpha | `ModelTransform::new`, `PrebuiltPiece::append` | `draw_pass_orders_option_exception_second_transform_and_alpha`, `draw_uses_view_yaw_path_origin_and_shared_flat_model_when_skipped` |
| §10.1 caller/cache resource lifetimes and release order | `PrebuiltPiece::append`, `shutdown`, `Drop` | `no26_shutdown_guard_release_order_and_reverse_destructor_no_double_release`, `caches_fail_independently_and_zero_id_keeps_both_previous_instances` |
| §10.2 surface/clipped/second/option pass order | `PrebuiltPiece::append` | `draw_pass_orders_option_exception_second_transform_and_alpha`, `draw_dispatches_processor_for_main_and_second_and_honours_skip` (literal reading; NO30) |
| §10.3 Bind (formerly Prelight) | `NetworkModelInstance::bind`, `ProcessorConstants` | `bind_private_copy_colour_bounds_source_lifetime_and_rejection`; foreign numeric inputs NO27 |
| §10.3 Deform: planes, rotation, flip, origin, diagonal, additive height and bounds | `Deformation`, `NetworkModelInstance::deform` | `deform_two_planes_centred_corners_midpoint_and_extrapolation`, `deform_rotation_flip_origin_relative_y_relief_and_height_point_two` |
| §10.3 FlipAndLight: winding, range rebasing, processor dispatch, transform, ReleaseModel | `NetworkModelInstance::flip_and_light`, `ProcessorLighting` | `flip_changes_transform_row_and_winding_not_positions_or_uvs`, `owner_lights_after_winding_before_release_and_keeps_processor_output` |
| §10.3 Clip 1: null/bounds gates, formats, ranges, outcodes, slab, fan, interpolation, source sharing, clamped bounds | `clip`, `clip_mesh`, `outcode`, `cut`, `lerp_vertex` | `no25_a_fan_and_uvs_b_single_triangle_c_drop_and_inclusive_boundary`, `no25_d_outside_anchor_f32_t_and_extended_colour_truncation`, `clip_both_planes_shared_source_nonshared_cuts_and_range_rewrite`, `clip_bounds_are_clamped_not_rebuilt_empty_unsupported_and_no_ranges`, `clip_unordered_bounds_nan_coordinate_and_whole_triangle_x_rejection` |
| §10.3 Clip 0: null/contained early-out; six-plane cut unspecified | `clip`, `UnspecifiedClipMode` | `mode_zero_only_rejects_the_unspecified_cut`; open: NO29 |
| §13 test 24: returned instance, corner-height deformation | `PrebuiltPiece::get_model_instances`, `Deformation::from_corners` | `model_conformance_24_returns_separate_instance_with_corner_heights` |

### Remaining owners (not closed)

These rows are an audit inventory, not Unsupported implementations or coverage
claims. A missing implementation of specified behavior still prevents closure.
The model-only subset above is the closure delivered in this run.

| Owner | Spec sections | Code / test / remaining work |
|---|---|---|
| cSC4NetworkOccupant | network §§1, 2.1–2.3 | `identity`, `NetworkPiece`, `NetworkBaseRecord`, `Xform`; `tests::{road_piece_identity,class_ids_and_types}`, `tests_record::network_record_v8_layout_and_round_trip`; base layout represented semantically |
| cSC4NetworkOccupant | network §2.4 | `NetworkPiece` orientation/cell/cost (`tests::{orientation_flip_rotation,occupied_cell_rounding}`); position, quad/UV/triangle and bbox operations remain; open: NO14 (slot discrepancy), NO32 (UV data) |
| cSC4NetworkOccupant | network §2.5 | `NetworkPiece::{init,shutdown}`, `tests::init_lifecycle`; StaticInit cache/message/render-state operations remain |
| cSC4NetworkOccupant | network §2.6 | `NetworkPiece::{add_path_connection,path_connection}`, `tests::path_connections`; path-cache build/release/reset remain; open: NO12 (matrix interpretation) |
| cSC4NetworkOccupant | network §2.7 | flags and managed status in `NetworkPiece`, `tests::{usable_message,managed_globally}`; SetLit/UpdateLighting/Draw remain; open: NO32 (rendering contract) |
| cSC4NetworkOccupant | network §§7.1, 11, 13 tests 1–12,25 | identity/flag/record/cell tests exist; draw and complete city placement integration absent; no owner closure |
| cSC4NetworkOccupantWithPreBuiltModel | network §§1, 2.8 | `identity`, `NetworkPiece` partial init/shutdown (`tests::prebuilt_and_tunnel_init`); complete constructor, model setup, path info and record remain |
| cSC4NetworkOccupantWithPreBuiltModel | network §§10.1–10.3 | `PrebuiltPiece` draw/cache behavior (`tests_model`); stable Arc OOM abort differs from native allocation-failure cache branch; no prebuilt owner closure |
| cSC4NetworkOccupantWithPreBuiltModel | network §§11,12,13 | prebuilt record and inherited base gaps prevent closure; open: NO30 |
| cSC4UndergroundOccupant | network §3.1 | `UndergroundPiece` partial fields/heights; extended geometry, wall/zipper helpers, Init, StaticInit and full Shutdown remain |
| cSC4UndergroundOccupant | network §3.2 | `UndergroundPiece` position/sort/model-choice/imaginary-connection helpers; no complete geometry lifecycle |
| cSC4UndergroundOccupant | network §§3.3–3.5,11,13 test 13; underground §§1,2,3,5–9 | `UndergroundRecord`, `UndergroundTail`, `tests_record` cover records; pipe/subway override helpers do not close their abstract base |
| cSC4UndergroundConnectionOccupant | network §3.6; underground §§1,4–9 | `identity` only; connection state, lifecycle, geometry and record remain |
| cSC4PowerLineOccupant | network §§1,5.1 | `PowerLine`, `tests::{pole_line_pylon_qi,power_line_slots}`; base shutdown forwarding and resolved live layout remain; open: NO13 |
| cSC4PowerLineOccupant | network §§5.2,11,13 tests 2,18 | raw `PowerLineRecord`, `tests::power_line_record_round_trip`; live/record mapping unresolved (NO13); creation belongs to power-line tool |
| cSC4PowerPoleOccupant | network §§1,4.1–4.2 | `identity`, `tests::pole_line_pylon_qi`; pole object, model, connections, lifecycle and cost remain |
| cSC4PowerPoleOccupant | network §4.3 | `PowerLineControl`, `tests::control_variables` only |
| cSC4PowerPoleOccupant | network §§4.4,11,13 tests 2,4,16,17 | pole record and connection initialization remain |
| cSC4PylonOccupant | network §§1,6.1–6.3,11,13 tests 2,19,20 | `identity`, `tests::{pole_line_pylon_qi,class_ids_and_types}` only; state, setters, lifecycle, drawing and record remain |

Network §§0,8,9 are overview/other owners. Section 13 tests 14–17,19–23,25
exercise other owners or city integration; they are not model-instance closure
tests. No real-game behavior is measured here: this module has no ABI/plugin
wiring.

## Placement demolition helper

`placement_demolition::compute_demolition_region_for_placement_cells` implements
`docs/specs/network-routines.md` §§2–8: accumulate bounds and lot rectangles from
the input cells, publish a cleared region whose stored highs include the extra
clear border, scan the lot rectangles for zoned cells whose filtered occupant
lookup misses, reject on a `0xFC00` zone-type mask, and otherwise run the
demolition preview with the temporary `ExclusionFilter` (QI `0xA9764C8C`
complement) and return its result unchanged.

The host trait exposes only the individual foreign primitives (lot manager, lot
lookup and bounds, zone byte, filtered first-occupant lookup, QI/release,
DemolishRegion). Regions use the shared `zones::CellRegion`; city sides
64/128/256 give region sides of at most 257.

`RegionOutput` keeps any displaced region instead of dropping it, matching the
spec's "the helper does not destroy the previous value".

Not covered: the native region/filter memory layout, allocation-failure branches
(including the null-filter preview), malformed or out-of-city input (reported
as `Unsupported`, which is this crate's validation policy, not native
behaviour; see NRT1). Nothing is wired into the C ABI or the plugin; the
`network_rules` callers keep their existing foreign bridge.

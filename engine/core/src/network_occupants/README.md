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
The model-only subset above remains the only closed owner. Round 2 adds
published pole, pylon and connection contracts below, with **zero new whole
owner closures**: the source specs omit required algorithms or give conflicting
layouts. Open rows are missing contracts, not newly invented Unsupported paths.

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
| cSC4UndergroundConnectionOccupant | network §3.6; underground §§1,4–9 | `ConnectionRecord`, `UndergroundConnection` record/shutdown/hidden-triangle/subscription contracts; `tests_connection`; open: UG1, UG5; detailed table below |
| cSC4PowerLineOccupant | network §§1,5.1 | `PowerLine`, `tests::{pole_line_pylon_qi,power_line_slots}`; base shutdown forwarding and resolved live layout remain; open: NO13 |
| cSC4PowerLineOccupant | network §§5.2,11,13 tests 2,18 | raw `PowerLineRecord`, `tests::power_line_record_round_trip`; live/record mapping unresolved (NO13); creation belongs to power-line tool |
| cSC4PowerPoleOccupant | network §§1,4.1–4.2 | `PowerPole`, `PoleFields`, `PoleConnection`, `PoleStatics`; `tests_power_pole`; open: NO33, NO34; detailed table below |
| cSC4PowerPoleOccupant | network §4.3 | `PowerLineControl`, `PoleStatics::init_once`, `PowerPole::init_published_state`; open: NO33 |
| cSC4PowerPoleOccupant | network §§4.4,11,13 tests 2,4,16,17 | type/cost-word/table-selection tests; native pool, pole record and cost formula remain open: NO33-NO35 |
| cSC4PylonOccupant | network §§1,6.1–6.3,11,13 tests 2,19,20 | `PylonRecord`, `Pylon`, `PylonGlobals`; `tests_pylon`; open: NO36, NO37; detailed table below |

Network §§0,8,9 are overview/other owners. Section 13 tests 14–17,19–23,25
exercise other owners or city integration; they are not model-instance closure
tests. No real-game behavior is measured here: this module has no ABI/plugin
wiring.

### cSC4PowerPoleOccupant (open)

All sections below refer to network-occupants.md. Tests are in
`tests_power_pole.rs` unless qualified. The Rust state is semantic: the native
pool's 0x124 allocation is metadata, not a claim about `size_of::<PowerPole>()`.
No raw pole record is guessed from contradictory offsets. Constructor inputs
for unpublished state are mandatory rather than assumed zero defaults.

| Spec section | Code item | Covering test or open remainder |
|---|---|---|
| §1.1 | `NetOccClass::PowerPole`, `PowerPole::new` | `tests::class_ids_and_types`, `connection_views_and_published_state`; native pool open: NO33 |
| §1.2 | `identity::{query_interface,null_this_returns_true}` | `tests::pole_line_pylon_qi`; delegate identity open: NO2 |
| §1.3 | `NetOccClass::class_id` | `tests::class_ids_and_types` |
| §1.4 | network-piece flag query belongs to other owner | no pole IsOfType algorithm assigned |
| §4.1 | `PoleFields`, `PoleConnection`, `PowerPole`, `ConnectionPointTable` | `connection_views_and_published_state`; missing defaults/coordinates, conflicting cell and model/record offsets open: NO33, NO35 |
| §4.2 | connection count/views, cell, distressed/ignored getters, field-118 storage, `PoleStatics::demolition_cost` | `connection_views_and_published_state`, `static_controls_load_once_and_cost_preserves_both_words`; mutation, render, geometry, zoom, output slots and resource algorithms open: NO33; quote formula open: NO34 |
| §4.3 | `PowerLineControl::load`, `PoleStatics::init_once`, `PowerPole::init_published_state` | `tests::control_variables`, `published_init_copies_models_and_clears_geometry_in_order`, `static_controls_load_once_and_cost_preserves_both_words`; LoadModel internals, full point init and SetDistressed effects open: NO33 |
| §4.4 | no fabricated codec or live reconciliation | open: NO35; common wall helper itself is defined by §3.4 |
| §11 quirks 2,6,11,12 | null-this metadata and signed 64-bit static cost | `tests::pole_line_pylon_qi`, static-cost test; record quirks open: NO35 |
| §13 test 2 | pole occupant type | `connection_views_and_published_state` |
| §13 test 4 | `POWER_POLE_POOL_OBJECT_SIZE` records native size | open: NO33; constant assertion does not establish a native pool implementation |
| §13 test 16 | no fabricated record | open: NO35 |
| §13 test 17 | static low/high words return an int64 unchanged | `static_controls_load_once_and_cost_preserves_both_words`; exemplar-to-cost formula open: NO34 |

### cSC4PylonOccupant (open)

Tests are in `tests_pylon.rs`. Raw record fields are deliberately distinct
from call/drag state and the shutdown reference because the spec aliases them
inconsistently. `read_into` preserves existing v5-only tail state when reading
v3/v4, releases the source before reading its replacement, and takes the legacy
path for both transforms below v4: these literal choices are recorded in NO37.
No host callback supplies a pylon-owned rendering/texture/geometry result.

| Spec section | Code item | Covering test or open remainder |
|---|---|---|
| §1.1 | `Pylon::from_state`, `NetOccClass::Pylon` | `type_and_call_drag_state`, `tests::class_ids_and_types` |
| §1.2 | `identity::{query_interface,null_this_returns_true}` | `tests::pole_line_pylon_qi`; contradictory generic class-IID prose open: NO10 |
| §1.3 | `NetOccClass::class_id` | `tests::class_ids_and_types` |
| §1.4 | network flag query belongs to other owner | no pylon IsOfType algorithm assigned |
| §6.1 | `PylonRecord`, `Pylon` supplied-state construction | `type_and_call_drag_state`; matrix/default and live-offset reconciliation open: NO36, NO37 |
| §6.2 | call/drag getters/setters, source getter, `init_published_lifecycle`, `shutdown_published_lifecycle`, `PylonGlobals` | `type_and_call_drag_state`, `lifecycle_order_inserted_and_uninserted`; render/texture/setter/geometry algorithms open: NO36; exact placement flags/renderer and source-offset reconciliation open: NO37 |
| §6.3 | `PylonRecord::{write,read_into}` | `v5_record_order_both_transforms_and_references`, `legacy_v3_and_current_v4_preserve_absent_tail`, `truncated_records_fail_at_every_boundary`, `failed_reference_read_has_already_released_old_source`; live mapping/ref IIDs/legacy interpretation open: NO37 |
| §11 quirks 6,8 | v5 write, v3/v4 read and own occupant type | record/type tests above; live mapping open: NO37 |
| §13 test 2 | own occupant type | `type_and_call_drag_state` |
| §13 test 19 | v5 two-transform/two-ref stream, legacy read | `v5_record_order_both_transforms_and_references`, `legacy_v3_and_current_v4_preserve_absent_tail`; exact legacy interpretation open: NO37 |
| §13 test 20 | shutdown base first, inserted removal, three unsubscriptions | `lifecycle_order_inserted_and_uninserted`; full placement side effects open: NO37 |

### cSC4UndergroundConnectionOccupant (open)

Tests are in `tests_connection.rs`. The shared §3.4 wall codec is reused for all
seven lists and preserves alpha. The record retains all specified raw fields;
it is not mapped onto conflicting runtime references. No base model/drawing
behavior is inferred from pipe/subway consumers.

| Spec section | Code item | Covering test or open remainder |
|---|---|---|
| network §1.1 | `NetOccClass::UndergroundConnection` | `tests::class_ids_and_types`; base leaves occupant type to subclasses |
| network §1.2 | `identity::query_interface` | `tests::subclass_qi_chains` |
| network §1.3 | `NetOccClass::class_id` | `tests::class_ids_and_types` |
| network §1.4 | network-piece flag query belongs to other owner | no connection IsOfType algorithm assigned |
| network §3.6 | `ConnectionRecord`, `UndergroundConnection::{hidden_triangles,shutdown,init_published_subscriptions}` | `seven_wall_record_order_round_trip_and_truncation`, `hidden_triangles_subscriptions_and_shutdown_order`; callback, packed states, creation/model/geometry and reference mapping open: UG1, UG5 |
| underground §1 | existing class/QI identity; base has no pipe/subway type selection | `tests::{class_ids_and_types,subclass_qi_chains}`; subclass factories are other owners |
| underground §4 | subclasses' type/network/QI/sort are other owners; base record implemented | seven-wall record test; subclass folded ABI entries require no base-owner arithmetic |
| underground §5 | `NetOccClass::sub_objects` view metadata | `tests::subclass_qi_chains`; native adjusting thunks are ABI glue outside this core-only task |
| underground §6 | callers, not additional base-owner behavior | no base-owner test obligation |
| underground §7 | quirk 5 assigns constant sort to subclasses | subclass behavior, not base-owner implementation |
| underground §8 | explicitly open base drawing/model building/descriptor use | open: UG1; further explicit-body gaps UG5 |
| underground §9 tests 1,7 | existing subclass identity/QI metadata | `tests::{class_ids_and_types,subclass_qi_chains}`; subclass runtime QueryOccupantInterface/sort not claimed; tests 2-6,8 assign pipe/subway behavior, not this owner |
| network §13 test 15 | seven wall writes in base record | seven-wall record test; pipe connection save class and subway runtime QI are subclass obligations |

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

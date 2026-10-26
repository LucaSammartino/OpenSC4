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

### Other owner audit

The detailed tables map pole, pylon and connection contracts to core code and
tests. Power pole, its connection record and power line are implemented for the
published core contracts, as is pylon after applying NO36; base, prebuilt,
underground and underground connection still have implementation work.
NO12–NO14, NO30, NO32–NO36, NO37 and NO43 are answered in the spec;
the rows below distinguish applied answers from remaining implementation work.
UG5/NO40 retain the runtime observation in network §3.6.6 (NO40-C).

| Owner | Spec sections | Code / test / remaining work |
|---|---|---|
| cSC4NetworkOccupant | network §§1, 2.1–2.3 | `identity`, `NetworkPiece`, `NetworkBaseRecord`, `Xform`; `tests::{road_piece_identity,class_ids_and_types}`, `tests_record::network_record_v8_layout_and_round_trip`; base layout represented semantically |
| cSC4NetworkOccupant | network §2.4 | NO14 applied by `NetworkPiece::{get_rotation,get_flip,get_orientation,set_variation,set_under_texture}`, `tests::orientation_flip_rotation`; NO32 numeric UV/zoom/triangle inputs: `surface`, `surface::tests::numeric_tables_zoom_and_independent_halves`; full geometry/draw integration remains |
| cSC4NetworkOccupant | network §2.5 | `NetworkPiece::{init,shutdown}`, `tests::init_lifecycle`; StaticInit cache/message/render-state operations remain |
| cSC4NetworkOccupant | network §2.6 | NO12 applied by `NetworkPiece::{add_path_connection,path_connection}` (eight type rows of four entry bytes), `tests::path_connections`; traffic-owned path-cache build/release/reset remain foreign |
| cSC4NetworkOccupant | network §2.7 | flags and managed status in `NetworkPiece`, `tests::{usable_message,managed_globally}`; NO32 numeric normal/RGB code and tests are in `surface`; SetLit/corner lighting/Draw integration remain implementation work, not an unanswered NO32 |
| cSC4NetworkOccupant | network §§7.1, 11, 13 tests 1–12,25 | identity/flag/record/cell tests exist; draw and complete city placement integration absent; no owner closure |
| cSC4NetworkOccupantWithPreBuiltModel | network §§1, 2.8 | `identity`, `NetworkPiece` partial init/shutdown (`tests::prebuilt_and_tunnel_init`); complete constructor, model setup, path info and record remain |
| cSC4NetworkOccupantWithPreBuiltModel | network §§10.1–10.3 | `PrebuiltPiece` draw/cache behavior (`tests_model`); stable Arc OOM abort differs from native allocation-failure cache branch; no prebuilt owner closure |
| cSC4NetworkOccupantWithPreBuiltModel | network §§11,12,13 | NO30 applied by `PrebuiltPiece::append`, `tests_model::{draw_pass_orders_option_exception_second_transform_and_alpha,draw_dispatches_processor_for_main_and_second_and_honours_skip}`; prebuilt record and inherited base implementation gaps still prevent owner closure |
| cSC4UndergroundOccupant | network §3.1 | `UndergroundPiece` partial fields/heights; extended geometry, wall/zipper helpers, Init, StaticInit and full Shutdown remain |
| cSC4UndergroundOccupant | network §3.2 | `UndergroundPiece` position/sort/model-choice/imaginary-connection helpers; no complete geometry lifecycle |
| cSC4UndergroundOccupant | network §§3.3–3.5,11,13 test 13; underground §§1,2,3,5–9 | `UndergroundRecord`, `UndergroundTail`, `tests_record` cover records; pipe/subway override helpers do not close their abstract base |
| cSC4UndergroundConnectionOccupant | network §3.6; underground §§1,4–9 | `ConnectionRecord`, `UndergroundConnection`, `ConnectionStatics`; UG5/NO40 settled bounds/descriptor/messages/Draw applied in `connection/pipeline.rs`; open only: NO40-C runtime observation; detailed table below |
| cSC4PowerLineOccupant | network §§1,5.1 | implemented: NO13 applied by `PowerLine::{set_connected_poles_acquired,get_connected_poles_acquired,shutdown_acquired}`; `power_line::lifecycle_tests::line_layout_and_shutdown_forwarding_conformance_74`, `tests::{pole_line_pylon_qi,power_line_slots}`; native view adjustment remains adapter responsibility |
| cSC4PowerLineOccupant | network §§5.2,11,13 tests 2,18 | implemented: `PowerLine::{from_record,record}`, `PowerLineRecord`; `tests::{power_line_record_round_trip,power_line_live_record_keeps_saved_cells_and_float_bits}`; creation belongs to power-line tool |
| cSC4PowerPoleOccupant | network §§1,4.1–4.2 | implemented: identity, constructor, getters, model, bounds, pick and pool policy; NO33/NO43 applied; detailed table below |
| cSC4PowerPoleOccupant | network §4.3 | implemented: `PoleStatics::initialize`, `PowerPole::{init,set_default_exemplar}`; `pole_init_guard_exemplar_refresh_and_destruction_conformance_71_73`; legacy `init_published_state` remains a prefix helper |
| cSC4PowerPoleOccupant | network §§4.4,11,13 tests 2,4,16,17,62 | implemented: nested record/shared tables/IIDs/cleanup/quote and portable pool policy; `pool::tests::native_pool_policy_conformance_62` plus record/quote tests below |
| cSC4PylonOccupant | network §§1,6.1–6.3,11,13 tests 2,19,20,40–42,45–48,52–53,56–59 | implemented: NO36 applied by `pylon/composition.rs`; constructor, record, placement, bitmap/label/billboard, Draw and messages covered by `tests_pylon` and `pylon/composition/tests.rs`; detailed table below |

Network §§0,8,9 are overview/other owners. Section 13 tests 14–17,19–23,25
exercise other owners or city integration; they are not model-instance closure
tests. The plugin's `netocc_shadow` compares the record codecs through a private
transport; it does not exercise these core owner geometry/Draw APIs or establish
native rendered-output agreement.

### cSC4PowerPoleOccupant

`power_pole.rs` owns the settled contracts below; tests are in
`tests_power_pole.rs` unless qualified. Status: implemented (NO33–NO35/NO43).
Model/lifecycle/bounds/Pick/Draw are in `power_pole/model.rs`, connection
construction/refresh in `power_pole/links.rs`, pool policy in `power_pole/pool.rs`.
Terrain, manager, stream, renderer and effect calls remain foreign primitives.
Pool tokens represent the specified offsets and rings, not native addresses;
the adapter owns actual native storage and interface adjustment. `fresh()`
implements constructor defaults; `new(fields)` and `init_published_state` are
explicit state/prefix helpers, while `init` implements the full lifecycle.
Exemplar identity/lifetime belongs to the base holder, including after Read;
`model::tests::unchanged_exemplar_uses_base_holder_identity_including_loaded_state`
covers its unchanged/changed decisions without a duplicate pole cache.

| Spec section | Code item | Covering test or explicit open remainder |
|---|---|---|
| network §1.1, §1.3 identity/type | `NetOccClass::PowerPole`, `PowerPole::fresh` | `tests::class_ids_and_types`, `pole_constructor_shared_tables_and_geometry_conformance_43_44` |
| network §1.2, §4.1 QI/delegate and null-this | `identity::{query_interface,null_this_returns_true}`; delegate is a foreign interface query | `tests::pole_line_pylon_qi`; core-only metadata, no ABI thunk claim |
| network §1.4 | network flag queries belong to the network-piece owner | no additional pole algorithm |
| network §4.1 constructor/layout/pool | `PoleFields::default`, `PowerPole::fresh`, `PoleConnection`, `pool::PolePool` | `pole_constructor_shared_tables_and_geometry_conformance_43_44`, `pool::tests::native_pool_policy_conformance_62` (112 blocks, full marker, 84 threshold, deletion/free-list/sole-pool behavior) |
| network §4.2 connection/cell/preview/highlight/sort/model enumeration | getters, `set_preview`, `set_highlight`, `sort_value`, `model_instances` | `connection_views_and_published_state`, `pole_connections_effects_messages_and_model_attributes_conformance_38_39` |
| network §4.2 bounds/transform/Pick, §4.3 Init/model/default-exemplar refresh | `model::{occupant_bounds,local_bounds,pick,get_transform,set_transform,init,set_default_exemplar}` | `model::tests::{occupant_bounds_conformance_54_55,pole_pick_conformance_68,own_transform_and_model_query_do_not_move_position_or_delegate,pole_init_guard_exemplar_refresh_and_destruction_conformance_71_73}`; NO33/NO43 applied |
| network §4.3 tuning/quote/static lifetime | `PowerLineControl::load`, `PoleStatics::{initialize,demolition_cost,shutdown}`, `POLE_TEXTURES` empty bindings | `tests::control_variables`, `pole_quote_conversion_once_and_independent_controls_conformance_17`, `static_controls_load_once_and_cost_preserves_both_words`; variant conversion is foreign |
| network §4.3 insertion message / §4.7 shutdown | `on_inserted_message`, `insert_lines`, `shutdown` | `pole_connections_effects_messages_and_model_attributes_conformance_38_39` |
| network §4.4 record gates/codec/shared table/cleanup | `PowerPole::{write,read_into}`, nested `PoleConnection::vertices`, `read_wall_into` | `pole_nested_record_iids_shared_overwrite_and_failure_cleanup_conformance_16`, `pole_legacy_v4_future_versions_and_raw_bool_read` |
| network §4.5 creation/consumption | power-line tool and power simulator are foreign owners | no additional pole algorithm |
| network §4.6 table selection/points and shared overwrite | `AttachmentTables`, `init_connection_points`, `attachment_points` | `pole_constructor_shared_tables_and_geometry_conformance_43_44`, pole record tests |
| network §4.6 insert/remove/break and retained records | `insert_lines`, `remove_connection_record`, `remove_connection`, `break_all_connections` | `pole_connections_effects_messages_and_model_attributes_conformance_38_39` |
| network §4.6 line-cell lookup/delegation; wire construction/cell walk | `line_cells`, `line_cells_to`, `links::{add_connection,rebuild_wires,enable_connection}` | `pole_cell_walk_conformance_65`; `links::tests` cover counted duplicates, nonowners, unsigned ordering, wet refresh accumulation and coincident cells; NO33/NO43 applied |
| network §4.7 distress | `set_distressed`, `stop_distress_effect` | `pole_connections_effects_messages_and_model_attributes_conformance_38_39` |
| network §4.7 position/model/zoom/Draw/DrawPowerlines | `model::{set_position_with_model,load_model,change_zoom_level,draw}`, `wire_quads`, `wire_submissions` | `model::tests::{model_resource_failure_preserves_old_delegate_success_releases_in_order,zoom_always_rebuilds_model_and_geometry_but_textures_only_on_shared_change,draw_has_three_lighting_queries_two_shade_resets_and_inherited_wire_state,failed_model_load_does_not_suppress_faces_and_null_plus_zoom_change_loads_twice}`, `pole_wire_quads_conformance_69` |
| network §4.7.1 floor/wall inputs, diagonal rotation, alpha, UVs, ordered edges | `create_floor`, `wall_quad`, `create_walls` | `pole_constructor_shared_tables_and_geometry_conformance_43_44`, `pole_diagonal_floor_conformance_66`; NO43 applied |
| network §7.2 reference ownership, §11 quirks 2,6,11,12,21,22,24,25 | distinct ref IIDs, shared table mutation, retained records, signed quote, highlight | record, quote and connection tests above; safe `Option` getter exposes the native unchecked-index precondition |
| network §11 quirk 23 off-map position | `set_position`, `model::set_position_with_model` | `pole_position_corner_ties_nan_off_map_and_extended_reciprocal`, `model::tests::pole_position_updates_delegate_but_keeps_own_transform_and_clean_bounds`; off-map floor height uses position.y |
| network §12 / §13 tests 2,4,16,17,35,38,39,43,44,54,55,62–73 | implementations/tests above, `model::destroy_resources` | tests above plus `pole_distress_retry_and_stop_modes_conformance_73`; test 35 native forwarding is adapter work, not a core algorithm |

### cSC4PowerPoleOccupant::tConnection

`PoleConnection` is the connection-record owner within `power_pole.rs`.
Status: implemented (NO33/NO43). Native container allocation is adapter work;
the portable owner supplies counted refs, copied curves, cell walking and refresh.

| Spec section | Code item | Covering test or explicit open remainder |
|---|---|---|
| network §§4.1,4.4 record/layout/reference ownership | `PoleConnection`, nested `vertices` codec, `PowerPole::{write,read_into}` | `pole_nested_record_iids_shared_overwrite_and_failure_cleanup_conformance_16`, `pole_legacy_v4_future_versions_and_raw_bool_read`; distinct pole/line IIDs and nested empty vectors |
| network §4.6 insertion/removal/retained records | `insert_lines`, `remove_connection_record`, `remove_connection`, `break_all_connections` | `pole_connections_effects_messages_and_model_attributes_conformance_38_39` |
| network §4.6 line-cell lookup/delegation/curve refresh | `line_cells`, `line_cells_to`, `links::{add_connection,rebuild_wires,enable_connection}` | `pole_cell_walk_conformance_65`; `links::tests::{owning_connection_copies_duplicate_lines_once_each_and_uses_computed_start,refresh_appends_water_curves_preserves_lines_and_stored_destination,enable_probes_even_with_no_match_and_uses_stored_unsigned_cell}`; NO33/NO43 applied |
| network §7.2, §11 quirks 6,21, §13 tests 16,39 | counted refs, retained records, nested vectors | record and connection tests above; no additional settled algorithm |

### cSC4PylonOccupant

`pylon.rs`, tests in `tests_pylon.rs`. There is one transform and one source ref;
the source serialized at displacement +6C is exactly the source released at
live +78. `fresh(retained_dispatch_type)` initializes UI=1 and dispatch state=0;
the uninitialized native dispatch-type word is an explicit retained input (NO44
answered). `new(states)` remains an explicit live-state seed. Position executes
both placement attempts around the implemented `rebuild_model`. Status:
implemented (NO36); `pylon/composition.rs` owns the settled bitmap, label,
billboard, Draw and message algorithms. Foreign primitives cannot supply a
precomposed native-owner bitmap, billboard or bounds.

| Spec section | Code item | Covering test or explicit open remainder |
|---|---|---|
| network §§1.1–1.3 identity/type/QI | `NetOccClass::Pylon`, `PylonRecord::new` | `tests::{class_ids_and_types,pole_line_pylon_qi}`; NO10 settled: type `0xA9B6D7DE` |
| network §1.4 | network-piece flag queries are another owner | no pylon-specific flag query |
| network §6.1 fields/defaults | `Pylon::{fresh,new}`, `PylonRecord::new`, cached boxes, texture/ref state | `pylon_constructor_conformance_45`, `pylon_lifecycle_packing_and_all_placement_edges`, record tests; dispatch type retains caller-supplied incoming word |
| network §6.2 Init/guard/packing/shutdown | `Pylon::{init,shutdown}`, `PYLON_RENDER_STATES`, `PylonGlobals` | `pylon_lifecycle_packing_and_all_placement_edges`; renderer probe result intentionally discarded |
| network §6.2 placement | `place_in_manager`, `place_in_renderer` | `pylon_lifecycle_packing_and_all_placement_edges` (matching states, null services, failures, other-bit preservation) |
| network §6.2.1 enums/bitmap/call/drag/ref setters | setters, `same_ref`, `replace_ref` | `pylon_setters_references_and_shared_blink_conformance_40_41` |
| network §6.2.1 animation | `animation_tick`, shared `PylonGlobals::blink_accumulator` | `pylon_setters_references_and_shared_blink_conformance_40_41` (sharing, discarded excess, modulo wrap, ineligible state) |
| network §6.2.1 position/visibility | `get_position`, `set_position`, `set_visibility`, `rebuild_model` | `pylon_position_visibility_and_model_refresh_conformance_42`, `main_billboard_and_registration_conformance_57` |
| network §6.2.2 cached local/world/occupant bounds, sort, transform copies | `bounds`, `world_bounds`, `get_bounding_box`, `sort_value`, `get_transform`, `set_transform` | `pylon_cached_bounds_and_transforms_conformance_46`; getters never rebuild |
| network §6.2.2 Pick | `pick`, foreign view/ray primitives in `PylonPickHost` | `pylon_pick_conformance_47`; UI gate, adjusted box, strict nearest distance, tie/miss preservation |
| network §6.2.2 ordinary texture selection/replacement | `rebuild_current_texture`, `PylonTextureHost` | `pylon_texture_replacement_conformance_48`; all table entries, ref/raw release, null binding and stale ID; state outside 0–6 explicitly unsupported by spec |
| network §6.2.3 Draw/messages | `draw`, `do_message` | `pylon::composition::tests::pylon_draw_pass_dirty_and_message_conformance_52_53`; dirty/pass gates, zero texture IDs, animation quotient and visibility transitions |
| network §6.2.4 MySim composition/resource lifetime | `compose_mysim`, `BitmapHost` | `mysim_pixel_predicate_and_resource_lifetimes_conformance_56`; three copies, protected/magenta pixels, background selection, absent portrait/background and owned releases |
| network §6.2.4 billboard/bounds/registration | `rebuild_model`, `PylonModelHost` | `main_billboard_and_registration_conformance_57`; float-store boundaries, repeated transform reset, failed texture continuation, blink and renderer/manager refresh |
| network §6.2.4 call-number texture/label geometry | `rebuild_label` | `label_replacement_geometry_and_failed_bitmap_conformance_58_59`; power-of-two size, failed bitmap/lock, font restore, UVs and call-zero retention |
| network §6.3 record | `PylonRecord::{write,read_into}` | `pylon_record_bytes_one_transform_signed_enums_and_iids`, `pylon_v3_v4_future_and_tail_preservation` (one transform, signed bytes/u16/string, ref cleanup/IIDs, v3/v4/future) |
| network §7.2, §11 quirks 6,8 | source/owner ownership, v5 write, type | record and lifecycle tests above; §6.3's corrected live offsets/IIDs govern older §7.2 shorthand |
| network §12 / §13 tests 2,19,20,40–42,45–48 | implementations/tests above | test 42's RebuildModel step remains explicitly open: NO36 |

### cSC4UndergroundConnectionOccupant

`connection.rs`, tests in `tests_connection.rs`. The record is the live source
of attached/linked refs: no duplicate shadow references. Host adapters adjust
views without an extra AddRef. UG5/NO40 settled owner code is in
`connection/pipeline.rs`. `ConnectionUnsupported::RuntimeObservationNO40C`
retains exactly the descriptor yaw matrices, attached ceiling side argument,
bounded native descriptor arrays and selected shade input observation in §3.6.6.
Only the published byte+28=0/discrete-rotation-0/unattached matrix case is
implemented; arrays and shade are explicit input data, never owner output oracles.

| Spec section | Code item | Covering test or explicit open remainder |
|---|---|---|
| network §§1.1–1.3; underground §1 | identity metadata, `ConnectionKind::{class_id,occupant_class,network_type,occupant_type}` | `tests::{class_ids_and_types,subclass_qi_chains}`, creation test; base does not assign a pipe/subway type |
| network §1.4 | network-piece flags belong to another owner | no connection-specific flag query |
| network §3.6 record, §7.2; underground §4 | `ConnectionRecord::{write,read_into}`, `read_wall_into` | `connection_record_exact_version_seven_lists_refs_and_partial_failure` (exact v2, seven independent lists, IIDs, attached −4, post-read latches, reached mutations) |
| network §3.6/3.6.1 static Init/callback/packing/subscriptions | `ConnectionStatics::{initialize,setup_packed_states,shutdown}`, `UndergroundConnection::init` | `connection_statics_latch_packing_enumeration_and_position_conformance_13` |
| network §3.6.1 creation surface/matrices/search/foreign network changes, fresh/loaded refs and descriptor identity | `UndergroundConnection::create`, `NEIGHBORS`, `descriptor_kind`, `compute_box` | `connection_creation_search_order_fresh_loaded_and_own_hit_conformance_10_11`, `descriptor_deformation_bounds_and_latch_worked_cases` |
| network §3.6 slot 41 shared texture reload; underground §4 | `ConnectionTextures::reload`, `ConnectionTextureHost` | `connection_shared_texture_reload_release_null_and_zoom`; each entry release/null/request, changed zoom and wrapping instance key |
| network §3.6.2 floor/wall builders; underground §4 | `create_floor`, `create_walls`, `add_wall_quad`, `model_state_changed` | `connection_unattached_builders_conformance_49`, `connection_wall_slope_append_conformance_50`, `descriptor_deformation_bounds_and_latch_worked_cases`; false-latch bounds/visibility notification applied |
| network §3.6 hidden triangles/ref setter/refresh/shutdown/post messages | `hidden_triangles`, `set_linked_occupant`, `refresh_attached`, `shutdown`, `post_occupant_message` | `connection_linked_refs_refresh_messages_query_and_shutdown` |
| network §3.6.1 ModelStateChanged | `model_state_changed`, `compute_box` | `descriptor_deformation_bounds_and_latch_worked_cases`, `connection_linked_refs_refresh_messages_query_and_shutdown`; one notification on false latch, computed surface/original-height bounds |
| network §3.6.1; underground §4 enumeration/position | `model_instances`, `set_position`, `position` | `connection_statics_latch_packing_enumeration_and_position_conformance_13` |
| network §3.6; underground §4 subclass slot 44/sort | `query_occupant_interface`, `sort_value` | `connection_linked_refs_refresh_messages_query_and_shutdown`; first QI is released, second ref is returned owned |
| network §3.6.4 DoMessage; underground §8 | `do_message`, `ConnectionMessage` | `connection_attachment_and_removal_messages_conformance_60_61`; counted equal-pointer attachment, query mismatch/fallback, linked removal order, context-word comparisons |
| network §§3.6.5–3.6.6 descriptor/Draw | `create_geometry`, `create_piece_model`, `draw`, `ConnectionDescriptor` | `descriptor_deformation_bounds_and_latch_worked_cases`, `connection_draw_pass_latch_windows_and_material_replay_worked_case`; open: NO40-C runtime observation above prevents full geometry/transform owner closure; NO41 words retained without interpretation |
| underground §5 thunks | `NetOccClass::sub_objects`; adapter supplies adjusted interfaces | `tests::subclass_qi_chains`; native thunk code is ABI glue outside core |
| underground §6 callers / §7 quirks / §8 questions | no extra owner algorithm beyond rows above | constant subclass sort and saved-flag override tested above |
| network §11 quirk 20 / §12 / §13 tests 15,49–50,60–61; underground §9 tests 1,7,9–13 | record, creation, lifecycle, QI, bounds, enumeration, builder and message tests above | settled UG5/NO40 applied; tests 2–6,8 belong to pipe/subway owners; full owner retains NO40-C |


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

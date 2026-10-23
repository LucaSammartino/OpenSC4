# Model instances

Pure Rust implementation of the **published eight-owner subset** in
`docs/specs/model-instances.md` §0.1. `mod.rs` exports semantic objects, not native
layouts. No C ABI, plugin, renderer registration or game replacement is added.
The module and most conformance tests existed in the base; this closure adds
unbound animated construction, public BuildFrame, resource-side sprite creation
delegation, and additional lifetime tests.

`Reference::constructed` exposes constructor count zero; `Reference::new` supplies
the first counted reference. Interface parts share the allocation and the owner's
native count policy. Getters borrow. `RwLock` protects shared state, including
getters used from a second thread. Native destruction removes the value; Rust's
allocation lifetime does not extend native usability after reference overflow.

Foreign operations are explicit providers: `ModelProvider` supplies resource
frames, headers and bounds; `DrawProvider` renders; `VertexLighting` supplies
lighting-manager colours; `SpriteInstanceBinding` allocates and binds the separate
animated-sprite owner. Resource-side CreateModelInstance implements its temporary
and result references around that binding callback. It assumes successful
allocation because §6 does not establish a safe allocation-failure result.

## Owner closure

Test names below refer to `tests.rs`. Repeated section numbers distinguish owner
contracts. §0.1 defines the boundary for every row: animated convenience slots
0–8, processor assignment, generic persistence-base services, native allocator
mechanics, and sprite binding/rendering algorithms have no published portable
algorithm in this subset. They are not counted as implementations here.
§7 describes callers and adds no owner operation. §8 repeats the cited contracts.

| Owner | Spec section | Code item | Covering test / open question |
|---|---|---|---|
| cS3DModelInstanceBasic | 1: query/count/lifetime | `Interfaces`, `Reference`, `NativeCount(Basic16)` | `reference_query_owner_and_final_release`, `count_widths_zero_releases_and_overflow_destruction` |
| cS3DModelInstanceBasic | 1: owner/custom/properties/attributes | `BasicInstance` getters/setters | `base_delegation_attributes_and_identity`, `owner_replacement_observes_basic_and_static_callback_order` |
| cS3DModelInstanceBasic | 1: bounds/sort/transform/pick/draw/resource | `BasicDerived`, `BasicInstance::{bounds,world_bounds,sort_value,get_transform,set_transform,pick,draw,resource_model}` | `base_delegation_attributes_and_identity` |
| cS3DModelInstanceBasic | 1.1: defaults, full draw bytes, ownership order | `BasicInstance::default`, `DrawAttributes`, `BasicInstance::set_owner` | `published_defaults_draw_record_and_material_isolation`, `owner_replacement_observes_basic_and_static_callback_order` |
| cS3DModelInstanceBasic | 1.2 / 8.1: identity gating | `BasicInstance::get_transform`, `Transform::make_identity` | `transform_identity_wraps_only_when_kind_is_nonzero` |
| cS3DModelInstance | 2: construction/query/count/container access | `StaticInstance::new`, `Interfaces`, `NativeCount(Instance32)`, mesh/material accessors | `static_material_properties_sort_and_mesh_references`, `count_widths_zero_releases_and_overflow_destruction` |
| cS3DModelInstance | 2: inherited custom/owner/draw records/properties | `StaticInstance` getters/setters | `release_counts_are_atomic_and_derived_owner_drops_once`, `published_defaults_draw_record_and_material_isolation` |
| cS3DModelInstance | 2: transform/draw/pick/sort/resource | `StaticInstance::{get_transform,set_transform,draw,pick,sort_value,resource_model}` | `static_transform_counter_bounds_query_and_draw`, `static_material_properties_sort_and_mesh_references`, `nearest_pick_metric_and_transform_are_explicit` |
| cS3DModelInstance | 2: SetModel/material flags | `StaticInstance::set_model_reference` | `ordinary_blend_only_changes_properties`, `published_defaults_draw_record_and_material_isolation` |
| cS3DModelInstance | 2.1: cached local/full world bounds | `StaticInstance::{bounds,refresh_bounds,world_bounds}`, `BoundingBox::transformed` | `static_transform_counter_bounds_query_and_draw`, `asymmetric_full_bounds_ignore_kind_and_reorder_endpoints`; exact transient precision open: MI11 |
| cS3DModelInstance | 2.1: Euclidean pick distance and traversal | `PickContext::{triangle_pick,mesh_pick,model_pick}` | `pick_vector_distance_behind_ties_nan_and_mesh_first_improvement`; exact transient precision open: MI11 |
| cS3DModelInstance | 4.1: material isolation and counted model | `ModelFrame`, `StaticInstance::set_model_reference` | `published_defaults_draw_record_and_material_isolation`, `failed_resource_frame_does_not_replace_or_retain_models`; vector-15 staging open: MI12 |
| cS3DModelInstanceAnimated | 1–2: inherited static/basic/container surfaces | `AnimatedInstance::instance`, `Interfaces`, `NativeCount(Instance32)` | `animated_unbound_construction_and_public_build_frame`, `count_widths_zero_releases_and_overflow_destruction` |
| cS3DModelInstanceAnimated | 3: GetFrame/GetFrameCount/SetAnimation | `AnimatedInstance::{frame,frame_count,set_animation}` | `animation_failed_build_preserves_frame_word`; null subrange path per graphics-formats §3.9 |
| cS3DModelInstanceAnimated | 3: BuildFrame and SetFrame | `AnimatedInstance::{build_frame,set_frame}` | `animated_unbound_construction_and_public_build_frame`, `animation_failed_build_preserves_frame_word` |
| cS3DModelInstanceAnimated | 3 / graphics-formats 3.9: speed/mode/Step/Advance | `AnimatedInstance::{set_speed,set_anim_mode,step,advance}` | `animation_speed_rate_zero_and_one_step_catchup`, `animation_pingpong_subtracts_new_direction_and_hold_zeroes_accumulator`, `animation_backwards_loop_and_single_frame`, `negative_speed_uses_direction_for_step_and_accumulator_subtraction` |
| cS3DModelInstanceAnimated | 3.1: constructor and failed builds | `AnimatedInstance::unbound`, `AnimatedInstance::build_frame` | `animated_unbound_construction_and_public_build_frame`, `animation_failed_build_preserves_frame_word` |
| cS3DModelInstanceAnimated | 3.1: nonfinite inputs and accumulator store | `AnimatedInstance::advance` | `playback_nonfinite_and_transient_threshold_vectors`; exact transient precision open: MI11 |
| cS3DModelProcessor | 4: constructor/SetModel/refusals/conversion | `ModelProcessor::{new,set_model,target_format}` | `processor_refusal_conversion_restore_and_release_counts`, `processor_unsupported_format_clears_both_references`, `processor_release_preserves_conversion_until_drop_and_rebind` |
| cS3DModelProcessor | 4 / 8.2: triangle normals and weights | `ModelProcessor::create_vertex_normals`, `walk_triangles` | `processor_equal_and_unequal_area_weighted_normals`, `all_triangle_walk_types_first_and_indices` |
| cS3DModelProcessor | 4 / 8.3: lazy normals/lighting/flip | `ModelProcessor::light_model`, `VertexLighting` | `processor_lighting_supplies_own_normals_and_literal_negated_rotation`; meaning of literal flip open: MI2 |
| cS3DModelProcessor | 4: mesh scores and insertion order | `ModelProcessor::sort_meshes` | `processor_mesh_order_has_largest_first_and_reversed_ties`, `sort_vectors_restart_equal_unordered_and_signed_zero` |
| cS3DModelProcessor | 4.1: separate refs/ReleaseModel/restoration/destruction | `ModelProcessor::{set_model,release_model,restore_originals}`, `Drop` | `processor_release_preserves_conversion_until_drop_and_rebind`, `processor_drop_releases_two_instance_parts_and_restores_identity`, `switching_processor_to_refused_model_restores_old_vertices` |
| cS3DModelProcessor | 4.1: stored score precision/restart/NaNs | `ModelProcessor::sort_meshes` | `sort_vectors_restart_equal_unordered_and_signed_zero`; exact transient precision open: MI11 |
| cS3DResSpriteAnim | 6: own query/resource count/borrowed data/destruction | `SpriteResource::Animation`, `Interfaces`, `NativeCount(Resource32)`, `Reference` | `factory_creation_types_zero_count_table_and_bad_tags`, `primitive_and_table_boundaries_and_resource_query_outputs` |
| cS3DResSpriteAnim | 6: CreateModelInstance delegation/lifetime | `Reference<SpriteResource>::create_model_instance`, `SpriteInstanceBinding` | `sprite_creation_delegates_with_temporary_and_counted_output` |
| cS3DResSpriteAnim | 6.1 / 8.4: parsed data/scales/sheets | `SpriteAnimation::read`, borrowed `SpriteResource::animation` | `sprite_shared_downward_sheet_and_opaque_header`, `sprite_relative_keys_and_upward_sources`, `sprite_all_own_mixed_wrap_and_zero_instance_boundaries`; opaque meanings open: MI1; wrapped interior-zero path open: MI13 |
| cS3DResSpriteAnimTable | 6: unread null block/query/count/destruction | `SpriteResource::Table`, `Interfaces`, `NativeCount(Resource32)`, `Reference` | `primitive_and_table_boundaries_and_resource_query_outputs`, `factory_creation_types_zero_count_table_and_bad_tags` |
| cS3DResSpriteAnimTable | 6.2: table block/count/entries/borrowed getter | `SpriteAnimationTable::{read,block,header,entry_count,entry}` | `sprite_factory_short_bad_tags_and_table_exact_size`, `primitive_and_table_boundaries_and_resource_query_outputs`, `sprite_serial_read_preserves_resource_identity_and_borrowed_data` |
| cS3DResSpriteAnimFactory | 6: CreateInstance/type/IID/failure output | `SpriteAnimationFactory::{create_instance,create_instance_into}`, `NativeCount(Factory32)` | `factory_creation_types_zero_count_table_and_bad_tags`, `primitive_and_table_boundaries_and_resource_query_outputs`, `count_widths_zero_releases_and_overflow_destruction` |
| cS3DResSpriteAnimFactory | 6: SerialRead dispatch/no release | `SpriteAnimationFactory::serial_read_into`, `SpriteResource::read` | `sprite_serial_read_preserves_resource_identity_and_borrowed_data`, `primitive_and_table_boundaries_and_resource_query_outputs` |
| cS3DResSpriteAnimFactory | 6.1: ReadAnim | `SpriteAnimation::read` | `sprite_shared_downward_sheet_and_opaque_header`, `sprite_all_own_mixed_wrap_and_zero_instance_boundaries`; open: MI1, MI13 |
| cS3DResSpriteAnimFactory | 6.2: ReadTable | `SpriteAnimationTable::read` | `sprite_factory_short_bad_tags_and_table_exact_size`, `primitive_and_table_boundaries_and_resource_query_outputs` |
| cS3DTransform | 1 / 1.2: dirty identity/counter/source rules | `Transform::make_identity`, `StaticInstance::{get_transform,set_transform}` | `transform_identity_wraps_only_when_kind_is_nonzero`, `transform_get_and_set_use_different_destination_counters` |
| cS3DTransform | 2.1: full bounds independent of kind | `BoundingBox::transformed` | `asymmetric_full_bounds_ignore_kind_and_reorder_endpoints`; exact transient precision open: MI11 |
| cS3DTransform | 2.1 / view-utilities 7: pick point kind dispatch | `Transform::point`, `PickContext::mesh_pick` | `asymmetric_transform_kinds_and_bounds`, `nearest_pick_metric_and_transform_are_explicit`; exact transient precision open: MI11 |
| Shared bounding-box helper | 5: centre and vertex bounds | `BoundingBox::{center,from_points}`, `VertexBlock::update_bounds` | `update_bounds_empty_negative_and_center`; array helper is not one of the eight owners |
| All eight owners | 9 / 9.1: valid-memory boundary | checked accessors, `walk_triangles`, `SpriteAnimationTable::read`, weak processor container | `primitive_and_table_boundaries_and_resource_query_outputs`, `processor_release_preserves_conversion_until_drop_and_rebind`; safety choices below |
| All eight owners | 10: independent conformance vectors | implementation items above | all 20 vector mappings below |

## Conformance vectors (§10)

These are executable Rust tests with independent constants in `tests.rs`; they
do not require game data or a plugin. Run `cargo test -p fc4-core model_instances`
from `engine/`.

| Vector | Test |
|---|---|
| 1 basic transform | `base_delegation_attributes_and_identity` |
| 2 sort texture | `static_material_properties_sort_and_mesh_references` |
| 3 animated processor refusal | `processor_refusal_conversion_restore_and_release_counts` |
| 4 crease normal | `processor_equal_and_unequal_area_weighted_normals` |
| 5 depth order | `processor_mesh_order_has_largest_first_and_reversed_ties` |
| 6 shared sheets | `sprite_shared_downward_sheet_and_opaque_header` |
| 7 defaults/properties | `published_defaults_draw_record_and_material_isolation` |
| 8 draw bytes | `published_defaults_draw_record_and_material_isolation` |
| 9 transform counters | `transform_get_and_set_use_different_destination_counters`, `transform_identity_wraps_only_when_kind_is_nonzero` |
| 10 count widths/overflow/zero release | `count_widths_zero_releases_and_overflow_destruction` |
| 11 owner events | `owner_replacement_observes_basic_and_static_callback_order` |
| 12 own/mixed sheets | `sprite_all_own_mixed_wrap_and_zero_instance_boundaries` |
| 13 restart/ties/NaNs/zeros | `sort_vectors_restart_equal_unordered_and_signed_zero` |
| 14 processor lifetime/refusals | `processor_release_preserves_conversion_until_drop_and_rebind`, `processor_refusal_conversion_restore_and_release_counts`, `processor_unsupported_format_clears_both_references` |
| 15 material isolation | `published_defaults_draw_record_and_material_isolation` (MI12 staging) |
| 16 asymmetric/inverted bounds | `asymmetric_full_bounds_ignore_kind_and_reorder_endpoints` |
| 17 pick metric/sign/ties/order | `pick_vector_distance_behind_ties_nan_and_mesh_first_improvement` |
| 18 nonfinite playback | `playback_nonfinite_and_transient_threshold_vectors` |
| 19 short primitives/table safety | `primitive_and_table_boundaries_and_resource_query_outputs` |
| 20 resource count/type versus IID | `primitive_and_table_boundaries_and_resource_query_outputs` |

## Explicit limits and open questions

There are no Unsupported stubs for settled published operations. MI1 and MI2
leave meanings open, but the opaque bytes and literal negated rotation are
implemented. MI11 leaves x87 control-word-dependent last bits open in §§2.1, 3.1
and 4.1: published binary32 stores use wider f64 transients without fused
operations, while point/pick vector helpers retain f32 storage. This is a
portable numeric limit, not exact extended-precision equivalence. MI12 leaves
§10 vector 15's second-copy timing ambiguous: test a separate frame-builder copy
before binding it, then independently apply that instance's own SetModel.
MI13 leaves wrapped interior-zero sheets in §6.1 open: conservatively reject
them. These existing questions live in `docs/specs/questions/model-instances.md`.

Safety choices are separate from native expectations: absent material/mesh data
is handled safely; malformed triangles are skipped; table sizing uses checked
arithmetic; no/repeated ReleaseModel returns an error and borrowed access uses a
weak handle. Failed SerialRead leaves previously parsed data intact. Draw-record
constructor bytes 2–3 are chosen as zero and thereafter preserved. The unexposed
subrange pointer stays null and catch-up stays false, as graphics-formats §3.9
requires. Native pool allocation, undefined invalid-memory behavior, arbitrary
external frame-provider partial writes, and active x87 precision controls are
not emulated by the Rust storage abstraction.

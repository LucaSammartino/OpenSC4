# Animated sprite owner

Clean-room behavior source: `docs/specs/animated-sprite.md`. Core only: no C ABI,
plugin wiring, native pointer layout, replacement/shadow claim or resource parser.
Renderer, resource acquisition and TrianglePick remain foreign interfaces.

The owner is **WIP** while the spec's open numeric, provenance and inventory
questions remain. Settled state and transaction contracts are implemented here.
The safe exact-dyadic numeric path rejects a transient/store that depends on
unqualified PC/RC rather than choosing host trig or an assumed numeric mode.
Nonzero hardware trig is explicitly Unsupported under AS6. There is no unsafe
code, original-owner callback, computed-quad provider or guessed native success.

`AnimatedSprite::default` implements published initialized fields. Uninitialized
zoom scales/metrics use `None`; the unpublished initial local-corner global is
not exposed as valid geometry. The initial draw vertex block is zero/white.
`from_snapshot` remains available for independently supplied state.
The shared draw record aliases flags, highlight and fade alpha. Both setter
surfaces stay synchronized; snapshot import takes the named fields as authoritative.
Shade words default to one. The two unwritten base-record bytes use a documented
portable zero choice and are preserved after any supplied record.

## Owner closure

Every row concerns `cS3DModelInstanceAnimatedSprite`. Tests below are Rust tests
in `tests.rs`, `contract_tests.rs` or `closure_tests.rs`; conditional tests are
implementation checks, never claims of native execution.

| Spec section | Code item | Covering test / explicit open boundary |
|---|---|---|
| 0 Summary | `AnimatedSprite`, cached quad pipeline | `clean_cache_protocol_and_ignored_false_returns`; open: AS6/AS7 numeric qualification |
| 1 Object/draw state | `DrawState::SPRITE`, `material` | `fade_bounds_nan_and_material_constants` |
| 1.1 Defaults/lifetime | `InstanceState::default`, `AnimatedSprite::default`, `SpriteArena` | `defaults_shared_surface_copies_and_equal_position_counter_wrap`, `references_wrap_unknown_iid_preserves_and_zero_release_destroys`, `basic_query_addrefs_and_last_release_drops_owner_and_zooms`; open: AS3 unpublished initial corners/native allocator inventory |
| 2 Creation/loading | `creation_route`, `create_sprite_from_key`, `load` | `resource_routes_keep_native_name_swap`, `load_transaction_repeated_prefix_binding_and_counted_lifetimes`; open: AS11 foreign factory IID/transfer |
| 2.1 Load transaction | `load_from_resource`, `OwnerResources`, `ZoomSlot`, `derive_metrics` | `load_transaction_repeated_prefix_binding_and_counted_lifetimes`; open: AS4/AS6 malformed/numeric qualification |
| 3 Sprite interface | position/view/frame/rate/playback/material methods | `view_and_frame_dirty_and_independent_flags`, `sequence_absolute_frames_boundary_equality_and_independent_clock_direction`; slot 0/native interface adjustment belongs to ABI, not a Rust pointer cast |
| 3.1 Placement | `set_position`, `set_transform`, `get_transform`, `look_at`, `set_orientation`, `Transform` | `defaults_shared_surface_copies_and_equal_position_counter_wrap`, `noncommuting_transform_and_inverse_do_not_translate_vectors`; open: AS6 hardware trig/PC-RC and AS10 LookAt spills |
| 3.1.1 Mask-1 schedule | `placement::stage`, zero-angle `set_orientation` | `conditional_stage_permutations_old_sources_and_signed_zero`; open: AS6 hardware transients/PC-RC |
| 3.1.2 Partial stages/CFG | `placement::stage`, `set_orientation` | `conditional_direction_conversion_and_all_zero_angle_masks`, `conditional_stage_permutations_old_sources_and_signed_zero`; open: AS6 general PC-RC schedule execution |
| 3.1.3 Full-mask schedule | full replacement branch in `set_orientation` | `conditional_direction_conversion_and_all_zero_angle_masks`; open: AS6 nonzero primitive transients |
| 3.1.4 Conditional analytic vectors | exact permutation/zero subdomain of `stage` | `conditional_stage_permutations_old_sources_and_signed_zero`; open: AS6 R3-C/L mode-dependent cancellations and nonzero full R3-M; no invented angle reachability |
| 3.2 Shared interface | owner/custom/properties/attributes, bounds/world bounds, sort/resource getters | `owner_replacement_counts_and_getters_are_shared_thread_safe`, `defaults_shared_surface_copies_and_equal_position_counter_wrap`, `shared_draw_record_aliases_material_fields_and_preserves_opaque_bytes`; open: AS3 exact native inventory/thunks, AS7 numeric getter qualification |
| 3.3 Evidence inventory | no separate executable behavior | open: AS3 exact 60-function accounting; foreign camera/context bodies are not implemented here |
| 4 Animation/clock | `Playback`, `tick`, `advance_frame`, `set_rate` | `vector_clock_raw_units_skip_and_no_skip`, `literal_reverse_single_frame_and_stopped_dirty_vectors`; open: AS5/AS6 numeric qualification; divergence returns a safe diagnostic, not a native status |
| 4.1 Lookup/packing | `derive_metrics`, `select_angles`, `decode_entry_exact`, `convert_bin_nearest_even` | `raw_decoder_literal_packing_flips_scale_and_rejection`, `conditional_direction_conversion_and_all_zero_angle_masks`; open: AS6/AS7 general PC-RC and AS10 Draw angle-result boundaries |
| 4.2 Exact stepping/returns | raw-byte setters, bidirectional `step`, `tick_with`, `TickResult::status` | `literal_reverse_single_frame_and_stopped_dirty_vectors`, `sentinel_nonfinite_and_nonprogress_clock_fail_transactionally` |
| 4.3 Controller boundary | `SequenceController`, `set_animation`, `select_frame`, `advance_frame` | `sequence_absolute_frames_boundary_equality_and_independent_clock_direction`; open: AS1 controller provenance; snapshot is not a native allocation or ownership claim |
| 4.4 Fade/highlight/self-light | `set_fade`, `set_highlight`, `set_self_lit_raw` | `setters_literal_completing_paths_and_unrelated_state`; open: AS7 unmasked/nonfinite whole-call qualification |
| 5 Drawing | `draw`, `draw_loaded`, `update_view_exact` | `clean_cache_protocol_and_ignored_false_returns`; open: AS6/AS7/AS10 numeric selection/geometry |
| 5.1 Quad/local box | `update_view_exact`, `update_stored_view_exact` | `independently_transcribed_q1_q2_q3_q4_q5`, `stored_metric_suboperation_shrinks_without_scale_rotation_or_dirty_changes`; open: AS7 mode-dependent numeric/malformed domain |
| 5.2 Cache/draw protocol | `draw`, `SpriteCache`, `SpriteDrawContext` | `cache_staleness_dirty_frame_only_and_counter_wrap`, `full_context_comparison_early_exit_and_unsupported_refresh`, `clean_cache_protocol_and_ignored_false_returns` |
| 6 Picking | `pick_cached`, `pick_quad`, `TrianglePicker` | `pick_uses_stale_vertices_and_actual_second_result`, `picking_short_circuits_and_uses_fan_triangles` |
| 7 Users | no owner behavior; resource attribution | open: AS2 shipped usage |
| 8 Quirks | native type routing, byte frames, no-skip clock | `resource_routes_keep_native_name_swap`, `frame_narrows_without_modulo_or_empty_check`, `no_skip_exact_two_differs_from_below_two` |
| 9 Open questions | explicit Unsupported / snapshot boundaries | AS1-AS7/AS9 retained qualifications, AS10/AS11 appended; no claimed complete native owner |
| 10.1 Zoom | signed early exit and actual Draw call protocol | `full_context_comparison_early_exit_and_unsupported_refresh`, `clean_cache_protocol_and_ignored_false_returns` |
| 10.2 Direction | literal B conversion and zero-angle selector | `conditional_direction_conversion_and_all_zero_angle_masks`; open: AS6 general numeric mode |
| 10.3 Loop | bidirectional loop and ping-pong | `vector_last_frame_loop_and_ping_pong`, `literal_reverse_single_frame_and_stopped_dirty_vectors` |
| 10.4 Clock | skip/no-skip advancement and remainder | `vector_clock_raw_units_skip_and_no_skip` |
| 10.5 Geometry/cache/orientation | Q1-Q5 and exact placement subdomain | `independently_transcribed_q1_q2_q3_q4_q5`, `cache_staleness_dirty_frame_only_and_counter_wrap`; open: AS6 R2-R1/R2/R3 nonzero hardware angles |
| 10.6 Finite schedules | conditional primitive permutations and signed zeros | `conditional_stage_permutations_old_sources_and_signed_zero`; open: AS6 R3-C/L PC-RC-sensitive and full nonzero R3-M |

Legacy helpers (`direction_index`, decoded `SpriteTable`, `FrameEntry`,
`billboard_corners`, zoom-prefix adoption) remain source conveniences. The owner
Draw/load path uses the raw consumer/transaction rules above; those helpers are
not evidence of the native record selection or successful Load contract.

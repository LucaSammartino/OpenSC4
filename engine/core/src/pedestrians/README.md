# Pedestrian owners

Implemented column path: `engine/core/src/pedestrians`.

This portable core owns `cSC4Pedestrian` and `cSC4PedestrianOneShot`.
`PedestrianRecord`, `AutomatonBase`, stack operations and serialization come
from `automata`; inherited state dispatch, tweens, routes and movement use
`vehicles::TrafficAutomaton` / `TrafficOwner`. There is no plugin or C ABI wiring.
All world callbacks are foreign controller, occupant, renderer, network or
pathfinder operations, not calls to the original pedestrian owner.

The spec leaves the saved pedestrian tail unnamed. It is preserved as the
existing record, without guessing aliases for transient runtime fields.
PN1 is settled: one-shot Init selects an unsigned byte count, and its own
animation tick charges the first live model's sampled final frame. The tick
updates opacity before advancing models and continues advancing after deferred
removal. Count accessors have no playback or lifetime effects.
PN2 and conformance test 2 concern cSC4SimFireCrew, outside these two owners.
PN3 records literal interpretations of summary-level details; those paths run.
Inherited traffic errors remain the shared traffic module's errors; this module
does not claim to settle that module's VH questions.

## Owner closure

Tests below are in `tests.rs` and `tests/one_shot.rs` (names without the module prefix).

| Owner | Spec section | Code item | Test / open |
|---|---|---|---|
| cSC4Pedestrian | pedestrians §1 | `CLSID_PEDESTRIAN`, `PedestrianRecord` | `construction_and_first_parameter_conformance`, `record_roundtrip_and_empty_inherited_tick` |
| cSC4Pedestrian | §2.1 | `construct`, `init`, `Runtime::initial_parameter` | `construction_and_first_parameter_conformance` |
| cSC4Pedestrian | §2.2 | `animation_tick` | `bee_line_crowd_reaction_and_animation_transitions` |
| cSC4Pedestrian | §2.3 | `enter_state`, `state_tick`, shared stack | `enter_state_flag_table_and_empty_stack`, `queue_strike_and_ignite_dispatch` |
| cSC4Pedestrian | §2.4 | `move_forward_with_collision`, `test_collision_expensive`, `bee_line_tick` | `collision_cadence_box_and_close_random_push`, `collision_movement_rejection_retry_and_ground_flag` |
| cSC4Pedestrian | §2.5 | `crowd_reaction`, `crowd_tick` | `bee_line_crowd_reaction_and_animation_transitions` |
| cSC4Pedestrian | §2.6 | `queue_tick`, `strike_tick`, riot ticks, `ignite_tick` | `queue_strike_and_ignite_dispatch`, `riot_leader_cell_memory_and_follower_distance` |
| cSC4Pedestrian | §2.7 | `traffic_owner`, `inherited_tick`, `base`, shared records | `record_roundtrip_and_empty_inherited_tick`; inherited VH limitations remain in vehicles |
| cSC4Pedestrian | §6 quirk 2 | `test_collision_expensive` | `collision_cadence_box_and_close_random_push` |
| cSC4Pedestrian | §7 | known initial-parameter operation | `construction_and_first_parameter_conformance`; PN1 consumer applies to one-shot |
| cSC4Pedestrian | §8 tests 1, 3, 4 and PR conformance | constructor, entry, collision, Init | `construction_and_first_parameter_conformance`, `enter_state_flag_table_and_empty_stack`, `collision_cadence_box_and_close_random_push` |
| cSC4Pedestrian | automata §6.4 | shared `PedestrianRecord`, `read`, `write` | `record_roundtrip_and_empty_inherited_tick` |
| cSC4Pedestrian | automata §6.5 (2,3,4,6,8,9,12) | pedestrian state dispatcher | crowd, queue/strike/ignite and riot tests above |
| cSC4PedestrianOneShot | pedestrians §1 | `CLSID_ONE_SHOT`, `PedestrianOneShot` | `one_shot_init_success_failure_and_property_bounds` |
| cSC4PedestrianOneShot | §2.1 (inherited) | `pedestrian.construct`, `pedestrian.init` | `one_shot_init_success_failure_and_property_bounds` |
| cSC4PedestrianOneShot | §2.2 overridden by §3.2 | `animation_tick` | `pn1_t1_fade_then_advance_ms_then_post_advance_frame_sample`, `pn1_t11_direct_tick_ignores_pause_and_stationary_flags` |
| cSC4PedestrianOneShot | §2.3 (inherited) | `pedestrian.enter_state`, `pedestrian.state_tick` | `enter_state_flag_table_and_empty_stack` |
| cSC4PedestrianOneShot | §2.4 (inherited) | `pedestrian.move_forward_with_collision` | both collision tests above |
| cSC4PedestrianOneShot | §2.5 (inherited) | `pedestrian.crowd_reaction`, `pedestrian.crowd_tick` | `bee_line_crowd_reaction_and_animation_transitions` |
| cSC4PedestrianOneShot | §2.6 (inherited) | pedestrian state ticks | queue/strike/ignite and riot tests above |
| cSC4PedestrianOneShot | §2.7 (inherited) | shared traffic and automata records | `record_roundtrip_and_empty_inherited_tick`; inherited VH limitations remain in vehicles |
| cSC4PedestrianOneShot | §3.1 | `construct`, `init`, byte count, unsigned bounded draw | `pn1_i1_constructor_count_five_without_model_setup` through `pn1_i10_reversed_bounds_wrap_span_and_use_product_high_half` (individual vector mapping below) |
| cSC4PedestrianOneShot | §3.2, §7 play-count consumer | `animation_tick`, `consume_play_count`, shared deferred removal | `pn1_t1_fade_then_advance_ms_then_post_advance_frame_sample` through `pn1_t11_direct_tick_ignores_pause_and_stationary_flags` (individual vector mapping below); PN1 answered, closed |
| cSC4PedestrianOneShot | §3.3 | `play_count`, `set_play_count` | `pn1_t12_byte_accessors_do_not_touch_playback_or_lifetime` |
| cSC4PedestrianOneShot | §6 quirk 2 (inherited) | pedestrian collision cadence | `collision_cadence_box_and_close_random_push` |
| cSC4PedestrianOneShot | §8 PR conformance | inherited first parameter draw | `one_shot_init_success_failure_and_property_bounds` |
| cSC4PedestrianOneShot | §8 PN1 conformance | initialization and animation tick | all 22 `tests::one_shot` vectors mapped below |
| cSC4PedestrianOneShot | automata §6.4 | `read`, `write`, shared pedestrian record | `record_roundtrip_and_empty_inherited_tick` |
| cSC4PedestrianOneShot | automata §6.5 (inherited) | pedestrian state ticks | state/collision/crowd/riot tests above |

Summary choices tracked by PN3: collision cadence resets after a test without a
catch-up burst; reaction selection is uniform at renderer completion; strike
insets its queried box by one cell; riot turns choose uniformly among walkable
headings on each new road cell; nearby followers mill in a random direction
each tick. These are literal implementations of the stated summaries, not
additional Unsupported branches.

## PN1 conformance vectors

Each vector has its own test in `tests/one_shot.rs`.

| Vector | Test |
|---|---|
| PN1-I1 | `pn1_i1_constructor_count_five_without_model_setup` |
| PN1-I2 | `pn1_i2_initial_parameter_base_speed_then_bounded_draw` |
| PN1-I3 | `pn1_i3_upper_endpoint_preserves_playback_state` |
| PN1-I4 | `pn1_i4_repeat_init_resets_speed_and_count_not_initial_parameter` |
| PN1-I5 | `pn1_i5_failed_init_retains_parameter_count_speed_and_retry_draw` |
| PN1-I6 | `pn1_i6_missing_or_short_property_uses_both_tuning_bytes` |
| PN1-I7 | `pn1_i7_zero_equal_bounds_empty_models_still_draw_without_removal` |
| PN1-I8 | `pn1_i8_equal_255_bounds_are_unsigned_and_still_draw` |
| PN1-I9 | `pn1_i9_adjacent_reversed_bounds_consume_zero_span_draw` |
| PN1-I10 | `pn1_i10_reversed_bounds_wrap_span_and_use_product_high_half` |
| PN1-T1 | `pn1_t1_fade_then_advance_ms_then_post_advance_frame_sample` |
| PN1-T2 | `pn1_t2_nulls_skipped_and_only_first_live_model_charges` |
| PN1-T3 | `pn1_t3_later_final_model_cannot_charge` |
| PN1-T4 | `pn1_t4_remove_false_precedes_remaining_model_advances` |
| PN1-T5 | `pn1_t5_zero_waits_for_final_frame_and_never_wraps` |
| PN1-T6 | `pn1_t6_empty_and_all_null_models_only_update_fade` |
| PN1-T7 | `pn1_t7_held_final_frame_charges_on_each_zero_ms_tick` |
| PN1-T8 | `pn1_t8_single_frame_model_charges_without_advancing` |
| PN1-T9 | `pn1_t9_zero_frame_count_uses_signed_negative_threshold` |
| PN1-T10 | `pn1_t10_large_delta_and_advance_result_charge_at_most_once` |
| PN1-T11 | `pn1_t11_direct_tick_ignores_pause_and_stationary_flags` |
| PN1-T12 | `pn1_t12_byte_accessors_do_not_touch_playback_or_lifetime` |

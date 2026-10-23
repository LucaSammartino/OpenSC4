# Watercraft core

Portable `cSC4Watercraft` storage and owner operations, implemented only from
`docs/specs/watercraft.md` and its referenced automata contracts. No native boat
calculation is accepted as a provider. The module is not connected to capi or the
plugin. Resource creation, terrain queries, berth arbitration, and effect playback
are foreign services; boat state and decisions belong to Rust.

## Owner closure

This is an audit, **not a complete-owner coverage claim**. An `open:` entry names
an unresolved question in `docs/specs/questions/watercraft.md`. Implemented portions
and their tests are listed even where the containing section remains open. Test
names below are in `tests.rs` unless noted otherwise.

| Owner | Spec section | Code item | Test / unresolved contract |
|---|---|---|---|
| cSC4Watercraft | §0 Summary | `Watercraft`, `Fleet` | Scope only; no extra executable contract. |
| cSC4Watercraft | §1 Class and fields | `WatercraftRecord::{read,write}`, `Watercraft::class_id`, runtime `Watercraft` fields | `record_only_saves_version_and_base_and_rejects_truncation`; open: WC3 (version acceptance outside documented v2). |
| cSC4Watercraft | §2 Drive parameters | `DriveParameters::{default,load}`, `Group::drive_parameters` | `absent_list_and_zero_list_have_distinct_dependent_defaults`, `zero_sign_nan_and_supplied_dependent_entries`, `first_present_ancestor_list_wins_even_when_all_zero`. |
| cSC4Watercraft | §3 Init and Shutdown | `initialize_runtime`, `Fleet::shutdown` | `initialization_acquires_once_shutdown_stops_and_drops_only_owned_resources`, `effect_failure_is_a_gap_not_success_and_shutdown_releases_partial_resources`; open: WC3 (fourth flag, base Init and resource failure policy). |
| cSC4Watercraft | §4 States | `Fleet::{push_state,pop_state,state_tick,state_tick_with_geometry,end_towing,end_docking}` | `state_entry_exit_and_remove_side_effect_order`, `final_state_pop_preserves_crash_and_adds_disappear`, `geometry_dispatch_pops_completed_docking_and_undocking`; open: WC4, WC5, WC7 (far docking, crash animation, following and blocked-undocking results). |
| cSC4Watercraft | §5 AI movement | `movement.rs`, `operations.rs` substeps | open: WC4, WC5, WC7; details in §5.1–§5.7 rows. |
| cSC4Watercraft | §5.1 Animation tick | `animation_budget`, `lifetime_tick`, `update_wakes`, `update_backwards_time`, `still_time_allows_animation` | `animation_early_return_and_two_idle_accumulations`, `lifetime_boundary_manager_order_and_retention_flags`, `parked_lifetime_asks_manager_before_override_and_no_manager_removes`, `lifetime_stack_exclusions_are_not_just_top_state_checks`, `wakes_wait_until_strictly_above_250ms_then_restart_in_order`; open: WC2, WC4 (full inherited/AI/effect/model step contracts, heading metric, caller's false-result handling). |
| cSC4Watercraft | §5.2 Picking cells | `candidate_directions`, `next_route_cell`, `clamped_frame_time` | `direction_order_excludes_reverse_and_respects_every_mask_bit`, `ferry_reader_skips_used_cells_only_once_and_neighbor_exhaustion_removes`; open: WC4, WC7 (AI motion, position/tilt, direction geometry, wandering/backtracking and route encodings). |
| cSC4Watercraft | §5.3 Requests and docking | `other_station`, `dock_requests`, `clear_dock_requests`, `docking_tick` | `ferry_public_vector_selects_berth_wait_or_remove_and_preserves_order`, `queued_occupied_berth_never_takes_another_waiting_spot`, `public_docked_vector_and_far_approach_gap`, `docking_final_approach_is_bounded_and_completion_uses_strict_distance`; open: WC4, WC7 (far approach, completion ordering, held-berth assignment, on-land return). |
| cSC4Watercraft | §5.4 Stations and routes | `set_station_information`, `initialize_current_route`, `set_neighbor_route`, DOCK exit in `pop_state` | `station_selection_is_per_axis_and_route_restart_preserves_used_count`, `ferry_reader_skips_used_cells_only_once_and_neighbor_exhaustion_removes`; open: WC7 (compressed-route format and route coordinates). |
| cSC4Watercraft | §5.5 Undocking | `undocking_tick`, `state_tick_with_geometry` | `undocking_moves_horizontally_and_does_not_overshoot`, `geometry_dispatch_pops_completed_docking_and_undocking`; open: WC7 (blocked/on-land return and zero away vector). |
| cSC4Watercraft | §5.6 Can it move? | `movement_possible`, `on_land` | `movement_flag_precedes_bounds_then_bounds_precede_crash_state`. |
| cSC4Watercraft | §5.7 Collisions | `boat_crash_threshold_exceeded`, `shore_crash_threshold_exceeded`, `determine_crash_case`, `set_state_internal` | `player_collision_public_ratios_are_strict`, `crash_case_angle_and_ai_override_do_not_reap_expired_crashes`; open: WC5 (footprints, size classes, inanimate hits, bump response and full collision dispatch). |
| cSC4Watercraft | §6 U-Drive-It | `commands.rs`, player substeps in `movement.rs` and `operations.rs` | open: WC4, WC6, WC7, WC8; details in §6.1–§6.3 rows. |
| cSC4Watercraft | §6.1 Drive tick | `release_tow_ahead`, `coast`, `movement_possible`, `clamped_frame_time` | `player_coasting_cannot_cross_zero_and_uses_unclamped_dt`, tow-ahead regression; open: WC4, WC5 (inherited tick, position and collision calculations). |
| cSC4Watercraft | §6.2 Commands | `translate_drive_commands`, `Fleet::player_commands`, `select_gear`, `rudder_angle`, `tow_button` | `commands_brake_before_reversing_and_do_not_overshoot_targets`, `command_steering_countersteers_returns_and_clamps`, `commands_require_aems_and_fire_consumes_only_successful_shots`, `skier_toggle_and_stop_and_command_translation_order`, `exact_gear_gates_and_rudder`; open: WC6, WC8 (effect placement, inherited translation and literal summary choices). |
| cSC4Watercraft | §6.3 Towing | `following_target`, `end_towing`, tow message and TOWED hooks | `tow_message_uses_owned_nearest_then_group_and_releases_both_links`, following-target regression; open: WC7 (following translation rate, radius choice and degenerate geometry). |
| cSC4Watercraft | §7 Messages | `Fleet::do_message`, `MessageDisposition` | `tow_message_uses_owned_nearest_then_group_and_releases_both_links`, `invalid_tow_pick_disarms_and_unarmed_messages_forward`; open: WC7 (nearest-distance ties). |
| cSC4Watercraft | §8 Quirks | `state_tick`, `animation_budget` | `idle_is_cumulative_and_strictly_above_sixty_seconds_falls_through`, `animation_early_return_and_two_idle_accumulations`; open: WC5 for actual crash sequence after the fallthrough. |
| cSC4Watercraft | §9 Debug text | `debug_extra_text` | `debug_text_has_exact_fields_width_and_negative_zero`. |
| cSC4Watercraft | §10 Open questions | No runtime behavior assigned | WC1–WC8 recorded in question file; WC1 external writer and WC2 external caller are not assigned to this owner. |
| cSC4Watercraft | §11 Conformance tests | Five public vectors mapped below | All five have core assertions; vector 4's complete collision response remains open: WC5. |

### Public conformance vectors (§11)

1. Default entries 21/36 and dependent entry 8: `absent_list_and_zero_list_have_distinct_dependent_defaults`.
2. Ferry berth/wait/remove at 250 units: `ferry_public_vector_selects_berth_wait_or_remove_and_preserves_order`.
3. Docking at 0.5 units / 0.5 degrees: `public_docked_vector_and_far_approach_gap`;
   state-pop consumption: `geometry_dispatch_pops_completed_docking_and_undocking`.
4. Player relative speeds 0.12/0.18: `player_collision_public_ratios_are_strict`
   verifies threshold classification only. Applying bump/crash to a second boat
   is **open: WC5**, so this vector is not claimed as fully closed.
5. Direction-2 order `2,3,1,4,0,5,7`, excluding 6:
   `direction_order_excludes_reverse_and_respects_every_mask_bit`.

The explicit `ContractGap` errors must not be treated as successful ticks.
`BaseCommandTranslation` requests the inherited translation after the owner's
commands; emitting the event does not implement that inherited calculation.
The `PlayerEffects` service reports actual fire playback and controls skier
running state; exact spatial placement remains WC6.

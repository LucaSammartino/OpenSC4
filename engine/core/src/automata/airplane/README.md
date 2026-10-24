# cSC4Airplane core

The owner is `engine/core/src/automata/airplane/mod.rs`, re-exported as
`fc4_core::automata::Airplane`. Shared aircraft placement and record codecs
remain in `../aircraft.rs`; shared state stacks remain in `../automaton.rs`.
There is no plugin wiring, C ABI change or foreign implementation of owner laws.

`Airplane` stores its base record, home placement/loaded flag, independent AI and
explicitly supplied player parameter records, optional flight phase/ammunition
and optional LOITER timer. Unknown construction defaults remain unknown; the
spec has not established a native Init/Shutdown contract (AF4). Reads need no
city and replace an owner only on success. Read resets transient state; first
placement Init restores saved home or captures the new placement once.

Parameter import uses named binary32 fields, whole-record ancestor selection,
encoded defaults, signed-zero fallback, and no range/non-finite sanitization.
An empty table is 18 zeros. Short raw records violate the input precondition;
they are never padded. Ancestor lookup deliberately has no cycle detector.
The adapter supplies the selected airplane record after group import; the Lua
importer's airplane/helicopter/watercraft alternative precedence is not an
airplane-owner decision. Native playerdrive selection remains AF5.

Numerical helpers expose the finite-input pre-force limiter, converted/capped
time step, candidate integration, final ground projection, scalar lift curve,
pressure, gravity, thrust gate, drag normalization and rolling resistance.
They do not commit a flight tick. The force frame, angular update and environmental
corrections remain AF2. No callback can provide substitute owner physics.
Intermediates use f64 to approximate x87, with binary32 stores at stage boundaries;
the conformance tests use the spec's explicit tolerances. Non-finite physics
comparison details are open under AF9; literal comparisons do not sanitize them.

## Owner closure

Tests are in `engine/conformance/tests/airplane.rs` (new numerical vectors) or
`engine/conformance/tests/aircraft.rs` (shared owner/record tests).
Every unsettled row references an existing spec question; it is not a claim of
a functioning native flight owner. Only explicitly specified stages are closed.

| Owner | Spec section | Code item and covering test, or open question |
|---|---|---|
| cSC4Airplane | aircraft §0 | `Airplane`; `aircraft_records_nest_base_and_home_without_traffic`; open: AF2/AF3 for full flight |
| cSC4Airplane | aircraft §1 | `Airplane::class_id`, `CLSID_AIRPLANE`; `aircraft_records_nest_base_and_home_without_traffic` |
| cSC4Airplane | aircraft §2 | `aircraft_altitude`, `clip_aircraft_to_world`; `altitude_uses_highest_surface_and_clip_keeps_outward_margin`; open: AF5 for DoPlayerDriveTick |
| cSC4Airplane | aircraft §3.1 | `import_airplane_record`, `AirplaneDriveParameters::{load,from_list,default}`, `Airplane::init_data`; `airplane_default_and_explicit_speed_parameters`, `airplane_derived_defaults_and_explicit_overrides`, `named_record_missing_fields_and_whole_record_inheritance`, `parameters_preserve_negative_nonfinite_and_signed_zero_rules`, `raw_short_record_is_not_zero_padded`; open: AF5 for LoadPlayerDriveParameters selection |
| cSC4Airplane | aircraft §3.2 | `init_data`, `airplane_effect_names`; `first_init_restores_loaded_home_and_never_recaptures_it`, `airplane_effect_resources_choose_fire_plane_propellers`; open: AF4 for probes, effect attachments, Init/Shutdown allocation, cleanup and failures |
| cSC4Airplane | aircraft §3.3 | `pursue_goal`, `begin_loiter`, `tick_loiter`; `chase_rejection_obeys_shared_final_state_and_stack_indices`, `phase_and_loiter_ammo_are_independent_of_goal_stack`; open: AF3 for AI_GotoDestination/Land/TakeOff/Taxi/Stop/TurnToVector/FollowRay/FollowVectorAndTurn/ApproachAltitude, PickRunwayDirection/EvaluateClearance/PickApproachBox/FindFireNearLocation, failures, timer boundary/restore and empty ammo; open: AF6 for dispatch transitions |
| cSC4Airplane | aircraft §3.4 | `set_airplane_state`, `update_physics` (Unsupported AF2); `phase_and_loiter_ammo_are_independent_of_goal_stack`, `name_only_crop_dust_and_unsettled_models_do_not_mutate_owners`; open: AF2 for CalculateForces complete frame, UpdatePhysics corrections, DetectCollisions and DoAnimationTick; AF4 for sound/effect actions; AF5 for TranslatePlayerDriveCommands and bombs |
| cSC4Airplane | aircraft §3.4.1 | `physics_seconds`, `limit_velocity`, `integrate_candidate`, `project_ground_velocity`; `time_conversion_caps_once_without_a_lower_clamp`, `pre_force_limiter_vectors`, `candidate_motion_uses_mean_velocity_and_no_second_cap`; open: AF2 for post-candidate correction/commit/collision/clip/attitude sequence, AF9 for non-finite comparison policy |
| cSC4Airplane | aircraft §3.4.2 | `force_scalars`, `lift_coefficient`, `ForceScalars::{rolling_resistance,drag_magnitude}`; `scalar_lift_curve_and_horizontal_inverse_cosine`, `pressure_ground_resistance_and_drag_normalization`, `reverse_throttle_gate_only_blocks_backward_ground_motion`; open: AF2 for attitude-derived lift direction and complete vector force |
| cSC4Airplane | aircraft §4 | Other owner (helicopter); no airplane behavior assigned |
| cSC4Airplane | aircraft §5 | Other owner (dispatch sources); shared `initialize_fire_plane_station_units` updated to settled signed count and wrapping mask; `fire_plane_station_with_three_units_sets_available_property_to_seven` covers vectors 3/6 |
| cSC4Airplane | aircraft §6 | `clip_aircraft_to_world`; `altitude_uses_highest_surface_and_clip_keeps_outward_margin` |
| cSC4Airplane | aircraft §7 | Open: AF2–AF6, AF9; AF1/AF7/AF8 have been settled for airplane parameters/station masks |
| cSC4Airplane | aircraft §8 | Vectors 1/2 in aircraft.rs parameter tests; 3/6 in station test; 4/5 and 8–13 in airplane.rs tests named above. Vector 7 belongs solely to helicopter §4.1, outside this owner assignment |
| cSC4Airplane | automata §6.1/§6.2 | Shared `AutomatonBase`/state stack; `chase_rejection_obeys_shared_final_state_and_stack_indices`, `aircraft_records_nest_base_and_home_without_traffic` |
| cSC4Airplane | automata §6.4/§14 vector 15 | `AircraftRecord`, `AircraftPlacementRecord`, `Airplane::{read,write}`; `aircraft_records_nest_base_and_home_without_traffic` includes same-input byte round-trip, `concrete_versions_accept_zero_but_aircraft_base_requires_one`, `truncated_owner_reads_are_atomic_at_every_byte_boundary`, `failed_aircraft_write_never_attempts_home_vector` |
| cSC4Airplane | automata §6.6 | Independent phase, phase-1 ammo refill, CHASE pop, name-only CROP_DUST, separate LOITER timer; `phase_and_loiter_ammo_are_independent_of_goal_stack`, `name_only_crop_dust_and_unsettled_models_do_not_mutate_owners`; open: AF3/AF4/AF6 as above |
| cSC4Airplane | automata-managers §1 | Class ID matches director registration; `aircraft_records_nest_base_and_home_without_traffic`; COM factory allocation remains outside core owner |
| cSC4Airplane | automata-managers §2–§4 | Foreign director/managers and traffic population; no new airplane-owned behavior assigned |
| cSC4Airplane | automata-managers §5–§8 | Vehicle-family slots/manager tests assign no additional airplane-owned behavior |

An in-game shadow needs an adapter to observe matching native airplane parameter
records and pre-correction candidate/scalar inputs/results. Those isolated stages
are feasible comparison targets with the documented tolerances. An end-to-end
trajectory shadow awaits AF2–AF6 and lifecycle integration; no game request is
made by this core-only change.

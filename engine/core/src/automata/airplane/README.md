# cSC4Airplane core owner

`mod.rs` owns the airplane, re-exported as `automata::Airplane`. Shared placement,
state and save codecs remain in `automata::aircraft` and `automaton`; ordered weapon
requests are shared in `aircraft_actions`. No plugin or C ABI changes.

Construction supplies three rounds, a clear drop flag and a zero remaining timer.
Read replaces only the saved record, preserving these unsaved values on existing
objects. LOITER requests dispatch state 5, spends one round on first visit without
a positive-ammo gate, and expires at countdown <= 0. Animation's countdown stage
uses converted/capped physics seconds before pursuing the goal. Phase requests
return ordered sound actions; repeating the airplane phase is a complete no-op.
The player-driven boolean is a supplied flag input independent of the goal stack.
Other constructor fields remain unknown under AF4.

Named parameter import selects whole ancestor records and preserves nonzero
negative/non-finite values. Numerical helpers expose the velocity limiter,
converted/capped time, candidate integration, ground projection, lift coefficient,
pressure, gravity, thrust, drag normalization and rolling resistance. They use f64
intermediates to approximate x87 and binary32 stage stores. These stages do not
commit a complete flight tick; AF2 still lacks lift frame, angular update and
environment corrections. Non-finite native equivalence remains AF10.

## Owner closure

All rows concern cSC4Airplane. New tests are in `aircraft_settled.rs`; prior
parameter/record tests are in `aircraft.rs` and numerical tests in `airplane.rs`,
all under `engine/conformance/tests`.

| Spec section | Code item | Test or explicit open question |
|---|---|---|
| aircraft §0 scope | `Airplane` | `vector20_loiter_spends_on_entry_countdown_expiry_and_unsaved_progress` |
| aircraft §1 identity | `class_id` | aircraft.rs: aircraft record round-trip test |
| aircraft §2 altitude/world bounds | shared `aircraft_altitude`, `clip_aircraft_to_world` | aircraft.rs: `altitude_uses_highest_surface_and_clip_keeps_outward_margin` |
| aircraft §2 player tick | Full native tick unavailable | open: AF2/AF5 |
| aircraft §3.1 parameter import/defaults | `AirplaneDriveParameters`, `import_airplane_record` | aircraft.rs parameter/default/ancestor tests; airplane.rs named record/non-finite tests (vectors 1/2/4/5) |
| aircraft §3.2 Init/effects | `init_data`, shared `airplane_effect_names` | aircraft.rs: `init_loads_independent_ai_and_player_lists`; probes/attachment/Init/Shutdown: open AF4 |
| aircraft §3.3 goals/runway/fire | `pursue_goal`, default CHASE/CROP_DUST pop | airplane.rs: `name_only_airplane_crop_dust_pops_without_a_flight_model`; complete steering/runway/fire sequencing: open AF3/AF6 |
| aircraft §3.3.1 LOITER/countdown/ammo | `visit_loiter`, `tick_loiter_with_multiplier`, `read` | `vector20_loiter_spends_on_entry_countdown_expiry_and_unsaved_progress` |
| aircraft §3.4 full flight | `update_physics` returns Unsupported | open: AF2 (complete force/attitude/corrections), AF5 (full player commands) |
| aircraft §3.4.1 translational stages | `physics_seconds`, `limit_velocity`, `integrate_candidate`, `project_ground_velocity` | airplane.rs: `time_conversion_caps_once_without_a_lower_clamp`, `pre_force_limiter_vectors`, `candidate_motion_uses_mean_velocity_and_no_second_cap` (vectors 8–10); non-finite policy: open AF10 |
| aircraft §3.4.2 force scalars | `force_scalars`, `lift_coefficient`, `ForceScalars` | airplane.rs scalar lift/pressure/resistance/thrust tests (vectors 11–13); full lift frame: open AF2 |
| aircraft §3.5 phase sounds | `set_airplane_state` | `vectors21_22_distinct_phase_actions_and_repeat_rules`; physics/collision phase triggers: open AF2/AF4 |
| aircraft §4 helicopter-only contracts | Other owner: `helicopter` | See helicopter/README.md |
| aircraft §4.3 common airplane weapon block | `weapon_requests` | `vector23_weapons_fallback_readiness_and_wrapping_use_count`; full translator/projectile/readiness cadence: open AF5 |
| aircraft §5 dispatch sources | Shared `initialize_fire_plane_station_units`, foreign source owners | aircraft.rs: station signed-count/wrapping-mask test (vectors 3/6); airplane dispatch/message conditions: open AF6 |
| aircraft §6 world margin | shared clip helper | aircraft.rs altitude/world-margin test |
| aircraft §7 open questions | Explicit Unsupported boundaries | open: AF2–AF6, AF10, AF11 (native helicopter adapter, no new airplane behavior) |
| aircraft §8 conformance | Parameter/numerical vectors 1–6/8–13 plus new airplane vectors 20/21/23 | Tests above; helicopter-only vectors are covered by the helicopter owner |
| automata §6.1/§6.2 shared base/stack | reused `AutomatonBase` | aircraft.rs: chase final-state/stack-index and nested record tests |
| automata §6.4 codec/home | `read`, `write`, shared placement | aircraft.rs: round-trip, accepted concrete version zero, atomic truncation, failed-write home-vector gate |
| automata §6.5 goal handlers | `pursue_goal`, `begin_loiter` | default pop tests and vector 20; full destination/land/fire sequencing: open AF3/AF6 |
| automata §6.6 independent phases/LOITER | phase setter/countdown | vectors 20/21; full AI switch: open AF3 |
| automata §14 vector 15 | shared aircraft records | aircraft.rs nested codec round-trip tests |
| automata §15 creation | Foreign manager/director | Full native allocation/lifecycle: open AF4 |

No full game shadow or trajectory equivalence is claimed while AF2–AF6 remain
open. This lane supplies portable core behavior and independent conformance inputs.

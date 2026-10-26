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
Constructor phase, motion, gains, filter/effect slots, dispatch state and search
anchor follow §3.2.1. Synthetic calibration inputs are exposed separately;
calibration convergence and full Init still require §9.1/§9.3 observations.

Named parameter import selects whole ancestor records and preserves nonzero
negative/non-finite values. Numerical helpers expose the velocity limiter,
converted/capped time, candidate integration, ground projection, lift coefficient,
pressure, gravity, thrust, drag normalization, rolling resistance and the force sum
with a caller-supplied stored lift vector. They use f64
intermediates to approximate x87 and binary32 stage stores. These stages do not
commit a complete flight tick; AF2 still lacks lift frame, angular update and
environment corrections. §3.4.1a now specifies non-finite limiter, direction
normalization and time comparisons; vector 28 covers their classification rules.
`closure.rs` implements bounded runway/search/approach, teardown, refresh and
ordered goal requests. Goal plans consume independent controller/service results;
they do not simulate the unresolved controllers or execute native callbacks.

## Owner closure

All rows concern cSC4Airplane. New tests are in `aircraft_settled.rs`; prior
parameter/record tests are in `aircraft.rs` and numerical tests in `airplane.rs`,
all under `engine/conformance/tests`.

| Spec section | Code item | Test or residual native observation | Closure |
|---|---|---|---|
| aircraft §0 scope | `Airplane` | settled: vector 20 LOITER/countdown/read | implemented |
| aircraft §1 identity | `class_id` | aircraft.rs: aircraft record round-trip | implemented |
| aircraft §2 altitude/world bounds | `aircraft_altitude`, `clip_aircraft_to_world` | aircraft.rs: `altitude_uses_highest_surface_and_clip_keeps_outward_margin` | implemented |
| aircraft §2 player tick | `update_physics` Unsupported | AF2 §9.1 attitude/contact outputs; AF5 §9.4 raw controls/readiness/projectile trace | open AF2/AF5 |
| aircraft §3.1 parameter import/defaults | `AirplaneDriveParameters`, `import_airplane_record` | aircraft.rs parameter/ancestor tests; airplane.rs vectors 1/2/4/5 | implemented |
| aircraft §3.2 Init/effects | `init_data`, `airplane_effect_names`, `calibration_inputs` | aircraft.rs: independent lists; closure: `calibration_seeds_are_synthetic_and_use_current_player_gains`; AF4 §9.3 attachment/failure/Init order; AF2 §9.1 calibration convergence | open AF2/AF4 |
| aircraft §3.2.1 constructor/Shutdown | `Default`, `shutdown` | closure: `vector33_airplane_constructor_and_teardown_precede_false_base` (AF4) | implemented |
| aircraft §3.3 complete steering | `pursue_goal`, `goal_actions` | airplane.rs: default crop-dust pop; AF3 §9.2 Stop/Turn/Taxi/TakeOff/Goto/Land control outputs and completion | open AF3 |
| aircraft §3.3.1 LOITER/countdown/ammo | `visit_loiter`, `tick_loiter_with_multiplier`, `read` | settled: vector 20 | implemented |
| aircraft §3.3.2 clearance/selection/fire | `evaluate_clearance`, `pick_runway`, `search_fire` | closure: vectors 29/30 and `fire_search_uses_box_first_occupant_and_balances_query_ownership` (AF3) | implemented |
| aircraft §3.3.3 approach outputs | `approach_box` | closure: `approach_worked_case_minimum_precedes_city_cap_and_uses_measured_legs` (AF3) | implemented |
| aircraft §3.4 full flight | `update_physics` Unsupported | AF2 §9.1 lift basis/angular update/surface and collision stage outputs; full player commands remain §2 boundary | open AF2 |
| aircraft §3.4.1 translational stages | `physics_seconds`, `limit_velocity`, `integrate_candidate`, `project_ground_velocity` | airplane.rs vectors 8–10; closure: `vector28_nonfinite_limiter_force_direction_and_time` (AF10) | implemented |
| aircraft §3.4.2 supplied-frame force scalars/sum | `force_scalars`, `lift_coefficient`, `ForceScalars::force_with_lift` | airplane.rs vectors 11–13 and supplied-frame sum; closure: vector 28 (AF10); lift construction stays in full flight row | implemented |
| aircraft §3.5 phase sound requests | `set_airplane_state` | settled: vectors 21/22; request causes stay in full flight/Init rows | implemented |
| aircraft §4 helicopter-only contracts | Other owner | helicopter/README.md | other owner |
| aircraft §4.3 weapon accounting stage | `weapon_requests` | settled: vector 23; AF5 §9.4 translator/readiness cadence/projectile initialization remains §2 boundary | implemented |
| aircraft §4.3.3 player refresh/callback requests | `refresh_player_parameters`, `enter_state`, `exit_state` | closure: `vector34_airplane_refresh_preserves_zero_and_callbacks_reload_ai` (AF5/AF6) | implemented |
| aircraft §4.4 ordered goal requests | `goal_actions` | closure: `vector31_fire_land_and_default_dispatch_order` (AF3/AF6); independent controller/query results, not a complete AI tick | implemented |
| aircraft §5 dispatch sources/messages | `initialize_fire_plane_station_units`, `goal_actions`; foreign source owners | aircraft.rs station vectors 3/6; closure: vector 31; AF6 §9.5 observer message envelope/failure callbacks | open AF6 |
| aircraft §6 world margin | shared clip helper | aircraft.rs altitude/world-margin | implemented |
| aircraft §7 residual/native adapter boundary | explicit Unsupported methods | AF2–AF6 §9.1–9.5 as above; AF11 §9.6 lifetime/complete enumeration/independent fixture history, not an airplane behavior answer | open AF2/AF3/AF4/AF5/AF6/AF11 |
| aircraft §8 specified portable conformance | parameter, codec, numerical and bounded action stages | aircraft.rs, airplane.rs, aircraft_settled.rs, aircraft_closure.rs vectors 28–34 above | implemented |
| automata §6.1/§6.2 shared base/stack | `AutomatonBase` | aircraft.rs final-state/stack-index and nested records | implemented |
| automata §6.4 codec/home | `read`, `write`, shared placement | aircraft.rs round-trip/version-zero/truncation/failed-write gate | implemented |
| automata §6.5 goal handlers | `pursue_goal`, `begin_loiter`, `goal_actions` | settled: vector 20; closure: vector 31; AF3 §9.2 controller outputs/completion, AF6 §9.5 external failure callbacks | open AF3/AF6 |
| automata §6.6 independent phases/LOITER | phase setter/countdown and bounded goal requests | settled: vectors 20/21; closure: vector 31; continuous controllers stay in §3.3 row | implemented |
| automata §14 vector 15 | shared aircraft records | aircraft.rs nested codec round-trip | implemented |
| automata §15 creation | Foreign manager/director | AF4 §9.3 native attachment/Init allocation and failure ordering | open AF4 |

Here “implemented” applies to the named bounded contract and its tests. Owner-wide
implemented credit remains partial while the explicit §9 observation rows are open.
The existing game shadow checks metadata/records only; it cannot validate these
flight/control stages or establish a complete aircraft population (AF11).

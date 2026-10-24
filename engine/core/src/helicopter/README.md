# cSC4Helicopter core owner

`mod.rs` owns the helicopter; `automata::Helicopter` re-exports the same type.
`settled.rs` contains the independently testable rotor controls, clearance,
takeover, chase decisions and runtime ownership records. Shared codecs remain
in `automata::aircraft`; shared weapon actions remain in `aircraft_actions`.
There is no plugin, C ABI or game-memory access here.

The runtime is absent before InitInstance. First allocation copies preset 0,
refreshes player parameters, overlays the first present AI record, then allocates
the ten transforms in specified order. PhysicsInit selects phase 0. Read restores
only the saved record; it creates no runtime and does not reset existing unsaved
state. Unknown constructor/control/velocity values remain optional under AF4/AF2.
The player-driven boolean is a caller-supplied flag input, independent of goals.

Phase changes return ordered AEMS/base/embedded notifications. Chase decisions
return startup, spotlight aim, fade, movement and height requests; the core applies
the settled teleport and scaled-position synchronization. MoveXZ and aim geometry
remain open, so this stage does not claim a complete chase tick. Spotlight activation
returns NeedsGeometry even for an inactive pair. Weapon requests call the base
translator before testing readiness, and preserve the missile spelling/fallback.

Runtime allocation faults report an unusable partial runtime without rollback;
they are not a clean native false return. A repeated call cannot turn that partial
runtime into success. Post-base Shutdown returns ordered cleanup actions and the
base result, including false. Present external slots represent retained owning
references; an adapter must execute their stop/clear/release in that order and must
never populate them from unretained borrowed pointers. Rust models ownership and
requests; it does not allocate native-size byte buffers or execute COM operations.

Floating helpers use binary32 operands/stores and f64 intermediates to approximate
x87, with the spec's tolerances. No non-finite flight equivalence is claimed (AF10).

## Owner closure

Tests below are in `engine/conformance/tests/aircraft_settled.rs` unless stated.
Every row concerns cSC4Helicopter. An open row is an explicit Unsupported boundary,
not a successful native flight/control tick.

| Spec section | Code item | Test or explicit open question |
|---|---|---|
| aircraft §0 scope | `Helicopter` | `vectors14_16_preset_overlay_order_and_read_reconstruction` |
| aircraft §1 identity | `class_id` | helicopter.rs: `vector15_same_input_record_round_trip_nests_base_and_home_without_traffic` |
| aircraft §2 altitude/world clipping | `get_altitude`, `clip_to_world`, shared helpers | helicopter.rs: `placement_capture_altitude_and_world_margin_reuse_aircraft_contracts` |
| aircraft §2 player tick | `do_player_drive_tick` | open: AF5 (full controls), AF2 (physics) |
| aircraft §3.1 inherited named-record rules | `DriveRecord::import`, `DriveParameters::load` | helicopter.rs: `missing_fields_are_zero_and_record_selection_uses_first_present_ancestor` |
| aircraft §3 other airplane contracts | Other owner: `automata::Airplane` | See airplane/README.md |
| aircraft §4.1 fields/conversion/zero fallback | `DriveField`, `DriveParameters::overlay` | helicopter.rs: `all_thirteen_named_fields_use_the_specified_destinations_and_exact_constants`, signed-zero/non-finite overlay test |
| aircraft §4.1.1 preset/reconstruction | `preset0`, `init_instance_with`, `physics_init`, `read` | `vectors14_16_preset_overlay_order_and_read_reconstruction` (vectors 14–16) |
| aircraft §4.2 rotor physics overview | `next_frame`, `yaw_pitch_roll_slide_real`, `move_xz_real`, `move_y_real` | open: AF2; helicopter.rs: `open_rotor_ai_lifecycle_and_player_methods_never_report_success_or_mutate` |
| aircraft §4.2 complete goal switches/claims | `pursue_goal`, `push_mission_loiter` | open: AF3; helicopter.rs: numeric default/final/chase rejection/mission timeout tests |
| aircraft §4.2.1 reset/hover/time | `RotorControls::{reset,hover}`, owner control methods, `helicopter_physics_seconds` | `vectors17_18_reset_hover_and_time_multiplier_eligibility` |
| aircraft §4.2.2 chase gates | `chase_actions`, `pursue_chase_stage`, `HelicopterChaseGeometry` | `vector19_chase_lead_opacity_and_distance_boundaries`; remaining MoveXZ/aim: open AF3/AF5 |
| aircraft §4.2.3 phase requests | `set_helicopter_state` | `vectors21_22_distinct_phase_actions_and_repeat_rules`; phase triggers/attachment creation: open AF2/AF4 |
| aircraft §4.2.4 building lift | `building_clearance_height`, `ai_control_lift_over_buildings` | `vector24_two_samples_home_floor_and_descending_demand` |
| aircraft §4.2.5 allocation/post-base Shutdown | `RuntimeOwnership`, `init_instance_with`, `shutdown_after_base` | `vector25_allocation_cleanup_order_false_propagation_and_fault_diagnostic`; pre-base scale/full native Init: open AF4 |
| aircraft §4.3 weapons | `weapon_requests`, shared `aircraft_actions::weapon_requests` | `vector23_weapons_fallback_readiness_and_wrapping_use_count`; remaining translator/projectiles/readiness cadence: open AF5 |
| aircraft §4.3.1 takeover | `refresh_player_parameters`, `on_player_take_over` | `vector26_takeover_direct_precedence_zero_fallback_and_gain_refresh` |
| aircraft §4.3.2 spotlight pair | `spotlight_activation_stage` | `vector27_spotlight_pair_gate_idempotence_and_geometry_continuation`; transform/aim continuation: open AF5 |
| aircraft §5 dispatch source owners | Other classes; no helicopter source implementation here | Aircraft-owned message/dispatch transitions: open AF6 |
| aircraft §6 world margin | `clip_to_world` | helicopter.rs: placement/altitude/world-margin test |
| aircraft §7 remaining questions | explicit Unsupported results; optional unknown constructor fields | open: AF2–AF6, AF10, AF11 (native shadow adapter) |
| aircraft §8 conformance | Existing helicopter vectors 7/record codecs; new vectors 14–19/22–27 above | `aircraft_settled.rs`, `helicopter.rs`, `aircraft.rs`; airplane vectors belong to airplane owner |
| automata §6.1/§6.2 base and state records | reused `AutomatonBase`, `StateExtra::Chase` | helicopter.rs: record round-trip and numeric/default/final-state tests |
| automata §6.4 concrete codec/home | `read`, `write`, `init_placement` | helicopter.rs: atomic truncation and round-trip tests; aircraft.rs: failed-write home-vector gate |
| automata §6.5 mission sequencing | `pursue_goal`, `push_mission_loiter` | Known default pop/6000-ms push covered in helicopter.rs; full claim/return/destination/LOITER switches: open AF3/AF6 |
| automata §6.6 phases/chase | phase setter and chase stage | vectors 19/22; complete movement/goal ordering: open AF3 |
| automata §12 stack quirks | reused `pop_state` | helicopter.rs: `default_pop_obeys_final_state_rules_and_empty_stack_has_no_handler` |
| automata §14 vectors 15/31 | reused record/state codec and chase target handling | helicopter.rs round-trip and chase rejection tests |
| automata §15 COM creation | Foreign director/manager, outside portable owner | Native lifecycle integration: open AF4; native shadow input access: open AF11 |

No end-to-end game shadow is claimed. It requires AF11's safe independent native
inputs and the coordinator's adapter; complete trajectories also require AF2/AF3.

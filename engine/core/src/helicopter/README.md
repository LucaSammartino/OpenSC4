# Helicopter core owner

`mod.rs` owns `cSC4Helicopter` (CLSID `8BA98EDA`). The legacy
`automata::Helicopter` name re-exports this same owner. No C ABI or plugin wiring.
Sources: aircraft.md §§2/4/8; automata.md §§6.2/6.4/6.6/14.

The named Lua record is a typed thirteen-float record, not a numeric exemplar
property or variable-length list. Group lookup is foreign; record selection,
zero fallback and conversion are owned here. An empty present record stops
inheritance. Multiplications use binary32 operands in the specified order.
Recordless parent cycles deliberately have no fabricated terminating fallback.

Preset construction and reconstruction after Read are unsettled (AF9, AF4).
`DriveParameters` therefore stores each unknown value as `None`; callers can
supply a known preset with `from_preset`. Zero fields preserve even unknown or
non-finite preset values. There is no guessed mass, power, flight phase or
player preset.

Save/load delegates to the existing aircraft and automaton codecs. Read needs
no city and replaces the owner only after complete success. The placement step
captures a fresh home's position once, or restores a loaded home once.
Phase changes return metadata; native sound actions remain open.

## Owner closure

All rows below concern cSC4Helicopter. Tests are in
`engine/conformance/tests/helicopter.rs` unless a different file is named.
An explicit Unsupported result is an open contract, not a successful flight tick.

| Spec section | Code item | Test or open question |
|---|---|---|
| aircraft §0 scope | `Helicopter` | `vector15_same_input_record_round_trip_nests_base_and_home_without_traffic` |
| aircraft §1 class identity | `Helicopter::class_id` | same record test, CLSID 8BA98EDA |
| aircraft §2 altitude/world clip | `get_altitude`, `clip_to_world`; shared aircraft helpers | `placement_capture_altitude_and_world_margin_reuse_aircraft_contracts` |
| aircraft §2 player tick | `do_player_drive_tick` | `open_rotor_ai_lifecycle_and_player_methods_never_report_success_or_mutate`; open: AF5 |
| aircraft §3.1 import/ancestor rules referenced by §4.1 | `DriveRecord::import`, `DriveParameters::load`, typed `DriveParameterSource` | `missing_fields_are_zero_and_record_selection_uses_first_present_ancestor`, `vector7_zero_record_preserves_preset_and_bank_only_does_not_merge_parent_pitch` |
| aircraft §4.1 thirteen fields, exact conversions | `DriveField`, `DRIVE_FIELDS`, `DriveParameters::overlay`, `load_drive_parameters` | `all_thirteen_named_fields_use_the_specified_destinations_and_exact_constants` |
| aircraft §4.1 signed zero, NaN/infinity, negatives | `DriveParameters::overlay` | `overlay_preserves_signed_zero_and_does_not_sanitize_negative_nan_or_infinity` |
| aircraft §4.1 initial preset | `DriveParameters::default/from_preset` | `unknown_constructor_preset_is_not_fabricated_and_known_overlay_can_fill_it`; open: AF9 / AF4 |
| aircraft §4.2 PhysicsInit, InitInstance | `physics_init`, `init_instance`; known home step `init_placement` | open: AF4; `open_rotor_ai_lifecycle_and_player_methods_never_report_success_or_mutate`, placement test |
| aircraft §4.2 NextFrame/engine power | `next_frame` | open: AF2; open-methods test |
| aircraft §4.2 YawPitchRollSlideReal (±890/250), MoveXZReal, MoveYReal | corresponding snake-case methods | open: AF2 (limits alone do not settle integration); open-methods test |
| aircraft §4.2 AI_ControlReset/Hover/MoveXZ/LiftOverBuildings | corresponding snake-case methods | open: AF3 (rate/band summaries do not settle controls); open-methods test |
| aircraft §4.2 AI_PursueGoal | `pursue_goal`, `push_mission_loiter` | dispatch/chase/loiter tests below; complete goals/claims/RESCUE ordering and control/teleport gates open: AF3; messages/dispatch open: AF6 |
| aircraft §4.2 UpdateSpotlight | `update_spotlight`, `HelicopterChaseGeometry` | geometry in chase test; cone/terrain update open: AF5; open-methods test |
| aircraft §4.2 SetHelicopterState/AEMS | `set_helicopter_state`, `update_rotor_sounds` | `phase_changes_are_independent_of_stack_and_mission_loiter_has_6000ms_timeout`; sound mapping open: AF4 |
| aircraft §4.2 OnPlayerTakeOver/TranslatePlayerDriveCommands, helicoptermissle/ufolaser | corresponding snake-case methods | open: AF5; open-methods test; no fabricated command encoding/projectile behavior |
| aircraft §6 world margin | `clip_to_world` | placement/altitude/world-margin test |
| aircraft §7 open questions | explicit `AircraftUnsupported` categories; unknown preset fields | AF2–AF6 / AF9, open-methods test |
| aircraft §8 vector 7 | loader and overlay | `vector7_zero_record_preserves_preset_and_bank_only_does_not_merge_parent_pitch` (other vectors belong to airplane/dispatch owners) |
| automata §6.1 shared fields/stack | `Helicopter::base/base_mut`, reused `AutomatonBase` | record round-trip and default/final dispatch tests; no duplicate base implementation |
| automata §6.2 CHASE reference/default state records | reused `StateExtra::Chase` codec, `pursue_goal` | record round-trip and chase rejection tests |
| automata §6.2 default numeric dispatch (AT17), CROP_DUST | `pursue_goal` | `default_dispatch_pops_every_unlisted_numeric_goal_including_crop_dust` |
| automata §6.4 concrete/base versions, nested record, loaded placement | `read/write`, `AircraftPlacementRecord`, `init_placement` | `vector15_same_input_record_round_trip_nests_base_and_home_without_traffic`, `reads_are_atomic_at_every_truncation_and_reject_bad_nested_versions` |
| automata §6.4 write-failure gate before home vector | reused `AircraftPlacementRecord::write` | aircraft.rs: `failed_aircraft_write_never_attempts_home_vector` |
| automata §6.6 flight phases 0/1/3, repeat no-op, separate goal stack | `set_helicopter_state` | phase/mission-loiter test; sound actions open: AF4 |
| automata §6.6 valid/invalid chase, lead vector, spotlight aim, hover +20 | `pursue_goal`, `HelicopterChaseGeometry::for_target` | `chase_rejects_null_unresolved_and_nonplayer_targets_and_keeps_valid_targets`; literal ×1000×dt; distance/teleport/control gates open: AF3 |
| automata §6.6 0/50/51/53/54/55/56 handlers | `pursue_goal`, `push_mission_loiter` | `unsettled_goal_handlers_are_explicit_and_preserve_the_owner`, phase/loiter test; complete switch bodies open: AF3 |
| automata §12 final states/stack indices | reused `AutomatonBase::pop_state` | `default_pop_obeys_final_state_rules_and_empty_stack_has_no_handler`, default-dispatch test |
| automata §14 vectors 15/31 | record/chase methods | round-trip/chase tests above (remaining vectors concern shared or other owners) |
| automata §15 creation | no registration; foreign manager/director | Outside this core-only owner; native creation/lifecycle failure details open: AF4 |

## Next shadow

A narrow in-game shadow is feasible after the coordinator adds a read-only
adapter: compare imported thirteen-field records and pre/post preset bits;
compare concrete/base versions and saved home vectors; compare phase no-op,
invalid-chase pops and numeric default pops. No flight-position/velocity shadow
can be exact until AF2/AF3 are answered. No game request is made by this lane.

# Automata core

The airplane owner and parameter implementation now live in
[`airplane/mod.rs`](airplane/mod.rs). Its current
[Owner closure table](airplane/README.md) supersedes the airplane rows below:
aircraft §3.1 and numerical §§3.4.1–3.4.2 now have exact constants and conformance
vectors. Station-mask overflow follows the settled wrapping rule. The older
helicopter rows below describe work outside core-airplane9.

`automaton.rs` supplies the shared `AutomatonBase`, state records and stack
contracts. `codec.rs` supplies vectors, occupant records and checked references.
The existing controller/script/manager modules and generic `Automaton` wrapper
retain their existing responsibilities. Aircraft owners in `aircraft.rs` hold
the shared base directly: they have no traffic record and no duplicate base.
Their exclusive mutation and ordinary read getters introduce no `RefCell`.
They are core Rust owners/codecs, with no plugin/C ABI registration.

`AircraftWorld`, `AircraftTargets` and `DriveParameterSource` represent foreign
world/occupant/exemplar queries. They do not implement flight forces, rotor
integration or goal steering on the owner's behalf.

## Owner closure

**The two owners are not closed.** The spec explicitly leaves parameter meaning
and both flight models open (AF1/AF2). Its summary-only AI, lifecycle, sound,
player and dispatch behavior also lacks sufficient rules for compatibility;
AF3–AF7 request those rules. `AircraftUnsupported` identifies the gaps. In
particular, a successful placement/parameter operation is not successful native
Init, a resource declaration does not instantiate effects, and chase geometry
does not claim a completed flight tick. No invented force model is substituted.

Tests below are in `engine/conformance/tests/aircraft.rs` unless noted. Each row
maps the section to actual code and a test, or explicitly identifies the gap.
Sections for the other owner and dispatch sources are outside the row owner's
assignment, rather than unimplemented owner behavior.

| Owner | aircraft.md section | Code item / test or open question |
|---|---|---|
| Airplane | §0 Summary | `Airplane`, `AircraftPlacementRecord`; `aircraft_records_nest_base_and_home_without_traffic`; flight behavior open: AF2/AF3 |
| Helicopter | §0 Summary | `Helicopter`, `AircraftPlacementRecord`; `aircraft_records_nest_base_and_home_without_traffic`; rotor behavior open: AF1/AF2/AF3 |
| Airplane | §1 Classes | `Airplane::class_id`, `CLSID_AIRPLANE`; `aircraft_records_nest_base_and_home_without_traffic` |
| Helicopter | §1 Classes | `Helicopter::class_id`, `CLSID_HELICOPTER`; `aircraft_records_nest_base_and_home_without_traffic` |
| Airplane | §2 Aircraft base | `aircraft_altitude`, `clip_aircraft_to_world`; `altitude_uses_highest_surface_and_clip_keeps_outward_margin`; player tick open: AF5 |
| Helicopter | §2 Aircraft base | Same shared geometry/test; player tick open: AF5 |
| Airplane | §3 Airplane | Detailed §3.1–3.4 rows below; open: AF1/AF2/AF3/AF4/AF5/AF6/AF7 |
| Helicopter | §3 Airplane | Other owner; helicopter rules are §4 |
| Airplane | §3.1 Drive parameters | `AirplaneDriveParameters::{load,from_list}`, `Airplane::init_data`; `airplane_default_and_explicit_speed_parameters`, `airplane_derived_defaults_and_explicit_overrides`, `nearest_parameter_list_stops_parent_inheritance`, `init_loads_independent_ai_and_player_lists`; property identity/player selection open: AF1; list edge cases open: AF7 |
| Helicopter | §3.1 Drive parameters | Other owner; the helicopter scaling/list mapping is §4, open: AF1 |
| Airplane | §3.2 Init and effects | `AircraftPlacementRecord::init_placement`, `airplane_effect_names`; `first_init_restores_loaded_home_and_never_recaptures_it`, `airplane_effect_resources_choose_fire_plane_propellers`; three probes/effect attachment/native Init failures and cleanup open: AF4 |
| Helicopter | §3.2 Init and effects | Other owner; helicopter InitInstance/PhysicsInit is §4, open: AF4 |
| Airplane | §3.3 AI goals | `Airplane::{pursue_goal,begin_loiter,tick_loiter}`; `chase_rejection_obeys_shared_final_state_and_stack_indices`, `phase_and_loiter_ammo_are_independent_of_goal_stack`; goto/takeoff/landing/approach/runway/taxi/fire steering and failure paths open: AF3; dispatch open: AF6 |
| Helicopter | §3.3 AI goals | Other owner; its goal dispatcher is §4 and automata §6.6 |
| Airplane | §3.4 Flight model | `Airplane::set_airplane_state`; `phase_and_loiter_ammo_are_independent_of_goal_stack`; `update_physics` returns Unsupported(AF2), checked by `name_only_crop_dust_and_unsettled_models_do_not_mutate_owners`; sounds/crash AEMS open: AF4; player commands/weapons open: AF5 |
| Helicopter | §3.4 Flight model | Other owner; its rotor model is §4 |
| Airplane | §4 Helicopter | Other owner; airplane rules are §3 |
| Helicopter | §4 Helicopter | `Helicopter::{pursue_goal,push_mission_loiter,set_helicopter_state}`, `HelicopterChaseGeometry`; `helicopter_chase_pops_invalid_target_but_reports_missing_controls_for_valid_target`, `phase_and_loiter_ammo_are_independent_of_goal_stack`; parameters open: AF1, rotor/attitude/collision open: AF2, controls/chase gates open: AF3, Init/sounds open: AF4, spotlight/player/weapons open: AF5, messages/dispatch open: AF6 |
| Airplane | §5 Air dispatch sources | Not owned by Airplane; foreign source bookkeeping. Shared property helper `initialize_fire_plane_station_units` covers §8 test 3; oversized masks open: AF8 |
| Helicopter | §5 Air dispatch sources | Not owned by Helicopter; foreign source bookkeeping. Same shared property helper/test; oversized masks open: AF8 |
| Airplane | §6 Quirks | `clip_aircraft_to_world`; `altitude_uses_highest_surface_and_clip_keeps_outward_margin` |
| Helicopter | §6 Quirks | Same shared geometry/test |
| Airplane | §7 Open questions | `AircraftUnsupported::question`; open: AF1/AF2 plus newly filed AF3–AF7 |
| Helicopter | §7 Open questions | `AircraftUnsupported::question`; open: AF1/AF2 plus newly filed AF3–AF7 |
| Airplane | §8 Conformance tests | Tests 1/2: `airplane_default_and_explicit_speed_parameters`, `airplane_derived_defaults_and_explicit_overrides`; test 3: `fire_plane_station_with_three_units_sets_available_property_to_seven` validates the shared station-property helper |
| Helicopter | §8 Conformance tests | No helicopter-specific test assigned by this section; shared records/placement/chase/phase tests cover its settled contracts |

## Shared automaton contracts

- **automata.md §§6.1/6.2:** `AutomatonBase::{set_state,push_state,pop_state}`
  is used by aircraft and the existing `Automaton` wrapper. Existing automata
  tests cover final-state refusal, DISAPPEAR insertion and stack indices;
  `chase_rejection_obeys_shared_final_state_and_stack_indices` checks an
  aircraft owner using those same contracts.
- **automata.md §6.4 and §14 test 15:** `AircraftRecord` writes exactly version
  1 plus `AutomatonBase`; `AircraftPlacementRecord` writes version 1, that
  aircraft record, then home `vec3`. Concrete versions 0/1 read, base version
  exactly 1. `concrete_versions_accept_zero_but_aircraft_base_requires_one`,
  `aircraft_records_nest_base_and_home_without_traffic`, and
  `truncated_owner_reads_are_atomic_at_every_byte_boundary` cover both owners.
  `failed_aircraft_write_never_attempts_home_vector` checks nested stream failure.
  Reads need no city; loaded placements are applied only at first Init.
- **automata.md §6.6:** flight phases 0/1/3 are independent of the stack;
  airplane phase 1 refills ammo to 3 only when it changes. An airplane pops
  CHASE, a helicopter pops an invalid/non-player target; a valid chase still
  needs AF3's distance/control rules. CROP_DUST remains name-only. Plane LOITER
  uses a separate 2-second timer and ammo decrement; helicopter mission LOITER
  is pushed with 6000 ms. Exact boundary, loaded timer and zero ammo are AF3.
- **automata-managers.md §§1/2/5:** managers and director stay foreign. IDs
  match §1; no traffic record, manager replacement, SDK slot guesses, simulator
  implementation, C ABI export, or plugin registration is added here.

Runtime defaults absent from the spec (initial flight phase/ammo) remain
`None`, rather than guessed ground/cruise values. Physics/valid-target chase
return Unsupported without moving the owner. Save/load uses the shared stream
contracts; successful reads replace the owner atomically and clear transient
phase/parameter state. AF4 asks for native reconstruction/failure details.

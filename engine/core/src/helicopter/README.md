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
x87, with the spec's tolerances. Shared time conversion now follows §3.4.1a (AF10);
the wider rotor model still has the §9.1 observation boundary.
`closure.rs` adds specified rotor power/frame order, home completion, startup,
claim release, state callbacks and bounded ordered goal/dispatch requests. Motion
kernels, continuous steering and native service callbacks remain explicit inputs.

## Owner closure

“settled” below names `engine/conformance/tests/aircraft_settled.rs`;
“closure” names `engine/conformance/tests/aircraft_closure.rs`.
Every row concerns cSC4Helicopter. An open row is an explicit Unsupported boundary,
not a successful native flight/control tick.

| Spec section | Code item | Test or residual native observation | Closure |
|---|---|---|---|
| aircraft §0 scope | `Helicopter` | settled: vectors 14–16 preset/overlay/read | implemented |
| aircraft §1 identity | `class_id` | helicopter.rs: vector 15 nested record | implemented |
| aircraft §2 altitude/world clipping | `get_altitude`, `clip_to_world` | helicopter.rs: placement/altitude/world-margin | implemented |
| aircraft §2 player tick | `do_player_drive_tick` Unsupported | AF5 §9.4 raw commands/readiness/projectile trace; AF2 §9.1 rotor motion/contact outputs | open AF2/AF5 |
| aircraft §3.1 inherited named-record rules | `DriveRecord::import`, `DriveParameters::load` | helicopter.rs: first-present ancestor and missing fields | implemented |
| aircraft §3 other airplane contracts | Other owner | airplane/README.md | other owner |
| aircraft §4.1 fields/conversion/zero fallback | `DriveField`, `DriveParameters::overlay` | helicopter.rs: thirteen fields/constants, signed-zero/non-finite overlay | implemented |
| aircraft §4.1.1 preset/reconstruction | `preset0`, `init_instance_with`, `physics_init`, `read` | settled: vectors 14–16 | implemented |
| aircraft §4.2 rotor physics overview | Unsupported motion kernels; `RotorPower::update`, `RotorFrameState::next_frame`, `physics_home_stage` | closure: vector 32; AF2 §9.1 attitude/slide/XZ/Y/transform and collision-stage outputs | open AF2 |
| aircraft §4.2 complete goal switches/claims | `pursue_goal`, `push_mission_loiter`, `goal_actions` | helicopter.rs default/final/chase/timeout; closure: vector 31; AF3 §9.2 MoveXZ controls and 0x36–0x38 claim/crime/rescue service traces | open AF3 |
| aircraft §4.2.1 reset/hover/time | `RotorControls::{reset,hover}`, `helicopter_physics_seconds` | settled: vectors 17/18; closure: vector 28 shared time classification (AF10) | implemented |
| aircraft §4.2.2 chase gates/movement/aim | `chase_actions`, `pursue_chase_stage` | settled: vector 19; AF3 §9.2 MoveXZ control outputs; AF5 §9.4 target/yaw/terrain spotlight geometry | open AF3/AF5 |
| aircraft §4.2.3 phase requests/triggers | `set_helicopter_state` | settled: vectors 21/22 cover setter; AF2/AF4 §9.1/§9.3 physics request causes and attachment creation | open AF2/AF4 |
| aircraft §4.2.4 building lift | `building_clearance_height`, `ai_control_lift_over_buildings` | settled: vector 24 | implemented |
| aircraft §4.2.5 allocation/Shutdown/full Init | `RuntimeOwnership`, `init_instance_with`, `shutdown_after_base`, `shutdown_with_claims` | settled: vector 25; closure: vector 33 resolves pre-base cleanup (AF4); AF4 §9.3 full native Init/attachment/failure order | open AF4 |
| aircraft §4.2.6 power/frame order/home completion | `RotorPower::update`, `RotorFrameState::next_frame`, `physics_home_stage` | closure: three vector 32 power/order/home tests (AF2/AF4), normal kernels explicitly supplied | implemented |
| aircraft §4.2.7 pre-base claim release | `shutdown_with_claims` | closure: `vector33_duplicate_claim_removal_precedes_base_and_cleanup` (AF4), duplicate/all-match/epsilon/sentinel cases | implemented |
| aircraft Init/attachment creation | `init_instance_with`, `physics_init`; full Init unavailable | AF4 §9.3 constructor remainder, effect attachment/failure/repeated-Init order and phase-request causes | open AF4 |
| aircraft §4.3 weapons | `weapon_requests` | settled: vector 23 covers accounting; AF5 §9.4 full translation/cadence/projectile initialization | open AF5 |
| aircraft §4.3.1 takeover | `refresh_player_parameters`, `on_player_take_over` | settled: vector 26 | implemented |
| aircraft §4.3.2 spotlight activation gate | `spotlight_activation_stage` | settled: vector 27; AF5 §9.4 target/yaw/terrain and inactive-pair output transforms still required | open AF5 |
| aircraft §4.3.3 state callbacks | `enter_state`, `exit_state` | closure: `helicopter_callbacks_keep_base_result_limits_mass_and_exit_dispatch` (AF5/AF6); refresh helper returned as a request | implemented |
| aircraft §4.4 bounded goal/startup/dispatch requests | `goal_actions`, `start_stage`, `set_dispatch_state` | closure: vector 31 and `startup_phase_gate_and_ordered_dispatch_repeat_rules` (AF3/AF6), phase-4 equality and repulsor-before-notify order | implemented |
| aircraft §5 dispatch sources/messages | Foreign source owners; `set_dispatch_state` | closure: startup/dispatch and callback tests; AF6 §9.5 observer/message envelope/service failures | open AF6 |
| aircraft §6 world margin | `clip_to_world` | helicopter.rs: placement/altitude/world-margin | implemented |
| aircraft §7 remaining native boundaries | explicit Unsupported results; optional unknown constructor fields | AF2–AF6 §9.1–9.5 as above; AF11 §9.6 complete registration/removal set and independent input fixture/history | open AF2/AF3/AF4/AF5/AF6/AF11 |
| aircraft §8 specified portable conformance | codecs, controls, bounded numerical/action stages | aircraft.rs, helicopter.rs, aircraft_settled.rs, aircraft_closure.rs vectors 28/31–34 | implemented |
| automata §6.1/§6.2 base and state records | `AutomatonBase`, `StateExtra::Chase` | helicopter.rs: record/default/final-state | implemented |
| automata §6.4 concrete codec/home | `read`, `write`, `init_placement` | helicopter.rs: atomic truncation/round-trip; aircraft.rs failed-write gate | implemented |
| automata §6.5 mission sequencing | `pursue_goal`, `push_mission_loiter`, `goal_actions` | helicopter.rs default/6000-ms push; closure: vector 31; AF3 §9.2 remaining claim/crime/rescue/completion controllers; AF6 §9.5 external callbacks | open AF3/AF6 |
| automata §6.6 phases/chase | phase/chase stages, `goal_actions`, `start_stage` | settled: vectors 19/22; closure: vector 31/startup; AF3 §9.2 continuous steering/mission branches | open AF3 |
| automata §12 stack quirks | `pop_state` | helicopter.rs: default/final-state/empty-stack | implemented |
| automata §14 vectors 15/31 | record/state codec and chase target handling | helicopter.rs round-trip/chase rejection | implemented |
| automata §15 COM creation/native observation | Foreign director/manager | AF4 §9.3 lifecycle integration/failure ordering; AF11 §9.6 retained-QI/borrowed-parameter lifetimes, complete population and deterministic input fixture | open AF4/AF11 |

“Implemented” applies to the named bounded contract and its tests. Whole-owner
implemented credit remains partial while the explicit §9 observation rows are open.
The game metadata shadow compares class/four record versions on its actual archive
subset or private fixture. It does not establish population completeness, expected
home/goals/parameters, or trajectory equivalence (§4.5/§9.6, vector 35).

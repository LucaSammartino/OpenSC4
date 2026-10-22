| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-watercraft4: public watercraft owner | Codex / `core-watercraft4` (engine, detached reserved worktree) | WIP: partial owner, WC3-WC7 blockers; cargo 4255/4255, clippy 0 diagnostics, ctest 67/67; ABI unchanged; no game/shadow claim | 1. Publish missing inventory/equations. 2. Finish named remaining methods below. 3. Reconcile questions and verify before any owner coverage credit. |

## Report

The complete cSC4Watercraft owner is **blocked by missing public contracts**. This is a committed, tested partial core implementation, not a completed 79-function owner or a coverage recommendation. The permitted specification does not enumerate all 79 attributed functions, so even an exact 79-method checklist cannot be completed without WC3. The table below inventories every named owner method visible in the permitted contract and distinguishes complete substeps from incomplete methods.

### Changes

- `b87fa6b2`: executable parameter loading, group ancestry, reusable automata record/state storage, runtime data, owner state hooks, docking requests, movement helpers, initial independent vectors and WC3-WC7 questions. Code was committed in the requested early implementation window.
- `266de0cb`: lifetime/manager ordering, cumulative idle budgets, tow landing selection using the Rust fleet, two-sided link cleanup, decoded ferry/neighbor reader stepping, direction masks, bounded final docking/horizontal undocking, effect reference ownership, and 30 independent public-contract tests. This is the code revision that passed the full gate.
- New files are under `engine/core/src/watercraft/`, with a single additive `lib.rs` registration. No plugin, CAPI, ABI, generated ABI table or tools changes. No runtime activation.
- The core never accepts native watercraft tick, motion, collision or constructor results. Providers expose foreign terrain, berth/retention resource services, and effect creation. Route cells are decoded network data; the unspecified compressed byte format is not claimed implemented. Events describe foreign side effects; stop-effect events retain acquired references until an adapter stops and drops them. Core state getters introduce no RefCell.

### Measured validation

- Initial `qbuild all` was invoked first, but overlapped subsequent edits: workspace cargo passed and clippy rejected three exact specification decimal constants. It is not represented as a pristine baseline. The constants now have narrowly scoped `approx_constant` allowances.
- Quick gate: core 2383/2383; dependent CAPI/conformance/formats compiled; CMake 4/4. Two earlier vector failures were corrected: removal releases the source berth; inherited occupant Read changes a loaded-status bit.
- Final `qbuild all` on committed code `266de0cb`: **4255 passed, 0 failed, 0 ignored** across the workspace; core **2388/2388**, including **30 watercraft vectors**; clippy **0 diagnostics**; CMake **4/4**; ctest **67/67**. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
- Required fmt check: all **8 new Rust files pass**. Including `lib.rs` reports pre-existing module/re-export ordering/formatting differences (125 output lines). The diff from the base in that file is only `pub mod watercraft;`; no mass reformat was applied. The first fmt invocation used relative paths and failed to resolve them; absolute-path checks produced the results above.
- Vectors cover parameter ancestry/dependent defaults/NaN and signed zero, save truncation/version/runtime separation, effect acquisition/release/failure/order, exact state entry/removal/idle thresholds/final-state handling, lifetime retention/parked/stack ordering, dock arbitration, direction-mask exhaustion for 8 headings x 256 masks, decoded route exhaustion, tow selection/disarming/link cleanup, movement ordering, bounded turn/translation, collision ratio boundaries, coasting, gear gates, wake timing, and debug formatting. They are synthesized from the public contract, not fitted native goldens. The missing native crash/movement RNG rules have no invented substitute and no RNG coverage claim.
- Game runs: **none**. The task restricts this lane to a pure Rust core and provides no bridge activating it. No native/shadow exactness or gameplay readiness is claimed. No request outstanding.

### Blockers and exact known remaining methods

- **WC3** (`docs/specs/questions/watercraft.md`): missing 79-function inventory, native constructor/destructor and inherited lifecycle/failure details, fourth Init flag, Read compatibility rules. `initialize_runtime` is only the specified owner portion, after inherited Init; full Init/Shutdown/Read/native ownership compatibility is not claimed.
- **WC4**: full `DoAnimationTick`, `Engine_AIBoatMotion`, `Movement_PositionNew`, and its helpers at `0x00569940`, `0x00569E70`, `0x0056A440`, `0x0056AF90`; inherited player/animation steps and sound/effect/model updates; far-approach `DoDockingTick`; full `DoPlayerDriveTick` and `DoFollowingTick`. State-only/math substeps exist, but these named methods remain incomplete.
- **WC5**: full `Engine_DetectCollision`, footprint/size and bump calculation, `Crash_InanimateCollisionDetection`, `DoCrashSequenceTick`, and reconciliation of conflicting crash-entry notification values. Threshold helpers and case/state dispatch are not collision-owner completion. Crash ticks update the specified accumulator and return an explicit contract gap instead of pretending to animate successfully.
- **WC6**: full `TranslatePlayerDriveCommands` speed/steering branches, sound/tween behavior, fire/skier positioning and effect feedback, and inherited translation. Exact gear, rudder, coasting and stopped-tug button substeps are implemented.
- **WC7**: full `Engine_AIPickLocation` wandering/backtracking/overlap and direction-coordinate convention; compressed route reader; ferry exhaustion/held-berth ownership ambiguity; blocked/on-land tick returns; following rate/radius; nearest-boat ties. `DoStateTick` still lacks full geometry-dependent dispatch. Decoded-cell stepping and modular direction priority are explicitly substeps, not complete picking/route support.

Finish these methods and reconcile the numbered choices before any ready status or whole-owner credit. WIP queue entry only; no merge, ref publication, push, restart, or further round was performed by this lane.

## Public-contract owner checklist (no whole-owner credit)

The specification does not enumerate the attributed 79 functions. WC3 requests that exact inventory. This checklist enumerates every owner method actually named in the permitted sections; inherited operations are not counted as implemented owner methods.

| Named owner method | Implementation / remaining contract |
|---|---|
| GetGZCLSID | Watercraft::class_id / CLASS_ID |
| Write, Read | WatercraftRecord; only documented version 2 accepted (WC3) |
| LoadDriveParameters | DriveParameters::load, Group ancestry, owner reload |
| Init | initialize_runtime is owner portion only; fourth flag/base behavior/failures WC3 |
| Shutdown | Fleet::shutdown owner order; native destructor/base behavior WC3 |
| HandleEnterState | Fleet::push_state owner hooks; notify conflict WC5 |
| HandleExitState | Fleet::pop_state owner hooks and inherited final-state stack rule |
| DoStateTick | Idle/crash accumulators/removals/queue; full geometry dispatch pending; crash sequence WC5 |
| EndTowing | Fleet::end_towing |
| EndDocking | Fleet::end_docking |
| DoAnimationTick | Budget/lifetime/wakes/backwards/still-time substeps; full inherited step/motion/sound/effect/model orchestration WC4 |
| Engine_AIPickLocation | Pending supported route selection; encoding/backtracking WC7 |
| Assist_PickNewDir | candidate_directions; exhaustive eight headings and 256 masks |
| Engine_AIBoatMotion | Missing equations WC4 |
| Movement_PositionNew and 0x00569940/0x00569E70/0x0056A440/0x0056AF90 | Missing equations and helper names WC3/WC4 |
| Ferry_OtherStationInformation | Watercraft::other_station |
| Ferry_DockRequests | Fleet::dock_requests; request ownership/current-station ambiguity WC7 |
| Ferry_ClearDockRequests | Fleet::clear_dock_requests |
| DoDockingTick | Final approach with literal post-movement completion (WC4); far AI WC4, land return WC7 |
| Ferry_SetStationInformation | Watercraft::set_station_information |
| Ferry_InitializeCurrentRoute | Watercraft::initialize_current_route; decoded network cells only WC7 |
| Ferry_SwitchActiveStation | Dock exit hook |
| SetNbrRouteData | Decoded network route ownership/near-cell skip, neighbor/ferry reader stepping; byte decoder WC7 |
| DoUndockingTick | Fleet::undocking_tick; blocked/land return and degenerate vectors WC7 |
| Engine_MovementPossible | Fleet::movement_possible with occupant cell rectangle |
| OnLand | Watercraft::on_land |
| Engine_DetectCollision | Threshold helpers only; full contact/bump/size/footprints WC5 |
| Crash_InanimateCollisionDetection | Missing hit equations WC5 |
| Crash_DetermineCase | Watercraft::determine_crash_case |
| SetStateInternal | Fleet::set_state_internal |
| DoCrashSequenceTick | Missing ramp/heel/RNG equations WC5; explicit ContractGap |
| DoPlayerDriveTick | Coasting and ahead-tow cancellation; inherited step/full movement WC4 |
| TranslatePlayerDriveCommands | Gear/rudder/tow-button helpers; speed/steering/fire/skier/tween/sound WC6 |
| DoFollowingTick | Behind-target geometry only; speed/radius/turn details WC4/WC7 |
| DoMessage | Reload, tow landing from foreign world point, Rust nearest-boat selection and links, forward disposition; distance tie WC7 |
| GetDebugExtraText | Watercraft::debug_extra_text |

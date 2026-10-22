| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-watercraft4: public watercraft owner | Codex / `core-watercraft4` (engine) | in progress | Implement supported owner contract; identify missing contracts; gate. |

## Handoff
- Goal: portable Rust cSC4Watercraft owner from watercraft sections 1-7,9,11 and automata watercraft states only.
- Detached HEAD reserved by supervisor; do not update branch refs.
- Initial qbuild all: cargo passed; clippy saw the new exact decimal constants and failed approx_constant. This overlapped edits and is not a pristine baseline; exact constants now locally allowed.
- Implemented parameter ancestry, base-only record, explicit runtime storage, state hooks/timers, docking request branches, movement substeps and typed foreign resources. Not whole-owner complete.
- WC3-WC7 filed; complete owner inventory and movement/crash/player/route equations are missing.
- Added 14 independent contract vectors; first quick build running (exec session 78827).
- Next: inspect quick result; finish supported route/tow/lifetime branches; full gate.
- No CAPI/plugin/ABI edits or game requests.

## Public-contract owner checklist (no whole-owner credit)

The specification does not enumerate the attributed 79 functions. WC3 requests that exact inventory. This checklist enumerates every owner method actually named in the permitted sections; inherited operations are not counted as implemented owner methods.

| Named owner method | Implementation / remaining contract |
|---|---|
| GetGZCLSID | CLASS_ID constant |
| Write, Read | WatercraftRecord; only documented version 2 accepted (WC3) |
| LoadDriveParameters | DriveParameters::load, Group ancestry, owner reload |
| Init | initialize_runtime is owner portion only; fourth flag/base behavior/failures WC3 |
| Shutdown | Fleet::shutdown owner order; native destructor/base behavior WC3 |
| HandleEnterState | Fleet::push_state owner hooks; notify conflict WC5 |
| HandleExitState | Fleet::pop_state owner hooks; inherited final-state handling pending |
| DoStateTick | Idle/crash accumulators/removals/queue; full geometry dispatch pending; crash sequence WC5 |
| EndTowing | Fleet::end_towing |
| EndDocking | Fleet::end_docking |
| DoAnimationTick | Pending ordered orchestration; inherited step/effect/model updates WC4 |
| Engine_AIPickLocation | Pending supported route selection; encoding/backtracking WC7 |
| Assist_PickNewDir | Pending explicit modulo-eight priority iterator |
| Engine_AIBoatMotion | Missing equations WC4 |
| Movement_PositionNew and 0x00569940/0x00569E70/0x0056A440/0x0056AF90 | Missing equations and helper names WC3/WC4 |
| Ferry_OtherStationInformation | Watercraft::other_station |
| Ferry_DockRequests | Fleet::dock_requests; request ownership/current-station ambiguity WC7 |
| Ferry_ClearDockRequests | Fleet::clear_dock_requests |
| DoDockingTick | Exact already-docked/final-approach substeps; full completion/far AI WC4 |
| Ferry_SetStationInformation | Watercraft::set_station_information |
| Ferry_InitializeCurrentRoute | Watercraft::initialize_current_route; decoded network cells only WC7 |
| Ferry_SwitchActiveStation | Dock exit hook |
| SetNbrRouteData | Decoded network route ownership/near-cell skip; byte decoder WC7 |
| DoUndockingTick | Fleet::undocking_tick; blocked/land return and degenerate vectors WC7 |
| Engine_MovementPossible | Fleet::movement_possible with occupant cell rectangle |
| OnLand | Watercraft::on_land |
| Engine_DetectCollision | Threshold helpers only; full contact/bump/size/footprints WC5 |
| Crash_InanimateCollisionDetection | Missing hit equations WC5 |
| Crash_DetermineCase | Watercraft::determine_crash_case |
| SetStateInternal | Fleet::set_state_internal |
| DoCrashSequenceTick | Missing ramp/heel/RNG equations WC5; explicit ContractGap |
| DoPlayerDriveTick | Coasting substep; inherited step/full movement WC4 |
| TranslatePlayerDriveCommands | Gear/rudder helpers only; remaining speed/steering WC6; buttons pending |
| DoFollowingTick | Behind-target geometry only; speed/radius/turn details WC4/WC7 |
| DoMessage | Pending reload/tow landing/forward disposition |
| GetDebugExtraText | Watercraft::debug_extra_text |

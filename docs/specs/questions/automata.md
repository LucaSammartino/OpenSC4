# automata: open questions
<!-- Questions about [automata.md](../automata.md). Append new entries at the END; when one is answered, move it to answered/automata.md. Rules: README.md. -->

## From the automata state behaviours and the effect-directory measurements (re-integrate1 lane, 2026-10-16)

Four RE reports, kept verbatim in `build-runs/re-integrate1-automata.md`, `re-integrate1-effects.md`,
`re-integrate1-vehicle-ticks.md` and `re-integrate1-air-water-ticks.md`. Their findings went into automata.md
§6.2, §6.5, §6.6, §8.1, §12-§14 and effects.md §3, §5.2, §7, §10.1, §10.2, §15-§17.

**Nothing needed striking here for the automata state questions**: automata.md's Q3-Q5 (filed above as AT3-AT5)
and AM1-AM12 were already answered by re-answers16 and re-answers20, and the effects questions EF1-EF5 lived in
effects.md §16 rather than in this file. Two of those are now closed or answered and are struck below so a
reader does not re-open them. AT6-AT19 continue automata.md §13's new Q7-Q20; EF1-EF10 continue effects.md §16.
The probe named is the one the report gives.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **AT6:** what is PlayerDrive's saved `f32` at `+0x28`? Set to 0 on enter and never touched by
  `cSC4Automaton::DoPlayerDriveTick`. [a drive-channel / throttle accumulator, inferred] **Probe:**
  `cSC4Vehicle::TranslatePlayerDriveCommands` (Mac `0x000a577c` family) and `DoPlayerDriveOffRoadTick`.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **AT7:** what reads Bumped's 4 x `vec3`? Base `DoBumpedTick` is `return !CheckAutomataFlag(1)`
  (Mac `0x000bb14e`) and `cSC4Vehicle::DoBumpedTick` is `return 0` (Mac `0x000a083e`); `HandleCollision`
  (Mac `0x000a091e`) writes them then pushes COLLIDE. [a leftover bump-response record, inferred] **Probe:**
  xrefs to `tAutomataStateBumped::Read`'s output and `cSC4Train::HandleCollision` (Mac `0x00393a4a`).

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **AT8:** PlopJump's units. `U(30,50) x 1/3600` and `kfGravityAccel = -5.5555563e-05` only make sense
  together if `dt` is ms and the result is m/ms. [inferred] **Probe:** one scripted plop and a position trace.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **AT9:** what pops the `final` RiotLeader after the riot end's `SetState(1, ...)` fails (automata.md §8.1)?
  The call is a no-op because the state was pushed `final=1`, and the leader is then `Release`d and
  unregistered. **Probe:** trace `cSC4DisasterInstanceRiot::Update`'s removal path.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **AT10:** `random_walk.turn`'s exact template offsets. `ImportBehaviorTable` writes the array/scalar into
  `+0x54`/`+0x58` and zeroes `+0x5C`, while `DoRandomWalkTick` reads amplitude at `+0x58` and bias at `+0x5C` -
  one is off by 4. **Probe:** a Lua table that sets `turn = {a, b}` and a breakpoint on `0x0050CF90`.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **AT11:** the mapping of emergency-vehicle states 100-103 to their ticks.
  `cSC4EmergencyVehicle::DoStateTick` `0x0052F490` switches into slots `0xC3`-`0xC6`; the names are by
  elimination, not from a name table. **Probe:** the emergency vehicle's vtable at slots 195-198.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **AT12:** the exact contract of `FUN_005096B0` (the per-tick behaviour refresh) on `state+0x24` vs
  `state+0x08`. **Probe:** `cSC4AutomataController::GetBehaviorTemplate` (`0x0051BAB0` / Mac `0x000b8ce0`).

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **AT13:** the fate of an aircraft or watercraft that finishes `CRASH_RUNAGROUND` / `CRASH_SINK` - neither
  transitions when its duration expires. [the manager later reaps it, inferred] **Probe:** trace
  `cSC4WatercraftManager` occupancy/cleanup.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **AT14:** whether `field259_0x2BC`'s idle budget never being re-zeroed is intentional lifetime-based
  despawning of idle boats or a missed reset (a boat that has spent 60 s total in IDLE is slow-removed).
  **Probe:** every writer of that field.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **AT15:** whether `Crash_SetStateInternal`'s slot-59 read is a "don't re-crash" guard - Ghidra dropped the
  comparison (Mac `0x003804a6`). [inferred] **Probe:** `x86dis` on the Windows inlined copy at `0x00568570`, or
  `0x00570270`'s callers.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **AT16:** whether the helicopter `AI_PursueGoal` `default:` really falls through into the `0x32` body.
  [a safety net, inferred] **Probe:** `x86dis` on the jump table at `0x0053BBA0`.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **AT17:** the semantics of automaton vtable slot 102 (Windows) / 109 (Mac), the `(0xB, 0)` crash effect/anim
  kill, and the names of slots 77 (`EnableMotion(bool)`) and 82 (`SetTransform`) - all absent from
  `docs/abi/interfaces.tsv`. [inferred names] **Probe:** dump those vtable entries.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **AT18:** what the city query in the blocked-wait exit path means (`DoStopSignTick` step 5:
  `spCity->vtable+0xA0` -> `vtable+0x84(netOccupant, automaton, routeInfo, &out)`; returning 0 pops the wait).
  Whether 0 means "intersection still blocked" or "release failed" is not determinable from the bytes.
  [inferred] **Probe:** name that interface method (likely `cISC4TrafficNetworkMap` / `cISC4TrafficSimulator`).

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **AT19:** whether `this+0x94` is the measured current speed (written as `distance/dt` in both `DoPathTick`s,
  zeroed at the end of the pull-over move tick), and whether Windows slot `0xA0` in `DoStopSignTick` step 3 is
  `ReturnToDefaultSpeed`. [inferred] **Probe:** `cSC4Automaton::GetSpeed`'s body (slot 34); dump the Windows
  vtable entry at `+0x280`.

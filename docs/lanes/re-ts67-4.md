| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-ts67-4: answer TS67 (phase positions +0x90C/+0x90E zeroed between FullTrips and FinishTripMap) | `re-ts67-4` / (RE spec lane) | ready: TS67 answered DIFFERS (entering 15 zeroes the positions, arm `0x0072110B`); traffic-simulator.md §3.2/§3.5/§4.7/§6.1 updated; questions indexed; no engine, no ABI | 1. traffic core lane: zero +0x90C/+0x90E on entering 15 (and 1) in `cycle.rs`, invert test `ts67_finish_trip_map_entry_keeps_phase_positions`; 2. re-run train22 traffic shadow, expect the 13 month lines exact |

## Report

**Answer (TS67, DIFFERS from the core's literal reading).** In WorkOnSimCycle (`0x00720F00`) the finish path
(`0x00721073`) loads the next state from `kNextState` (`0x00AB3480`), stores it and dispatches the new state's entry
arm through byte index `0x00721564` and jump table `0x0072153C` (decoded from the exe bytes, not the decompiler):

- states 15 and 1 -> arm `0x0072110B`: lot iterator := first lot (`+0x904`), +0x90C := 0 (`0x00721123`),
  +0x90E := 0 (`0x0072112A`), `+0x838` := 1.
- states 16, 17, 18 -> arm `0x007210B0`: source iterator Reset (`0x007107A0`), +0x90C/+0x90E := 0
  (`0x007210BE`/`0x007210C5`, EBX = 0 from `0x00720F21`), `+0x838` := 1, `+0x884` := 0; for 18 also `+0x430`
  vector := 9 zeros.

So 16 -> 17, 17 -> 18 and 18 -> 15 all zero the positions. The trip phases' only writes are the per-source/per-lot
ones (`0x0071E276`/`0x0071E27D`, `0x0071D0FD`/`0x0071D11F`, `0x0071D51C`/`0x0071D53E`, `0x0070FFE0`/`0x0070FFE7`);
FullTrips' exhausted end (`0x0071D806`) does not touch them. The re-route context `+0x83C`/`+0x840` is not reset by any
arm (consistent with the shadow matching it). Deadline stops run no entry arm.

**Spec changes** (docs/specs/traffic-simulator.md): §3.2 table rows 16, 15, 1 and a new paragraph "Phase positions
across the trip phases" (exact write/reset contract); §3.5 step 11 (state 15 resume from (0, 0)); §4.7; §6.1 row 14.
TS67 moved to questions/answered/traffic-simulator.md; QUESTIONS.md regenerated. Export-only (re_query asm + exe
bytes); REA not used.

Implementer pointers: core `cycle.rs` entry action for state 15 (and 1); test
`ts67_finish_trip_map_entry_keeps_phase_positions` must now expect (0, 0).

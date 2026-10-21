| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-ts67fix4: traffic TS67 entry resets of the phase positions (states 15 and 1) | `verify-ts67fix4` / (engine lane) | ready: code 518c5a2b, dae88670; no ABI change; gate cargo 3636 passed, clippy 0, ctest 62/62; game verify-ts67fix4-t2032-a clean ref_warnings=0, traffic shadow 72/72 months clean, resyncs=0; verify-ts67fix4-n2028-a (normal speed) clean ref_warnings=0, traffic shadow 23/23 clean, resyncs=0; no questions filed | - |

## Report

What changed (engine/core only, no plugin source or ABI change; the plugin DLL changes through the core):

- `engine/core/src/traffic_simulator/cycle.rs` `enter_state`: states 15 (FinishTripMap) and 1 (lot sweep) share
  one entry arm per traffic-simulator.md §3.2 rows 15/1 and the TS67 answer: lot iterator reset (`finish_resume`
  := None) and phase positions (+0x90C/+0x90E) := (0, 0). The `+0x838` := 1 flag has no core field (comment says
  why). States 16/17/18 already zeroed the positions on entry (confirmed, unchanged). No reset at the FullTrips
  exhausted end and none of the re-route context x/z (+0x83C/+0x840), as the answer says.
- `tests.rs`: `ts67_finish_trip_map_entry_keeps_phase_positions` (literal reading) replaced by
  `ts67_trip_phase_transitions_zero_phase_positions`: a finished FullTrips with (100, 128) enters 15 at (0, 0);
  a stale resume point is dropped and the walk then stores the first lot (1, 1); 16 -> 17 and 17 -> 18 finishes
  leave (0, 0) after the phase had stored a source cell; entering 1 zeroes too.
- `shadow.rs`: comment no longer calls TS67 open.

Measured:

- Gate (`qbuild all`): cargo test 3636 passed / 0 failed, clippy 0 diagnostics, fc4verify build 183/183, ctest
  62/62.
- Game verify-ts67fix4-t2032-a (turbo, traffic_shadow = 1, to 2032-01-01): verdict clean, ref_warnings=0; traffic
  shadow 72 month lines, 72 with every mismatch count 0 (before: 59 of 72, the 13 differing months all the TS67
  `18 -> 15, progress 0` phase-position kind); resyncs=0 in all 72 months; INCOMPLETE 0.
- Game verify-ts67fix4-n2028-a (normal speed, traffic_shadow = 1, to 2028-01-01): verdict clean, ref_warnings=0;
  traffic shadow 23/23 month lines clean, resyncs=0 in all, 863 deliveries compared, INCOMPLETE 0.

Questions: none (no remaining mismatch).

Notes: `rustfmt --check` reports a diff in shadow.rs near line 1390 that predates this lane (left as is).

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| quitcrash28: native construction teardown finding; kind engine | Codex / quitcrash28 / detached HEAD (reserved branch quitcrash28) | ready; c7d5b60e9, fc69c58e0, d82940552, 4401d214a; cargo 7877 passed, clippy 0 diagnostics, CTest 119/119, tools 494 passed; 5 expected exit-crash findings, 3 clean controls; shadows 0/0; ABI false; questions none | - |

- Result: native teardown fault for an action-created incomplete construction object; tick-owned RequestQuit is not necessary. Keep the findings verdict; do not suppress it or enable diagnostic switches for ordinary suite runs.
- Mechanism (re-nm28 plus re_query): Shutdown 0x005D0F94 dereferences the null owned dust effect (+0x13C complete object); Init precedes successful SetLot. Native StartConstruction 006CC81F ignores SetLot's result and registers the occupant at 006CC837. Runtime did not separately trace failed SetLot versus an earlier effect release.
- Minimal: `engine/tests/scenarios/scripts/quit-construction28.script`, one airport (8,11), 2026-01-30, end 2026-02-01; qualified row in suite.tsv set findings.
- Runs below: shared queue, plain (enable=none, disable=simulator,budget), parallel=1 before [ini], at most 3 in flight; slot copies only, Documents untouched. Same DLL SHA256 4118de34a73ee876e1fe9dd2a03e343fde263a3ae588d4048008e2a3a74d01c6.
- `qc28r2-nativepaths-legacy`: end=1, actions 1/0, clean-exit-crash at 0x005D0F94.
- `qc28r2-nativepaths-post`: native app command queued=1, end=1, actions 1/0, identical exit crash; delivered by main-loop message pump, not a synchronous tick call.
- `qc28r2-nativecontrols-pause`: Pause()=1, paused=1; end=1, actions 2/0, same posted-quit crash.
- `qc28r2-nativecontrols-save`: full SaveCity ok=1 in 11125 ms (bounded save grace); end=1, actions 2/0, same posted-quit crash.
- `qc28r2-nativecontrols-demolish`: Lua Demolish status=0/payload=0 before teardown; end=1, same crash. Successful removal of the orphan is unverified; do not count this as a cleared-construction control.
- `qc28r2-construction-instant`, `-postinstant`, `-postidle`: all end=1, actions 1/0, clean, reached PostSystemServiceShutdown; no foreign exceptions/reference warnings, shadows 0/0. Animation-off avoids the incomplete object; it is a separate control.
- Retained round-1 `qc28-port2`: wait until May and dezone did not cure the same exit crash (624dd491 DLL); not a proven removal either.
- Fix to partial harness: native legacy target-post 00930273 is void (the apparent false was an unlock return); dispatch unavailability now fails, never terminates as clean. autotest_quit_post/script_port_instant/script_port_probe remain off by default.
- Suite tests now recognize accepted posted quit and reject failed-dispatch false-clean logs. Real replay `python build-runs/measure-quit28.py` printed 5 clean-exit-crash, 3 clean, 2 harness-error, 1 stalled; shutdown=True only for the 3 clean controls.
- Exclude `qc28r2-paths-post`, `qc28r2-controls-*` (old partial DLL): queued=0 then process termination, not completed clean exits. `qc28r2-dispatch-single` correctly stalled. Earlier qc28-portplain/fullplain had failed actions/stale DLLs; earlier save stalled.
- Tests: `python -m pytest tools/tests -q` printed `494 passed in 110.72s`; focused suite printed `22 passed in 0.51s`. q.py wrapper failed before pytest (Bash mapping access denied); direct PowerShell invocation passed.
- Gate: `python <MAIN>/.claude/lane-bin/qbuild.py --wt <WT> all` invoked once, printed qbuild PASS; cargo 7877 passed/0 failed, clippy exit=0/diagnostics=0, build 102/102, CTest 119/119 (312.3 s). Three earlier quick builds passed; no Rust source or C ABI exports changed.
- gen27 s0-default/s0-grow clean requirement is conditional on a harness quit fix: not applicable to this native finding; no full-pair clean claim. Separate mid-run crash and twelve last-reference warnings are unassigned.
- Landing: kind engine, actual full detached HEAD in build-runs/merge-queue/quitcrash28.json; clean control logs supplied for credit. No pending game requests, unresolved implementation changes, or question IDs; demolition-removal uncertainty above remains a limit of the finding.

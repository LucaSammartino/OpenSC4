| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| reach27 native entry census; engine | Codex / .claude/worktrees/reach27 / reserved reach27 (detached HEAD) | WIP; code 4d26a816e, census facba8599; ABI unchanged; Rust 7839, clippy 0, CTest 115/115; reach27-r2-calibration-1, reach27-r2-house-off-1, reach27-r2-house-on-1 clean; monthly 5659/5731 exact each; questions none | 1. Investigate load-only overlay self-lighting mismatch. 2. Fence remaining late-load fixtures and repeat canonical reach-on. 3. Review baseline occupant queue exception before ready. |

Handles EXCEPTION_BREAKPOINT/SINGLE_STEP and STATUS_WX86_BREAKPOINT/SINGLE_STEP; default off. qbuild all once: 7839 Rust passed, clippy 0, CTest 115/115; no owned Rust files or C ABI changes.
Canonical request preserved unchanged apart from label/DLL and appended reach keys: Konradshohe, turbo, requested end 2032-01-01, cap32/month, 73 windows. Both runs warn about 5 unknown reach keys despite confirmed installation/cap/window/output.
(1)/(4) Off/on clean: exceptions, stalls, rtc, overruns, ref_warnings, panics, structured exceptions and INCOMPLETE=0; no crash/hang in either house run. Actual verdict dates off/on=2032-01-08/2032-01-06.
Month-end exact off/on=5659/5731 each. All 72 discrepancies per run are occupant queued-message ordering: mismatch count equals unmatched messages; records, queries and returns exact. Whole-city equality waived by coordinator.
Load-only exception on: four overlay replay field-0 flags original=0x91/core=0xd1 (bit 6 self-lighting), plus their cumulative summary; off load exact, monthly overlays exact. Cause unresolved; late-load fixtures run after arming, contaminating window 1.
(2) Early real-game control BEFORE full roster: PlaceZone=260/260 (256 monthly+4 scripted; initial load's 64 excluded), five windows; CanCreateLot observed=17712; traps=35944=2*(260+17712), aborted/exhaustions=0. Native DLL A=29/29 B=44/44; translated WoW64 breakpoint/step=7/7 each with cap and window re-arm.
Counts are capped lower bounds: native same-entry stress observed 2892/10000; concurrency losses accepted by coordinator. Monthly constructed probes paused process-wide 72 times. House PlaceZone=0 (fixtures excluded); CanCreateLot=2304 across 72 windows, capped.
(3) Loaded seconds off/on=1128.250/1658.969, overhead=1.4704x (<2x); harness wall=1145/1676s, 1.4638x. Traps=18551190, 11182.4 per loaded second.
Roster=35648; armed=35274; skipped=374: exception/loader 43, jump/hook 258, unsafe prefix 69, changed entry 3, unreadable/non-executable 1. Every address/reason in log/TSV; late conflicts, ignored, aborted and slot exhaustion=0.
docs/abi/reach.tsv is canonical-house, with build/DLL/date/month provenance; reach-pilot.tsv preserves pilot-on-2 exactly. ABI metadata changed mid-run: initial ad03d35ff/19373cfa recovered; all 35648 address/size pairs identical to final 1b90b551 table.
| Observed canonical metric | Functions | Known bytes |
|---|---:|---:|
| Reached | 7883 | 1580524 |
| At least 20 capped entries | 6410 | 1288351 |
| At least 3 windows hit | 6426 | 1270541 |
| Both thresholds | 6148 | 1221078 |
New functions FIRST reached per window, Jan..Dec by year (partial first/last; includes late-load/exit activity); totals sum to 7883:
2026: 6026,1008,75,114,35,62,11,19,3,4,16,4 = 7377.
2027: 7,10,3,7,0,1,3,1,1,2,0,0 = 35.
2028: 0,2,1,3,0,7,3,0,0,0,0,1 = 17.
2029: 0,0,4,4,1,0,1,0,0,0,0,0 = 10.
2030: 0,5,0,0,0,379,1,0,0,0,0,0 = 385.
2031: 0,0,2,0,0,0,0,0,2,0,2,0 = 6; terminal 2032-Jan: 53.
Evidence: build-runs/game-results/reach27-r2-{calibration,house-off,house-on}-1.{log,verdict,reach.tsv}; build-runs/reach-r2-{report,input-check,import}.py; engine/build-x86/Testing/Temporary/LastTest.log. Zero-sized ABI rows: 2 reached, counted as 0 known bytes.

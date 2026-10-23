| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netmgr-exact5: network-lot record 11 shadow | Codex, `.claude/worktrees/glue-netmgr-exact5` | in progress | Diagnose record 11 mismatch and validate to 2032. |

## Handoff
- Goal: fix netmgr record 11 mismatch and investigate 19 resynced lines; real-city shadow exact to 2032.
- Done: initial handoff commit; diagnosed missing (20,103) as absent north-column walk (§6.3 NM10/NM11).
- Implemented origin walk, SGI pending order (§8.5 NM13), captured RNG input (§17.3), exact config comparison; removed NM10 suppression and record resyncs.
- Focused tests: first 39/40 (expected-order assertion corrected), second 40/40; new north-column/RNG reload test awaits gate.
- Evidence copied locally: `build-runs/regress-train149-shadows.log` and `.out`; original had 72/73, 19 resyncs, 10 un-compared config choices.
- Implementation milestone: `7812f988`; source unchanged while first native queue job is pending.
- Full `qbuild all` started on that source: Rust stopped at 740 passed/1 failed (conformance nm16_23 expected old event order), clippy failed `int_plus_one` in ordered.rs. Failed logs preserved as `build-runs/qbuild-{test,clippy}-failed-r1.log`.
- OUTSTANDING: qbuild all session `70839`, native queue label `qbuild-20261023-195549-build-ctest`; collect that SAME session to terminal before another native request. No game request submitted.
- Next after terminal: change pending prime comparison `n >= count+1` to `n > count`; reverse nm16_23 texture expectations (0x62 then 0x60); format assertion at tests.rs:1397; new north-column test uses out-of-city coordinates (CITY=64x64), change (20,103) to (20,31) and triggering rect (21,120,22,122) to (21,40,22,42).
- Then check owned formatting; run corrected workspace tests/clippy and `qbuild verify`; submit unique game label `netmgr-exact5-r1-2032` (draft `build-runs/netmgr-game-request.draft`).
- Game wait helper: `python build-runs/wait-netmgr-result.py build-runs/game-results/<label>.verdict`, collect same operation; no qwait. Summary helper `build-runs/summarize-netmgr.py <label>` prints exact clean/total and resync count.
- Constraints: no tools/ access; no native signature guesses; game requests unique, wait with same exec session (no qwait).
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-glue-netmgr-exact5-r1-manifest.json`.

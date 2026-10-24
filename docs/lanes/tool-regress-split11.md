| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| tool-regress-split11 (parallel house shadows) | `tool-regress-split11` / tooling lane | in progress | Resolve six tooling failures, rebalance measured costs, prove same-DLL parity and speed. |

## Handoff

- Goal: all tooling tests pass or prove failures on main; split >=35% faster; identical shadow counts/mismatches to 2032 on one DLL.
- Done: prior implementation `3ffd15df6`, sorting fix `c46aa442c`, prior report `a7e057034`; details retained below.
- Next: restore combined default until explicit --split; reproduce main failures; isolated cost probes, then rebalance and repeat split/combined.
- Prior gate qbuild PASS, ctest 81/81; regress tests 17/17; broader suite 141/147.
- Prior runs 611 FAIL bad grouping; 612 PASS 664 s 51/52 exact; 613 combined stalled/truncated FAIL 455 s.
- Snapshot build-runs/split11-main.dll SHA256 0a67d38e6a8c1fbf969f7e483132fa17ead29076f3a0d248d8ddff2037ede82c.
- No ABI changes, questions, coverage-map edits or newly enabled shadow keys.
- R2 milestone: combined is merge-train default; --split explicitly enables 3 groups with fourth-batch combined cadence. CLI flags mutually exclusive.
- R2 tests: python build-runs/check-main-tooling.py printed main WorkflowDryRun 7 tests, six failures; lane all tooling 147 tests, same six failures (141 pass). Logs main-workflow-tests.out and tooling-tests-r2.out prove pre-existing failures against main's source.
- R2 timing probe is waiting for train199 to clear, then snapshots coordinator integration DLL to split11-r2-main.dll and submits four independent families to 2026-07-01. qwait session 41038 waits for split11-r2-costs.json.
- R2 probe finished: run 614, all four clean; DLL SHA256 4853374130ffde73a29adf15e197802a7b4f79d307141fdb23983bc4eb3bdfd1. Traffic projects ~384 s. Resources have a 152 s shutdown tail, project ~367 s total; do not normalize their overshot final date as simulation cost.
- Updated LPT weights give traffic and resources separate groups; all other house shadows should share the third group. Regression grouping/cadence tests passed 2/2 before final resource-tail correction; rerun with full suite below.
- Next exact step: python build-runs/validate-split.py (combined then split on immutable R2 snapshot); output split11-r2-measurements.json. Full games not started yet. Then tooling suite and qbuild verify, record parity/wall results.
- R2 runs 615/616 both FAIL with watchdog stalls: combined truncated 2027-02-15 (203 s reported wall); split groups truncated 2028-11-02 / 2032-05-11 / 2030-02-17 (276 s reported wall). Python invocation times differ: 210.00 / 496.77 s; never substitute one timing source for the other.
- Split groups estimated 383.5/367.4/353.2 s. 51/52 partial labels exact; propmgr 150/151 mismatches=1, flammability 36/36. Combined 52/52 partial labels exact, flammability 9/9. No full-horizon parity or speed proof.
- Next: run python build-runs/retry-combined.py then the same with --split, once each; they preserve unique *-retry evidence and wait for coordinator regress before submitting. If stalls repeat report them, do not weaken watchdogs. Then final tooling suite/native gate and hold queue WIP unless acceptance is met.

## Report

Partial closure at the round's two-thirds context limit. Scheduling and summary infrastructure is implemented
and measured, but the requested under-seven-minute run and exact split/combined parity are **not achieved**.
Do not treat this lane as performance or coverage sign-off.

`tools/regress.ps1 -ShadowSplit N` accepts 1..5 (1 preserves combined behavior; larger values require `-Queue`).
It partitions the existing single house profile plus candidate extras, submits every group and replacement
before waiting, retains group logs, and concatenates a single shadows log under named headers. The ordinary
summary sums comparison lines and reset counts across groups; its verdict is the worst group verdict, with
missing or blocked groups failing. Profile output names `shadows split N` or `shadows combined`.

Longest-first weights are timestamp-gap proxies from train196 (residential 198.290 s, development 66.596 s,
demand 59.856 s, etc.), not independent CPU measurements. Configuration helpers remain with their owner.
State-sharing families remain in one process: occupant/property/city-detail/lot projections; automata/owners;
resource cache/weather readers; overlays/simgrid readers. Corrected assignments have estimated weights
198.3/149.2/149.1 s, but actual completion offsets were 85/174/660 s. **These estimates need replacement**
with isolated measured costs, especially traffic/demand/pollution/resources in group 3.

`merge-train.py regress` now passes 3 by default, 1 on batch numbers divisible by four, and 1 with `--combined`.
The train state records `shadow_split`, `shadow_profile`, and the run record path;
`build-runs/regress-<tag>.json` records the profile and verdict. `run --combined` also works. Ops instructions
are in `docs/ops/running-the-game.md`. No engine source, ABI, default shadow key, or coverage map changed.

## Validation

- `python <MAIN>/.claude/lane-bin/qbuild.py --wt <WT> verify` printed `qbuild: PASS`:
  build 262/262 edges (123.6 s), ctest 81/81 (225.4 s), total 353.2 s. Native source remained unchanged after it.
- `python -m unittest discover -s tools/tests -p "test_regress*.py"` printed `Ran 17 tests` / `OK` on the final
  source. Includes actual PowerShell 5.1 parsing, no lost/duplicated keys, helper/state-family preservation,
  nonempty balanced proxy groups, summed real run_summary output, missing-group failure, and fourth-train cadence.
- Broader `python -m unittest discover -s tools/tests`: `Ran 147 tests`, `FAILED (failures=6)`; 141 passed.
  All six failures are untouched `test_laneguard.WorkflowDryRun` gate-ERROR assertions. See
  `build-runs/tooling-tests.out`; the quiet Bash wrapper also failed with CreateFileMapping Win32 error 5.
- Actual `coverage.read_regress` parsed all 52 labels without modification. `newest_regress` correctly selected
  reachable main's `regress-train196.out` rather than claiming unmerged lane evidence. The existing delta reader
  parsed the aggregate initial output as a baseline in the combined attempt; no reader changes were needed.

All games used a single immutable DLL snapshot built from main's engine at initial base `ffb8f38c2`, SHA-256
`0a67d38e6a8c1fbf969f7e483132fa17ead29076f3a0d248d8ddff2037ede82c`.
Runs were submitted only after coordinator regressions cleared: native verification waited behind train197,
and the explicit combined attempt waited behind train198. All foreground operations have completed.

- [Run 611](../runs/split11-initial-611.md), `split11-s3`: **FAIL**, 868 s. Real execution exposed a PowerShell
  hashtable property-sort bug that placed all switches in group 3; groups 1/2 stalled on shutdown. Fixed in
  `c46aa442c` with explicit expressions and a balance assertion. Failed evidence remains separate. Group 3
  did run the full house profile clean to 2032, but the parent regression failed and is not a combined sign-off.
- [Run 612](../runs/split11-fixed-612.md), `split11-s3-fixed`: **PASS**, 664 s, all three groups clean and zero
  harness problems. **51/52 labels exact**: `traffic 72/72`; `flammability 53/54`, mismatch total 1. Every other
  mismatch total is zero. The run record contains every exact `<label> <clean>/<total>` line. Target <420 s missed.
- [Run 613](../runs/split11-combined-613.md), `split11-c1`: **FAIL/incomplete**, 455 s, stalled at **2029-02-08**,
  watchdog_stalls=1, foreign_exceptions=0, ref_warnings=0. `traffic 36/38`, mismatch total 70. Every measured
  count is recorded, but these truncated counts cannot establish 2032 parity or a comparable wall time.

## Remaining work

No missing spec contract or native signature question was encountered; questions: **none**. No new shadow was
enabled or credited. Strict parity remains unproved, and the split's flammability mismatch remains unresolved.

When no coordinator regress is queued/running, run `python build-runs/probe-shadow-costs.py` for the prepared
short isolated traffic/demand/pollution/resources timing experiment; its first invocation refused submission
because train198 was active, so **no probe game ran**. Rebalance from those results, then repeat fresh split and
combined runs to 2032, using the corrected split as the delta baseline. Require every count and mismatch total
to agree and all shadow lines exact before declaring success. Evidence and helper scripts stay in this worktree's
`build-runs/`; `split11-analysis.out` compares the corrected split against the incomplete combined attempt.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| tool-regress-split11 (parallel house shadows) | `tool-regress-split11` / tooling lane | WIP, coordinator hold: qbuild PASS, ctest 81/81; tooling 141/147 with all six failures reproduced on main; run 617 full combined 52/52 exact but watchdogs; run 618 split 51/52 partial, repeated stalls; ABI unchanged | (1) Investigate watchdog stalls and propmgr query-list divergence; (2) repeat a complete same-DLL 2032 pair and prove >=35% end-to-end speed reduction before READY. |

## Report

Coordinator hold retained. Tooling test failures are proved pre-existing and the groups have measured
weights, but **strict 2032 parity and a valid >=35% wall-time reduction remain unproved**. Split retries
stalled before the endpoint. No READY or coverage sign-off is claimed.

This round committed the combined-default policy in `19f8664cd` and measured rebalance in `4ea92f600`.
`merge-train.py regress` and `run` now use combined unless explicitly passed `--split`; opt-in splitting
uses three groups except every fourth batch stays combined. `--combined` and `--split` are mutually
exclusive. Existing train-state/run-record profile fields remain intact. Ops instructions were updated.

[Timing probes, run 614](../runs/split11-costs-614.md), ran only after train199 cleared. Traffic took
27.781 s to July 2026; demand 3.625 s and pollution 4.937 s to July 4. Resources reached July 1 after
15.625 s but continued for a 151.734 s shutdown tail to September 2032. Their final days/s must not be
used as simulation cost. Projected group weights are **383.5 / 367.4 / 353.2 s**: traffic alone,
resource/weather readers together, all remaining house shadows together. Residential uses the earlier
measured ~65 s simulation interval; smaller unisolated families still use train196 proxies. These are
estimates, not successful full-run timings. Occupant/property/city-detail/lot readers, automata/owners,
resource/weather readers and overlays/simgrid remain with their state-sharing families and helpers.

`regress.ps1` still has exactly one house `shadows =` profile line, matching main's current house
switches when tested. Queue submission, concatenated group logs, summed shadow counts, worst-group
verdicts and the ordinary delta output remain as implemented in the preceding round.

### Final checks

- `python <MAIN>/tools/tests/test_laneguard.py WorkflowDryRun -v` ran main's actual source:
  **7 tests, six failures**. All six are the same `WorkflowDryRun` methods as on this lane:
  `test_parked_lane_does_not_start`, `test_refuses_to_restart_after_stop_mid_run`,
  `test_refuses_to_run_unguarded_when_the_agent_type_is_unknown`, `test_refuses_to_start_with_stop`,
  `test_restarts_on_continue`, `test_starts_without_stop`. Fixture gates return ERROR. Evidence:
  `build-runs/main-workflow-tests.out`; this establishes the six failures pre-exist on main.
- Final `python -m unittest discover -s tools/tests -v`: **147 tests in 170.697 s, six failures**,
  exactly those main failures; **141 passed**, including every regression test. Evidence:
  `build-runs/tooling-tests-final.out`. An intermediate default-policy run had three stale assertions
  expecting split=3 (nine failures total); those expectations were corrected before this final gate.
- `python <MAIN>/.claude/lane-bin/qbuild.py --wt <WT> verify`: **qbuild: PASS**; build **166/166**
  edges in 8.1 s (150 sccache hits, zero misses), **ctest 81/81** in 116.9 s, total 129.1 s.
  Request `qbuild-20261024-164921-build-ctest`; evidence `build-runs/qbuild-final.out` and
  `fc4verify-{build,ctest}.log`. One queued verify only; no direct MSVC configure/build invocation.
- `python build-runs/summarize-r2.py` exercised actual, unchanged `coverage.read_regress` on all four
  summaries: all **52 labels** parsed each time. `newest_regress` selected reachable main evidence
  `regress-train196.out`, not these unmerged runs. Delta comparisons ran without reader changes.

### Real games and timing limits

All R2 games used Konradshohe and one immutable coordinator-integration DLL snapshot:
`build-runs/split11-r2-main.dll`, SHA256
`4853374130ffde73a29adf15e197802a7b4f79d307141fdb23983bc4eb3bdfd1`.
Submissions waited for coordinator regress requests/running markers to clear, including train199 and
train200. The games used that DLL; the native gate above verified this lane's unchanged engine tree.

- [Run 615, combined](../runs/split11-r2-combined-615.md), `split11-r2-c1`: **FAIL**, shadow stalled
  2027-02-15 with one watchdog trip. **203 s** legacy wall / **210.00 s** full invocation.
  52/52 labels exact only over the truncated interval; `flammability 9/9`. Replacements reached 2032
  clean with zero watchdogs. This is not a full-horizon baseline.
- [Run 616, split](../runs/split11-r2-split-616.md), `split11-r2-s3`: **FAIL**, shadow group dates
  2028-11-02 / 2032-05-11 / 2030-02-17, watchdog trips 1/2/2. **276 s** legacy wall / **496.77 s**
  invocation. 51/52 labels exact over unequal intervals; `propmgr 150/151`, mismatch total 1;
  `flammability 36/36`. Replacements reached 2032 with one watchdog trip.
- [Run 617, combined retry](../runs/split11-r2-combined-retry-617.md), `split11-r2-c1-retry`:
  **parent FAIL**, native queue verdict **clean to 2032-01-02**, but watchdog_stalls=2.
  **52/52 labels exact**, `flammability 54/54`, `propmgr 220/220`, mismatch totals zero.
  Native simulation 815.922 s; queue wall 832 s; **846 s** legacy wall / **971.09 s** invocation.
  Replacements reached 2032 clean with zero watchdogs.
- [Run 618, split retry](../runs/split11-r2-split-retry-618.md), `split11-r2-s3-retry`: **FAIL**,
  every shadow group stalled again, each with one watchdog trip, at 2027-04-04 / 2028-09-05 /
  2027-11-08. Replacements also stalled at 2029-06-09 with one watchdog trip. **213 s** legacy wall /
  **603.28 s** invocation. 51/52 partial labels exact; `propmgr 68/70`, mismatch total 2;
  `flammability 16/16`. Neither a valid speed measurement nor full-horizon parity evidence.

The legacy `regress wall time` is sampled before summary/delta processing (`regress.ps1`'s `$wall`
assignment), so complete Python invocation times include additional processing. Do not mix the two
series, or claim a speed reduction from the truncated split retry. No successful under-seven-minute
split or valid >=35% reduction has been demonstrated. One retry of each profile was retained; stalls
repeated. All foreground operations have now completed; no additional game or verify is outstanding.

### Exact comparison evidence

Columns below are **combined retry run 617 -> split retry run 618** (clean/total); mismatch sums
follow. Combined reached the requested horizon; split did not. Therefore these unequal counts are
**not** a full-horizon interaction/parity test. Machine evidence: `build-runs/split11-r2-final.json`.

```text
advisors 73/73 -> 23/23; mismatches 0/0
aura 71/71 -> 21/21; mismatches 0/0
automata 72/72 -> 22/22; mismatches 0/0
automataowners 72/72 -> 22/22; mismatches 0/0
budget 72/72 -> 22/22; mismatches 0/0
citydetail 1/1 -> 1/1; mismatches 0/0
civic 72/72 -> 22/22; mismatches 0/0
clock 73/73 -> 23/23; mismatches 0/0
commercial 12/12 -> 18/18; mismatches 0/0
crime 71/71 -> 22/22; mismatches 0/0
dbpf 73/73 -> 27/27; mismatches 0/0
demand 2240/2240 -> 673/673; mismatches 0/0
development 913/913 -> 263/263; mismatches 0/0
feature 75/75 -> 23/23; mismatches 0/0
flammability 54/54 -> 16/16; mismatches 0/0
flora 72/72 -> 22/22; mismatches 0/0
history 79/79 -> 25/25; mismatches 0/0
landfill 72/72 -> 22/22; mismatches 0/0
landvalue 72/72 -> 22/22; mismatches 0/0
lotcfg 293/293 -> 93/93; mismatches 0/0
lotconf 73/73 -> 23/23; mismatches 0/0
lotdev 73/73 -> 23/23; mismatches 0/0
lotrec 73/73 -> 23/23; mismatches 0/0
lua 73/73 -> 23/23; mismatches 0/0
mysims 73/73 -> 23/23; mismatches 0/0
neighbours 72/72 -> 22/22; mismatches 0/0
netmgr 73/73 -> 23/23; mismatches 0/0
netocc 72/72 -> 22/22; mismatches 0/0
netrules 73/73 -> 22/22; mismatches 0/0
occmgr 72/72 -> 22/22; mismatches 0/0
ordinance 72/72 -> 22/22; mismatches 0/0
overlayrecords 292/292 -> 92/92; mismatches 0/0
overlayreplay 292/292 -> 92/92; mismatches 0/0
overlays 73/73 -> 23/23; mismatches 0/0
parks 72/72 -> 22/22; mismatches 0/0
plumbing 144/144 -> 45/45; mismatches 0/0
pollution 72/72 -> 22/22; mismatches 0/0
power 381/381 -> 150/150; mismatches 0/0
propmgr 220/220 -> 68/70; mismatches 0/2
protection 288/288 -> 88/88; mismatches 0/0
residential 1137/1137 -> 343/343; mismatches 0/0
scres 149/149 -> 69/69; mismatches 0/0
simgrid 73/73 -> 23/23; mismatches 0/0
strings 72/72 -> 22/22; mismatches 0/0
terrain 592/592 -> 189/189; mismatches 0/0
tract 145/145 -> 45/45; mismatches 0/0
traffic 72/72 -> 15/15; mismatches 0/0
tutorial 146/146 -> 46/46; mismatches 0/0
variant 1/1 -> 1/1; mismatches 0/0
weather 73/73 -> 33/33; mismatches 0/0
zonedev 365/365 -> 115/115; mismatches 0/0
zones 292/292 -> 92/92; mismatches 0/0
```

The old run 612 `flammability 53/54` difference was a hotspot-count disagreement (original 6, mirror 7)
at 2031-05-04; its grids and getters matched. Current-DLL combined run 617 is `flammability 54/54`,
but current split attempts did not reach that date, so the old discrepancy is **not explained away**.
Run 618's propmgr differences are query-list mismatches on 2026-03-01 and 2026-12-01. Native/interaction
investigation is still needed; no native signatures or missing spec contracts were guessed.

Questions: **none**. Engine source, C ABI exports/version, default shadow keys, coverage map and
coverage-shadows table were not changed. Keep combined as the default and this queue entry **WIP**
until a complete same-DLL pair establishes every shadow count/mismatch total and valid speed.

## Previous round report (superseded by the R2 report above)

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

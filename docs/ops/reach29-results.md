# reach29 measurement and unresolved drivers

State: **WIP**. The +3-point target is not met. No simulation algorithms or ABI
exports changed. `reach_lifecycle` is default-off and is qualified only on the
empty Tempelhof scenario; do not enable it for the mature-city rows.

## Fixed census and byte results

The completed suite-health28 `sh28-census3` was frozen as
`build-runs/suite/reach29-before`: 179 scenario/profile runs. Attribution is
the same local coverage map, names, interfaces and executable for both measures.
`reach29-complete` adds nine measured runs from `reach29-r2`,
`reach29-control` and `reach29-views-qualified`; failed rows remain in the union.
The earlier full-roster `reach29-r1` and rejected-snapshot
`reach29-views-final` remain separate exploratory artifacts.

| Metric | Before | Final union |
|---|---:|---:|
| Game denominator | 5,584,344 | 5,584,344 |
| Suite-exercised bytes | 1,439,504 | 1,581,894 |
| Suite-verified bytes | 409,033 | 440,779 |
| Suite-verified percentage | 7.324638% | 7.893120% |
| Net verified change | | +31,746 bytes / **+0.568482 points** |

The 82 baseline-exact owners had 278,088 unexecuted implemented bytes.
New runs entered 133,820 of those bytes; 117,757 retain exact keys. New findings
remove 86,011 previously credited bytes under the conservative whole-key rule.
[Per-owner gaps, new bytes and verdicts](reach29-owner-results.tsv) include all
82 owners. `build-runs/reach29-final-delta/functions.tsv` lists each function;
`mismatches.tsv` records each first mismatch with its scenario and log line.
Both the helper and `python tools/coverage.py --suite reach29-complete`
printed **440779 bytes (7.9%)**; the baseline coverage command printed
**409033 bytes (7.3%)**.

## Drivers and qualification

The trigger map's load/save paths and RE queries for ResidentialSimulator Init
(`006feadd`, 2620 bytes), Read (`007009a0`, 1201) and the native city lifecycle
identified a measurement aperture: old tracing omitted initialization/teardown.
The new option starts after PostAppInit, suspends observer fixtures, checkpoints
after native city shutdown and stops before plugin hook restoration.
The largest observed exact gains include AutomataScriptSystem 22,778 bytes,
ResidentialSimulator 9,627, BudgetSimulator 6,959 and AdvisorSystem 6,416.

Four rows are registered in set `reach29`. Ordinary reach controls were clean
for all four cities; house/Tegel accepted 5/5 actions and growth 19/19.
The final view row accepted 18/18 actions, reached 2197-04-20, and was clean.
Lua TakeSnapshot returned status 2 in exploratory runs and was removed;
dated native lighting actions supplement the calibrated zoom/scroll commands.
All runs used the shared queue, with at most four requests in flight.

Lifecycle tracing corrupted three mature cities with both the complete roster
and the reduced 3,146-function exact-owner roster. Lifecycle-off controls were
clean with the same DLL. This is an **instrumentation blocker**, not an assigned
engine defect. No positive counters were credited from those crashed runs.
The empty lifecycle run was clean, end-reaching, 19/19 actions, 1,667 reached
functions with the reduced roster (10,747 with the complete roster before the
agricultural footprint correction). Its real initialization/teardown supplies
the bulk of the added coverage; it does not broaden shadow comparison scope.

## Engine findings (also present with lifecycle off; not fixed)

First evidence in `reach29-r2/reach29-views.log`: lotdev line 3806,
48 mismatches, first `s29@2,69` 4.3152113 / 4.31521177; pollution line 3152,
`month.recycled` 5156.035380870104 / 5155.97802734375; regionalcity line 3611,
14 mismatches, first `balance[3]` 35560.166666666664 / 35560.16796875.
Control `reach29-control/reach29-views.log` reproduces these at lines
4161, 3497 and 3967. Its neighbours line 4427 has four mismatches: regional
water exported 65988/0 and garbage imported 1000/0; city garbage exported
55420/54420 and water imported 123912/57924. Full lines are in the delta TSV.

## Validation

`python -m pytest tools/tests -q`: final **498 passed in 491.40s**.
Initial 496/497 run hit an unrelated 20-second junction-test timeout; that test
passed alone in 0.84s, then all 497 passed, before adding the roster test.
`qbuild.py --wt <WT> all` ran once: **7882 Rust tests passed**, clippy
**0 diagnostics**, CTest **124/124**, overall **PASS**. Its tree-change note
covers scenario/document qualification during the run; harness C++ and Rust
did not change after the initial native build used by the game requests.
Next: repair mature-city lifecycle instrumentation, exercise remaining
formats/overlays and player drivers, then repeat the fixed-census measurement.

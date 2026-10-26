# reach29 measurement and lifecycle qualification

State: **harness qualified; coverage target NOT met**. No simulation algorithms
or ABI exports changed. `reach_lifecycle` is default-off; the reduced exact-owner
roster is now qualified on all four scenarios. The complete roster remains
unqualified.

## Continuation: corrected-source measurement

`reach29-repaired` used DLL SHA-256
`b84f195978e72d0dd1642e9cde21b31f30b95e0dffa0437a37c78caa3e1c07a7`.
All four runs were **clean, end-reaching, 47/47 actions accepted**; shadows were
**1236/1328 exact**. Reached functions were house **1889**, Tegel **1834**,
growth **1672**, views **1816**. Each trace records the excluded operand;
all have zero aborted steps and thread-slot exhaustions. No compare or replace
profile was run. Runs changed isolated slot copies only.

The fixed before/after census is the original 179-run `reach29-before` plus
these four rebuilt-DLL runs, tagged `reach29-repaired-union` (183 runs).
Earlier corrupt runs and the old-DLL safe-roster control are kept separately.
Both sides use the same attribution and 5,584,344-byte denominator.

| Metric | Before | Corrected union |
|---|---:|---:|
| Suite-verified bytes | 409,033 | 376,658 |
| Suite-verified percentage | 7.324638% | 6.744893% |
| Net verified change | | **-32,375 bytes / -0.579746 points** |
| Newly executed under baseline-exact keys | | 140,626 bytes |
| New bytes whose keys remain exact | | 109,276 bytes |
| Previously verified bytes removed by new findings | | 141,651 bytes |

The **+3-point target is not met**. Increased execution exposed mismatches,
which the conservative whole-key rule correctly removes from suite credit.
[All 82 owner results](reach29-repaired-owner-results.tsv) include new byte
counts and first mismatch references. References are scenario and raw log
line in `build-runs/suite/reach29-repaired/`; full lines are in
`build-runs/reach29-repaired-delta/mismatches.tsv`.

New findings in West Weburg: residential **4218**, 91/71680 cells differ
(first EQ 191.47592163085938 / 191.47593688964844); zone development **3795**,
12 list mismatches, 377/420 lots. Reproduced findings: pollution **3154**,
regionalcity **3596**, lotdev **3792**, neighbours **4146**. These are findings,
not simulation fixes. Only 112,605 unexecuted bytes remain in the original
baseline-exact owner groups whose keys are still exact; exercising all would
bound this cohort's credit at 489,263 bytes. Broader exact keys or coordinator
engine findings work is needed before this cohort can meet the original target.

The tracer fix excludes bogus entry **00707865**, an operand of the native
`mov eax,[00b43d94]` at **00707864**. Patching it changed the global pointer
loaded by tract initialization. Four-way splits isolated it; the relocated
regression prints **11/11 exclusions, 1/1 valid entry, 11/11 header**.
The CLI also now preserves an explicit `reach_contracts` when `--reach` is used.

Validation: `python -m pytest tools/tests -q` printed **499 passed in 633.69s**;
the focused suite parser run printed **21 passed in 0.61s**. Required final
`qbuild.py --wt <WT> all` ran once: Rust **7882 passed**, clippy **0 diagnostics**,
CTest **124/124**, overall **PASS**. Round two used **28 requests, at most 4 in
flight**: 20 diagnostic splits, 4 old-DLL controls, 4 corrected-source runs.
The old-DLL empty-city control stalled in a dialog after reaching its end date;
the corrected-source growth replay is clean. No save or third-party DLL is
committed. The first measurement helper completed the gap CLI, then was
interrupted during its optional comparison; final coverage commands pin the
original `--regress C:/Users/tlori/FreeCity4/build-runs/regress-train264.out`.

## Prior-round evidence (superseded by the corrected-source union above)

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

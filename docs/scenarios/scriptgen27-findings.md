# Generated scenario discovery: scriptgen27

The fixed `gen-tegel-s0-default` scenario is registered under `sets=gen` in
[`suite.tsv`](../../engine/tests/scenarios/suite.tsv). It preserves all 79 actions
from `gen27-tail-tegel-s0-default`, seed 0, medium, default mix, one month starting
2029-08-21 in FC4 Test/Tegel. End 2029-09-21 was reached: verdict **clean**,
53 actions accepted and 26 refused; **256 of 267 shadow lines exact**. This
one-month scheduling smoke is separate from the two-year discovery sweep.
DLL SHA256: `624dd491bf975ab88b30b87a2e0abf1f1d13dfb05d2f9b61d6b51f44bd41fb46`.

## Every non-exact Tegel line

Line numbers refer to the retained [native log](../../build-runs/suite/gen27-tail/worker1/build-runs/game-results/gen27-tail-tegel-s0-default.log).
All eleven lines are listed, including repeat samples and refusals. A delta is
`ours - original`. Byte counts describe opaque serialized fields; the log does
not identify a numeric type for those bytes. Refusals have no measured field
delta and must not be described as eight or two differing payloads.

The action window begins after the previous sample of that shadow key, or at
the beginning of the run if it has no previous sample. These windows provide
reproduction context, not proof that an action caused the difference.
Script line numbers below refer to the [fixed script](../../engine/tests/scenarios/scripts/gen-tegel-s0-default.script).

- **A, script L5-43:** August 21 markers/speed 2/resume/turbo on, then August 23
  terrain rectangle `(57,61)-(89,93)` height 270 kind 1; 16 road/street/pipe/
  power-line/rail/avenue operations around it; all 16 zone types at adjacent
  rectangles; airport `(11,92)` orientation 3 size 0. There are 39 completed
  action attempts. The script retains every coordinate and argument.
- **B, script L44-59:** September 2 rezone types 10 then 2 at `(0,126)-(3,127)`,
  dezone there, road `(0,126)-(11,126)`, street `(3,126)-(3,127)`, terrain there
  height 270 kind 0, disaster survey, save, speed 0, pause, resume, speed 2,
  turbo off/on, clockmode 0, lightinghour 4. There are 16 completed attempts.
- **None:** no completed script actions since that key's preceding sample.

| Key | Native date (month) | Log line | Fields and measured difference | Actions in preceding window |
|---|---|---:|---|---|
| plumbing | 2029-08-25 (August) | 3322 | `pipes` 33/0, delta -33; mismatches=1. Grid itself has 0 differing cells despite `exact=1`. | A |
| landvalue | 2029-08-26 (August) | 3429 | Intrinsic 179/4096 cells differ, max absolute delta 46; total 161/4096, max 46; slope 25/256, max 31.046839475631714. `meanAlt` 284.31512/282.47906, delta -1.83606. Total mismatches=366. `settled=0`, pass-end input sample. | A |
| traffic | 2029-08-30 (August) | 3457 | Working statistics: 36/544 serialized bytes differ, first +382, original `AE47813F00000000`, ours `0000000000000000`; mismatches=1. | A |
| netmgr | 2029-09-01 (September) | 3670 | `orig-only=14`, mismatches=14: original destroyed network occupants that ours kept after terrain. Log lists first 12 occupant/cell examples. Message diagnostics: original changes 174/37 versus ours 0/0 for 0x89D3E9F3/0xE8BDA2E7 (deltas -174/-37). Followed record and 708 lot configurations agree. | A; prior netmgr sample L2611 |
| occmgr | 2029-09-01 (September) | 3692 | `msgorder=0/5475`, mismatches=5475; queued inserts/removes differ in order/identity. `pending orig/core=0/5+0/0`; `destroy=4282/0/742` as logged (axes not named). Occupant record, queries and returns agree. | A |
| dbpf | 2029-08-20 (August record; live September) | 3895 | 8 refusals, mismatches=8; 27,250 records/raw/payload comparisons reported. No differing field value or payload bytes logged. | A+B |
| dbpf_bounds | 2029-08-20 (August record; live September) | 3896 | 109,000/109,000 bounds checks clean, mismatches=8; no differing field values logged. | A+B |
| dbpf | 2029-09-01 (September record; live September) | 3907 | 2 refusals, mismatches=2; 0 records compared in rotation slice. No differing field values logged. | None; prior sample L3895 |
| dbpf_bounds | 2029-09-01 (September record; live September) | 3908 | 0/0 bounds checks clean, mismatches=2; no differing field values logged. | None; prior sample L3896 |
| traffic | month to 2029-09-03 (September) | 3914 | 5/1206 record fields mismatched: published statistics x3, working statistics x2; 2 resyncs. Earlier 36/544-byte differences recur; September 3 published record has 4/544 bytes differing, first +532, original `803F0000803F0000`, ours `C8420000C8420000`. Trip lists, holder properties and tract grids have zero mismatches. | B; prior sample L3457 |
| traffic | 2029-09-04 (September) | 3918 | Published statistics between deliveries: 4/544 bytes differ, same +532 bytes as L3914; mismatches=1. | None; prior sample L3914 |

Raw [nonexact.tsv](../../build-runs/suite/gen27-tail/nonexact.tsv) retains every
line, exact script windows and action results. [monthly.tsv](../../build-runs/suite/gen27-tail/monthly.tsv)
and [first_seen.tsv](../../build-runs/suite/gen27-tail/first_seen.tsv) record counts
and first observed keys. The fixed script includes the later September 12 edge
actions and terminal disasters too; they occur after the eleven lines above.

## Reading discovery sweep evidence

`tools/scenario_runs.py summarize --tag gen27-r3` writes the agreed summary,
per-key union, per-action counts and first mismatches, plus `monthly.tsv`,
`first_seen.tsv` and `nonexact.tsv`. Monthly rows include line/exact counts,
new keys within each scenario, new keys in manifest order across the suite,
and first mismatching keys. New keys are reach observations, not coverage credit.
The activity month follows actual game execution, while `sample_date` preserves
the shadow's own date; stale DBPF dates and the planner's -4713 sentinel must
not move a sample into the wrong execution month. Reach functions are `-` when
the log lacks a reach summary. No reach ini key was present on merged main at
submission, and the pinned DLL does not supply that measurement.

The 48-request round-3 sweep is tier-2 discovery: seeds 0/1/2, four mixes,
Konradshohe/Tegel/Spandau/New City, 24 months. Requests use default (`normal`)
priority and `close_dialogs = 1`; the worker also scans the main coordinator
queue so waiting regress requests take precedence. The suite can use monthly
first-seen keys and mismatch dates to choose shorter regression windows. This
does not assert that a month without new keys has no new input behavior.

`python tools/scenario_discovery.py --tag gen27-r3 --freeze` stages scripts for
completed scenarios with non-exact lines under the ignored `freeze/` directory
and proposes agreed-format rows in `suite-additions.tsv`. It leaves the tracked
suite untouched. Pending jobs are excluded; script text is preserved rather
than regenerated with possibly changed generator code.

## Round-3 measured results

The following Konradshohe runs are terminal: eight reached the two-year date
(six clean, two corrupt), four stalled earlier. All twelve found non-exact
shadows and are fixed `gen` suite scenarios. Other city results are still being
collected. Labels are `gen27-r3-<city>-s<seed>-<mix>`; all use `profile=shadows`.
Action refusals include deliberate edge/error-path attempts.

| City | Seed | Mix | Verdict | End reached | Actions ok/failed | Shadows exact/lines |
|---|---:|---|---|---|---:|---:|
| Konradshohe | 0 | default | corrupt | yes | 444/500 | 4210/4340 |
| Konradshohe | 0 | grow | corrupt | yes | 115/101 | 3973/3999 |
| Konradshohe | 0 | churn | clean | yes | 398/47 | 4162/4255 |
| Konradshohe | 0 | edge | clean | yes | 286/103 | 4003/4029 |
| Konradshohe | 1 | default | clean | yes | 462/482 | 4176/4303 |
| Konradshohe | 1 | grow | clean | yes | 107/109 | 3900/3947 |
| Konradshohe | 1 | churn | clean | yes | 407/38 | 4183/4269 |
| Konradshohe | 1 | edge | clean | yes | 246/143 | 4021/4048 |
| Konradshohe | 2 | default | stalled | no | 90/89 | 763/788 |
| Konradshohe | 2 | grow | stalled | no | 21/14 | 344/351 |
| Konradshohe | 2 | churn | stalled | no | 19/2 | 235/239 |
| Konradshohe | 2 | edge | stalled | no | 37/14 | 501/507 |

The [summary](../../build-runs/suite/gen27-r3/summary.tsv) retains verdict, end
and throughput for every requested scenario. [actions.tsv](../../build-runs/suite/gen27-r3/actions.tsv)
gives run/ok/failed by action. [findings.tsv](../../build-runs/suite/gen27-r3/findings.tsv)
retains each first mismatching key, actual exceptions and watchdog stalls with
seed, native line and preceding script action; [nonexact.tsv](../../build-runs/suite/gen27-r3/nonexact.tsv)
retains all non-exact lines and action windows. These findings do not identify
engine fixes or establish novelty against an idle baseline.

Tools validation on final tooling source: `python -m pytest tools/tests -q`
printed **378 passed in 267.34s** ([log](../../build-runs/suite/gen27-r3/tools-tests-final.log)).
Native parser validation printed `scripts=300 actions=100800 parse_errors=0`;
five generator tests passed ([log](../../build-runs/suite/gen27-r3/native-parser.log)).
Real input metadata/hashes cover all four FC4 Test cities, and the validation
helper printed `Documents source hashes unchanged: 4/4`.

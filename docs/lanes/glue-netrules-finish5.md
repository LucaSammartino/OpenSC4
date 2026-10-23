| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netrules-finish5: finish network rules shadow measurement | Codex / `glue-netrules-finish5` (engine lane) | READY measured work: merged d056c1b9d; cargo 6696/0, clippy 0 diagnostics, native 223/223 edges, CTest 70/70; run345 clean to 2032-02-01/ref0, netrules 75/75 mismatch-clean, 74/75 explicit exact, 84 evaluations/24 tables; no new questions; two inherited C ABI exports | (1) Review initial pre-readiness status before all-report exactness/profile admission; netrules_shadow remains out of shadows profile. |

## Outcome

The network-rules shadow now has a fault-free real-city run through 2032-02-01.
The measured work is READY. All comparison counters are zero, but the plugin reports
74/75 explicit exact samples because its initial load status precedes rule readiness.
`netrules_shadow` stays default-off and is **not added to the shadows profile**.
No full every-report exactness or automatic verified/profile-admission credit is claimed.

## Changes and diagnosis

Merged `glue-netrules-shadow5` at `d056c1b9d`, including the NR16/NR17 record-only
observer, detached replay and table comparison from source `ef0469c1a`. This finish lane
changed game inputs and records only; no engine source changes followed that merge.
The merge adds C ABI exports `fc4_network_rules_snapshot` and `fc4_network_rules_replay`;
ABI version was not bumped. Intersections remain explicitly unobserved.

Preserved prior evidence in `build-runs/prior-netrules-a.log` and `prior-netrules-b.log`.
The prior budget-enabled run starts with funds=834 and reaches funds=-103615 on
2028-08-01 immediately before unexpected city shutdown. Its counters show no shadow
mismatch. Later controls confirm that the original central placement script destroys
the fixture's residential population and bankrupts it.

The accepted input uses `build-runs/netrules-finish-peripheral-actions.txt`: 12 initial
evaluator-bearing network types, five annual actions through 2031, then a January 2032
mark and digest. Runs are shortened to two cells at x120..121 and x124..125 inside the
logged 128x128 city map. The request uses the run270 provider profile minus netmgr,
netlot, nettool and demolition (38 actual providers), retaining the original network
manager, network lot configuration, network tools and demolition. Full switches are
recorded in run345. No native signature was inferred or changed.

## Retained controls

- Run342, `netrules-finish5-spandau-2032-20261024a`: clean/ref0 but timeout at
  2019-03-29 with repeated emergency pauses. 212/212 mismatch-clean, 101/212 explicit
  exact, 24 evaluations. It never reached the 2026 script; no 2032 credit.
- Run343, `netrules-finish5-konrad-2032-20261024b`: headline clean/ref0 to 2032,
  75/75 mismatch-clean, 74/75 explicit exact, 54 evaluations. **Not accepted**: full
  log has 14 caught placement SEH and one later foreign shutdown exception after the
  headline counters. Our own DLL/PDB resolves RVA DC1FF1/DC1551 to `RegionToC` /
  `cFC4Demolition::DemolishRegion`. Diagnostic helper reads only our DLL/PDB.
- Run344, `netrules-finish5-konrad-2032-20261024c`: leaving demolition native eliminates
  those faults (SEH0/foreign0/ref0), but central placements leave R population0 and
  funds=-101143; ended on 2029-03-01. 40/40 mismatch-clean, 39/40 explicit exact,
  366 evaluations. This led to the peripheral placement correction.

The first event watcher timed out at nine minutes; a second printed the valid Spandau
verdict before a cleanup error. These were helper failures, not replacement game
verdicts. The corrected event watcher handles Created/Changed/Renamed and unregisters
events on exit. All later waiters, including the accepted run, reached terminal exit0.

## Validation

One stable-source `qbuild.py --wt <WT> all` reached terminal PASS: cargo 6696 passed,
0 failed, 0 ignored; clippy 0 diagnostics; native build 223/223 edges; CTest 70/70.
That same operation routed native build+CTest through the coordinator queue as
`qbuild-20261024-002209-build-ctest` (339.1 seconds), without a duplicate verify.
Evidence: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log` and
`fc4verify-ctest.log`. Only game inputs and documentation changed afterward.

## Accepted 2032 measurement: run345

```
netrules-finish5-konrad-2032-20261024d 75/75
```

This is the mismatch-clean count; explicit plugin exactness is 74/75. The final
monthly line is:

```
netrules shadow (month) 2032-02-01: mismatches=0 table_mismatches=0 incomplete=0 unavailable=0 evaluations=84 new_evaluations=0 tables=24 epoch=1 ready=1 exact=1 intersections=unobserved
```

Verdict clean at 2032-02-01, 126.828 seconds; full-log foreign exceptions0, SEH0,
watchdog stalls0, reference warnings0, Rust panics0, rtc0, overruns0, dialogs_closed0,
resynced0. Only the two expected startup x87 warnings remain. All 19 actions executed.
Final funds=56221, population R/C/I=15309/2494/7620. All 73 monthly samples and shutdown
are exact=1. DLL SHA256:
`3c724b5ed15cac3d5a6d3bc0a8238b1a54efbb3e3f3bcaaaa526dfc4c61365f5`.

Run record: `docs/runs/netrules-finish5-konrad-2032-20261024d-345.md`.
Request, verdict and full log: `build-runs/game-queue/` and `build-runs/game-results/`.
Main `run:` lines were checked before assigning IDs; highest was341 and local
records342-345 are distinct. All control evidence remains separate from accepted run345.

## Limits and next step

The single non-exact sample is the initial load status: epoch0, ready0, tables0,
evaluations0, exact0. It is not an observed semantic mismatch and must not be counted
as a successful comparison. No numbered questions were filed: there are no semantic
differences or newly inferred native contracts. No turbo-only difference was observed,
so a normal-speed mismatch control was not required.

The coordinator should review pre-readiness reporting before admitting the key to the
shadows profile or claiming every-report exactness. Coverage tables and tools were
not edited. No build or game request remains outstanding. READY queue cites only
the accepted run345; failed/partial controls are retained in their run records.

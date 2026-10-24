| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-simgrid-lotrec5: SG3 simgrid and LR12 master lot record shadows | Codex / `glue-simgrid-lotrec5` (engine, detached HEAD) | in progress; round 2 integration | Resolve main merge, native gate, new 2032 run, refresh main, READY queue |

## Handoff

- Goal: integrate existing shadows with moving main; no new implementation scope.
- First required merge committed as 90fd92058 (main c11352c34); all conflicts resolved.
- Keep independent native WriteVariant capture from this lane; combine additive module/source/config lists.
- Stable qbuild all: cargo 6860/0, clippy 0 diagnostics. Native build/CTest pending, request qbuild-20261024-020014-build-ctest.
- Collect SAME running exec session 64760; no duplicate build. Logs: build-runs/qbuild-test.log and qbuild-clippy.log.
- Next: collect native terminal result; new unique label lotrec-autumn-integrated to 2032; run 359 collision grep found no matches.
- Before queue: merge main again and repeat native gate if anything merges.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-simgrid-lotrec5-r2-manifest.json.
- Detached HEAD intentional; never switch branches or update shared refs.

## Changes

Merged `glue-simgrid-lots-shadow5` (`e37c0c33c`) and `re-lotrec-lr12`
(`73c81a8b7`). Implemented LR12 in `f79a9c26f`, native variant capture in
`545141775`, and known-key registration in `f062d2f96`.

`lotrec_shadow=1` observes the current master-list lots at load and every month.
It reacquires the borrowed manager, walks the registered safe iterator, acquires
each Serializable view by QI, captures Write, then releases that view and
unregisters/C-frees the iterator. Inventory, capture, reader and comparison
failures remain explicit in the denominator. One observation-wide reference map
preserves aliases without recursing into units, designates or occupants.

The production core reads each complete record and re-encodes all 27 fields,
including configuration metadata, float bits, populations, properties and trips,
without Init. Native property variants are captured independently through their
published interface. No live simulation mutations or owner replacement occur.
Both `simgrid_shadow` and `lotrec_shadow` remain default off. No public ABI change.

## Validation

Five focused Rust conformance tests passed. Native T58-T65 accounting cases live
in `engine/conformance/tests/lot_record_observation_test.cpp`; CTest executed the
test successfully. Existing safe-iterator tests cover unregister/C-free.
Formatting checks passed for the three new Rust files.

Final stable-source `qbuild all`: **PASS**, cargo **6702 passed / 0 failed**,
clippy **0 diagnostics**, native build **86/86** (4.3 s), full CTest **70/70**
(82.3 s). Native request `qbuild-20261024-004339-build-ctest`, total 88.5 s.
Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`,
`fc4verify-ctest.log`. All asynchronous operations were collected to completion.

Earlier native request `qbuild-20261024-001350-build-ctest` passed 224/224 and
70/70, explicitly marked tree changed. First full stable gate
`qbuild-20261024-003021-build-ctest` passed cargo 6702/0, clippy 0 diagnostics,
86/86 and 70/70. The final repeat followed the measured unknown-key diagnostic.

## Real-city measurements

[r1](../runs/lotrec5-2032-r1-20261024.md): clean to 2032-01-01, ref_warnings=0,
simgrid 73/73 and lotrec 73/73. Six misleading unknown-key warnings were corrected
by registering the working keys. The original verdict/log remain under
`build-runs/game-results/lotrec5-2032-r1-20261024.*`.

[r2](../runs/lotrec5-2032-r2-20261024.md): Konradshohe, turbo to 2032-01-01,
**clean**, **ref_warnings=0**, all harness problem counts zero including other
WARNING lines; 48.031 s real, 61 s wall. DLL SHA256 prefix `969505f1`.
The exact clean/total tallies from this run's actual plugin log are:

```text
simgrid 73/73
lotrec 73/73
```

Every load/month line has `mismatches=0`. Final lines (2032-01-01):

```text
simgrid-shadow: phase=month julian=2463233 roles=86 expected=86 developers=12 exact=86/86 mismatches=0 differences=0 unavailable=0 cells=311552 bytes=423538 mode=record-only scope=SG3
lotrec-shadow: phase=month julian=2463233 expected=677 visited=677 captures=677 exact=677/677 mismatches=0 differences=0 errors=0 empty=0 bytes=247966 mode=record-only scope=master-lots excluded=off-list,units,occupants,unspecified-props
```

Evidence: `build-runs/game-results/lotrec5-2032-r2-20261024.verdict` and `.log`.
No mismatch or turbo-only difference; no normal-speed discriminator needed.
This verifies SG3's owner inventory and LR12's master-lot record scope, not all
lot-related classes, occupant payloads or simulation algorithms.

## Questions and landing

No new questions. LR12 is answered by the merged prerequisite. No outstanding
build or game request. No game files committed. Clean-room restrictions prevented
reading/editing the regression script or coverage tables: the coordinator should
credit both exact shadows and add `simgrid_shadow=1` / `lotrec_shadow=1` to the
regress shadows profile after review. The implementation is READY.

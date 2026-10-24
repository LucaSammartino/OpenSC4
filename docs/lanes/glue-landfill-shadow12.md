| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-landfill-shadow12: landfill owner shadows | Codex / glue-landfill-shadow12 | ready: measured developer and occupant work | Coordinator credit/profile review; independent zone verification remains parked LG31 |

## Changes

- `2c4b2946f`: implemented landfill §6.4 / LG33: ignore nested holder rejection when the stream remains good, preserving the cursor for the landfill tail. Added literal version-boundary and sticky-error tests.
- Extended the existing default-off `landfill_shadow` occupant probe from 9 to 11 cases, adding rejected plain-holder versions 1 and 3. All accepted records are written and reloaded into fresh native objects.
- Applied §7.3 / LG32's concrete void SetPosition contract through a corrected occupant-interface prefix; score position and bounds rather than residual EAX/AL.
- `8db352b74`: corrected the C ABI test's obsolete holder-rejection expectation and checked the misaligned tail's exact position words and bounds.
- C ABI exports unchanged. No native zone calls, private field access, or native captures used as Rust inputs.

## Validation

- Focused reader tests: 2 passed, 0 failed (`build-runs/focused-reader.log`). Explicit owned-file fmt: 2 files, no differences. `git diff --check` clean.
- Final `qbuild.py all`: **PASS**, workspace tests **7214 passed, 0 failed, 0 ignored**; clippy **0 diagnostics**; queued native build **46/46 edges PASS**, full CTest **81/81 PASS**. Native request `qbuild-20261024-145728-build-ctest`; verdict in `build-runs/build-results/`; final logs `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. No source changed during this gate.
- First full gate failed on the obsolete C ABI test (591 passed / 1 failed); clippy passed, native build 262/262 and CTest 81/81 passed. Its logs are preserved as `build-runs/gate1-*.log`; first verdict `qbuild-20261024-144548-build-ctest.verdict` notes the test-source correction during that run. Both asynchronous checks were collected to completion, without duplicate requests.

## Real city

Run record: [run 511](../runs/glue-landfill-shadow12-2032-20261024a-516.md). Konradshohe, turbo, original landfill developer, `enable=simulator`, `landfill_shadow=1`, requested end date 2032-01-01.

`glue-landfill-shadow12-2032-20261024a 73/73` — landfill developer monthly shadow lines, every mismatch count zero, resynced lines zero.

`landfill occupant probe summary exact=11/11 created=18 destroyed=18 reloads=7/7; public factory/interfaces; authored inputs; SetPosition void LG32; nested-holder rejection LG33; no zone calls`

AUTOTEST **clean**, date **2032-01-13**, real_seconds=26.000, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, **ref_warnings=0**. Evidence: `build-runs/game-results/glue-landfill-shadow12-2032-20261024a.verdict` and `.log`. No measured differences; no normal-speed diagnostic required. No game request outstanding.

## Questions and next steps

No new questions. Existing **LG31**, `docs/specs/questions/landfill.md`, explicitly parks independent native `cSC4LandfillZone` verification: no public acquisition/call/lifetime contract is published for the specified zone methods and record operations. This lane does not claim zone verification from the developer's serialized zones. Overall landfill verification therefore remains partial pending LG31; this ready entry contains the measurable developer/occupant work requested by the task's partial-closure rule.

Coordinator: review and credit these exact measured owners, retaining the zone limitation. This engine lane did not read or edit coverage tools/tables or the regress profile because the overriding clean-room rule forbids `tools/`; any profile/credit update belongs to the coordinator. The shadow stays default-off behind the existing ini key.

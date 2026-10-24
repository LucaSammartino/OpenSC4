| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-regionalcity-shadow13 (regional city shadow) | Codex / `glue-regionalcity-shadow13` | in progress; cargo 7408/0, clippy clean, CTest 81/81; turbo 61/73 exact, normal 63/73 exact, both clean/ref_warnings=0 through 2032 | File RY8 precision conflict, close balance discrepancy, record runs 632/633 and finish measured READY report. |

## Handoff
Goal: default-off regionalcity_shadow comparing live regional-city records at load/monthly through 2032.
Done: observer/bridge 93cfd16e3, SDK string fix f95ee4ee6, formatting f4edbbef1, enum fix 8cd3c9890, reviewed returns 7842bb59f. No public C ABI change.
Final all on 7842bb59f PASS: cargo 7408 passed/0 failed/0 ignored (51 summaries), clippy 0 diagnostics, native build 47/47 edges, CTest 81/81; queue qbuild-20261024-221326-build-ctest TERMINATED.
Owned Rust fmt PASS; focused tests 2/2 PASS. Logs build-runs/qbuild-{test,clippy}.log, fc4verify-{build,ctest}.log.
Game A TERMINATED: regionalcity13-r2-2032-20261024a 61/73; turbo; clean, ref_warnings=0; verdict date 2032-01-10, 30.110 real seconds, DLL sha prefix b6211054.
Game B TERMINATED: regionalcity13-r2-normal-20261024b 63/73; normal; clean, ref_warnings=0; verdict date 2032-01-01, 108.859 real seconds, same DLL.
Each run: 85 records decoded, 15555 fields/observation, 1135515 total fields, 73 observations (load + 72 months). Every record roundtrip and every other getter is exact.
ONLY differences: derived commodity balance[3] for serial 1407436211: 12 turbo observations, 10 normal observations, one difference each. Native has 24-bit rounding; core get_commodity_balance uses ordinary f64 operations.
Example A 2026-05-01 native=-11584.5830078125 core=-11584.583251953125; B 2026-08-01 native=-12247.5830078125 core=-12247.583251953125.
NEXT EXACT STEP: append RY8 to docs/specs/questions/regional-city.md (NOT FILED YET): reconcile regional-city §5's 'computed in double precision' with foundations §18.2–18.3 (main thread 24-bit; double operands enter unrounded; round EACH arithmetic operation). QUESTIONS.md lists RY8 next free; 7 open/0 answered, existing open entries read and none cover balance.
Likely correction after resolving literal spec conflict: crate::x87::r24(r24(produced+imported)-r24(demanded+exported)); helper already exists. Core getter engine/core/src/regional_city/mod.rs:257 currently plain f64. Do not silently force native FPU or use tolerances.
Meaningful precision test vectors: produced=16777217, imported=-16777216, demanded=exported=0 ->1 (no operand pre-round); produced=16777216, imported=1, demanded=16777216, exported=0 ->0 (round intermediate sum); observed garbage vector above tests final rounding.
Also fix measured instrumentation issue: load log date is -4713-11-24 (PublishedJulian is 0 at load); use current native simulator date at load as existing shadows do. Load comparison itself is exact.
No game requests or build operations outstanding. No question/run records/merge-queue entry written yet. Context guard last 149834/150000: budget stop; only handoff committed now.
Highest run number observed 631 (rechecked after both games); create unique records 632/633 if still free. Helper build-runs/regionalcity-result.py prints exact counts/first-last/differences; build-runs/wait-regionalcity.py waits once without qwait.
Failed gate evidence preserved build-runs/regionalcity-native-include-failure.txt, *-enum-failure.txt, *-enum-failure-full.log, *-return-review-failure-full.log.
Keep regionalcity_shadow default-off and OUT of regress profile until exact; forbidden tools files remain untouched. Coordinator must append the key to the current single shadows string after merge/verification; never edit coverage maps.
After correction: focused tests, owned-file fmt, stable all, unique queued real-city run through 2032; every difference must be covered by filed question or eliminated. Report exact counts honestly; current shadow is NOT verified.

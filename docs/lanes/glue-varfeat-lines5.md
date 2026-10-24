| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-varfeat-lines5: standard variant and feature shadow lines | Codex / `glue-varfeat-lines5` (engine lane, detached HEAD) | ready; cargo 6757/6757, clippy clean, CTest 69/69; run 342 clean through 2032, ref_warnings=0; variant 1/1, feature 75/75 | Coordinator review and coverage credit for cRZVariant / cSC4FeatureManager |

## Report

Implementation commit `640b40342` adds reporting only in `FC4VariantShadow.h` and `FC4FeatureShadow.cpp`.
Existing diagnostics and comparison logic are retained. The variant's bounded case group and every feature
query sample now emit standard lowercase labels, city dates, counts, and `mismatches=N`. Variant failures
and refusals contribute to N; feature differences, refusals, and lifetime differences contribute to N.
A final feature totals line includes shutdown lifetime results and also reports unsuccessful capture attempts.
Dates come from the current city simulator through existing interfaces, with the published date as fallback.
Both shadows remain default-off behind their existing INI keys. No C ABI exports, shared configuration,
regress profiles, coverage maps, or tools changed. No questions filed.

`qbuild all` passed on the final implementation: workspace tests **6757 passed, 0 failed, 0 ignored**;
clippy **0 diagnostics**; queued native build **220/220 edges**, CTest **69/69**, overall **PASS** (341.0 s native).
The separately requested `qbuild verify` also reached terminal **PASS**: native build **88/88 edges**,
CTest **69/69**, overall 87.0 s. Native request labels were `qbuild-20261024-003139-build-ctest` and
`qbuild-20261024-004532-build-ctest`; neither was duplicated while outstanding.
Rust logs are `build-runs/qbuild-test.log` and `qbuild-clippy.log`; latest native logs are
`build-runs/fc4verify-build.log` and `fc4verify-ctest.log`. `git diff --check` passed.

[Run 342](../runs/glue-varfeat-lines5-2032-20261024a-342.md), label `glue-varfeat-lines5-2032-20261024a`,
ran Konradshohe with both keys and turbo through **2032-01-01**: **clean**, `ref_warnings=0`,
foreign exceptions/watchdog stalls/RTC/overruns all zero. There are no outstanding game requests.
The coordinator-generated verdict recognizes both labels under **shadow lines**, with every mismatch count zero
and no resynced lines. An independent check of the task-supplied summary regex returned:

```text
variant 1/1
feature 75/75
```

Retained original exact totals:

```text
variant shadow: total cases=70 checks=420 exact=420 failures=0 refusals=0 clones=71 scalar_shape_HG7=0 unexplained=0
feature-shadow: totals samples=74 exact=13172 diff=0 refusal=0 resync=0 lifetime_exact=15 lifetime_diff=0 app_hold_remaining=1
```

The feature line count is 74 query samples plus one final totals group. The standard lines preserve all existing
domain exclusions; this reporting fix adds no comparison coverage for excluded operations.
Full game evidence is retained at `build-runs/game-results/glue-varfeat-lines5-2032-20261024a.log` and `.verdict`.
No local tools scripts were accessed or run; the queue supplied the summary in its verdict.

Next: coordinator review and coverage credit for the measured shadows. No implementation work remains open.

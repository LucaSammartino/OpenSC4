| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lotconf-capacity7 (lot configuration capacity verification) | Codex / glue-lotconf-capacity7 | ready | Land; coordinator can credit lotconf verification with capacity through 2032. |

## Changes

GLUE22 was already present: the requested merge reported already up to date. Implemented its §4.1 capacity host in `FC4BuildingCapacityHost.h`, shared by the production lot configuration manager and shadow. City slot 53 lends the provider; QI for IID 0x07813EAE adds a tracked reference, slot 39 lends the profile, capacity at +8 is copied as uint16, and only the QI reference is released. Borrowed profiles are never released or retained.

New C export `fc4_lot_configuration_capacity` reuses Rust `BuildingProfile::from_exemplar` to select the capacity map entry for purpose/wealth's developer type, default zero, with uint16 storage. Expected values come only from independently loaded resource properties. The shadow expands primary building IDs through slot 19, ignores its return when reading the resulting set, and compares both configuration capacity bounds and every production-host per-type result. No native capacity supplies expected values.

§4.2 costs are outside the configuration owner surface; actual occupancy requires a lot instance. Neither is represented as an unmeasured capacity gap. The dated summary format remains `lotconf shadow YYYY-MM-DD: ... mismatches=N`. Defaults remain off; the existing house-profile key needs no addition. No tools or coverage maps were edited. C ABI exports changed (one additive helper); ABI version unchanged.

The first run exposed a comparison timing issue: the observer's PostCityInit callback precedes the native manager's fill. The initial lotconf comparison now runs at ConnectionsReady, after native init recipients, per foundations §11.6. Monthly and save comparisons remain unchanged. Commits: implementation 71083c856; lifecycle correction c4cd3ac31.

## Validation

Focused capacity test: 1/1 passed, covering developer selection rather than summation, missing entry/default, and unsigned uint16 truncation. Explicit-file Rust fmt passed. The Bash q.py wrapper failed with Win32 error 5; direct native cargo succeeded.

Final-source `qbuild all`: **PASS**, cargo **6980 passed / 0 failed / 0 ignored**, clippy **0 diagnostics**, native build **50/50 edges**, full CTest **79/79**. `all` delegates the native verify steps to the coordinator build queue: `qbuild-20261024-071650-build-ctest`, native verdict PASS in 78.8s (build 2.7s, CTest 74.5s). Logs: `build-runs/qbuild-{test,clippy}.log`, `build-runs/fc4verify-{build,ctest}.log`. The prior source's gate also passed; the second gate followed the measured lifecycle diagnostic and source correction.

**lotconf-capacity7-2032-20261024b 73/73** exact with capacity included; [run 455](../runs/lotconf-capacity7-2032-20261024b-455.md). Clean turbo run beyond the requested 2032-01-01 end date, finishing 2032-01-16 in 32.812s. Initial comparison 2026-01-29; final monthly comparison 2032-01-01. Every line: exact=1909/1909, pairs=107845, objects=25210, detail=1, refused=0, mismatches=0. This adds 3818 range-bound checks, 4366 production-host profile checks and 1909 availability checks per line to the previous 97752 pairs. raw_foundation_tags=1908 remain separately recorded under PL23's unspecified-field scope.

All safety counts zero: foreign_exceptions, watchdog_stalls, ref_warnings, rtc, overruns, Rust panics, structured exceptions, incomplete lines, other warnings. Shutdown reached PostSystemServiceShutdown. Evidence: `build-runs/game-results/lotconf-capacity7-2032-20261024b.{verdict,log}`.

The earlier clean run [454](../runs/lotconf-capacity7-2032-20261024a-454.md), label `lotconf-capacity7-2032-20261024a`, preserves the diagnostic: 72/73 exact, all monthly capacities exact, load-only mismatches=3643 before native fill. This is superseded by the corrected-source 73/73 result, not discarded.

## Questions and next steps

No new questions or missing spec contracts. No outstanding game requests. Ready to land; the coordinator can credit lotconf verification from run 455. Detached HEAD is intentional; supervisor publishes the reserved lane branch.

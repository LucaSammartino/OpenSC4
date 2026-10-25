| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-disasters-replace16 (disasters house-profile ignition fix) | `Codex` / `glue-disasters-replace16` (engine lane) | Ready: cargo 7503/0, clippy 0 diagnostics, CTest 83/83; d16-null-r3-only-b and d16-null-r3-house-b clean through 2032; d16-null-r3-shadow-b 19/19 exact; all ref_warnings=0; C ABI unchanged | (1) Coordinator may append disasters to the tested house profile. (2) Complete flame visuals, remaining host writes and specialized interfaces. |

Fixed the supplied house-profile crash: native FireOccupant::SetTarget used static D7/D8 curves installed only by native fire-instance Init (disasters.md 5.3/5.11); the Rust instance never installs those native statics.
The existing live host now owns the fire interface, target references, burning/removal flags and controllers. It uses native occupant storage without native fire initialization or target/stage calls. Position/bounds follow the target; Create/Destroy messages retain the inserted COM identity. Stage/counter/hosed getters synchronize from Rust; hosing routes back to the runtime.
Regression tests cover forbidden native target/stage calls, target ownership, identity, lifecycle message identity/idempotence, state synchronization and hosing. Existing default-off factory and native manager-slot tests remain enabled.
Shadow fix `d50221be3`: EndCity's DropAll now matches PruneGone's final-holder policy for detached Keep-owned targets; ordinary releases remain audited while flag 1 is set.

Final gate: qbuild all PASS; cargo 7503/0, clippy 0 diagnostics, native build 47/47 edges, CTest 83/83. Verdict: `build-runs/build-results/qbuild-20261025-015801-build-ctest.verdict`. Its tree-change annotation records committing the already-tested shadow source during the run; source contents stayed stable.
Final DLL SHA256: `c197db2f6690a8b330b34a7531b5fa95fa85d4f5587bbc64ce939c042fb672a0`.
Final turbo runs: `d16-null-r3-only-b` clean at 2032-01-18; `d16-null-r3-house-b` clean at 2032-01-01; `d16-null-r3-shadow-b` clean at 2032-01-21. All foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0.
Exact coverage line: `d16-null-r3-shadow-b 19/19` (fire 17/17, riot 0/0, quiet 2/2; mismatches=0; differences by kind: none). Original steps 0, ours 0: this shadow run does not establish sustained fire/riot parity.
Evidence: `build-runs/game-results/d16-null-r3-{only,house,shadow}-b.{log,verdict}` and the matching requests under `build-runs/game-queue/done/`. The house list was recovered from enabled-subsystem lines in the supplied crash log, without reading tools. Both replacement runs use seed 3970341637 from that log.

Earlier evidence remains distinct: first audit failure in `build-runs/d16-r3-first-ctest-failed.log`; shadow-a was 11/11 exact but ref_warnings=1 at DropAll, corrected above. House-a exercised one ignition through the adapter and reached 2032 clean.
Still missing: flame rendering, aura/effect/lot-demolition and other existing host writes, specialized volcano/effect/toxic interfaces and non-fire typed instance contracts. The disasters key and shadow stay default-off. Ready covers the measured replacement scope, not complete disaster conformance.
Questions: none added; no spec, tools, coverage tables or C ABI exports changed.

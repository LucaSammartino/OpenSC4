| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-regionalcity-replace23 (RegionalCity ownership prerequisites) | `Codex` / `glue-regionalcity-replace23` (engine) | ready for steps 1–3 only; cargo 7788/0, clippy 0 diagnostics, native build 226/226, ctest 101/101; rc23-r5-handles2032-20261025a 73/73 exact, clean through 2032, ref_warnings=0 | Implement record element bridges, cache/connection host callbacks, and the default-off COM replacement; prove registration and both replacement configurations. |

Implemented steps 1–3: merged main; added an opaque Rust RegionalCity owner in capi; parameterized the core connection/deal lists and their host interfaces while preserving the existing Rc-backed defaults.

The C owner uses Mutex state with copied scalar/string getters and setters. Its actual lists retain host interface handles through copied callback tables; Clone acquires and Drop releases. Invalid list updates roll back without replacing existing contents. No native object layout crosses the transport.

Five focused owner tests passed, covering live callback reads, clone/release balance, replacement/rejection, shutdown and concurrent getters/setters. The native C transport ownership test also passed.
Full gate on stable source: cargo 7788 passed, 0 failed, 0 ignored; clippy 0 diagnostics; native build 226/226 edges; CTest 101/101. Native request: `qbuild-20261025-090805-build-ctest`. Logs: `build-runs/qbuild-{test,clippy}.log`, `build-runs/fc4verify-{build,ctest}.log`.

`rc23-r5-handles2032-20261025a 73/73`
Clean through 2032-01-01, ref_warnings=0. Each observation decoded 85 records and compared 16008 fields with mismatches=0. The scripted 2029 full save returned ok=1 (executed on 2029-01-20).
Run artifacts: `build-runs/game-results/rc23-r5-handles2032-20261025a.{verdict,log}` and `build-runs/game-queue/done/rc23-r5-handles2032-20261025a.req`.

Still missing: record element read/write using cFC4NeighborConnection/cFC4NeighborDeal, UpdateCityCache and ConnectionHost bridges, the COM owner/default-off regionalcity registration, registration tests, and regionalcity-only/full-house replacement runs through 2032. Generic record writing accepts host element callbacks; record decoding still constructs the default core elements.
Only the existing default-off regionalcity_shadow key was used. The native class is not replaced and no replaced coverage credit is claimed.
The public DLL export set is unchanged. No questions filed; no tools files or run-record documents edited.

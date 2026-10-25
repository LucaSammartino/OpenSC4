| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netlotobj-replace22: network lot ownership | Codex / `glue-netlotobj-replace22` (engine) | Ready measured partial: cargo 7641 passed; clippy 0 diagnostics; CTest 91/91; netlotobj22-r3-shadow-20261025 73/73; netlotobj22-r3-house32-20261025 clean through 2032, ref_warnings=0 | Complete crew/configuration contracts; confirm current house profile before three-class credit |

## Report

- Ownership fixes in `52a788ed5` and `31aa93259`: lot views retain occupants/shared configurations; borrowed cache returns do not leak references; changed cells replace stale views; iterator callbacks pin borrowed lots.
- This round extends seam operation 75 to replace the cached lot inside its callback, verify the old occupant remains usable, stop iteration early, and check the replacement's exact reference count. The DLL test checks both occupants return to their initial counts.
- Registration/ownership coverage also includes both manager factories, repeated borrowed returns, retained lots after manager destruction, and configuration lifetime.
- Stable final source: `qbuild all` PASS; Rust 7641 passed, 0 failed, 0 ignored; clippy 0 diagnostics; queued native build 57/57 edges and CTest 91/91, including `plugin_netlot_ownership`.
- Native verdict: `build-runs/build-results/qbuild-20261025-064259-build-ctest.verdict`; full logs remain under `build-runs/`.
- Exact shadow line: `netlotobj22-r3-shadow-20261025 73/73`; every mismatch count 0, 0 configuration differences, 0 resyncs; clean at 2032-01-05, ref_warnings=0.
- Shadow run record: `build-runs/game-results/netlotobj22-r3-shadow-20261025.verdict` and matching `.log`.
- Replacement run: `netlotobj22-r3-house32-20261025`, clean at 2032-01-01, ref_warnings=0; record `build-runs/game-results/netlotobj22-r3-house32-20261025.verdict` and matching `.log`.
- Replacement log confirms both manager factories and successful Init with 525 lots and 543 configurations. Both runs used DLL SHA256 prefix `c0e2ad3e`.
- House32 retains the documented 32-key profile; equality to the current forbidden regression script remains unconfirmed. The request in `build-runs/game-queue/done/` preserves every key.
- Still missing: crew creation/interface contract and Rust-backed crew construction; the configuration COM bridge remains incomplete. The lot/configuration views were already plugin-created; no three-class coverage flip is claimed.
- Existing replacement keys and default-off shadow setting are unchanged. Missing crew contract remains marked `// SPEC-GAP:`; no questions filed under the latest lane rules.
- C ABI exports unchanged. No coverage-map, regression-script, README, or run-record edits. No game requests outstanding.

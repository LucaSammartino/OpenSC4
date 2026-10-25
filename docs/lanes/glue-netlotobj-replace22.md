| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netlotobj-replace22: configuration bridge | Codex / `glue-netlotobj-replace22` (engine) | Ready measured partial: cargo 7642 passed; clippy 0 diagnostics; CTest 91/91; netlotobj22-r4-shadow-20261025 73/73; netlotobj22-r4-house52-20261025 clean through 2032, ref_warnings=0 | Specify crew creation/Init and configuration IID/container ABI; complete three-class replacement |

## Report

- `3e16477d6` adds the SDK configuration getter slots, atomic COM lifetime over the shared records, ID/piece/name metadata, and caller-held configuration references that survive release of the lot/shared owner.
- The manager now bridges forced configurations to Rust by exact host token. New export `fc4_netlot_set_next_configuration_token` distinguishes configurations sharing a saved ID; foreign pointers are never dereferenced and null clears the choice. ABI exports changed; ABI version unchanged.
- Tests cover duplicate-ID selection through actual lot-object creation, consumption of the forced choice, balanced occupant references, getter results, binary names, QueryInterface rejection/null output, retained configuration arrays, and manager forced/foreign/null handling. Existing manager registration and borrowed-lot callback tests remain covered.
- Stable final source: `qbuild all` PASS; Rust 7642 passed, 0 failed, 0 ignored; clippy 0 diagnostics; native build 47/47 edges and CTest 91/91, including `plugin_netlot_ownership`.
- Native verdict: `build-runs/build-results/qbuild-20261025-075311-build-ctest.verdict`. Failed initial gate evidence is preserved in `build-runs/r4-initial-failed-*`; the DLL-only seam now avoids SDK allocation without a game allocator.
- Replacement: `netlotobj22-r4-house52-20261025`, clean through 2032-01-01 in 108.047 seconds, ref_warnings=0, disabled=-. The 52-key profile comes from the recorded `regress-train229-replace.req`; the queued request preserves every key.
- Replacement Init confirms 525 lots and 543 configurations; populated exemplar arrays run with the game allocator. Both city runs used DLL SHA256 prefix `96fd609a`.
- Exact shadow line: `netlotobj22-r4-shadow-20261025 73/73`; all mismatch counts 0, 0 configuration differences, 0 resyncs; clean at 2032-01-09, ref_warnings=0.
- Run records: `build-runs/game-results/netlotobj22-r4-house52-20261025.verdict` and `build-runs/game-results/netlotobj22-r4-shadow-20261025.verdict`, with matching `.log` files. No requests outstanding.
- The train229 crash was not reproduced with its full replacement profile. Its raw DLL offsets do not establish function attribution, so no root-cause claim is made.
- Still missing: crew creation and six-argument Init contract/Rust-backed crew construction; configuration IID and output-set layout/allocator. These gaps remain explicit `// SPEC-GAP:` comments; the set bridge remains inactive. No three-class replacement credit is claimed.
- Existing replacement keys and default-off shadows are unchanged. No questions filed under the latest lane instructions; no coverage-map, regression-script, README, or run-record edits.

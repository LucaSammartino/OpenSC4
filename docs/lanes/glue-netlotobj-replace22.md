| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netlotobj-replace22: network object ownership | Codex / `glue-netlotobj-replace22` (engine) | Ready measured partial: cargo 7643 passed; clippy clean; CTest 91/91; netlotobj22-r6-livecfg-shadow-20261025 73/73; netlotobj22-r6-livecfg-house52-20261025 clean through 2032, ref_warnings=0 | Specify crew creation/Init and configuration IID/container; complete three-class replacement |

## Report

- Lot getters and iterator snapshots now carry the Rust core's exact configuration token into the plugin cache. Distinct configurations sharing a saved ID retain their own object arrays; changing the selected configuration rebuilds the view even when occupant and saved ID match.
- Two additive C exports: `fc4_netlot_get_lot_with_configuration`, `fc4_netlot_iterate_lots_with_configuration`. Legacy exports and the 16-byte lot record remain compatible; ABI version unchanged.
- Tests cover colliding IDs, forced choices, unresolved Read before Init, optional/absent outputs, early iterator termination, reentrant snapshot removal, and retained configuration lifetime after cache replacement. Existing registration and reentrant crew-shutdown ownership checks also pass.
- Focused network ABI tests: 17/17. Owned Rust formatting passed. Final `qbuild all`: 7643 passed, 0 failed/ignored across 52 result groups; clippy 0 diagnostics; queued native build 216/216; CTest 91/91 including `plugin_netlot_ownership`.
- Native verdict: `build-runs/build-results/qbuild-20261025-092459-build-ctest.verdict`; full logs remain in `build-runs/`.
- Exact shadow line: `netlotobj22-r6-livecfg-shadow-20261025 73/73`. All lines have mismatches=0; clean through 2032-01-10, ref_warnings=0.
- Full preserved 52-key house profile: `netlotobj22-r6-livecfg-house52-20261025`, clean through 2032-01-01 in 111.265 seconds, ref_warnings=0, disabled=-. Init reports 525 lots, 543 configurations and 13 network tools.
- Run records: `build-runs/game-results/netlotobj22-r6-livecfg-shadow-20261025.verdict` and `build-runs/game-results/netlotobj22-r6-livecfg-house52-20261025.verdict`, with matching logs and requests under `game-queue/done/`. Both used DLL SHA256 prefix `81d20927`; no requests outstanding.
- Still missing: Rust-backed crew construction and its six-argument Init contract; configuration IID and output-set layout/allocator. Existing SPEC-GAP comments remain; the unspecified set bridge stays inactive. No complete three-class replacement credit is claimed.
- Shadows remain default-off. No questions filed, spec edits, tools edits, README tables or docs/runs records.

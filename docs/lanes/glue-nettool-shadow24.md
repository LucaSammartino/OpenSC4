| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-nettool-shadow24 (native/Rust network planning shadows) | `Codex` / `glue-nettool-shadow24` (engine lane) | ready; cargo 7715/0; clippy clean; CTest 100/100; nettool-plan24-r2-final2032 clean, ref_warnings=0; both shadows exact 73/73; ABI unchanged | Coordinator: review scoped coverage and add exact keys to the shadows profile; remaining internal/bridge comparisons below |

Default-off `nettool_plan_shadow` compares cSC4NetworkTool road/street draft planning, ordered cells, return, failure, valid/straight/total counts and cost against private native tools on identical clear-land city inputs.
Default-off `powerline_plan_shadow` compares cSC4PowerLineTool draft selection/status/counts/cost, running Rust selection through the subclass callback; native draft poles/connections are undone before shutdown.
Fixed the replay's missing subclass callback and one-step selection bypass. Added an ABI regression covering one-, five- and thirteen-step drafts with a refusing base solver.
Final full gate: `qbuild all` PASS; cargo 7715 passed/0 failed; clippy 0 diagnostics; native build 93/93 edges; CTest 100/100 (`qbuild-20261025-074808-build-ctest`). Owned Rust formatting check passed.
Final game: `nettool-plan24-r2-final2032`, Konradshohe, turbo, reached 2032-01-16; verdict clean, foreign_exceptions=0, ref_warnings=0, Rust panics=0.
Exact log lines: `nettool plan shadow: city shutdown: exact 73/73 scope=draft-status-cost`; `powerline plan shadow: city shutdown: exact 73/73 scope=draft-status-cost`.
All 73 lines per shadow have mismatches=0 and unavailable=0: 876 road/street vectors and 438 powerline vectors. Evidence: `build-runs/game-results/nettool-plan24-r2-final2032.log` and `.verdict`.
Preserved diagnostics: `nettool-plan24-diag1` (occupied-cell costs/unavailable selection); `nettool-plan24-r2-2032` (network 73/73, powerline 0/73 from return-path mismatch). Both reached 2032 cleanly.
Coverage rows supported only in the measured scopes: cSC4NetworkTool via `nettool_plan_shadow`; cSC4PowerLineTool via `powerline_plan_shadow`. No whole-owner coverage claim.
Remaining: direct cSC4VertexHtConstraintSatisfier, cSC4NetworkCellInfo and ReadBridge*/cBridge* comparisons; occupied-cell demolition and reused poles/models. These are not verified by aggregate planning totals.
Questions filed: none; existing SPEC-GAP comments retain the bounded host limitations. No tools, coverage tables, regress profile or ABI exports changed.

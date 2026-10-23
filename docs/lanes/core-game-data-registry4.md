| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-game-data-registry4 (first portable registry core) | Codex / `core-game-data-registry4` | in progress | Read published contract; baseline; implement owned algorithms and literal tests |

## Handoff
- Goal: first portable owned game-data registry; no App/CAPI/plugin/default/ABI changes.
- Worktree reused, detached HEAD; initial status clean; no lane commits above main.
- Read sections 1–7 and GDR1–4; literal ledgers fixed in build-runs/registry-literal-ledgers.md before production.
- Baseline completed: qbuild exit0, Rust5372/0, clippy0 diagnostics, native217/217 PASS, CTest68/68.
- Stats PID30532 caused a long delay; no lane process action taken. Request now resolved by completion.
- Coordinator request: build-runs/coordinator-request-core-game-data-registry4-baseline.md.
- Engine tree c30f4d9ac1d78880e81cae198b7a461307350c58 unchanged for baseline; tree-change flag was docs-only.
- Owned core and16 literal tests moved from held drafts into engine/core/src/game_data_registry; additive lib export only.
- GDR5 filed for anonymous leaf entry-token/raw-key collision; portable input is tagged unsupported.
- Next: one serial module-focused test run, source review, then committed-source qbuild all once.
- Authority: docs/specs/game-data-registry.md; safe tokens model identity only.
- No game requests; no native or whole-owner promotion intended.

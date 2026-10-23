| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-game-data-registry4 (first portable registry core) | Codex / `core-game-data-registry4` | in progress | Read published contract; baseline; implement owned algorithms and literal tests |

## Handoff
- Goal: first portable owned game-data registry; no App/CAPI/plugin/default/ABI changes.
- Worktree reused, detached HEAD; initial status clean; no lane commits above main.
- Read sections 1–7 and GDR1–4; literal ledgers fixed in build-runs/registry-literal-ledgers.md before production.
- Baseline qbuild all pending: Rust/clippy logs complete; native RC/CTest unmeasured.
- Sole baseline descendant is stats PID30532; no process action authorized/taken.
- Coordinator request: build-runs/coordinator-request-core-game-data-registry4-baseline.md.
- Next: draft core outside compiled source tree while baseline engine source stays unchanged.
- Authority: docs/specs/game-data-registry.md; safe tokens model identity only.
- No game requests; no native or whole-owner promotion intended.

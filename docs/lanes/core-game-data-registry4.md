| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-game-data-registry4 (first portable registry core) | Codex / `core-game-data-registry4` | in progress | Read published contract; baseline; implement owned algorithms and literal tests |

## Handoff
- Goal: first portable owned game-data registry; no App/CAPI/plugin/default/ABI changes.
- Worktree reused, detached HEAD; initial status clean; no lane commits above main.
- Next: record baseline via qbuild all and read contract sections 2–7 and GDR1–4.
- Authority: docs/specs/game-data-registry.md; safe tokens model identity only.
- No game requests; no native or whole-owner promotion intended.

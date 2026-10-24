| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-widgettk-wt14 (private buffer and graphics contracts) | `Codex` / `re-widgettk-wt14` (RE lane) | in progress | 1. Resolve WT14 from export. 2. Add vectors and run doccheck. |

## Handoff
- Goal: close WT14 for core-uiwin-close13; spec text only.
- Done: added WT14 behavior, service boundary, ownership corrections and 14 vectors; moved WT14 to answered archive.
- Next: regenerate questions index, review diff, run doccheck, commit report and enqueue.
- Evidence: slot150 0099D0ED; draw-service slot5 0082B980; SetBufferToDrawTo 0099C6F8; byte checks establish signed OR dimension gate and format (1,0).
- Facts: actual paths are docs/specs/widget-toolkit.md and docs/specs/questions/widget-toolkit.md (brief's doubled paths are typos).
- No ABI changes or game requests; use export/decomp only, no REA binary available.

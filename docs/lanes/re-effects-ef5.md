| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-effects-ef5: EF15, EF16 and EFP7 owner contracts | Codex / `re-effects-ef5` | in progress | Verify residual nested commands and numeric contracts; publish specs and run doccheck |

## Handoff
- Goal: answer EF15/EF16 in effects.md and EFP7 in effects-parser.md; no engine reads or edits.
- Done: EF16 commit 8627cfed corrects §19.52; descriptor +0C is 0059A730 for both registrations.
- Done: EF15 §§19.53–19.56, 28 EC277–304 vectors; EFP7 §14, 17 NQ vectors. Questions archived; predictions unexecuted.
- Next: regenerate questions index, verify numeric/format assertions and doccheck; commit final report and queue.
- Closure table is inside engine/; task forbids reading engine/. Used commands5b's docs lane report to identify all three residual boundaries.
- Worktree intentionally detached; commit HEAD without changing refs. Queue kind re per task.
- No game runs, ABI changes or ABI proposals planned.

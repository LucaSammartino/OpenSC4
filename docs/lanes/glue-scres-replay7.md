| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-scres-replay7: physical file replay | Codex, `glue-scres-replay7` (detached HEAD) | in progress | Apply SCR10, gate, and measure a 2032 turbo run. |

## Handoff
- Goal: natural physical file-parser replay exact with zero resyncs; preserve scres and house shadows.
- Done: reused reserved worktree; read native6 report and operating rules.
- Next: read SCR10 and existing observer/core implementation, then implement physical line observation.
- Keep new observation default-off behind ini; no public C ABI changes planned.
- No build or game operation outstanding.

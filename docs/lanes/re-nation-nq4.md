| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-nation-nq4: tutorial directory case mapping | `Codex` / `re-nation-nq4` (RE, detached HEAD) | in progress | Resolve NQ4 from export, add vectors, regenerate index and run doccheck. |

## Handoff

- Goal: close core-nation10 NQ4 with behavior and conformance vectors, no code edits.
- Read core-nation10 lane report and question via git show; report identifies ASCII-only folding as provisional.
- Owner closure table is under engine/ and cannot be read under this task's explicit prohibition; use the published lane report and NQ4 contract instead.
- Next: inspect nation lowercase call targets and their implementation via re_query.py; determine encoding and locale dependence.
- Main RE source root: C:/Users/tlori/FreeCity4; no REA binary open, use local export.
- No game requests, ABI proposals or engine changes planned.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-netocc-close5: core network occupant owner closure | Codex; core-netocc-close5 (detached HEAD) | in progress | Audit spec ownership and close complete owners |

## Handoff
- Goal: close complete network occupant owners in existing Rust module; no ABI/plugin edits.
- Done: reused reserved clean detached worktree; read lane rules and report format.
- Next: read module and targeted spec sections; implement missing owner behavior and closure tests.
- Sources: only allowed specs and engine implementation. No game requests outstanding.
- Required gate: qbuild all once final source is stable; manifest run4-core-netocc-close5-r1-manifest.json.

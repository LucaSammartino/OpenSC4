| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-animated-sprites4: animated sprite owner | Codex / `core-animated-sprites4` (engine lane) | in progress | 1. Read public contracts; implement complete specified owner behavior or file precise contract holes. |

## Handoff
- Goal: pure Rust animated sprite instance owner, core only; no resource-owner or model_instances edits.
- Reserved worktree reused; detached HEAD; no pre-edit baseline rerun (fresh coordinator baseline supplied).
- Done: initial clean status and lane metadata; spec sections 0-3 read.
- Next: read sections 4-6/10 and narrow model-instance/base contracts, then implement and focused quick gate.
- Spec currently lists only slots 0-22; task requests 60 functions, so reconcile inventory before promotion.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-animated-sprites4-r1-manifest.json.
- No game request outstanding; core-only work has no shadow or ABI surface.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-pr-harvest5: contributor spec harvest | Codex; re-pr-harvest5 | in progress | Harvest selected PR10 and PR9 commits; review and validate. |

## Handoff

- Goal: harvest ten selected PR10 commits and PR9 e968dbd1, preserving newer main material.
- Worktree: reserved detached HEAD; do not switch branches or update refs.
- Done: read lane rules and CONTRIBUTING.md Specs requirements; initial tree clean.
- Next: cherry-pick 46ecc345 through c95be639 in the requested order, then e968dbd1.
- Deferred: a8b30439 and a33fa02e (concurrent occupant specs); a12daabf (drawing-fonts implementation).
- Resolve generated indexes by regeneration; coverage map changes limited to spec column; symbol duplicate keys retain main.
- Review every added spec section for translated control flow; no engine/plugin/capi edits.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-pr-harvest5-r1-manifest.json.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-pr-harvest5: contributor spec harvest | Codex; re-pr-harvest5 | in progress | Harvest selected PR10 and PR9 commits; review and validate. |

## Handoff

- Goal: harvest ten selected PR10 commits and PR9 e968dbd1, preserving newer main material.
- Worktree: reserved detached HEAD; do not switch branches or update refs.
- Done: 46ecc345 -> a8dc40e6; 8853d10c -> 5e8196c6, original authors and -x trailers retained.
- Next: resolve 5517422f (generated QUESTIONS index and names union), then remaining commits in order.
- Conflict policy applied: regenerated QUESTIONS; symbol table retains pre-harvest rows on duplicate keys.
- Git reports packed-refs.lock permission warnings during commit, but detached HEAD commits succeed; shared refs remain read-only.
- Deferred: a8b30439 and a33fa02e (concurrent occupant specs); a12daabf (drawing-fonts implementation).
- Resolve generated indexes by regeneration; coverage map changes limited to spec column; symbol duplicate keys retain main.
- Review every added spec section for translated control flow; no engine/plugin/capi edits.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-pr-harvest5-r1-manifest.json.

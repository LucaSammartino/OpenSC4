| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-pr-harvest5: contributor spec harvest | Codex; re-pr-harvest5 | in progress | Harvest selected PR10 and PR9 commits; review and validate. |

## Handoff

- Goal: harvest ten selected PR10 commits and PR9 e968dbd1, preserving newer main material.
- Worktree: reserved detached HEAD; do not switch branches or update refs.
- Done: 46ecc345 -> a8dc40e6; 8853d10c -> 5e8196c6, original authors and -x trailers retained.
- Done: 5517422f -> ec33f456 (render core); original authors and -x trailers retained.
- Next: resolve 128498f4 (QUESTIONS, names, coverage map), then remaining commits in order.
- Coverage policy tightened to existing main rows and spec column only: omit contributor additions/reclassification/key renames to preserve row order and first-match semantics.
- Conflict policy applied: regenerated QUESTIONS; symbol table retains pre-harvest rows on duplicate keys.
- Git reports packed-refs.lock permission warnings during commit, but detached HEAD commits succeed; shared refs remain read-only.
- Deferred: a8b30439 and a33fa02e (concurrent occupant specs); a12daabf (drawing-fonts implementation).
- Resolve generated indexes by regeneration; coverage map changes limited to spec column; symbol duplicate keys retain main.
- Review every added spec section for translated control flow; no engine/plugin/capi edits.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-pr-harvest5-r1-manifest.json.
- Harvest milestone: f02a528f resolved with generated index, symbol union, and spec-column-only coverage; next source 47f2fde4.
- Harvest milestone: 4048a59f resolved with generated index, symbol union, and spec-column-only coverage; next source f02a528f.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-pr-harvest5: contributor spec harvest | Codex; re-pr-harvest5 | blocked: coordinator STOP for reboot; all eleven cherry-picks committed | Resume clean-room review, doccheck, final report and queue after STOP is lifted. |

## Handoff

- Goal: harvest ten selected PR10 commits and PR9 e968dbd1, preserving newer main material.
- Worktree: reserved detached HEAD; do not switch branches or update refs.
- STOP 2026-10-23: coordinator reboot (sandbox logon-session leak); no queue written. All eleven sources committed, latest harvest tip 229aeb6b. Full clean-room review and doccheck NOT RUN; not ready to land.
- Taken in order: 46ecc345 -> a8dc40e6; 8853d10c -> 5e8196c6; 5517422f -> ec33f456; 128498f4 -> 5a106945; 4048a59f -> 4ee7425d; f02a528f -> c6ac2b76; 47f2fde4 -> 9b7befdc; 9087568e -> 16713743; 13cf2b33 -> 788f2a53; c95be639 -> 5cf8acf6; e968dbd1 -> 229aeb6b. Original authors and -x trailers retained.
- Next: run build-runs/harvest-review.py (read-only added-text pager: 84 pages, 0-83; no pages reviewed yet). Review all harvested additions against CONTRIBUTING.md Specs; rewrite translated control flow and assembly snippets via apply_patch. Then questions.py check, doccheck.py check, final preservation audit, Report and kind=re ready queue.
- Coverage policy tightened to existing main rows and spec column only: omit contributor additions/reclassification/key renames to preserve row order and first-match semantics.
- Conflict policy applied: regenerated QUESTIONS; symbol table retains pre-harvest rows on duplicate keys.
- Git reports packed-refs.lock permission warnings during commit, but detached HEAD commits succeed; shared refs remain read-only.
- Deferred: a8b30439 and a33fa02e (concurrent occupant specs); a12daabf (drawing-fonts implementation).
- Resolve generated indexes by regeneration; coverage map changes limited to spec column; symbol duplicate keys retain main.
- Review every added spec section for translated control flow; no engine/plugin/capi edits.
- Review tooling: build-runs/harvest-normalize.py enforces prior symbol rows and original coverage row set; harvest-signposts.py already applied (do not rerun). INVENTORY-M6 unchanged; no inventory generator found in tools search.
- Final coverage diff vs 66f935e6: only spec cells on cSC4ConstructionOccupant, SC4QuadGrid, four region-dialog classes, cSC4View3DWin and cSC4SubBudgetWindow. Original row order and every non-spec cell preserved; all contributor added rows/key renames/reclassifications omitted. Verify again before ready.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-pr-harvest5-r1-manifest.json.
- Harvest milestone: e968dbd1 resolved via regenerated index and spec-only coverage normalization. All eleven requested sources applied; next clean-room review and validation.
- Harvest milestone: c95be639 resolved; main's newer construction material merges cleanly. Next e968dbd1, then full clean-room review and doccheck.
- Signpost milestone: 13cf2b33 resolved; main's dedicated TextDecal spec and vectors remain authoritative. Imported interface supplement; omitted superseded duplicate TextDecal behavior (including older color threshold). Main OP1/OP2-TD answers retained; main's signpost OP2 question moved to archive with contributor evidence. Next c95be639.
- Harvest milestone: 9087568e resolved with generated index, symbol union, and spec-column-only coverage; next source 13cf2b33.
- Harvest milestone: 47f2fde4 resolved with generated index, symbol union, and spec-column-only coverage; next source 9087568e.
- Harvest milestone: f02a528f resolved with generated index, symbol union, and spec-column-only coverage; next source 47f2fde4.
- Harvest milestone: 4048a59f resolved with generated index, symbol union, and spec-column-only coverage; next source f02a528f.

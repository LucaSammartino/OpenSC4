# re-occprops

| lane | branch | base | state at stop | spec | merge queue |
|---|---|---|---|---|---|
| re-occprops | re-occprops | main | in progress | docs/specs/occupants-props work (extend lot-records.md + occupant-manager.md) | build-runs/merge-queue/re-occprops.json |

## Handoff
- Goal: specify the occupants+props pool (33 owners, 453 fns, 66,254 bytes) so coverage-map rows cSC4Occupant*, cSC4*Occupant, cSC4ConstructionOccupant, cSC4SignpostOccupant, cSC4TextDecalOccupant, cSC4*OccupantFilter become specified.
- Scope: extend docs/specs/lot-records.md (§9 construction detail), docs/specs/occupant-manager.md (generic cSC4Occupant base), and add signpost/text-decal/filters where they fit. NOT a new spec. Exceptions: tools/coverage-map.tsv additive rows only.
- Done:
  - worktree created at base a2e6fdf5.
- Next: read docs/specs/lot-records.md outlines (§9, §10, §12) + docs/specs/occupant-manager.md outline; grep docs/abi/vtables.tsv + interfaces.tsv for cSC4Occupant classes; then query re_query.py for class sizes/vtables.
- Facts: <MAIN> = C:/Users/tlori/FreeCity4; <WT> = C:/Users/tlori/FreeCity4/.claude/worktrees/re-occprops; budget 150000; today 2026-10-20. Questions go in NEW docs/specs/questions/occupants-props.md (OP1+). No decomp/ committed; no engine/ reads; abi rows to build-runs/abi-rows-re-occprops.tsv if needed.
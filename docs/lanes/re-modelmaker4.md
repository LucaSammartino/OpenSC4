| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-modelmaker4 (RE spec: exact cSC4ModelMaker owner, docs/specs/model-maker.md) | `re-modelmaker4` / (RE spec lane) | in progress (round 1/3 done, partial: 7777 of 34705 bytes specified, inventory §20 lists 70 remainder rows) | see Handoff |

## Handoff
- Goal: docs/specs/model-maker.md specifying the whole cSC4ModelMaker owner (105 functions, 34705 bytes per
  tools/coverage.py --owners) with the §20 inventory remainder empty; only then claim whole-owner credit.
- Done (round 1): e1f9e323 §1-5 (identity/QI/vtables, state layout, Init/DoPostCityInit/Shutdown/dtor, save
  record v2, DoMessage table of 28 IDs); a9b2c5eb §8 occupant handlers + deferred queue (ops 1-8, coalescing);
  b487f840 §8.8 Moved outline, §19 conformance M1-M12, §20 inventory; ca743ee8 §9.4 prop state, §12.1-12.3.
- Next (round 2), in this order, updating §20 rows from "remainder" to the section and recomputing the totals
  line: DoOccupantHilighted 0x493A00 (§8.9); §6 filters (CreateOccupantFilters 0x493EF0,
  OccupantFilterIterationFunction 0x495360, 0x5EA370/0x5EA550, 0x5D5xxx); §7 DetermineLoadRadius 0x48EB50,
  UpdateModelLoadRect 0x48F660, UnloadUnnecessaryModels 0x492530, RemoveModelsInappropriateForRotation 0x492780,
  ComputeCityExclusionRects 0x48ED20; §9.1-9.3 (0x9D8E0B6 path 0x4933A0, DoLotStateChanged 0x493D60,
  ApplyStateEffectToModels 0x493C80, DoBuildingCompletionChanged 0x48FFF0); §10 view changes (0x495B00, 0x495D90,
  0x4959C0, LoadOccupantModelsForZoomAndRotation 0x495430); round 3: §11 DoLightingChanged/RebuildDynamicTexture,
  §12 AddModel(s)/LoadModelsCommon/Foundation/Network/Underground/FindModelTransform/placeholder/ChangeModels,
  §13 CreateOccupantShadow, DoTerrainHiding*, 0x445F10-0x447B00 and 0x8D08E0-0x8D0E10 reach helpers.
- Facts: REA down all round ("No app is open"); fallback re_query asm + decomp/. Read/Write run on this+4 (their
  Ghidra field labels are ECX-relative: add 4). Decomp of DoMessage/handlers is garbled (args, returns): verify
  with `re_query.py asm --full --max 2000`. Helpers (gitignored, recreate if lost): build-runs/fn.py <addr>
  prints a decomp body; vt.py vt <va> <n> dumps a vtable; ids.py <hex...> finds doc mentions of IDs.
  Owner list: `python tools/coverage.py --owners <WT>/build-runs/owners.tsv`, filter column 3 == cSC4ModelMaker.
- No questions filed, no ABI rows, no game runs. Not done: coverage-map spec column (tools/, not ours to edit).

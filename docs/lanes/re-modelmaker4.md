| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-modelmaker4 (RE spec: exact cSC4ModelMaker owner, docs/specs/model-maker.md) | `re-modelmaker4` / (RE spec lane) | partial after round 2/3: 13942 of 34705 bytes specified, §20 lists 51 remainder/outline rows (20763 bytes); no whole-owner credit; no questions, no ABI rows, no game runs | 1. §10.4 LoadOccupantModelsForZoomAndRotation from asm 2. §9.1 0x4933A0 jump table 3. §9.5 DoBuildingCompletionChanged 4. round-3 list below |

## Report (round 2)

- Commits: b791dd7c §6 occupant filters (types filter class incl. its Write/Read 0x5EA370/0x5EA550, zoom/rotation
  filter mask contract, OccupantFilterIterationFunction) and §7 (fast F/C rounding, DetermineLoadRadius per-zoom
  radii from system memory, UpdateModelLoadRect, exclusion rects, rect intersect 0x48ED90, UnloadUnnecessaryModels,
  RemoveModelsInappropriateForRotation, mask helpers); c06500c1 §10.1-10.3 view changes (+ §10.4 outline);
  9e7631a1 §9.2 DoLotStateChanged, §9.3 ApplyStateEffectToModels; d1b72a80 §9.1 outline; e024a65c §19 M13-M19.
- §2 corrected: +0x78..+0x88 are per-zoom load radii (not budgets); added +0x50, +0x8C..+0xAC, +0xC2; +0xB0/+0xB4
  are constant all-zoom/all-rotation masks.
- Quirks recorded: unload body only runs at <= 32 MiB reported memory (§7.4); lot-state args swapped (§9.2);
  foundation QI unchecked (§9.3); city-cell load rect passed to the manager-cell query in DoViewTranslationChange
  (§10.2); filter Read accepts a bad second version (§6.2); 0x48EAE0 is a model-maker method despite its export name.
- Measured: tools inventory §20 = 105 rows; specified 13942 B, remainder 20763 B in 51 rows (round 1: 8163 / 69).
- REA unavailable all round ("No app is open"); everything from re_query asm + decomp/ (static fallback).
- Next (round 3): §10.4 0x495430 (1329 B, decomp unusable: trace asm, uses 0x48EA00/0x48EA30); §9.1 0x4933A0 case
  mapping of table 0x49367C and tail; §9.5 DoBuildingCompletionChanged 0x48FFF0; §11 DoLightingChanged/
  RebuildDynamicTexture; §12 AddModel(s)/LoadModelsCommon/Foundation (0x5D5xxx helpers)/Network/Underground/
  FindModelTransform (0x5D7710)/placeholder/ChangeModels/0x4906B0/0x490820/0x48F380; §13 CreateOccupantShadow,
  DoTerrainHiding*, 0x445F10-0x447B00 and 0x8D08E0-0x8D0E10.
- Helpers (gitignored): build-runs/ca.py (compact asm), inv.py ADDR=SECTION (updates §20 rows and totals), fn.py,
  vt.py, ids.py, rd.py.

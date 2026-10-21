| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-modelmaker4 (RE spec: exact cSC4ModelMaker owner, docs/specs/model-maker.md) | `re-modelmaker4` / (RE spec lane) | parked after round 3/3 (final): 20607 of 34705 bytes specified, 26 remainder rows (14098 B) listed in §20; no whole-owner credit; no questions, no ABI rows, no game runs | a new lane for the remainder: AddModels/AddModel/LoadModelsCommon/Foundation/FindModelTransform/placeholder, CreateOccupantShadow, DoTerrainHidingForCells, RebuildDynamicTexture, 0x445F10-0x447B00, 0x8D08E0-0x8D0E10 |

## Report (round 3, final)

- Commits: 0663fe1a §9.1 full (jump table 0x49367C: x0 lit test, x1 default, x2..x6 -> k 0,1,4,2,3; t, y unused;
  always true) + §12.4 ChangeModels/RestoreStaticTextures 0x48F380/ApplyTextureToModels 0x4906B0/
  SwapTexturesByOffset 0x490820 (+0x8000 texture instances); d1f4de41 §10.4 0x495430 from asm (id-list
  comparison, reload vs in-place yaw rotation by -(+0xF8), mask setters); c99738ca §9.5 DoBuildingCompletionChanged
  (sink = height*(1-c)*1.05f, bit 0 under construction); 5c38b369 §12.5 helpers (LotAtOccupant 0x48F200,
  RotateAboutAxis 0x5D7710, foundation setters 0x5D5xxx, container helpers); 39b137ac §12.6 network/underground
  loading, §13.1 DoTerrainHidingForOccupantCells; f3a7b9fc §11.1 DoLightingChanged (eps 1e-4f, rebuild 5
  textures; occupant loop has no model effect); a5f777c0 §19 M20-M23 and §20 remainder list.
- Measured (build-runs/inv.py over §20, 105 rows): specified 20607 B (round 2: 13942), remainder 14098 B in 26
  rows (round 2: 20763 / 51).
- Facts: 0x00A80990 is a shared double 0.0 (cos 1, sin 0), so §10.4's rotation is a pure Y rotation;
  0x278128A0 is an occupant type (DoLightingChanged filters on it) and is the flag every ChangeModels call passes.
- REA unavailable all round ("No app is open"); static fallback (re_query asm + decomp/) for everything.
- Not done (parked, no fourth round): see §20 "Remainder after round 3".

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

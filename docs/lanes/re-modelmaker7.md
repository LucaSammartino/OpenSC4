| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-modelmaker7: the model maker remainder (`cSC4ModelMaker`) and the background loader (`cSC4ModelBackgroundLoader`), completion of model-maker.md (RE spec lane) | `re-batch7` branch (outside contributor checkout, fork amarret/OpenSC4; on main `5c0ff782`, after re-scape7) | **ready** (docs only). model-maker.md: §8.8 move in full; §11.2 `RebuildDynamicTexture` and the tint kernels §11.3/§11.3.1; §12.7–§12.12 model loading, the transform record, the placeholder and the occupant shadow; §13.2 terrain hiding; §14 `AddModels` in full; §15.3–§15.6 the loader's tasks, worker, main thread and helpers; §20 inventory rebuilt; MM2 answered; both owners claimed: specified +40,395 bytes (+110 functions) | MM1 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`)
and the Ghidra export of the two owners. Every body of the round 3 remainder and the loader's worker and main-thread
functions was disassembled. Written on 2026-10-23. Upstream parked model-maker after round 3/3, and no open lane
covers either owner.

### What changed

- §8.8 Moved: the moved instance's record is read first (slot `+0x24`), so its rotation is kept; only the
  translation changes. The renderer re-queue is described.
- §11.2 `RebuildDynamicTexture` in full:
  - the two services and the texture key;
  - the five dynamic textures (k = 4 takes the second instance);
  - the descriptor words and the per-format kernel table;
  - mip sizes `max(w, block) × max(h, block)`, halved per level;
  - the colour channel through the `+49152.0f` bit trick;
  - update or create.
- §11.3 the six tint kernels `0x008D08E0`..`0x008D0E10`; §11.3.1 the DXT endpoint and index rules with their quirks
  (checked again on the bytes of `0x008D0E10`).
- §12.7–§12.12, all new:
  - `LoadModelsCommon` and `LoadFoundationModels`;
  - `CreatePlaceHolderModelInstance`: the base by footprint, and `r = (3 × viewRot + rot) % 4`;
  - the 0x34-byte transform record and `FindModelTransform`, with the terrain tilt and its 0.866/0.5 limits;
  - `AddModel` and `CreateOccupantShadow`.
- §13.2 `DoTerrainHidingForCells`: the per-type mask/bits rules and the write-back. §14 `AddModels` as a full table
  (the outline tag is dropped).
- §15:
  - the task record and the curve offsets, the embedded clockwise `SpiralIterator`;
  - Start with its screen-width estimate (170, 91, 46, 20, 10 at 1024), Refocus, `ClearResourceSet`;
  - `ThreadFunction`: budget, slice limit, progress formula, posts, and the sleep capped at 5,000,000 (default
    1,000,000);
  - `ContinueLoadTask`, `LoadResourcesForOccupant` and the resource set;
  - `ProcessLoadedCells`;
  - the node and vector-insert helpers.
- §20: the inventory rebuilt from the current owner map: 88 + 22 rows, with no remainder.
- docs/specs/questions: MM2 answered and archived (new answered/model-maker.md); QUESTIONS.md regenerated.

### Quirks found

- `LoadModelsCommon`: only the first model sets the result, the building flag `0x40` and the development call.
- `LoadFoundationModels` reads `0x88FCC3E3` and never uses it.
- `FindModelTransform`:
  - adds the offset unrotated in the flag case;
  - limits only positive x and z of the terrain normal;
  - its 0.0 tilt constant gives negative zeros.
- `AddModel`: `rot` is unused and the holder is not null-checked. `AddModels`: the building and prop results are the
  state effect's.
- `DoTerrainHidingForCells`: the base-texture query passes cell indices relative to the hiding rect, and the `mode`
  pass is overwritten by the rebuild.
- `ContinueLoadTask`: costs nothing when `0x0B579C20` is false.
- Refocus resets the spiral but keeps the task's state.
- A non-zero task type would never finish (no code makes one).

### Coverage

- tools/coverage-map.tsv: `cSC4ModelMaker` and `cSC4ModelBackgroundLoader` → model-maker.md, before the `cSC4Model*`
  catch-all.
- Specified bytes 4,393,012 → 4,433,407 (+40,395, 78.7% → 79.4%, `cov_local.py`, on `5c0ff782` + the three lanes
  before).
- The ten rows `0x00445F10`..`0x00447B00` that round 3 listed belong to `cSC4AnimationCoordinator` in the current
  owner map and are not counted here.

### Not covered

- MM1 (the curve units: the loader's sleeps of 1,000,000 and 5,000,000 point to microseconds).
- What the overlay manager does with a model shadow (overlay-manager.md §3.7).

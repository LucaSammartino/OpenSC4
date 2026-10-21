| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lot-record-answers4 (spec answers LR1-LR5 for lot-records.md) | `re-lot-record-answers4` / (spec lane) | ready, docs only: LR1-LR4 answered (LR1 MATCHES, LR2 MATCHES, LR3 DIFFERS, LR4 DIFFERS) in lot-records.md 3.2/3.5/4, tests T17-T25; LR5 answered (7.1 wall add/removal, 7.2 UpdateRetainingWalls 0x005E37F0, AddRetainingWallModel mask rules, UpdateSurroundingRetainingWalls 0x005E3960); no code, no ABI, game=false | 1. Engine: implement 7.2 (hash-diff model refresh, mask only via model build, keep all-zero walls) with T22-T25. 2. Engine: conform `lot_records::lifecycle` to LR3 (Remove has no defined return; always SetUnit(0)+SetTravelDesignate(0)) and LR4 (Write never writes the ID read: un-Inited lot writes 0). |

## Report

**What changed (docs/specs/lot-records.md, static RE: REA had no app open, so re_query.py asm + decomp/ only)**

- 3.5 Init step 1 (LR1): `GetCenter` `0x006BF560` = rect centre in metres, `((16 x1 + 16 (x2+1)) * 0.5, altitude,
  same for z)`, exact; Init asks terrain `0x00B43CF4` slot 13 at (x, z); `SetAltitude` `0x006BF5C0` acts only when
  `|a - alt| > 0.001` (double constant `0x00AA2270`), then writes `GetLotSlope(lot, false)` into both slope fields
  (field 7 `+0x2C`, field 8 `+0x28`, mapping checked in `Write` `0x006C0080`). Quirk: terrain altitude within 0.001
  of 0 leaves altitude and slopes 0.
- 3.5 SetFlags (LR2): flags stored first; bit-0 change + non-null terrain view: x outer, z inner, inclusive,
  ascending; LevelCell(x, z, altitude) / RemoveCellLevelling(x, z) (view slots 16/17).
- 4 membership slots (LR3): Add `0x006C0940` (append only if absent, always SetUnit), Remove `0x006C08B0` (erase
  all matches, always SetUnit(0) then SetTravelDesignate(0), no defined AL), GetMembers `0x008085F0`; Shutdown and
  RemoveLotFromItsBusinessUnit ignore AL.
- 3.2 field 2 / 3.5 (LR4): `Write` writes only the configuration object's ID or 0; the ID read (`+0x30`) is never
  written; a read-but-never-Inited lot saves 0.
- 7.1 (LR5 partial): `AddRetainingWallOccupant` add path (edge hashes, mask, position, size, type, insertion,
  flag bit 2; AL is 0 even on success, `0x005E325B`), `GetLotWallEdgeFlags` `0x005E2CE0` (neighbour-cell bits,
  vertex altitude hash with round-to-nearest-even), removal path (location cell, first wall, flag bit 2 cleared).
- 7.2 (LR5 rest, round 2, static RE: REA still had no binary open): `UpdateRetainingWalls` `0x005E37F0` order
  (flag bit 0; location-cell wall lookup; missing wall or failed QI `0x295D1162` -> `AddRetainingWallOccupant(lot,
  true)`; recompute hashes, ignore AL and `top`; per edge remove-then-add model when the hash changed; one
  `SetEdgeHashes` `0x005E2930` plain copy; Release W then w). Never changes position/size/type/flags; keeps a wall
  whose hashes all go to 0; null `w` not tested. `AddRetainingWallModel` `0x005E3400`: AL = 1 always; mask bit set
  on model build success, cleared on failure, untouched on removal. `UpdateSurroundingRetainingWalls` `0x005E3960`:
  ring lots (slot 9, flags 0xF) in order, not the lot itself, each released.
- 17.3: tests T17-T25.

**Questions:** LR1-LR5 moved to docs/specs/questions/answered/lot-records.md (LR5 MATCHES add path / DIFFERS update); QUESTIONS.md regenerated; none open.

**Commits:** a1051b84 (LR1-LR4 spec), 72fe4661 (answers moved), 3d543072 (7.1 add path), ab3f54cf (removal path), b5ce7dd5 (7.2, LR5 answered).

# lot-records: answered questions (archive)
<!-- Archive for [lot-records.md](../../lot-records.md). Lanes do not read this file. Open questions are in ../lot-records.md. -->

<!-- from: core-lotrecords4 (2026-10-21) -->
- **LR1 - section 3.5 Init step 1:** which point does `GetCenter` (lot slot 15) return for a lot rect (cell centre
  of the rect in metres?), and which slope does `SetAltitude` (slot 17) store in fields 7 and 8 ("also sets both
  slopes")? [both come from the host: `LotHost::center_altitude_and_slope(rect)`]
  **ANSWERED 2026-10-21 (re-lot-record-answers4):** MATCHES (with exact rules) — section 3.5 Init step 1: GetCenter (0x6BF560) = ((16*x1 + 16*(x2+1)) * 0.5, altitude, (16*z1 + 16*(z2+1)) * 0.5) in metres; Init passes (x, z) to terrain 0x00B43CF4 slot 13 GetAltitude; SetAltitude (0x6BF5C0) acts only when |a - altitude| > double 0.001, then stores a and writes GetLotSlope(lot, false) (BDS slot 36 -> lot developer slot 30 = GetAreaSlope(rect, false)) into both field 7 (+0x2C) and field 8 (+0x28).

<!-- from: core-lotrecords4 (2026-10-21) -->
- **LR2 - section 3.5 SetFlags:** in which order are the rect cells locked / unlocked (x outer or z outer), and is
  the lock call the terrain view's per-cell lock with the lot altitude as given? Observable only through the terrain
  view's call order. [x outer, z inner, x1..=x2, z1..=z2]
  **ANSWERED 2026-10-21 (re-lot-record-answers4):** MATCHES — section 3.5 SetFlags (0x6BF220): flags stored first; on a bit-0 change with a non-null SL::TerrainView: x outer x1..=x2, z inner z1..=z2, ascending; set -> view slot 16 LevelCell(x, z, altitude as stored, re-read per cell), clear -> view slot 17 RemoveCellLevelling(x, z).

<!-- from: core-lotrecords4 (2026-10-21) -->
- **LR3 - section 4 / lot-developer.md section 5.7 Remove:** what does `cISC4BusinessUnit::Remove(lot)` return in
  AL, and does it still call `lot.SetUnit(0)` / `SetTravelDesignate(0)` when the lot is not a member? [true when
  the lot was a member; both setters always]
  **ANSWERED 2026-10-21 (re-lot-record-answers4):** DIFFERS — section 4 membership slots: Remove (0x6C08B0) has no defined return (AL = low byte of SetTravelDesignate's EAX: 0 without a designate, else the old designate's Release result); it erases every matching node and always calls lot.SetUnit(0) then lot.SetTravelDesignate(0), member or not. Callers ignore AL.

<!-- from: core-lotrecords4 (2026-10-21) -->
- **LR4 - section 3.2 field 2:** a lot that was read but never `Init`ed (the crime-only lot of section 14) holds no
  configuration object, only the ID read. Does its `Write` save 0 (no object) or the ID read? [the ID read]
  **ANSWERED 2026-10-21 (re-lot-record-answers4):** DIFFERS — sections 3.2 field 2 and 3.5: Write (0x6C0080) writes only the configuration object's GetID or 0 and never the ID read (+0x30), so a read-but-never-Inited lot saves 0.

<!-- from: core-lotrecords4 (2026-10-21) -->
- **LR5 - section 7 / lot-developer.md section 7.2 AddRetainingWallOccupant:** how does the add path compute the
  wall type, edge mask, the four edge hashes, position and size of the new wall from the lot, and what does
  `UpdateRetainingWalls(lot)` change on an existing wall? Without it a core can read and write walls but not build
  them. [not implemented; record Write/Read and the T7 mask growth only]
  **ANSWERED 2026-10-21 (re-lot-record-answers4):** MATCHES (add path) / DIFFERS (update) — sections 7.1 and 7.2: the add path is AddRetainingWallOccupant 0x005E3070 (edge hashes from GetLotWallEdgeFlags 0x005E2CE0, mask from non-zero hashes, rect-centre position, size (16w, top - a, 16d), AL always 0). UpdateRetainingWalls 0x005E37F0: flags bit 0 clear -> nothing; no wall at the location cell or QI 0x295D1162 fails -> AddRetainingWallOccupant(lot, true); else recompute hashes and, per edge i = 0..3 whose hash changed, remove model i (old != 0) then add model i (new != 0, inserted in the renderer), then SetEdgeHashes(new) once if any changed. Position, size, type, lot flags are never changed and an all-zero wall is kept; the mask changes only in AddRetainingWallModel 0x005E3400 (bit set when the model builds, cleared when it fails, untouched on removal). UpdateSurroundingRetainingWalls 0x005E3960 runs UpdateRetainingWalls on every lot of GetLotsSurroundingLot(lot, 0xF) in ring order, not on the lot itself.

<!-- from: glue-lotrecords4 (2026-10-22) -->
- ~~**LR8 - sections 3.1, 4, 7.1/7.2 and 15, complete-owner runtime interception boundary:** what published
  registration or interception contract lets an independently loaded plugin replace these three owners while
  retaining the native zone developer and lot developer? The class-object store uses GZCOM on load
  (foundations sections 2.7/3.6), but new lots are constructed directly by the zone developer
  (`zones.md` section 12, `0x006BF430` + `0x006C1E50`); `PickUnitToJoin` constructs business units directly,
  without the COM directory (`lot-developer.md` section 5.7, constructor `0x006C0800`); and native
  `AddLotFoundation`/`ClearLot` call the nonvirtual retaining-wall helpers (sections 7.1/7.2/15,
  `0x005E3070`, `0x005E37F0`, `0x005E3960`), including direct wall construction (`0x005E2F40`).
  Please specify the supported redirection boundary for those creation/helper paths, its calling convention,
  installation/restoration lifetime and ownership obligations, or explicitly require replacing the calling
  developers together. Factory registration and method addresses alone do not specify an interception contract.
  [blocked before runtime wiring: no `enable=lotrecords` registration, no partial factory or same-owner forwarding;
  existing Rust owner implementations unchanged; LR6/LR7 remain open]~~
  **ANSWERED 2026-10-22 (re-lotrecords-boundary4):** MATCHES — section 15.1 proves the direct constructor/helper edges, stack ABI, storage and reference transfers. Factory registration covers load only. With no established direct-edge installation/restoration contract, complete-owner replacement must include the zone developer/lot manager for lots and the lot developer for units/wall helpers. No independent native-developer interception contract is invented. Sections 4/7 link the boundary; T28-T30 distinguish factory-only substitution. LR6/LR7 remain open.

<!-- from: glue-lotrecords4 (2026-10-22) -->
- ~~**LR9 - section 3.1 versus published cISC4Lot slot 4, property-holder view:** section 3.1 places the
  plain property-holder view at lot +8 and the traffic-source view at +0x1C, but `docs/abi/interfaces.tsv`
  (`cISC4Lot`, vtable `00ab19b0`, slot 4, target `006c2050`) describes `AsISCPropertyHolder` as returning
  this +0x1C. Which offset and view does that slot actually return, and are the section 3.1 QI view offsets
  correct? [no runtime view/cast implemented; preserve both published sources until reconciled]~~
  **ANSWERED 2026-10-22 (re-lotrecords-boundary4):** DIFFERS — section 3.1 offsets match native bytes; slot 3 is the borrowed property holder (+8), slot 4 the borrowed traffic source (+0x1C). The old ABI method label is wrong. Constructor writes, exact QI/adjustor bytes and native TrafficSourceIterator caller 0x00710841 prove the distinction. QI accepts only IID 1, lot and Serializable. Sections 3.1/13.2 and T26-T27 specify it; four exact ABI row corrections are proposed under build-runs/pending-abi, awaiting coordinator review/application/gate.

# lot-records: open questions
<!-- Questions about [lot-records.md](../lot-records.md). Append new entries at the END; when one is answered, move it to answered/lot-records.md. Rules: README.md. -->

<!-- from: core-lotrecords4 (2026-10-21) -->
- **LR1 - section 3.5 Init step 1:** which point does `GetCenter` (lot slot 15) return for a lot rect (cell centre
  of the rect in metres?), and which slope does `SetAltitude` (slot 17) store in fields 7 and 8 ("also sets both
  slopes")? [both come from the host: `LotHost::center_altitude_and_slope(rect)`]

<!-- from: core-lotrecords4 (2026-10-21) -->
- **LR2 - section 3.5 SetFlags:** in which order are the rect cells locked / unlocked (x outer or z outer), and is
  the lock call the terrain view's per-cell lock with the lot altitude as given? Observable only through the terrain
  view's call order. [x outer, z inner, x1..=x2, z1..=z2]

<!-- from: core-lotrecords4 (2026-10-21) -->
- **LR3 - section 4 / lot-developer.md section 5.7 Remove:** what does `cISC4BusinessUnit::Remove(lot)` return in
  AL, and does it still call `lot.SetUnit(0)` / `SetTravelDesignate(0)` when the lot is not a member? [true when
  the lot was a member; both setters always]

<!-- from: core-lotrecords4 (2026-10-21) -->
- **LR4 - section 3.2 field 2:** a lot that was read but never `Init`ed (the crime-only lot of section 14) holds no
  configuration object, only the ID read. Does its `Write` save 0 (no object) or the ID read? [the ID read]

<!-- from: core-lotrecords4 (2026-10-21) -->
- **LR5 - section 7 / lot-developer.md section 7.2 AddRetainingWallOccupant:** how does the add path compute the
  wall type, edge mask, the four edge hashes, position and size of the new wall from the lot, and what does
  `UpdateRetainingWalls(lot)` change on an existing wall? Without it a core can read and write walls but not build
  them. [not implemented; record Write/Read and the T7 mask growth only]

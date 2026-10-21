# lot-records: open questions
<!-- Questions about [lot-records.md](../lot-records.md). Append new entries at the END; when one is answered, move it to answered/lot-records.md. Rules: README.md. -->

<!-- from: core-lotrecords4 (2026-10-21) -->
- **LR5 - section 7 / lot-developer.md section 7.2 AddRetainingWallOccupant:** how does the add path compute the
  wall type, edge mask, the four edge hashes, position and size of the new wall from the lot, and what does
  `UpdateRetainingWalls(lot)` change on an existing wall? Without it a core can read and write walls but not build
  them. [not implemented; record Write/Read and the T7 mask growth only]

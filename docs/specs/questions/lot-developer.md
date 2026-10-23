# lot-developer: open questions
<!-- Questions about [lot-developer.md](../lot-developer.md). Append new entries at the END; when one is answered, move it to answered/lot-developer.md. Rules: README.md. -->

<!-- from: From the return-channel audit (2026-10-16) -->
- **LD33:** (lot-developer.md §6.6) cSC4LotBaseTextureOccupant slot 9 UpdateVertexAltitudes(bool, bool, ptr): what does
  it return? §6.6 passes it through as slot 45's AL. [passed through unchanged, as the original does]

<!-- from: From the return-channel audit (2026-10-16) -->
- **LD34:** lot-developer.md §11.1 gives slot 8 AbandonBuilding as "AL of SetState", and lot-records.md §13.2 gives lot
  slot 34 SetState as "none defined". So is AbandonBuilding's result undefined? tract-developer.md's interface table
  lists it as plain AL. [passed through; the tract core never reads it]

<!-- from: core-construction5 (2026-10-23) -->
- **LD35 — §6.7.15:** please specify the explicitly open scaffold mesh, SetScaffoldHeight, CreateCrane model geometry, Draw, DoRotationChange, and inner crane exact position/swing. [Unsupported for these visual operations; inner side/size draws and all outer fit rules implemented]

<!-- from: core-construction5 (2026-10-23) -->
- **LD36 — §6.7.6:** what are the scaffold/completion interpolation endpoints and the milliseconds-per-day divisor, including state 0 and removal state 3? [accumulate saved ms and animate cranes; phase interpolation Unsupported pending endpoints]

<!-- from: core-construction5 (2026-10-23) -->
- **LD37 — §6.7.7 steps 3–4:** which y/anchor does CreateAndPlaceProp use, which initial values/roles do the two saved crew booleans have, and is RandomDirection2 normalized? [literal area minimum y, first crew byte = roof flag, second false; pass the two drawn direction components to the foreign automata service]

<!-- from: core-construction5 (2026-10-23) -->
- **LD38 — §6.7.1/.7/.14:** what are the 6×5 sim-grid IDs and render-state fields; dust/scaffold effect creation parameters; crane saved y; and whether RemoveRectangle clips its four candidate parts to the input area when the cut crosses its boundary? [foreign render preparation/effect creation; crane y zero because CreateCrane accepts x/z; literal four full-extent candidates without clipping]

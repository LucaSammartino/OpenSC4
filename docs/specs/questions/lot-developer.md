# lot-developer: open questions
<!-- Questions about [lot-developer.md](../lot-developer.md). Append new entries at the END; when one is answered, move it to answered/lot-developer.md. Rules: README.md. -->

<!-- from: From the return-channel audit (2026-10-16) -->
- **LD33:** (lot-developer.md §6.6) cSC4LotBaseTextureOccupant slot 9 UpdateVertexAltitudes(bool, bool, ptr): what does
  it return? §6.6 passes it through as slot 45's AL. [passed through unchanged, as the original does]

<!-- from: From the return-channel audit (2026-10-16) -->
- **LD34:** lot-developer.md §11.1 gives slot 8 AbandonBuilding as "AL of SetState", and lot-records.md §13.2 gives lot
  slot 34 SetState as "none defined". So is AbandonBuilding's result undefined? tract-developer.md's interface table
  lists it as plain AL. [passed through; the tract core never reads it]

<!-- from: core-construction5b (2026-10-23) -->
- **LD39 - §6.7.2/§6.7.15 missing texture-scale properties:** the code-default column is “—” for
  sfPlankTextureScale, sfScaffoldTextureScale and sfGirderTextureScale, while missing properties keep
  the code default. What values survive when these properties are absent? [Core keeps the existing
  `None` tuning values and propagates an explicit NaN into dependent UV/texture-transform fields,
  rather than choosing a numerical scale. Supplied scales, including all shipped values, are implemented.]

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

<!-- from: growth-exact34 (2026-10-28) -->
- **G34-LD1 - §§6.3, 6.7.4: construction without a building.** What is the complete public-interface
  lifecycle and persistence contract for a construction occupant whose `SetLot` finds no building?
  §6.3 requires creating and inserting it; §6.7.4 initializes it before returning on the missing building,
  while §6.7.14 says shutdown releases the current effect without a null test. The existing glue safeguard
  in `FC4ConstructionSetup.h` skips that scene before Init. In `qcheck --replace lotdev`,
  `gen27-spandau-s2-default` is clean/ref_warnings=0 but differs in 103 records (89 only relabelled),
  with identical DIGESTs and sampled RNG. The first content difference is the occupant manager.
  The reference has four construction occupants; replacement has three, plus one fewer object in each
  of classes 2977AA47, 4A1628C5 and 6A1628BA. The log reports a refused construction at 03:14:58.982.
  Run: `build-runs/golden-runs/compare-20261028-031440-e64205ad/result`;
  label `golden-27c1ede8a3f940bb8586e7e7cc6007c5`. `--replace none` is exact (qcheck-2).
  Please specify how this partial scene survives native shutdown and what it must save; the safeguard
  was not removed on the basis of save differences alone.

- **G34-LD2 - §§4.1, 5.4, 5.6: lot property creation and visit counts.** Does the nonvirtual
  `RemoveBuildingZotProp(lot, 6)` leave an absent C999C45E property absent, or create a zero-valued one?
  Slot 11 explicitly returns zero without a property, but §5.6 does not state the nonvirtual helper's
  absence behavior. On `ui29-volcano`, lotdev alone replaced differs only in the lot record C9BD5D4A:
  expected size 238079, actual 238699; several lots have an extra 20-byte zero-valued C999C45E property.
  The first lot's payload also differs at offset 122 (reference 0x26, replacement 0x20), so please clarify
  any missing daily-cycle or counter-update behavior before treating property creation as the entire cause.
  Run: `build-runs/golden-runs/compare-20261028-031440-6adb2921/result` (qcheck-1);
  all other 152 records, DIGESTs and sampled RNG agree. `--replace none` is exact (qcheck-2).

# fire-flames: open questions
<!-- Questions about [fire-flames.md](../fire-flames.md). Append new entries at the END; when one is answered, move it to answered/fire-flames.md. Rules: README.md. -->

<!-- from: re-flames (2026-10-20) -->
- **FF1 - §2:** the wall flag of a model point is "unit normal z > 0". With y up, roofs and walls facing -z or +-x
  get top flames (`fire_N`) and only faces towards +z get wall flames (`wfire_N`). Is that what players see, or are
  building meshes stored with z up? A look at a burning building in game would settle it. [as read]

<!-- from: core-fire-flames4 (2026-10-22) -->
- **FF2 - complete owner/body inventory and lifecycle:** the lane brief counts 2 owners / 16 functions / 3513 bytes, but this spec publishes only cSC4Flame::Read, Update, UpdateTransform; the triangle helper; GetPointsOnModel; and the fire occupant's CreateFlames, SetStage, Init, ViewUpdate. Please publish the complete address-to-owner/body inventory, including constructors, destruction, visibility/made-byte transitions, writing, and any other counted bodies. What are a newly constructed flame's made/visible/wall/side values, and does destruction stop an effect before releasing it? disasters §5.11 says Init builds flames, whereas §4 here only updates existing flames; which call order and return/failure behavior apply, including initial stage -1 and stages outside 0..4? [portable core requires explicit initial flame flags; implements only the published visual bodies; no complete-owner claim or invented persistence writer/destructor algorithm]

  **Partial proof 2026-10-22 (re-small-renderer-contracts4):** §§4.1/4.3 establish defaults, Init/stage rules, shutdown versus direct destruction, persistence order and partial failures; table enumerates 21 bodies/3740B including two mislabeled/unnamed owners. §4.4 exactly reconciles 16/3513 as three flame bodies/555 plus thirteen reachability-helper bodies/2958; it excludes the actual sampler and flame constructor, and separately proves seven adjusted QI/destructor/read/write entries. **PARKED after round 3/3:** complete operand/byte contracts for twelve non-triangle helpers and inherited-occupant shared-slot membership beyond those seven wrappers remain; exact helper addresses/sizes are in §4.4. No whole-owner completion or fourth round.

<!-- Remaining: FF1 and FF2. -->

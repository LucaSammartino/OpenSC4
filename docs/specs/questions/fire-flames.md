# fire-flames: open questions
<!-- Questions about [fire-flames.md](../fire-flames.md). Append new entries at the END; when one is answered, move it to answered/fire-flames.md. Rules: README.md. -->

<!-- from: re-flames (2026-10-20) -->
- **FF1 - §2:** the wall flag of a model point is "unit normal z > 0". With y up, roofs and walls facing -z or +-x
  get top flames (`fire_N`) and only faces towards +z get wall flames (`wfire_N`). Is that what players see, or are
  building meshes stored with z up? A look at a burning building in game would settle it. [as read]

<!-- from: core-fire-flames4 (2026-10-22) -->
- **FF2 - complete owner/body inventory and lifecycle:** the lane brief counts 2 owners / 16 functions / 3513 bytes, but this spec publishes only cSC4Flame::Read, Update, UpdateTransform; the triangle helper; GetPointsOnModel; and the fire occupant's CreateFlames, SetStage, Init, ViewUpdate. Please publish the complete address-to-owner/body inventory, including constructors, destruction, visibility/made-byte transitions, writing, and any other counted bodies. What are a newly constructed flame's made/visible/wall/side values, and does destruction stop an effect before releasing it? disasters §5.11 says Init builds flames, whereas §4 here only updates existing flames; which call order and return/failure behavior apply, including initial stage -1 and stages outside 0..4? [portable core requires explicit initial flame flags; implements only the published visual bodies; no complete-owner claim or invented persistence writer/destructor algorithm]

  **Partial proof 2026-10-22 (re-small-renderer-contracts4):** §§4.1/4.3 establish defaults, Init/stage rules, shutdown versus direct destruction, persistence order and partial failures; table enumerates 21 bodies/3740B including two mislabeled/unnamed owners. Residual: reconcile legacy 16/3513 scope and inherited wrappers; no complete inherited-occupant claim.

<!-- from: core-fire-flames4 (2026-10-22) -->
- **FF3 - §2 triangle/index and failure boundaries:** for ceil(k/3), does a final partial triple read the physical indices beyond the first primitive's count, and is the primitive's first-index offset honored? What happens for missing indices/vertices, zero-area triangles, nonfinite coordinates, or an area draw for which no cumulative total compares >= r? Which generator class is clock-seeded here, and what exact float-store/accumulation boundaries apply to area, normal, barycentrics and placement? [literal sequential triples from the first primitive's offset, including readable trailing physical indices; RzRandom and float32 operations; zero-length normal uses IEEE division; malformed data or no selectable triangle produces an explicit portable error, not a claimed native false/fallback]

  **Partial proof 2026-10-22 (re-small-renderer-contracts4):** §2.1 DIFFERS: first-index offset ignored; complete physical triples, unchecked buffers/indices, unchecked zero-length normal; unordered selection accepted and all-ordered-less selection goes one past end; local cRZRandom(-1), u stored f32. Residual: exact area and complete barycentric/placement rounding sequence.

<!-- Remaining: FF1, FF2 and FF3. -->

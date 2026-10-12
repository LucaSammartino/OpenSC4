# Open spec questions

Questions from engine authors that the spec writers need to answer from the original. Answered questions move into
the relevant spec, and their entry here is deleted.

## Flora (flora.md)
- FO1. Is `data3` of InsertOccupant / RemoveOccupant really the occupant type (§15 Q2)? The core assumes so.
- FO2. `BoundingBoxesIntersect` with flags (0, 0): are the boxes closed (touching faces overlap) or open? The core
  uses closed boxes.
- FO3. What flags byte does the key-0 default record really have (the original leaves it uninitialised)? Its §14
  quirk only shows when bit 0 is clear. The core uses 7.
- FO4. `GetMoistureType` with no weather simulator: does the original crash (no null check)?
- FO5. Does `period = 0` really fault (division by zero)? The core gives `blocksPerMonth = 0`.
- FO6. Do null corners in `GetOccupantsByStandardCityCells` mean the whole city (§15 Q5)?
- FO7. Is the terrain tolerance 0.001 a `double` or a `float` constant? The core compares it as a `double`.

## Aura (aura.md)
- A8. Does the aura's `Read` clear (or replace) the department map and the hotspot list before reading, or append?
  The core replaces both, which is identical on a fresh object.

## Pollution (pollution.md)
- PO5. The spec names the `1.5 × 2^23` round-half-even trick only for the deal amounts. Do the other
  `roundHalfEven` sites (§4.4.4, Dispose, slot 53, §4.11) use it too? The core rounds the f32 exactly there (the
  two agree below 2^22).

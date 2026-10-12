# Open spec questions

Questions from engine authors that the spec writers need to answer from the original. Answered questions move into
the relevant spec, and their entry here is deleted.

## Flora (flora.md)
- FO1. Is data3 of the Insert/RemoveOccupant messages the occupant type?
- FO2. Is `BoundingBoxesIntersect` inclusive at the edges (the (0,0) case)?
- FO3. The flags byte of the default record.
- FO4. `GetMoistureType` when there is no weather simulator.
- FO5. Behaviour with period = 0.
- FO6. Null corner arguments in `GetOccupantsByStandardCityCells`.
- FO7. Is the 0.001 constant `float` or `double`?

## Aura (aura.md)
- A8. Does `Read` clear or replace the department map and the hotspot list, or append to them?

## Pollution (pollution.md)
- PO5. Do the other round-half-even sites also use the `1.5 · 2^23` trick (and so share its range limit)?

## Crime (crime.md)
- C9. §16 question 4: the order of the crime-type list.

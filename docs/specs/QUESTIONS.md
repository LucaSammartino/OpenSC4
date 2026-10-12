# Open spec questions

Questions from engine authors that the spec writers need to answer from the original. Answered questions move into
the relevant spec, and their entry here is deleted.

## Pollution (pollution.md)
- PO1. Are `0.1` (deal amounts, §4.4) and `0.01` (efficiency, §4.7, and slot 55) `float` or `double` literals?
  Under foundations §18 this decides whether the constant is rounded to `float32` first. The implementation
  takes them as `float`. (1/32767 is answered: a `float32` multiply.)
- PO2. The exact algorithm of `SetOccupantListUsedCapacity` (§4.9.3, the spec's own question 4). The
  implementation uses its own distribution.
- PO3. Should a failed `Read` release the current department first? The spec says release-then-read; the
  implementation leaves the simulator unchanged on failure.
- PO4. `GetPollutionGrid(4+)`: can the original ever return a grid there? The implementation returns none.

## Crime (crime.md)
- C1. §6.9 uses the tuning's tract size, which differs from the grid's after a tract-size reload. Which one does
  the original use? The implementation uses the tuning's.
- C2. What is property `0xAA12FCB0`? The implementation uses it as a fallback string for the type name.
- C3. The icon instances of the shipped crime types are not in the spec. The implementation uses 0.
- C4. Does `rhe` ever receive values above 2^22 (where the `1.5 × 2^23` trick stops being exact)? The
  implementation saturates to i32.
- C5. In `SelectProp`, should a cell with no lot match incidents that have no lot? The implementation says yes.
- C6. Is `loadedFromSave` set when `Read` fails on a stream error? The implementation sets it.
- C7. Behaviour with no ordinance simulator. The implementation leaves the factor unchanged.
- C8. The operation order of the §11.3 distance. The implementation computes `sqrt((dx² + dy²) + dz²)` in f32.

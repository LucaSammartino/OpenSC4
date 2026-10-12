# Open spec questions

Questions from engine authors that the spec writers need to answer from the original. Answered questions move into
the relevant spec, and their entry here is deleted.

## Aura (aura.md)
- A1. Test 14 (cadence): the implementation runs one pipeline step in SimNewDay before it compares, so state 3
  becomes 4 on day D + 5, and 4 < 5 gives false. Is that the intended reading?
- A2. State 5 step adaptation (§6.5): is `n = lots processed this call / 1024` the original's divisor, and is the
  step adapted at the end of every state-5 call? The implementation keeps the step per simulator, not
  process-wide (§6.5 says shared *[inferred]*).
- A3. With no Parks department the original dereferences null. The implementation uses funding 1.0. Which
  behaviour should a replacement match?
- A4. Is an empty curve (no points) treated like an absent one? The implementation treats a curve stored as a
  single scalar as absent.
- A5. Overflow when storing into the int16 grids: wrap, saturate, or something else? The implementation
  saturates and stores NaN as 0.
- A6. The population grids are assumed to have the aura grid's geometry. Confirm, or give the mapping.
- A7. Are the riot coordinates (§6.16) cells or tracts? The implementation uses cells.

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

## Flammability (flammability.md)
- F1. The prop/flora erratum is not applied everywhere. §1 still says "buildings first, else flora, else props";
  §14.3 (the add path) and the labels of test 2 also use the old names. The implementation follows the numeric
  IDs (prop `0xA823821E`, flora `0x74758926`, priority building > prop > flora).
- F2. Message `0xC53D10AA` is handled by the original but not subscribed. Should it be subscribed?
- F3. A curve stored as a single value: is it a constant curve or absent?
- F4. Should `Read` append to the hotspot list or replace it?
- F5. What happens when the plumbing simulator or the zone manager is missing?
- F6. The integer conversion of occupant property `0x29244DB5`. The implementation reads the first element and
  truncates floats toward zero.
- F7. §15 question 3: is a removed occupant still listed when RemoveOccupant recomputes the tract?

# Open spec questions

Questions from engine authors that the spec writers need to answer from the original. Answered questions move into
the relevant spec, and their entry here is deleted.

## Demand (demand.md)
- D1. Format of the "embedded nodes" in the save (the spec's own open question 1).
- D2. Code defaults for the nation economy exemplar when it is missing.

## Ordinance (ordinance.md)
- O1. Recycling-center effect: the SDK says `int32`, but the spec's `× 100 > 0` reads as a fraction. Which type,
  and which units?
- O2. Keying of ordinance entries after a load. The implementation keys by class ID at Read, then re-keys by
  GetID at Init. Confirm.

## Weather (weather.md)
- W1. Encoding of the cell iterator's state byte in the save. The implementation chose 0 never reset,
  1 running, 2 exhausted.
- W2. Order and precision of the moisture height filter, and the edge reflection for non-square grids.
- W3. What the saved Gaussian cache word holds when the "has cached" flag is 0.
- W4. windSpread is not saved, so a loaded simulator draws a different wind until Init re-reads the tuning. Is
  that what the original does?

## Plugin diagnostics
- P1. Month digest: what should "C and I population" mean? The implementation uses summed commercial/industrial
  supply (jobs) from the demand nodes. Check the demand IDs it uses against the original.

# Open spec questions

Questions from engine authors that the spec writers need to answer from the original. Answered questions move into
the relevant spec, and their entry here is deleted.

## Power (power.md)
- PW1. Test 5 expects 449; computed under foundations §18 (one f32 rounding per operation) the core gets 450.
  Which is right, and if 449, which operation is not 24-bit?
- PW2. §6.3 step 1 "reset an inverted range": reset to what (the empty range, or the full map)?
- PW3. Are `AddLocallyFundedObject`, the secondary info and the base cost all inside the `if line ≠ 0` branch?
- PW4. Which department do `SetLocalFullFunding` and `IsLocallyFundedObjectInDepartment` use: the item's or the
  power department?
- PW5. Are the plant/consumer counters signed or unsigned (matters for the slot return values on underflow)?
- PW6. Where does the §5.4 "poles in rect" list come from (an occupant-manager query, or the simulator's pole list)?
- PW7. Is `DemolishLots` called at all when there are no nuclear plants (Nuclear Free Zone)?
- PW8. Is the tuning `0.01` a `float` or a `double` literal?
- PW9. What does `Read` do on a stream error part-way (keep partial state, roll back, release references)?
- PW10. When a removed pole was not in the pole list, do `costBase` and the recompute request still change?
- PW11. Does `Init` look the department up again after a load?
- PW12. Is the unpowered-percentage computation `double` or 24-bit?
- PW13. §6.4 says a NaN ratio gives `used = 0x80000000` (and the core's `ftol32` returns `0x80000000` for NaN and
  out-of-range values), but foundations §18.1 now says `_ftol` truncates to `int64`, so NaN keeps the low 32 bits
  **0**. Is this truncation (and power's other `trunc` sites) an inline 32-bit `fistp` or `_ftol`?

## Plumbing (plumbing.md)
- PL1. NearestWater's ring at clipped map edges: Chebyshev rings clipped to the map, or skipped rings?
- PL2. `Shutdown` does not clear the grids-initialized flag, so a second `Init` finds no grids. Is that right?
- PL3. Can state 3 ever see a current producer already set? The implementation floods from it.
- PL4. After state 7 the iterator ends as Done at (0, 63): exactly how does state 7 iterate (rectangle, step,
  end condition)?
- PL5. Bitmap word count `(w·h)/32` and bit `x & 31` for city sizes that are not multiples of 32: some cells cannot
  be marked and some share a bit. Confirm this is the original layout.
- PL6. Max funding computes as `f32(120 × 0.01f) = 1.1999999`; the spec says 1.2. `float` or `double` 0.01?
- PL7. The export deal gets flags `0x28` when the city is not connected: confirm.

## Protection (protection.md)
- PR1. `GetMaxPossibleDispatchCount` is not described. The implementation returns `GetTotalDispatchCount`.
- PR2. The order of the station-effect property reads, and what happens when one fails part-way. The
  implementation reads centre, outer, radius, position and does not stop.
- PR3. Are the constants `0.01` and `100` `double` or `float`? The implementation takes them as `double`.
- PR4. Which tract size S does the "zoned" check use (grid or tuning)? The implementation uses the grid's,
  clamped to 1..64.
- PR5. What exactly does "the item's department exists" mean (non-null item department, or lookup succeeds)?
- PR6. What does `GetStrikingStations` return? The implementation returns the number appended.
- PR7. The order of the nine message subscriptions. The implementation uses the §5 table order.
- PR8. The exact condition for removing a jail from the locally funded list.
- PR9. The layout of save records older than version 3.
- PR10. Behaviour with a huge `nextCall` (the spec's Q6). The implementation saturates.
- PR11. Are the cycle averages integer or float divisions? The implementation divides as float.
- PR12. A station statistics vector shorter than 4 elements. The implementation pads with zeros.
- PR13. Does a police re-read release the old helicopter source?
- PR14. An absent jail capacity property. The implementation reads 0.
- PR15. A demolished jail stuck in a jailbreak: with capacity 0 the jailed fraction is NaN and `NaN < 0.8` is
  false, so it may never leave the list. Confirm.

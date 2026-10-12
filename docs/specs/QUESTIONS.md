# Open spec questions

Questions from engine authors that the spec writers need to answer from the original. Answered questions move into
the relevant spec, and their entry here is deleted.

## Power (power.md)
- PW2. §6.3 step 1 "reset an inverted range": reset to what (the empty range, or the full map)?
- PW3. Are `AddLocallyFundedObject`, the secondary info and the base cost all inside the `if line ≠ 0` branch?
- PW4. Which department do `SetLocalFullFunding` and `IsLocallyFundedObjectInDepartment` use: the item's or the
  power department?
- PW5. Are the plant/consumer counters signed or unsigned (matters for the slot return values on underflow)?
- PW6. Where does the §5.4 "poles in rect" list come from (an occupant-manager query, or the simulator's pole list)?
- PW7. Is `DemolishLots` called at all when there are no nuclear plants (Nuclear Free Zone)?
- PW9. What does `Read` do on a stream error part-way (keep partial state, roll back, release references)?
- PW10. When a removed pole was not in the pole list, do `costBase` and the recompute request still change?
- PW11. Does `Init` look the department up again after a load?
- PW12. Is the unpowered-percentage computation `double` or 24-bit?

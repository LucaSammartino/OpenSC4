# Open spec questions

Questions from engine authors that the spec writers need to answer from the original. Answered questions move into
the relevant spec, and their entry here is deleted.

## Power (power.md)
- PW14. §15 test 5: the exact product 500 × 0.9f is 449.99998808 (0.9f = 0.89999998), not 450.0000067; it still
  rounds to 450.0 at 24 bits. Please correct the figure.

## Protection (protection.md)
- PR16. §18 test 23 ("returns 900 + unarrested") holds only if no month rehabilitates in between. Should the test fix
  the rehabilitation rate to 0? (Vector pr18_23 does.)
- PR17. §18 test 24 / foundations §4.3: what does a response curve return for a NaN input? The core returns the last y,
  so "a NaN jailbreak chance makes no random draw" is only reachable through the pure start test.
- PR18. §6.6 `SetStationFunding`: which department is used, the item's or the simulator's own? (The core uses its own.)

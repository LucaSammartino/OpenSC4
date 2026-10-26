# traffic-simulator: open questions
<!-- Open questions about docs/specs/traffic-simulator.md; answered entries move to answered/traffic-simulator.md. -->

## TC29-STATS-1: state-2 entry and per-switch statistics

Blocked case: `berlin-spandau` in census `sh28-census3`, 2001-09-23, state 15 -> 2,
working-statistics record first difference at serialized offset 332, 47/544 bytes.
Original begins `0000000000000000`, copy `120002722F3E5B00`; the difference recurs
on 2001-11-07 and 2001-12-07 when entering state 2 (11/544 bytes each).
Evidence: `build-runs/run7/census3/berlin-spandau.shadows.txt` in the main checkout.
Offsets 314..529 are the twelve `u16,f32,s32,s32,f32` switch records of §6.1 row 16.
§3.2/§4.9 specifies resetting the network rows and record counts, but does not name
these switch fields; save-records §12 Q2 requires preserving them until specified.
What are their meanings, entry resets, accumulation, and publication rules? Are
their values valid immediately after load, or reconstructed by a later phase?
Please include a worked state-2 entry with a nonzero loaded switch record.

## TC29-STATS-2: published statistics trailing floats between deliveries

Blocked case: `berlin-spandau`, 2001-10-06, state 16 -> 17, and `berlin-tegel`,
2029-09-04, state 18 -> 18: the *pre-delivery* record check finds 4/544 bytes
different in published statistics, first at offset 532; original context
`803F0000803F0000`, copy `C8420000C8420000`. Evidence: the corresponding census
`.shadows.txt` files under `build-runs/run7/census3/` in the main checkout.
These are the two unnamed floats at serialized offsets 530 and 534 (§6.1 row 16).
Which operations update them, with which units and inputs, and when? Can a public
getter mutate this published block? The copy currently preserves the saved values
as required by save-records §12 Q2; neither comparison nor tolerance is relaxed.

# Protection implementation questions

## PR-MAPS30-01 — Census coverage differs from a fresh cone rebuild

Open; maps-cities30; 2026-10-26. Please provide a conformance vector or clarify
the historical inputs needed to reproduce these loaded coverage grids.
Sections 4 and 11 retain saved grids, whereas the current month shadow
rebuilds them from live stations (§5.1). I cannot establish from the present
evidence whether the differences are retained saved state or an arithmetic
error in the fresh rebuild; no comparison was weakened or skipped.

Evidence in `build-runs/suite/sh28-census3/`:
- `maxisland-darris.log:3826`, 2097-05-01, police: eight cells differ by 1,
  including (12,37) original/ours 33/32; efficiency and dispatch radius agree.
- `maxisland-indereast.log:3875`, 2086-07-01, fire: 919/1024 cells differ,
  max difference 79; one station, no dispatches; efficiency and dispatch
  radius agree. The accompanying unadjusted comparison also differs.
- `maxisland-lamburg-villa.shadows.txt`, 2051-08-01, police: adjusted 334
  cells and unadjusted 363 cells differ by 1; one station, no dispatches.

The existing shadow runs simgrid §7.1's table/exact-distance rule and the
specified f32 multiply, subtraction and half-even rounding. Please identify
the necessary same-call inputs/comparison boundary before changing those rules.

# Protection implementation questions

## PR-MAPS30-01 — Census coverage differs from a fresh cone rebuild

Open; maps-cities30; 2026-10-26. Value-only request for §§5.1, 7.1 and 11;
the affected implementation item is stopped pending these values.

Evidence in `build-runs/suite/sh28-census3/`:
- `maxisland-darris.log:3826`, 2097-05-01, police: eight cells differ by 1,
  including (12,37) original/ours 33/32; efficiency and dispatch radius agree.
- `maxisland-indereast.log:3875`, 2086-07-01, fire: 919/1024 cells differ,
  max difference 79; one station, no dispatches; efficiency and dispatch
  radius agree. The accompanying unadjusted comparison also differs.
- `maxisland-lamburg-villa.shadows.txt`, 2051-08-01, police: adjusted 334
  cells and unadjusted 363 cells differ by 1; one station, no dispatches.

For each sample above, return a numeric tuple for its first differing cell:
`(cell_x, cell_z, int16_after_Read, UpdateUnadjusted_calls_since_Read,
int16_after_last_UpdateUnadjusted, int16_at_sample)`.
For the last rebuild, also return each station's input tuple in list order:
`(tract_x, tract_z, radius_in_tracts_f32_bits, centre_f32_bits, outer_f32_bits)`
and the cone contribution at that cell as an int16. Return the x87 control
word at rebuild entry as uint16 hex. Use uint32 hex for f32
bits and decimal for integers; if no rebuild ran, return count 0 and omit
its tuples. For Lamburg Villa use the first cell listed in the cited file.

This distinguishes retained saved values from a fresh rebuild using values
alone. The existing shadow follows simgrid §7.1's table/exact-distance rule
and the specified f32 operations. Comparisons and arithmetic remain strict.

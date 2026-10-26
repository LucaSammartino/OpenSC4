# Power implementation questions

## PW-MAPS30-01 — AgePowerPlant arithmetic in the West Weburg census

Open; maps-cities30; 2026-10-26. Value-only conformance-vector request for
§7.4; the affected implementation item is stopped pending these values.

Evidence: `build-runs/suite/sh28-census3/maxisland-west-weburg.log:4597`,
2197-05-04, state 1: property 0xC910B4CF is original/ours
0.32092583179473877/0.3209258019924164 and
0.04157695174217224/0.04157694801688194 for two plants. The cycle's grids,
production, consumption, requirement, powered/unpowered counts and other
compared properties agree. The same two value pairs recur in
`build-runs/suite/maps-cities30-4/maxisland-west-weburg.log:6301`, 2197-05-05.

For each of those two plants, return the same-call numeric tuple:
`(x87_control_word_u16, wear_before_bits, rate_bits, capacity, efficiency_bits, used, effCap,
usage_bits, usage_curve_bits, department_funding_bits, funding_curve_bits,
curve_product_bits, age_increment_bits, wear_after_bits)`.
Here `curve_product` is the product of the two curve results and
`age_increment` includes the rate. Use uint32 hex for all f32 bit patterns
and uint16 hex for the entry control word. Use signed decimal for capacity,
used and effCap; identify each tuple by
its observed wear_after value above. No addresses or implementation are
requested. No tolerance or comparison change was made.

## PW-MAPS30-02 — Indereast shortfall allocation numeric witness

Open; maps-cities30; 2026-10-26. Value-only request for §§6.3–6.4;
the affected implementation item is stopped pending a numeric vector.

Evidence: `build-runs/suite/maps-cities30-5/maxisland-indereast.log:6681`,
2086-08-05, state 1: one order-dependent flood, no iterator start reproduced
the original's powered-next mask (460 cells not exact; next powered count
original/ours 331/321). The published grid and counts agree. One wear value
also differs: original/ours 0.16816721856594086/0.16816723346710205; this
witness does not establish that its cause is the same as PW-MAPS30-01.

For that original flood, return the numeric input tuple
`(seed_x, seed_z, rowMin, rowMax, supply, imp, total, factor_f32_bits,
avail_after_exports, rand_return, iterator_start, iterator_step, block_count)`
and the ordered block rows `(bx, bz, need, n, avail_before, powered,
avail_after)`, with powered as 0 or 1. Include the entry mark and exit
powered-next bitmaps as 64 rows of 64 bits. For the plant identified by the
wear value above, also return the PW-MAPS30-01 ageing tuple. Use uint32 hex
for f32 bits and decimal integers otherwise; values only, no implementation.

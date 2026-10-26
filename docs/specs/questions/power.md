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
`(wear_before_bits, rate_bits, capacity, efficiency_bits, used, effCap,
usage_bits, usage_curve_bits, department_funding_bits, funding_curve_bits,
curve_product_bits, age_increment_bits, wear_after_bits)`.
Here `curve_product` is the product of the two curve results and
`age_increment` includes the rate. Use uint32 hex for all f32 bit patterns
and signed decimal for capacity, used and effCap; identify each tuple by
its observed wear_after value above. No addresses or implementation are
requested. No tolerance or comparison change was made.

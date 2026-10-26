# Power implementation questions

## PW-MAPS30-01 — AgePowerPlant arithmetic in the West Weburg census

Open; maps-cities30; 2026-10-26. Please supply an arithmetic-order conformance
vector for §7.4 when the plant is not using its full effective capacity.
The implementation follows `wear + usageCurve(used / trunc(capacity * efficiency))
* fundingCurve(departmentFunding) * rate`, rounding each operation to f32 as
foundations §18.3 specifies. I cannot explain the remaining difference from
the published specification and have left this item unchanged.

Evidence: `build-runs/suite/sh28-census3/maxisland-west-weburg.log:4597`,
2197-05-04, state 1: property 0xC910B4CF is original/ours
0.32092583179473877/0.3209258019924164 and
0.04157695174217224/0.04157694801688194 for two plants. The cycle's grids,
production, consumption, requirement, powered/unpowered counts and other
compared properties agree. No tolerance or comparison change was made.

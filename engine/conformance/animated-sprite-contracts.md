# Qualified animated sprite contracts

Source: public animated-sprite.md §§4.4,4.1,5.1,5.2,6.
Executable independent assertions live in core/src/animated_sprite/contract_tests.rs.
Native/game execution: UNEXECUTED. These are portable algorithm tests.

| Test group | Independent expectations |
|---|---|
| Setters | -100/-200, threshold bits 3F7D70A3/A4/A5, -1/1, quiet NaN, off sentinels/NaN; full highlight 0/255/256/repeat; raw self-lit 0/1/2/80; unrelated state/cache preservation |
| Geometry | Literal Q1/Q2/Q3/Q4/Q5 corners and boxes; Q4 uses returned endpoints -8/32 and context width 20; shrink, omitted lower corners, unsymmetrized X minimum |
| Decoder | Public raw bytes [2,6,48,32,16,-16,-3,4], frame 5, scale 2; literal page/metrics/UVs |
| Cache | Mpp/camera-only staleness, frame-only metric refresh, view dirty rebuild, byte counter equality/wrap and change, full zoom 258 and rotation 256 repeat refresh |
| Protocol | Incoming highlight pass/signed zoom early return; stored highlight ignored; exact ordered primitives/arguments/stride; ignored false callbacks |
| Pick | Stale translated positions, triangles 012 then 230, first-hit short circuit, actual second boolean |
| Boundaries | Inexact division, tiny input, zero width, malformed packing, holder 5 and unsupported rotation return explicit errors |

No expected values are calculated by production helpers or native output.
R2-R ideal orientation vectors remain qualified geometric UNRUN, and the 72
R3 conditional raw trig-stage examples are neither angle-derived native
goldens nor claimed as owner execution. Historical 29/4306/67 counts are not
current validation. This does not reconcile the 60/10482 owner catalogue.

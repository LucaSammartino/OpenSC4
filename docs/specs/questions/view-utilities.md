# view-utilities: open questions
<!-- Questions about [view-utilities.md](../view-utilities.md). Append new entries at the END; when one is answered, move it to answered/view-utilities.md. Rules: README.md. -->

<!-- from: core-view-utilities-complete4 (2026-10-22) -->
- **VU12 — native numeric mode and remaining stores:** §2.2 establishes inherited x87 precision/rounding and known camera stores, but does not give the call-site control word or the complete operation/store graph for coordinate projection, corners, predicates and transforms (only the first default-kind vertex is detailed). Which precision/rounding mode is the supported native contract, and what are the remaining store boundaries/order, including default-kind vertices two/three and length-to-extent normalization? [Portable implementation names its nearest/even f64/libm evaluation policy and known stores explicitly; no x87 or native bit identity/full-owner numeric completion is claimed. Geometry/order semantics and checked UB boundaries are implemented independently.]

<!-- from: core-view-utilities-complete4 (2026-10-22) -->
- **VU13 — generic numeric index conversion outside bias range:** §12 describes formats 6/7 as floor via bias plus exact-integer correction, with format7 narrowed to f32, but §4.4 shows bias conversion is not floor outside its unit-spacing range. What exact expression, comparison/correction, wrapping and nonfinite behavior does the generic decoder use at ties, large magnitudes and NaNs? [Checked decoder implements finite values in [-2^22,2^22), preserving outputs and returning None outside that explicit precondition; all integer formats and unknown-format preservation are implemented. No exact whole-body credit for 0080B090.]


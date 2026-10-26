# view-utilities: open questions
<!-- Questions about [view-utilities.md](../view-utilities.md). Append new entries at the END; when one is answered, move it to answered/view-utilities.md. Rules: README.md. -->

<!-- from: core-view-utilities-complete4 (2026-10-22) -->
- **VU13 — generic numeric index conversion outside bias range:** §12 describes formats 6/7 as floor via bias plus exact-integer correction, with format7 narrowed to f32, but §4.4 shows bias conversion is not floor outside its unit-spacing range. What exact expression, comparison/correction, wrapping and nonfinite behavior does the generic decoder use at ties, large magnitudes and NaNs? [Checked decoder implements finite values in [-2^22,2^22), preserving outputs and returning None outside that explicit precondition; all integer formats and unknown-format preservation are implemented. No exact whole-body credit for 0080B090.]

<!-- from: glue-viewutil-apply12 (2026-10-24) -->
- **VU19 — object-angle zero sign:** §2.1 gives −rotation × π/2, but does not specify whether negation is applied to the integer input or floating result. Stored-float32 native GetObjectYRotationAngle(0) is 0x80000000, while the current portable integer-product adapter gives 0x00000000 (sample 16, every month of clean run viewutil12-r1-turbo-20261024a to 2032). Please specify the zero sign and operation/store order for the full signed-int32 domain. [Retain the existing portable arithmetic until clarified; expose the signed-zero difference without masking it. This body remains default-off and is not credited exact.]

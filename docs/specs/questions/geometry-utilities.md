# geometry-utilities: open questions
<!-- Questions about [geometry-utilities.md](../geometry-utilities.md). Append new entries at the END; when one is answered, move it to answered/geometry-utilities.md. Rules: README.md. -->

<!-- from: re-geometry (2026-10-20) -->
- **GU1 - §4.4:** Does any caller of `RaysIntersect2D` pass a null output, so that the direction-difference test is ever used? The polygon clipper always passes one; the eight calls in `0x00752FC3`-`0x0075530A` were not traced. [the rule is specified as read either way]

<!-- from: core-geometry-utilities4 (2026-10-22) -->
- **GU2 - §§1–6 numeric sequence:** Please publish the operation association and float-store boundaries for the formulas, including dot/cross products, normalisation, trigonometry, mesh lighting and byte conversion. Are constants such as 0.3/0.7 double or float operands, and which comparisons retain extended temporaries? What are the NaN/overflow conversion rules for the colour integer? [Core uses left-associated expressions, separate 24-bit operations through the existing foundation arithmetic, f32 constants/stores, and nearest-even colour conversion; nonfinite/out-of-i32 colour conversions use the x87 indefinite integer. No exact x87/extreme-input credit claimed until confirmed.]

<!-- from: core-geometry-utilities4 (2026-10-22) -->
- **GU3 - §6.1 builder lifecycle:** Please specify Reset's changed flag, array retention and bounds state; BeginPrimitive/EndPrimitive behaviour when another primitive is open or none is open; whether AddVertex requires BeginPrimitive and updates its count; when setters mark changed; empty Mesh bounds; destructor/resource ownership. [Reset retains allocated arrays, empties bounds and marks changed; only BeginPrimitive/AddVertex/EndPrimitive mark changed; EndPrimitive closes the last primitive if present; AddVertex appends without implicitly closing or creating a primitive. Rust owns and drops its arrays. These choices remain provisional.]

<!-- from: core-geometry-utilities4 (2026-10-22) -->
- **GU4 - §6.4 text texture failures:** Please specify size/rect mutations on failed surface creation/upload, font-mode restoration timing on each failure, surface/font ownership and release order, text width/height integer types, dimension overflow/negative-size handling, and any surface row pitch. [Pure core requests a packed BGRA surface through a host trait, restores font mode after drawing/on surface failure, leaves outputs untouched before upload, writes size/rect after upload even when upload fails, and rejects unrepresentable dimensions. Actual font/surface ownership is delegated to the host; no native lifecycle claim.]

<!-- from: core-geometry-utilities4 (2026-10-22) -->
- **GU5 - §§4.6, 5.2, 6.1 unordered/capacity cases:** What exact operands win min/max ties and NaN comparisons in overlap outputs and builder bounds? What happens when the initial polygon or an intermediate clipped polygon exceeds the 40-point scratch capacity, and when max exceeds 40? [Core uses ordered conditional selections retaining the first bound on ties/unordered comparisons; clipping uses dynamically sized safe scratch storage with the published 40-point intermediate capacity, avoiding native out-of-bounds behaviour.]

<!-- from: core-geometry-utilities4 (2026-10-22) -->
- **GU6 - complete owner inventory:** The lane assignment cites 37 functions/7509 bytes and transforms/sorting, but the published geometry spec lists 20 geometry functions, builder methods/helpers, and four nS3D helpers, with no transform or sorting contracts or complete owner inventory. Please publish the precise 37-entry owner/method checklist (including any destructors, bounds/corner helpers and primitive closure) and any omitted contracts. [Implement only the named published behaviours; do not invent transform/sort methods or claim reconciled 37/37 ownership.]

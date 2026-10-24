# map-view: open questions
<!-- Questions about [map-view.md](../map-view.md). Append new entries at the END; when one is answered, move it to answered/map-view.md. Rules: README.md. -->

<!-- from: core-map-view-close12 (2026-10-24) -->
- **QV15 - §4.3 lot mean missing prose:** the Lot mask and crime icons subsection starts with
  "of a bad lot rectangle"; the promised mask/mean rule paragraph is absent. Vector 24 fixes type-bit
  masking, minimum one and the 1/2 half case, but does not distinguish ceil(mean) from nearest(mean)
  for means such as 4/3. What is the exact numerator/rounding rule and rectangle coordinate domain?
  [literal inclusive cell rectangle, nearest integer with positive halves up and minimum one;
  vector 24 implemented; general lot rounding remains bounded pending the missing contract]


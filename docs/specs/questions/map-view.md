# map-view: open questions
<!-- Questions about [map-view.md](../map-view.md). Append new entries at the END; when one is answered, move it to answered/map-view.md. Rules: README.md. -->

<!-- from: re-mapview (2026-10-20) -->
- **QV2 - §4.1:** source 76 (traffic volume): which grid it draws for the kept travel types (`0x6B7C5CDD`) and how.
  [as read]
- **QV3 - §1, §3.2:** the panels `0x8A2871B1`-`0x8A2871C4`, `0xAA32BCD4`/`0xAA32BCD5`; the groups' remembered views
  behind the buttons `0x5005`, `0x5008` and `0x8002`; the toggle `+0x9AC` of `0x8100`/`0x8101`; the command
  `0x6A935CF1`. [as read]

<!-- from: core-map-view5b (2026-10-23) -->
- **QV14 - §5.2: ring intermediate precision:** which x87 precision-control and rounding-control modes
  apply to the ring helper, and at which operations (including square roots) are intermediates rounded?
  Does the stated formula require 24-, 53- or 64-bit significands between its explicit binary32 stores?
  Please add a near-threshold pixel vector that discriminates these modes. [literal formula implemented
  with the explicit binary32 stores and binary64 intermediates; §10.15-16 pass; this is a portable numeric
  choice, not a claim of native edge-pixel/weight equivalence for all coordinates; invalid domains remain QV10]

<!-- from: core-map-view-close12 (2026-10-24) -->
- **QV15 - §4.3 lot mean missing prose:** the Lot mask and crime icons subsection starts with
  "of a bad lot rectangle"; the promised mask/mean rule paragraph is absent. Vector 24 fixes type-bit
  masking, minimum one and the 1/2 half case, but does not distinguish ceil(mean) from nearest(mean)
  for means such as 4/3. What is the exact numerator/rounding rule and rectangle coordinate domain?
  [literal inclusive cell rectangle, nearest integer with positive halves up and minimum one;
  vector 24 implemented; general lot rounding remains bounded pending the missing contract]


# map-view: open questions
<!-- Questions about [map-view.md](../map-view.md). Append new entries at the END; when one is answered, move it to answered/map-view.md. Rules: README.md. -->

<!-- from: core-map-view-close12 (2026-10-24) -->
- **QV15 - §4.3 lot mean missing prose:** the Lot mask and crime icons subsection starts with
  "of a bad lot rectangle"; the promised mask/mean rule paragraph is absent. Vector 24 fixes type-bit
  masking, minimum one and the 1/2 half case, but does not distinguish ceil(mean) from nearest(mean)
  for means such as 4/3. What is the exact numerator/rounding rule and rectangle coordinate domain?
  [literal inclusive cell rectangle, nearest integer with positive halves up and minimum one;
  vector 24 implemented; general lot rounding remains bounded pending the missing contract]

<!-- from: drawshadow29 (2026-10-26) -->
- **DS29-1 - Same-invocation MapView snapshot/interface binding:** reach selected this owner
  (house: 5,574 reached bytes; gen27: 15,665, including 24 Update entries; other candidates: zero).
  For Update `0x007A2F60` and both DoMessage entries `0x007A54D0`/`0x007A56E0`, please publish
  the complete native-state field offsets/types and subobject adjustments needed to initialize
  the Rust owner and compare every changed field: selection/source/layer, dirty and last clock,
  palettes, highlight/traffic state, retained collaborators and ordered pending view requests.
  The spec publishes some individual offsets, but docs/abi/layouts.md has no MapView record.
  Also identify the bitmap surface/presentation interface boundary for Update's raster output;
  section 4 describes bitmap drawing rather than calls on a widget's cGZDrawContext.
  No pilot comparison or complete method credit is claimed pending this binding; native drawing
  continues unchanged. This requests interface facts, not original implementation code.


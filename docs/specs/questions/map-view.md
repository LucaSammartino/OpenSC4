# map-view: open questions
<!-- Questions about [map-view.md](../map-view.md). Append new entries at the END; when one is answered, move it to answered/map-view.md. Rules: README.md. -->

<!-- from: re-mapview (2026-10-20) -->
- **QV1 - §2, §3.1:** what the terrain (slots 84, 86), zone manager (slots 17, 18) and `0x4A0B47E6` calls display.
  (The 3D view filter a data view installs is occupant-manager.md §13.2.8, re-occfilters 2026-10-21.) [as read]
- **QV2 - §4.1:** source 76 (traffic volume): which grid it draws for the kept travel types (`0x6B7C5CDD`) and how.
  [as read]
- **QV3 - §1, §3.2:** the panels `0x8A2871B1`-`0x8A2871C4`, `0xAA32BCD4`/`0xAA32BCD5`; the groups' remembered views
  behind the buttons `0x5005`, `0x5008` and `0x8002`; the toggle `+0x9AC` of `0x8100`/`0x8101`; the command
  `0x6A935CF1`. [as read]

<!-- from: core-map-view4 (2026-10-23) -->
- **QV5 - §4.2, §10: contradictory 16-bit conversions:** at scale 1024 the written signed factor is
  floor(0x7FFF00 / 1024) = 8191, and (-512 * 8191) arithmetic-shifted right 16 is -64, hence index 64;
  §10 instead requires 65. At scale 256 the written unsigned/signed factors are 65535/32767, whereas the prose
  says the default factors are 256 and unsigned values are divided by 256. Which formulas, default factors and
  negative rounding rule are authoritative? [both 16-bit conversions Unsupported; no replacement convention]

<!-- from: core-map-view4 (2026-10-23) -->
- **QV11 - §4.1: developer layers:** which raw grid does the documented default layer 1 select? Are layers 3
  and 4 distinguished beyond both selecting slot 13, and what is the layout/geometry/iteration rule for the
  slot-11 entries used to construct the layer-6 debug grid multiplied by 12? [default/unknown layers refused;
  preserve selected layer 3/4 in the raw request; debug selects layer 6 but does not construct its grid]

<!-- from: core-map-view5b (2026-10-23) -->
- **QV14 - §5.2: ring intermediate precision:** which x87 precision-control and rounding-control modes
  apply to the ring helper, and at which operations (including square roots) are intermediates rounded?
  Does the stated formula require 24-, 53- or 64-bit significands between its explicit binary32 stores?
  Please add a near-threshold pixel vector that discriminates these modes. [literal formula implemented
  with the explicit binary32 stores and binary64 intermediates; §10.15-16 pass; this is a portable numeric
  choice, not a claim of native edge-pixel/weight equivalence for all coordinates; invalid domains remain QV10]

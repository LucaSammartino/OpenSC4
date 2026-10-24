# region-view: open questions
<!-- Questions about [region-view.md](../region-view.md). Append new entries at the END; when one is answered, move it to answered/region-view.md. Rules: README.md. -->

<!-- from: re-regionview7 (2026-10-23) -->
- **RGV1 - §1:** what the view's `+0xDC` holds (given to the region manager's slot 11 before `GetCity`). It is
  not written by the view's own code. [inferred: the current region]

<!-- from: re-regionview7 (2026-10-23) -->
- **RGV2 - §6.1:** the unclaimed-land price is property `0x6A9C7718` of the resource `(0x6534284A, 0xE7E2C2DB,
  0xC8E8F6C5)`, which in the shipped data is the "Crime Simulator" exemplar without that property, so 10000 is
  shown. Is another resource meant, or is the property expected from a plugin? [unknown]

<!-- from: re-regionview7 (2026-10-23) -->
- **RGV3 - §4 step 3.3:** the GZCOM object of class `0x0AE6320E` (IID `0x2AE63219`) that draws the airport and
  seaport icons: what its slots 17, 21, 22, 23 and 38 do. [inferred: a blitter; target, colour key or alpha
  settings, then a blit]

<!-- from: re-regionview7 (2026-10-23) -->
- **RGV4 - §4 step 3.2:** the label's second line (the mayor) is drawn at the name's `y` plus the mayor font's
  height. Is font slot 51's `y` the top of the text, so the two lines overlap when the fonts differ? [unknown]

<!-- from: re-regionview7 (2026-10-23) -->
- **RGV5 - §2.3, §4.1:** the city entry's hit-test runs `+0x44` and its coverage spans `+0x50` (grid) and `+0x5C`
  (selection) are built by the region screen. This spec reads them in the formats given in §4.1; the builder
  (region-screen.md, `RecalculateImagesFromCitySnapshots`) should confirm them. [inferred]

<!-- from: re-regionview7 (2026-10-23) -->
- **RGV6 - §6 step 5:** the interface `0x00008810` asked of every signpost child, whose slot 86 then gets the view's
  slot 87 value. [inferred: a button; the notification target]

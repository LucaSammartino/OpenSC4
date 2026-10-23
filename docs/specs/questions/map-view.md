# map-view: open questions
<!-- Questions about [map-view.md](../map-view.md). Append new entries at the END; when one is answered, move it to answered/map-view.md. Rules: README.md. -->

<!-- from: re-mapview (2026-10-20) -->
- **QV1 - §2, §3.1:** the 3D view filter a data view installs: what `0x4A0B47E0` (occupant types), `0x4A0B47E1`,
  `0x4A0B47E2` and `0x4A0B47F0` change in the filter object (`0x0079FE60`, `0x0079FB90`), and what the terrain
  (slots 84, 86), zone manager (slots 17, 18) and `0x4A0B47E6` calls display. [as read]
- **QV2 - §4.1:** source 76 (traffic volume): which grid it draws for the kept travel types (`0x6B7C5CDD`) and how.
  [as read]
- **QV3 - §1, §3.2:** the panels `0x8A2871B1`-`0x8A2871C4`, `0xAA32BCD4`/`0xAA32BCD5`; the groups' remembered views
  behind the buttons `0x5005`, `0x5008` and `0x8002`; the toggle `+0x9AC` of `0x8100`/`0x8101`; the command
  `0x6A935CF1`. [as read]
- **QV4 - §4.1, §4.3:** the rule of `BlockMapByLot` (`0x0079FEF0`) and its argument `0x4A0B47E8`, the crime incident
  markers (`0x0079E600`), and the developer grid of layer 5 (`0x007A2080`). [as read]

<!-- from: core-map-view4 (2026-10-23) -->
- **QV5 - §4.2, §10: contradictory 16-bit conversions:** at scale 1024 the written signed factor is
  floor(0x7FFF00 / 1024) = 8191, and (-512 * 8191) arithmetic-shifted right 16 is -64, hence index 64;
  §10 instead requires 65. At scale 256 the written unsigned/signed factors are 65535/32767, whereas the prose
  says the default factors are 256 and unsigned values are divided by 256. Which formulas, default factors and
  negative rounding rule are authoritative? [both 16-bit conversions Unsupported; no replacement convention]

<!-- from: core-map-view4 (2026-10-23) -->
- **QV6 - §2: raw property and resource failures:** except scale and desirability layer, what defaults apply to
  absent fields, and what happens with wrong property types, duplicate IDs, odd ramp lists, invalid terrain
  modes, travel-type lists longer than eight or entries outside a byte, resource-key overflow and allocation
  failures? [only documented scale/layer defaults and block-property presence used; other missing/malformed
  consumed fields explicitly Unsupported; raw lists retained without inventing ramp or travel conversions]

<!-- from: core-map-view4 (2026-10-23) -->
- **QV7 - §5: highlight comparison and collection:** what fields, ordering, duplicates and radius equality does
  0x007A1390 compare? What radius data belongs to kinds 6-8 (dash rather than kind 9's explicit none), what is
  kind 0's behavior, and how are collection failures handled? [collect raw matching building records and the
  documented coverage radii for kinds 1-4; kinds 6-8 retain unspecified radius, kind 9 has none; no equality,
  deduplication, ordering or dirty-change decision claimed, including on repeated identical inputs]

<!-- from: core-map-view4 (2026-10-23) -->
- **QV8 - §2, §4.3: owned palette and resampling algorithms:** what are the ramp endpoint, duplicate-stop,
  ordering, interpolation and rounding rules of InterpolateRampToPalette, and the coarse-grid resampling
  algorithm/boundary rules of SC4InterpolateGrid? [nonzero SetDataView stops at ramp generation; raw supplied
  256-entry palettes exercise lookup/emission helpers only; interpolation Unsupported, not delegated]

<!-- from: core-map-view4 (2026-10-23) -->
- **QV9 - §1, §3.1, §7: state outside the specified prefix:** what are the constructor's initial current view,
  notification value and last-redraw clock, repeated Init/Hook behavior, tick clock width/wrap semantics, and
  the timestamp/dirty commit points after empty sources, failed/partial updates or filter-reset calls? Does
  draw-nothing clear or preserve an existing bitmap? [portable construction takes explicit clock/notification
  baselines and an unknown current view; no native defaults claimed; clock regression and repeated Init/Hook
  refused; none stops at QV1 reset boundary; partial redraw intents retain dirty and last-redraw state]

<!-- from: core-map-view4 (2026-10-23) -->
- **QV10 - §4.2-§4.3: out-of-domain floats and overlay cells:** what happens for nonfinite or truncated float
  indices outside 0-255, which colours count as coloured cells, and what origin/extent/depth transformation
  maps the grid to marked cells? [refuse unsupported float indices; overlay helper plans only explicitly
  supplied coloured cells with complete raw geometry, without inferring inclusion, depth or transforms]

<!-- from: core-map-view4 (2026-10-23) -->
- **QV11 - §4.1: developer layers:** which raw grid does the documented default layer 1 select? Are layers 3
  and 4 distinguished beyond both selecting slot 13, and what is the layout/geometry/iteration rule for the
  slot-11 entries used to construct the layer-6 debug grid multiplied by 12? [default/unknown layers refused;
  preserve selected layer 3/4 in the raw request; debug selects layer 6 but does not construct its grid]

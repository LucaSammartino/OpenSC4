# city-tools-input: open questions
<!-- Questions about [city-tools-input.md](../city-tools-input.md). Append new entries at the END; when one is answered, move it to answered/city-tools-input.md. -->

<!-- from: the re-citytools4 lane (2026-10-21) -->
- **CT1 - §2.3, what the 3D view does with a handler's result:** the handlers return a bool, read as "handled".
  How does `cSC4View3DWin` walk its control stack and persistent controls on an input event (top first? stop at
  the first true? does `ShouldStack` false change the push)? Owner: the 3D view's input forwarding, not specified
  anywhere. [top control first, stop on true *[inferred]*]
- **CT2 - §3.7 step 6, window manager slot 19 (`+0x4C`) after every place-lot mouse move:** which call is it
  (redraw request, cursor refresh)? [unknown]
- **CT4 - §2.1, the base constructor:** not re-read; confirm it zeroes `+0x04..+0x20` and the reference count
  starts at 0. [zeroes all *[inferred]*]
- **CT6 - §8.2, the query tool's `+0x98`:** the constructor stores 3; no function of §8 reads it. Who reads it
  (the dialog or route code)? [unknown]
- **CT7 - §8.4 step 5, the legend dialog:** the exact script key passed to `SC4CreateWindowFromScript` and the
  placement arithmetic (window slots 41, 44, 45, 56) of the two legend panes; `SetLegendMinimized`
  (`0x004C4630`) swaps their visibility and moves the minimised pane. [next RE round]
- **CT9 - §8.6 step 2, the automaton and stoplight hits:** what do the vehicle manager's slot 34 and the traffic
  network's slot 37 do with the picked automaton and stoplight prop (select or track it, toggle the light)? (City
  slot 43 is `GetVehicleManager` and slot 40 `GetTrafficNetwork` in the SDK; the automaton filter accepts vehicles
  `0xC7CC75F1` and pedestrians `0x496E5783`, `0x004C3AB0`; re-query 2026-10-21.) [unknown]

<!-- from: the re-querytool4 lane (2026-10-21, round 2) -->
- **CT13 - §8.11.3/§8.11.4, the tip gauge data:** the meaning of callout slot 4's values (12, 10) and slot 14's
  value 5 with the fraction (an icon size and a gauge bar?). Round 3 named the services in §8.11.4 (police
  `0x00B43D98`, fire `0x00B43D78`, pollution `0x00B43D9C`, slots per protection.md / pollution.md), so only the
  callout slots remain. On the callout's primary vtable `0x00AB69D0`, slot 4 (`0x007982C0`) takes one held pointer
  (`+0x130`) and slot 14 (`0x007984D0`) a held pointer (`+0x140`) and a float clamped to [0, 1]; the integer
  arguments (12, 10) and (5) of §8.11.3 do not fit those shapes, so the interface the tip display calls through
  must be checked again. [unknown; callout slots only]

<!-- from: re-query (2026-10-21) -->
- **CT14 - §8.13, §8.12.1, §8.12.4, remaining dialog/writer details:** specify the neighbour row fillers
  `0x104B`–`0x104E`, `0x1066`–`0x106A`, all MySim icon `0x105B` branches, and the complete debug description
  writer content/value sourcing (`DescribeBuilding`, `DescribePopulation`, `DescribeDemand`, `DescribeCell`,
  `WriteDetailedDebugInfo`). Kind priority and all one/two-record resource choices are now §8.16.6; kinds 4/5
  count records rather than look up another window. Writer ordering is §8.16.7. [partially answered 2026-10-23;
  owner closure still open]

<!-- from: re-query (2026-10-21) -->
- **CT15 - §8.15:** route mode: `MakeRoutePath`, `GetBuildingOccupantPaths`, `GetNetworkOccupantPaths`,
  `DoesRouteCrossCell`, `GetRouteTravelTypes` and the legend's travel-type toggles in detail. [outline]

<!-- from: re-plop (2026-10-22) -->
- **CT16 - §3.13, the toll booths:** the side codes the network occupant's slot 38 gives, the booth's cells per
  direction, and what `SeekCompatibleNetworks` (`0x004BF6B0`) tests (its flag mask `0x400000`). [summary]

<!-- from: re-plop (2026-10-22) -->
- **CT17 - §3.14, §3.2 step 6, the feedback:** the preview model (`GetBuildingPreviewModel`, `0x004C0680`), the
  constraint tiles and the colours of `SetVisualUserFeedback`, `SetConstraintFeedback` and
  `SetPropOccupantDrawAttributes`, and what the views `0xA` and `0xF2` registered with the view manager show
  (*[inferred]* the water-pipe and subway layers). [summary]

<!-- from: core-query-tool5 (2026-10-23) -->
- **CT18 - §8.12.1, empty-cell sound conflict:** conformance test 10 requires a failure sound for an empty
  non-zone cell although DescribePick's numbered steps omit it; should that sound be emitted? Occupant slots
  `0x1026`–`0x1028` are confirmed null and the six funding-slider related sequences are now §8.16.1, including
  hospital staff's additional IDs `4`, `0x3F800000`. [partially answered 2026-10-23; sound still open]

<!-- from: core-query-tool5 (2026-10-23) -->
- **CT19 - §8.12.4/§8.14, remaining numeric contracts:** verify aura averaging's inclusive endpoints against
  instructions; airport noise enumeration, signed aggregation/clamp; property-to-curve slope construction,
  duplicate/unsorted/empty knots; missing query records, zero commercial capacity, nonfinite/out-of-range
  numeric conversions and overflowing summed budget costs. Best-developer ranking, wealth geometry/constant,
  commute sampling, evaluation of an already constructed finite ordered curve, and the fixed lost-luggage
  resource key are now §8.16.2–§8.16.5. [partially answered 2026-10-23; exceptional values and remaining numeric
  instructions still open]

<!-- from: core-query-tool5 (2026-10-23) -->
- **CT20 - §8.7/§8.10/§8.11.4/§8.12.1, remaining traffic/debug contracts:** specify
  `GetBuildingOccupantTrafficInfo`, `GetNetworkOccupantTrafficInfo`, their trip-volume helpers, network
  debug backing fields/capacity context and complete description-writer bodies (shared with CT14).
  Debug enable's application-byte/keyboard source and the distinction between building-pick bookkeeping
  and subsequent cell writers are now §8.16.7. [partially answered 2026-10-23; traffic tips and writer bodies
  remain Unsupported, with no native owner callbacks]

<!-- from: core-query-tool5b (2026-10-23) -->
- **CT21 - §8.16.1/§8.12.4, funding cost refresh target:** §8.16.1 says the related-ID sequences are in addition
  to a separate `ID + 2` cost update "in §8.12.4", and QCT5-3 requires that separate update. However §8.12.4
  specifies only the helper's `ID + 0x100` money child, and §8.13 also specifies `ID + 0x100` on notification.
  Is `ID + 2` a typo, or another update? If separate, specify which sliders, its amount, lookup parent and
  initialization/notification/write-back timing. The core retains the explicit `ID + 0x100` helper and
  notification behavior and marks the additional update Unsupported rather than inventing cost controls.

<!-- from: core-place-lot5 (2026-10-23) -->
- **CT22 - §3.4 ring-finished message:** specify the body of `DoPlopRingEffectFinished`.
  The spec names the helper but gives no state transitions or effects. [Unsupported;
  other message branches implemented; no native owner callback]

<!-- from: core-place-lot5 (2026-10-23) -->
- **CT23 - §3.12 clearing/zoning calls:** what are the explicit checkSlope and skipSize
  values for the two PlaceZone calls, and do failures from demolition/dezoning/zoning
  abort placement? [literal published sequence: run both calls, preserve the foreign
  service's default checkSlope/skipSize, no unspecified early returns]

<!-- from: core-place-lot5 (2026-10-23) -->
- **CT24 - §3.11 versus §4 quirk 10, facing equality:** the detailed formula keeps
  the player's rotation when `own >= best - 4`, but quirk 10 says another facing
  with "at least 4 more" points changes it. Which governs at exactly four more?
  [literal §3.11 formula: own 3 / best 7 keeps the player; covered by a test]

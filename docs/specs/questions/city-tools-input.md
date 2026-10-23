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
- **CT14 - §8.13, §8.12.1, §8.12.4:** the neighbour connection dialog in detail (`0x004CC6E0`: the kind codes, the
  looked-up window for kinds 4 and 5, the row fillers of `0x104B`–`0x104E`, `0x1066`–`0x106A`), the MySim icon
  control `0x105B`, and the debug description writers (`DescribeBuilding`, `DescribePopulation`, `DescribeDemand`,
  `DescribeCell`, `WriteDetailedDebugInfo`). [summary]

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

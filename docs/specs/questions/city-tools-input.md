# city-tools-input: open questions
<!-- Questions about [city-tools-input.md](../city-tools-input.md). Append new entries at the END; when one is answered, move it to answered/city-tools-input.md. -->

<!-- from: the re-citytools4 lane (2026-10-21) -->
- **CT1 - §2.3, what the 3D view does with a handler's result:** the handlers return a bool, read as "handled".
  How does `cSC4View3DWin` walk its control stack and persistent controls on an input event (top first? stop at
  the first true? does `ShouldStack` false change the push)? Owner: the 3D view's input forwarding, not specified
  anywhere. [top control first, stop on true *[inferred]*]
- **CT2 - §3.7 step 6, window manager slot 19 (`+0x4C`) after every place-lot mouse move:** which call is it
  (redraw request, cursor refresh)? [unknown]
- **CT3 - §3.2 step 5, the two dwords `+0x128`/`+0x12C`:** read from the building-development service's record
  (offsets `+0x30`/`+0x34`) for the chosen building; constructor defaults `0xFFFE7960` (-100000) and -1. What are
  they and who reads them (`CanPlaceLot` or `PlaceLot`)? [unknown]
- **CT4 - §2.1, the base constructor:** not re-read; confirm it zeroes `+0x04..+0x20` and the reference count
  starts at 0. [zeroes all *[inferred]*]
- **CT5 - §3.2 step 4 / §3.7, the bodies of `PlaceLot` (`0x004C1740`), `CanPlaceLot` (`0x004C1110`), the facing
  helpers (`0x004BE750`, `0x004BEB70`, `0x004BFB30`), `SetVisualUserFeedback` (`0x004C2660`),
  `SetConstraintFeedback` (`0x004C2090`) and `GetBuildingPreviewModel` (`0x004C0680`):** unspecified; needed
  before the place-lot tool can be claimed. [next RE round]

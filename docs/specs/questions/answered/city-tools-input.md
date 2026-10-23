# city-tools-input: answered questions

- **CT2 — §3.7, WM slot 19 after mouse movement: which operation?**
  **ANSWERED 2026-10-23 (re-citytools-ct5):** DIFFERS — input-message queue flush,
  identified by the identical call in SC4FlushInputMessageQueue; conditional on held WM,
  unconditional with respect to pick/feedback changes. §3.7 and §6 give the rule/vectors.
- **CT22 — §3.4, body of DoPlopRingEffectFinished.**
  **ANSWERED 2026-10-23 (re-citytools-ct5):** DIFFERS — clear the overlay flag, then
  recalculate coverage if a 3D view is held; no ring release or placeability change. §3.4/§6.
- **CT23 — §3.12, explicit zoning flags and clearing failures.**
  **ANSWERED 2026-10-23 (re-citytools-ct5):** DIFFERS — slope check and skip-size both
  true, checkLotInside false, commit true on both zoning calls; dezoning skipFunds = zero
  tool cost, zoning skipFunds true. Every clearing stage gates the next; no rollback. §3.12/§6.
- **CT24 — §3.11 versus §4, equality at a four-point frontage improvement.**
  **ANSWERED 2026-10-23 (re-citytools-ct5):** MATCHES — detailed formula governs:
  own 3 / best 7 keeps player rotation; own 3 / best 8 changes it. Corrected quirk 10. §3.11/§6.

<!-- re-query-ct5: resolved portions only; residual questions retain their IDs in the open file. -->
- **CT14 (resolved portion) — neighbour kinds/resources and writer ordering.**
  **ANSWERED 2026-10-23 (re-query-ct5):** DIFFERS — §8.16.6 specifies last-successful network-kind priority
  and the one/two-record scripts for every kind; kinds 4/5 count connection records rather than perform a
  separate window lookup. §8.16.7 distinguishes the building pick from later cell-writer ordering. Row fillers,
  MySim branches and complete writer bodies remain open under CT14.
- **CT18 (resolved portion) — occupant null entries and funding related IDs.**
  **ANSWERED 2026-10-23 (re-query-ct5):** DIFFERS — §8.16.1 confirms null occupant entries `0x1026`–`0x1028`
  and gives all six funding lists. Hospital staff stores four IDs: `0x1023`, `0x1024`, `4`, `0x3F800000`;
  truncating this to two changes the original contract. The empty-cell sound remains open under CT18.
- **CT19 (resolved portion) — selection, geometry, commute, curve evaluation and lost-luggage resource.**
  **ANSWERED 2026-10-23 (re-query-ct5):** DIFFERS — §8.16.2–§8.16.5 define ordered developer ranking with
  truncated-demand ties and a `(-1000,-1000)` sentinel; right-aligned wealth geometry using `0x3EAAAAAB`;
  unsigned commute sampling with zero mapped to 255; endpoint-clamped ordered-curve evaluation; and one
  fixed `(type,group,instance)` airport exemplar key. The remaining numerical domains, noise and curve
  construction stay open under CT19.
- **CT20 (resolved portion) — debug enable source and writer phase distinction.**
  **ANSWERED 2026-10-23 (re-query-ct5):** MATCHES — §8.16.7 requires application debug functionality plus
  Shift/Ctrl/Alt, and separates building-pick query-data bookkeeping from the later building, population,
  demand, EQ/HQ and cell writer sequence. Full traffic/debug helper contracts remain open under CT20.

- **CT11 - §8.7, the tip display tail of `DoMessage`:** the exact callout calls (slots 4 and 14), the `SetTip`
  (`0x004C5BA0`) contract and which of 3D view slots 40/41 run on the shown and empty paths. [next RE round]
  **ANSWERED 2026-10-21 (re-querytool4):** MATCHES - §8.11.3: a non-zero icon with a callout shows the callout (slot 4 image, slot 14 secondary image or (null, 0, 0), `SetTip`, then 3D view slot 41 clears the cursor text); otherwise the callout is hidden and 3D view slot 40 sets the cursor text (key `0x2A456CB1`, 0, title, body, 0).

<!-- from: the re-citytools4 lane (2026-10-21) -->
- ~~**CT8 - §8.6, the click modifier bits `0x01` and `0x02`:** which keys are they (Shift / Ctrl, or the mouse
  button state)? [unknown]~~
  **ANSWERED 2026-10-21 (re-query):** MATCHES — Shift (`0x01`) and Ctrl (`0x02`): the window's key-modifier word
  (budget-window.md §4.0, BW5: `0x1` Shift, `0x2` Ctrl, `0x4` Alt). `IsDebugQueryEnabled` (`0x004C4700`) reads the
  three keys itself (`0x10`, `0x11`, `0x12`) (city-tools-input.md §8.6).

<!-- from: the re-citytools4 lane (2026-10-21) -->
- ~~**CT10 - §8.6 step 3:** what is residential simulator slot 59 called with the quarter-resolution cell (a debug
  commute or travel test)? [unknown]~~
  **ANSWERED 2026-10-21 (re-query):** MATCHES — slot 59 is `ToggleTractTracking(int, int)` (SDK order), called with the cell
  divided by 4; its body in this build is a shared empty one, so the Ctrl-click does nothing (§8.6).

<!-- from: the re-citytools4 lane (2026-10-21) -->
- ~~**CT12 - §8.10, the query tool remainder:** 183 functions, 60,518 bytes after round 3 (zone and neighbour
  dialogs, description writers, 87 filler bodies of the §8.12.3 table and their typed workers, route code, rating
  helpers). The lane parked after its third round; needed before the owner can be claimed. [next RE lane]~~
  **ANSWERED 2026-10-21 (re-query):** DIFFERS — the remainder is now §8.12.4 (every filler of the table, by what it shows),
  §8.13 (`PlayOccupantQuerySound`, the zone-based and neighbour dialogs, the dialog's win proc), §8.14 (the building
  values and the subjective ratings) and §8.15 (route mode, outline); §7 claims the owner. The neighbour dialog's
  details and the debug description writers stay at summary level (CT14).

<!-- from: the re-citytools4 lane (2026-10-21) -->
- ~~**CT3 - §3.2 step 5, the two dwords `+0x128`/`+0x12C`:** read from the building-development service's record
  (offsets `+0x30`/`+0x34`) for the chosen building; constructor defaults `0xFFFE7960` (-100000) and -1. What are
  they and who reads them (`CanPlaceLot` or `PlaceLot`)? [unknown]~~
  **ANSWERED 2026-10-22 (re-plop):** MATCHES — the int64 plop cost of the chosen building (`BuildingProfile` `+0x30`,
  `GetBuildingProfilePtr` = service slot 40; exemplar property `0x49CAC341`, building-development.md). `CanPlaceLot`
  reads it: the cost is 0 when it is 0, otherwise `GetLotDevelopmentCost(rect)` plus it (§3.10); `PlaceLot` charges
  that cost (§3.12). The defaults are the int64 −100000 (unknown).

<!-- from: the re-citytools4 lane (2026-10-21) -->
- ~~**CT5 - §3.2 step 4 / §3.7, the bodies of `PlaceLot` (`0x004C1740`), `CanPlaceLot` (`0x004C1110`), the facing
  helpers (`0x004BE750`, `0x004BEB70`, `0x004BFB30`), `SetVisualUserFeedback` (`0x004C2660`),
  `SetConstraintFeedback` (`0x004C2090`) and `GetBuildingPreviewModel` (`0x004C0680`):** unspecified; needed
  before the place-lot tool can be claimed. [next RE round]~~
  **ANSWERED 2026-10-22 (re-plop):** DIFFERS — the bodies are §3.9 (`GetLotCenterAndCellCoverage`), §3.10 (`CanPlaceLot`,
  `CanDemolishLot`), §3.11 (`GetAutomaticFacingAndCellCoverage`, the street count) and §3.12 (`PlaceLot`); §7 claims
  the tool. The toll-booth picker (§3.13) and the preview and feedback drawing (§3.14) stay at summary level (CT16,
  CT17).

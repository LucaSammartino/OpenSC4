# city-tools-input: answered questions
- **CT11 - §8.7, the tip display tail of `DoMessage`:** the exact callout calls (slots 4 and 14), the `SetTip`
  (`0x004C5BA0`) contract and which of 3D view slots 40/41 run on the shown and empty paths. [next RE round]
  **ANSWERED 2026-10-21 (re-querytool4):** MATCHES - §8.11.3: a non-zero icon with a callout shows the callout (slot 4 image, slot 14 secondary image or (null, 0, 0), `SetTip`, then 3D view slot 41 clears the cursor text); otherwise the callout is hidden and 3D view slot 40 sets the cursor text (key `0x2A456CB1`, 0, title, body, 0).

<!-- from: the re-citytools4 lane (2026-10-21) -->
- ~~**CT8 - §8.6, the click modifier bits `0x01` and `0x02`:** which keys are they (Shift / Ctrl, or the mouse
  button state)? [unknown]~~
  **ANSWERED 2026-10-21 (re-query):** Shift (`0x01`) and Ctrl (`0x02`): the window's key-modifier word
  (budget-window.md §4.0, BW5: `0x1` Shift, `0x2` Ctrl, `0x4` Alt). `IsDebugQueryEnabled` (`0x004C4700`) reads the
  three keys itself (`0x10`, `0x11`, `0x12`) (city-tools-input.md §8.6).

<!-- from: the re-citytools4 lane (2026-10-21) -->
- ~~**CT10 - §8.6 step 3:** what is residential simulator slot 59 called with the quarter-resolution cell (a debug
  commute or travel test)? [unknown]~~
  **ANSWERED 2026-10-21 (re-query):** slot 59 is `ToggleTractTracking(int, int)` (SDK order), called with the cell
  divided by 4; its body in this build is a shared empty one, so the Ctrl-click does nothing (§8.6).

<!-- from: the re-citytools4 lane (2026-10-21) -->
- ~~**CT12 - §8.10, the query tool remainder:** 183 functions, 60,518 bytes after round 3 (zone and neighbour
  dialogs, description writers, 87 filler bodies of the §8.12.3 table and their typed workers, route code, rating
  helpers). The lane parked after its third round; needed before the owner can be claimed. [next RE lane]~~
  **ANSWERED 2026-10-21 (re-query):** the remainder is now §8.12.4 (every filler of the table, by what it shows),
  §8.13 (`PlayOccupantQuerySound`, the zone-based and neighbour dialogs, the dialog's win proc), §8.14 (the building
  values and the subjective ratings) and §8.15 (route mode, outline); §7 claims the owner. The neighbour dialog's
  details and the debug description writers stay at summary level (CT14).

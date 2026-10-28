# bulldozer-tool: answered questions (archive)
<!-- Archive for [bulldozer-tool.md](../../bulldozer-tool.md). Lanes do not read this file. Open questions are in ../bulldozer-tool.md. -->

<!-- from: re-bulldozer (2026-10-20) -->
- ~~**BT1 - §1, §2, §4:** what occupant slots 11, 16 and 17 (with 5) do here (the cell rectangle, a highlight handle
  and highlight level), and what the building development simulator's slot 36 object is that the tool holds. (Lot
  slots 33 and 39 are `GetState` and `IsInDemolishableState`, re-occfilters.) [as read]~~
  **ANSWERED 2026-10-27 (re-x-sims):** MATCHES — the object held from the building development simulator's slot 36 is
  its GetLotDeveloper result (global 0x00B43D68, §1). Occupant slot 11 is GetBoundingCityCells (min x, min z, max x,
  max z of the occupant's bounding box), slot 16 GetHighlight (the current 4-bit highlight level, so the tool's
  "handle" is just the previous level) and slot 17 SetHighlight(level, direct) (the tool sets level 5 with 0 and later
  restores the saved level with 0; a changed level posts message 0xE6E85114, or delivers 0x99EF1145 at once when
  direct) (§4 step 5, §2).

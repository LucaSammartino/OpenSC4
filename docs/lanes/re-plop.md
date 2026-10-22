| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-plop: the place-lot tool's cells, placement test, automatic facing and placing, folded into city-tools-input.md §3 (RE spec lane) | `re-batch5` branch (outside contributor checkout, fork amarret/OpenSC4; on main `f01d1ce4`, after re-query) | **ready** (docs and RE tables only). city-tools-input.md §3.9 (the lot's cells), §3.10 (`CanPlaceLot`, `CanDemolishLot`), §3.11 (the automatic facing, the street count), §3.12 (`PlaceLot`), §3.13 (toll booths, summary) and §3.14 (feedback, summary) added; §3.1, §3.2, §3.4 and §3.6 corrected or extended; the place-lot tool and `cSC4PlaceLotDemolishFilter` claimed as specified; CT3 and CT5 answered, CT16–CT17 filed; 29 names (3 new, 26 renamed). Specified coverage +21,346 B / +38 functions | CT16 (toll booths), CT17 (preview and feedback drawing), CT2 |

## Report

Static RE only: the instruction bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`), the
Ghidra export and the SDK headers. Written on 2026-10-20 as a separate `plop-tool.md` and held back when `main` gained
city-tools-input.md §3 (re-citytools4: the tool's lifecycle and input, with the bodies left open as CT5); this is that
work folded into §3.

### What changed (docs/specs/city-tools-input.md)

- §3.9 (new): `GetLotCenterAndCellCoverage`: odd sizes centre on the cell under the point, even sizes on the nearest
  corner; the rotation swaps the size; the terrain bounds.
- §3.10 (new): `CanPlaceLot`: the last placed rectangle test, the cost (`GetLotDevelopmentCost` plus the plop cost),
  `CanCreatePloppedLot` and its messages, `CanDemolishLot` (flags `0x2800`, `0x800` for a subway station).
- §3.11 (new): the automatic facing (score per rotation, the keep rule) and the street count (avenue 4, road 2,
  street or one-way 1).
- §3.12 (new): `PlaceLot`: the tutorial rectangle test, funds, the clearing filter that spares power poles and lines,
  dezoning, the zone types 15 / 11 / 12, `CreateLotIfPossible`, `ConstructDefaultLot`, the withdrawal, the aura
  effect, the pipe and subway connections, the deposit after a failed building, the placement and ring effects.
- §3.13, §3.14 (new, summary): the toll-booth picker; the preview model and the feedback.
- Corrections and additions, read in the bytes:
  - §3.1: the cheat `BuildingPlop` builds the tool with `(1, 0, 0, 0)`, not all 0 (`0x007F3637`; view3d-window.md
    agrees); the first argument is a mode.
  - §3.2 step 4: the pickers appear only on the `LotPlop` and `BuildingPlop` cheats (`0x004C149C`, `0x004C152B`).
  - §3.2 step 5: `+0x128` is the int64 plop cost from `GetBuildingProfilePtr` (`BuildingProfile` `+0x30`).
  - §3.2 step 6: what sets `+0x140` (`0xA` for occupant group `0x1401`, `0xF2` for `0x1302`, `0x004C0BCF`–`0x004C0BFB`).
  - §3.4, §3.6: `0x00B43CF4` is the terrain, not the city; slot 6 is its cell width.
- §4 quirks 9–11, §6 place-lot tests, §5 and §7 (the place-lot tool claimed as specified).
- Kept upstream where it was right: the stored rectangle is in world units (`0x004C31A7`–`0x004C3216`; the held spec
  said cells), and `OnMouseMove` tries the manual rotation before toll-booth mode (`0x004C33C5`, `0x004C33F5`).

### Names and coverage

- re/symbols/plop-tool.tsv: 29 names (3 new: `AddPlopAuraEffect`, the `cSC4PlaceLotDemolishFilter` constructor and
  type test; 26 replace auto names). The `names_windows.tsv` delta (−26 +29) is identical to `merge_names.py`'s; no
  non-auto upstream name is replaced.
- tools/coverage-map.tsv: `cSC4ViewInputControlPlaceLot` and `cSC4PlaceLotDemolishFilter` → city-tools-input.md
  (they fell to `cSC4ViewInputControl*` and `cSC4*`).

### Measured

`tools/coverage.py` on the local (Steam) export, measurement only: after re-query 3,430,796 bytes in 15,262
functions; with this lane 3,452,142 bytes in 15,300 functions (+21,346 B, +38 functions; denominator unchanged).
`docs/coverage-report.md` was not regenerated (the tool accepts only the GOG exe).

### Questions

- Answered and moved to questions/answered/city-tools-input.md: CT3 (the plop cost), CT5 (the bodies, now §3.9–§3.12).
- Filed: CT16 (the toll-booth rules), CT17 (the preview and feedback drawing, the views `0xA` and `0xF2`).
- The held lane's own questions are settled or carried over: PT1 (when the pickers show) and PT4 (the land price is
  `GetLotDevelopmentCost`; the tutorial test compares the lot's cells with the task's rectangle) answered in §3.2 and
  §3.10–§3.12; PT2 and PT3 became CT16 and CT17.

### Not covered

CT2 (window manager slot 19 after each mouse move) stays open; the toll-booth picker and the feedback drawing are at
summary level.

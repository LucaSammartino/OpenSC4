| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-mysimtools7: the MySim tools (change house, change job, move in, dispatch), new spec mysim-tools.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-render3d7) | **ready** (docs only). mysim-tools.md §1–§6: the four tools and their vtables, the building tools' shared slots and `Init(sim)` (badge windows, portrait, wealth marks), the hover eligibility and callout of each tool, the job callout's group icon table, the EQ/HQ band texts, the clicks, the dispatch tool's pylon filter, pick, click and drag, hover and messages; questions MST1–MST3; owners claimed: `cSC4MySimInputControl*` (ChangeHouse, ChangeJob, MoveIn, Dispatch): specified +12,016 bytes (+38 functions) | MST1–MST3 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 38 claimed functions were disassembled. The vtables `0x00A85800`, `0x00A858D0`, `0x00A85A20`, `0x00A85988`,
`0x00A85998` and `0x00A8596C` were read to tell the three building tools apart and to place the dispatch tool's
slots. Written on 2026-10-23. mysims.md and mysim-windows.md named the tools but did not describe them.

### What changed

- New docs/specs/mysim-tools.md and questions/mysim-tools.md (MST1–MST3).
- Read on the bytes:
  - the three building tools share one layout and four slots, and differ only in `Init(sim)`, the hover tests and
    the click;
  - the move-in tool keeps the `cISC4MySim` interface where the others keep the object;
  - the job callout's skin follows the building's occupant groups with the query tool's icon table;
  - the dispatch tool's double-click clock (500), drag offset, saved position and state, and its six messages.
- QUESTIONS.md regenerated.

### Quirks found

- Two lot lookups are used without a null test.
- A wealth outside 1–3 passes a stale meter fill.
- `OnMouseExit` hides the badges but not the callout or the highlight.
- The dispatch tool's `Shutdown` releases one of the hovered pylon's two references.

### Coverage

- tools/coverage-map.tsv: the `cSC4MySimInputControl*` row now cites mysim-tools.md.
- Specified bytes 4,593,770 → 4,605,786 (+12,016, 82.3% → 82.5%, `cov_local.py`, on `bb641878` + re-batch7 and
  re-render3d7).

### Not covered

- The move-in constructor (owner `cSC4MySim`) and the shared comparison helper `0x00437AD0`.

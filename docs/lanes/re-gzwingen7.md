| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzwingen7: the general window `cGZWinGen` and the flat rectangle `cGZWinFlatRect`, new spec widget-wingen.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzwinbtn7) | **ready** (docs only). widget-wingen.md §1–§12: identity and fields, the generator flags (named from the script properties), the 32-slot `cIGZWinGen` table, `SetWinGenFlag`, `SetWinProc` and the window-procedure messages, `DoMessage`, layout (`SetArea` with minimum size and procedure limits, user-area maths, title bar, side bar, `on_layout`, `place_controls`), `Init`/`Shutdown`, painting, move and resize with the cursor choice, hit tests, keys and closing, colour bars and dividers, the whole flat rectangle; quirks, eight cases; owners claimed: `cGZWinGen`, `cGZWinFlatRect` and the `fn:` flat-rect thunk: specified +8,574 bytes (+74 functions) | WGN1–WGN3 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 74 functions were read in the decompile. The paint calls, the side-bar placement, the size clamps, the drag
switch, the cursor switch and `calc_size_mode` were checked on the instructions; four calls turned out to take
arguments pushed before an argument-less getter. The vtables `0x00ADC678`, `0x00ADC5F8`, `0x00AE20A0` and
`0x00AE2050` were read and compared with the `cGZWin` table. Written on 2026-10-23. ui-windows.md and
legacy-control.md cited the class but no spec described it.

### What changed

- New docs/specs/widget-wingen.md.
- New docs/specs/questions/widget-wingen.md (WGN1–WGN3); QUESTIONS.md regenerated.

### Quirks found

- With the moveable and sizeable flags and no title bar, the move mode replaces every resize edge.
- `GetBackgroundImage(out)` AddRefs the image without a null check.
- `AddColorBar` returns true when the flat rectangle cannot be created.
- Moving a window (same size) does not lay out its controls or recompute its user area.
- `kill_self` drops its result for a window that is not modal.

### Coverage

- tools/coverage-map.tsv: new rows `cGZWinGen`, `cGZWinFlatRect` and `fn:__ZThn228_NK14cGZWinFlatRect12GetFillColorEv`
  → widget-wingen.md, before the `cGZWin*` catch-all.
- Specified bytes 4,626,465 → 4,635,039 (+8,574, 82.9% → 83.0%, `cov_local.py`).

### Not covered

- `cGZWinGenTitleBar` (its own owner); the 2D draw-context slots and the graphics-system colour conversions,
  cited by slot (WGN1, WGN2).

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-ticker7: the news ticker `cSC4NewsTickerWinProc`, new spec news-ticker.md (RE spec lane) | `re-batch7` branch (outside contributor checkout, fork amarret/OpenSC4; on main `5c0ff782`, after re-callout7) | **ready** (docs only). news-ticker.md §1–§8: identity and fields, `Init` and the timer callback, show/hide, subscription, the window and message tables, the priority-ordered advice list and its HTML builder (differences from the advisor window), the three-line flipper (selection, rotation, the 40 ms scroll) and its line builder, quirks, four conformance cases; questions NTK1–NTK4; owner claimed: `cSC4NewsTickerWinProc`: specified +7,764 bytes (+16 functions) | NTK1–NTK4 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 16 claimed functions were disassembled. The HTML strings are statics made by a start-up initializer
(`0x00A76300`–`0x00A76A40`); they were read from its `push`/`mov ecx` pairs and the data section. The mood jump
tables (`0x00773960`, `0x00774050`) and the vtables were read from the data section. Written on 2026-10-23. No
upstream lane covers this owner (view3d-window.md only creates it).

### What changed

- New docs/specs/news-ticker.md.
- Read on the bytes:
  - the layout (timer, the two controls, the list, the three flipper places, the scroll state);
  - `Init` (child IDs, the line height of font `0xAA0F4AB4`, the 3-line rest rectangle, the 2-line bar);
  - the timer message `0x0A344006` sent through message server 2;
  - the list insert by descending priority;
  - the four advice messages, including `AdviceRemovedAll` by advisor ID;
  - the flipper rules (priority gate 59, after-`last` then from-head search, rotation, clearing);
  - the 3000 ms / 40 ms timing;
  - the line markup (language 15 uses the `SIZE=2` face).
- docs/specs/questions/news-ticker.md: NTK1–NTK4 (new prefix NTK); QUESTIONS.md regenerated.

### Quirks found

- `AdviceRemoved` does not take the advice out of the list.
- Mood-7 list rows repeat the title.
- An unreachable append branch in `DoFlipperMessage`.
- Without a rotation the flipper jumps up a whole line for 3 s.

### Coverage

- tools/coverage-map.tsv: `cSC4NewsTickerWinProc` → news-ticker.md, before the `cSC4*WinProc` catch-all.
- Specified bytes 4,500,986 → 4,508,750 (+7,764, 80.6% → 80.7%, `cov_local.py`, on `5c0ff782` + the ten lanes
  before).

### Not covered

- The two list controls.
- The advice slot and flag meanings (NTK1, NTK2).

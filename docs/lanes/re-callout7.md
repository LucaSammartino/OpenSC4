| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-callout7: the callout box `cSC4WinCalloutBox`, new spec callout-box.md (RE spec lane) | `re-batch7` branch (outside contributor checkout, fork amarret/OpenSC4; on main `5c0ff782`, after re-advwin7) | **ready** (docs only). callout-box.md §1–§7: identity and fields, `Init`/`Shutdown`/destructor, the callout interface's setters and dirty bits, `GZPaint` (content measurement and the text surface, the side choice and the four box/arrow layouts, the 5 × 5 skin composition with its arrow cells, the final draw with icon and meter), quirks, five conformance cases; questions WCB1–WCB3; owner claimed: `cSC4WinCalloutBox`: specified +8,194 bytes (+24 functions) | WCB1–WCB3 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 24 claimed functions were disassembled. The 5.8 KB paint routine was read on the disassembly, branch by branch:
the side table, the jump tables `0x00799DA4` (layout) and `0x00799DB4` (arrow cells), and every blit's source and
destination rectangle. Written on 2026-10-23. No upstream lane covers this owner. ui-feedback-owners.md and
city-tools-input.md mention it only as a held object.

### What changed

- New docs/specs/callout-box.md.
- Read on the bytes:
  - the interface slots (skin and padding, title and body, fonts, icon, meter with count and fill, point, offset
    rectangle, bounds) and their dirty bits;
  - the measurement (title +4, body under it, meter row, padding, icon);
  - the side rules and the per-side box, window and arrow rectangles;
  - the skin's 5 × 5 cells (corners, edges, two background bands, four arrow cells);
  - the two-pass meter draw.
- docs/specs/questions/callout-box.md: WCB1–WCB3 (new prefix WCB); QUESTIONS.md regenerated.

### Quirks found

- The maximum text width comes from the bounds' height.
- The text surface is remade on every content change (its size test uses the content size) and is not unlocked
  here.
- The near-edge tests use the cell width vertically.
- The meter's vertical offset is the horizontal padding.

### Coverage

- tools/coverage-map.tsv: `cSC4WinCalloutBox` → callout-box.md, before the `cSC4Win*` catch-all.
- Specified bytes 4,492,792 → 4,500,986 (+8,194, 80.5% → 80.6%, `cov_local.py`, on `5c0ff782` + the nine lanes
  before).

### Not covered

- The font, surface and draw-context interfaces (WCB1–WCB3).
- The callers' texts and placement.

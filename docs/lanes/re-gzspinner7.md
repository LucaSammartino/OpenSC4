| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzspinner7: the spin control `cGZWinSpinner`, new spec widget-spinner.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzslider7) | **ready** (docs only). widget-spinner.md §1–§7: fields, the 35-slot `cIGZWinSpinner` interface, flags, value and caption (plain, number, money), cell and size rules, layout and hit test, lifecycle, keys, mouse, the accelerating auto-repeat, painting (outline, fill, flat and image buttons, text), quirks, four cases; owner claimed: `cGZWinSpinner`: specified +4,707 bytes (+51 functions) | WSP1 (unwritten flat button colour), WSP2 (`ClearFontColor`) |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 51 functions were read in the decompile; the constructor, `SetArea`, `SetValue`, the flat button paint, the colour
accessors, `Init` and `SetFontStyle` were checked on the instructions, because most interface methods run on the
`+0xD8` pointer and the decompile's field names are relative to it. The vtables `0x00AE0B90` and `0x00AE0DF0` and the
format `0x00A8281C` were read. Written on 2026-10-24.

### What changed

- New docs/specs/widget-spinner.md and docs/specs/questions/widget-spinner.md (WSP1, WSP2); QUESTIONS.md regenerated.

### Quirks found

- The flat buttons use a colour field (`+0x16C`) that nothing writes.
- `ClearFontColor` passes the unset sentinel through the native-to-packed conversion, so it never clears.
- The four-colour outline setter and getter convert colours, the single-colour ones do not.
- `SetMinMax` does not re-clamp the value; `SetFlag(0x1000)` does not refresh the caption; `SetButtonImageArea` does
  not enable the custom rectangle; `SetFontStyle` does not check for the control manager.
- Losing capture keeps the hover part.

### Coverage

- tools/coverage-map.tsv: new row `cGZWinSpinner` → widget-spinner.md, before the `cGZWin*` catch-all.
- Specified bytes 4,703,623 → 4,708,330 (+4,707, 84.2% → 84.3%, `cov_local.py`).

### Not covered

- The draw-context slots (WGN1) and the language utility's number and money formats.

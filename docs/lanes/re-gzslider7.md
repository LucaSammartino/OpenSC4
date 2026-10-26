| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzslider7: the slider `cGZWinSlider` and the old scroll bar `cGZWinScrollbar`, new spec widget-slider.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzdialogs7) | **ready** (docs only). widget-slider.md §1–§6: both classes' fields, the `cIGZWinSlider` and `cIGZWinScrollbar` interfaces, value and fraction setters, layout and thumb placement, the point-to-fraction drag, the scroll bar's parts, line/page steps and 200 ms auto-repeat, both flat and bitmap paints (4-frame and 12-frame strips), lifecycle, quirks, four cases; owners claimed: `cGZWinSlider`, `cGZWinScrollbar`: specified +5,222 bytes (+50 functions) | WSL1 (callers of scroll bar slots 17–19), WSL2 (zero-length tracks in shipped windows) |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 50 functions were read in the decompile; the value and fraction setters (including the x87 equality and clamp
tests), the point-to-fraction mapping, the thumb placement, both slider paints, the scroll bar's bitmap paint and the
`cIGZWinScrollbar` stubs were checked on the instructions. The five vtables (`0x00AE04D8`, `0x00AE049C`,
`0x00AE0810`, `0x00AE07D8`, `0x00AE0788`) and the floats `0x00A84D2C`, `0x00A81054`, `0x00A81228`, `0x00A80AA8`
were read. Written on 2026-10-24.

### What changed

- New docs/specs/widget-slider.md and docs/specs/questions/widget-slider.md (WSL1, WSL2); QUESTIONS.md regenerated.

### Quirks found

- `SetValue` compares with the current value before clamping, so an out-of-range value notifies without a change.
- A zero-length track (the default 32×96 old scroll bar) turns a thumb drag into a NaN fraction, read back as
  `min + 0x80000000`.
- Vertical sliders grow upward, vertical old scroll bars downward.
- The line and page divisors keep the constructor's 32 when the range changes; `GetPageSize` uses `range + 1`.
- The slider's frame rectangle keeps sizes in its right/bottom; with an orientation other than 1 or 2 and an image,
  `SetArea` ignores the area.
- Only the slider sends the release notice `0x887113A4`; neither destructor releases the image.
- Ten of the old scroll bar's methods are assigned to the `cGZWinSlider` owner, and its bitmap paint
  (`0x009C40F4`) to `cGZDrawContext`.

### Coverage

- tools/coverage-map.tsv: new rows `cGZWinSlider`, `cGZWinScrollbar` → widget-slider.md, before the `cGZWin*`
  catch-all.
- Specified bytes 4,698,401 → 4,703,623 (+5,222, 84.1% → 84.2%, `cov_local.py`).

### Not covered

- The draw-context slots (WGN1) and the callers of `cIGZWinScrollbar` slots 17–19 (WSL1).

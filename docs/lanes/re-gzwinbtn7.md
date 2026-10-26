| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzwinbtn7: the toolkit button `cGZWinBtn`, new spec widget-button.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzwin7) | **ready** (docs only). widget-button.md §1–§8: identity and fields, the button flags and styles, the 53-slot `cIGZWinBtn` table, `SetBtnFlag` and its notifications, `SetStyle`, lifecycle, layout (frame index per style and state, text position and size, preferred size), painting, mouse and key input, flash and tips, quirks, seven cases; widget-toolkit.md §18.3 names slots 40 and 62; owner claimed: `cGZWinBtn`: specified +7,818 bytes (+87 functions) | none new |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 87 functions were read in the decompile, and the stack-argument tricks (getters called with arguments that stay
on the stack for the next call) were checked on the instructions. The vtables `0x00ADDD50` and `0x00ADDAF0` were
read. Written on 2026-10-23. No spec described the button before.

### What changed

- New docs/specs/widget-button.md.
- widget-toolkit.md §18.3: `GetChildWindowFromCursorPoint` is slot 40 and recurses through itself; its own test is
  `IsPointInWindowScreenCoordinates` (slot 62).

### Quirks found

- Every completed click flips the "on" flag, also for a plain push button.
- A combined mask in `SetBtnFlag` is stored without a notification.
- A flash count of −1 or −2 never ends.
- Frame modes 3 and 4 exist in the layout code but cannot be set.

### Coverage

- tools/coverage-map.tsv: new row `cGZWinBtn` → widget-button.md, before the `cGZWin*` catch-all.
- Specified bytes 4,618,647 → 4,626,465 (+7,818, 82.7% → 82.9%, `cov_local.py`).

### Not covered

- The font, image and tooltip-manager internals, cited by slot.

# widget-wingen: answered questions (archive)
<!-- Archive for [widget-wingen.md](../../widget-wingen.md). Lanes do not read this file. Open questions are in ../widget-wingen.md. -->

<!-- from: re-gzwingen7 (2026-10-23) -->
- ~~**WGN2 - §8.1, §9.2:** the window-control manager's `GetColor` (slot 32) result is passed whole to window slot
  106 but cut to 16 bits for flat-rect slot 8, which turns it into `r, g, b` through graphics-system slot 36. Is the
  stored colour a 16-bit native (display-format) value, and are graphics-system slots 34/36 the RGB ↔ native
  conversions? [inferred]~~
  **ANSWERED 2026-10-23 (re-gzwinctrl7):** the manager stores each colour as packed `0x00RRGGBB`; `GetColor`
  (`0x00997B8B`) returns the graphics system's slot 34 `(r, g, b)`, a native colour, and its slot 34 setter twin
  (`0x0099761A`) turns a native colour back with graphics-system slot 36. So slot 34 is RGB → native and slot 36
  native → RGB (as widget-toolkit.md's colour conversions also use them). The `& 0xFFFF` in `AddColorBar` keeps only
  the low 16 bits of the native value, which loses nothing only in a 16-bit display format. widget-wingen.md §8.1
  and widget-ctrlmgr.md §2 updated.

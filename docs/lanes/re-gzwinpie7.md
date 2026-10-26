| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzwinpie7: the radial menu `cGZWinPieMenu`, new spec widget-piemenu.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzwintip7) | **ready** (docs only). widget-piemenu.md §1–§8: fields and item records, the 57-slot `cIGZWinPieMenu` table, layout on eight compass directions, hit testing by angle and distance, open and close, selection and its notifications, mouse and keys, painting with the optional renderer, lifecycle, quirks, three cases; owner claimed: `cGZWinPieMenu`: specified +2,651 bytes (+15 functions). The 50 pie functions in the `cGZWinCOMDirector` owner are specified here too and count when that owner is claimed | claim `cGZWinCOMDirector` once its other functions are specified |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 15 owner functions and the 50 interface functions attributed to `cGZWinCOMDirector` were read in the decompile;
the `<shared>` getters and setters were disassembled and the angle constants read. The vtables `0x00AE3420` and
`0x00AE3508` were read. Written on 2026-10-24.

### What changed

- New docs/specs/widget-piemenu.md.

### Quirks found

- `Init` replaces only a background image that is already set.
- `SetRenderer` releases the old renderer before AddRef'ing the new one.
- `SetSelection` notifies a change even when nothing changed.

### Coverage

- tools/coverage-map.tsv: new row `cGZWinPieMenu` → widget-piemenu.md, before the `cGZWin*` catch-all.
- Specified bytes 4,674,424 → 4,677,075 (+2,651, 83.7% → 83.8%, `cov_local.py`).

### Not covered

- The item renderer interface and the font slots, cited by number.

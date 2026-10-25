| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzaccelres7: the key accelerator text resource (`cRZWinKeyAcceleratorRes`, `cGZWinKeyAcceleratorResFactory`), new spec widget-accelres.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzoutline7) | **ready** (docs only). widget-accelres.md §1–§4: identity, slots and factory, the text format (lines, the split, key tokens and the key-name table, message values, key codes), quirks, three cases; widget-toolkit.md's coverage status rows refreshed; owners claimed: specified +3,166 bytes (+18 functions) | none new |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 18 functions were read in the decompile; the second-`=` branch and the token loop were checked on the
instructions. The vtables `0x00ADA5F4`/`0x00ADA5C0` and the key-name strings were read. Written on 2026-10-24.

### What changed

- New docs/specs/widget-accelres.md.
- widget-toolkit.md §10 coverage table: rows for the accelerator resource, graphs, clipboard and cheat codes now point
  to their specs.

### Quirks found

- `down` after a key sets key-down and is then also read as the Down arrow, replacing the key.
- Bindings of a lone modifier are stored but never match.
- Key names match by "contains", in table order.

### Coverage

- tools/coverage-map.tsv: new rows `cRZWinKeyAcceleratorRes` (before `cRZWinKeyAccelerator*`) and
  `cGZWinKeyAcceleratorResFactory` (before `cGZWin*`) → widget-accelres.md.
- Specified bytes 4,743,130 → 4,746,296 (+3,166, 84.9% → 85.0%, `cov_local.py`).

### Not covered

- The persist-resource base slots (`cGZPersistResource`).

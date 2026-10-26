| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gztipmgr7: the tooltip manager `cGZWinToolTipMgr`, widget-tooltip.md §6 (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzwinpie7) | **ready** (docs only). widget-tooltip.md §6: the object and its vtable, `Init` (seven subscriptions, the tooltip window, the 750 ms timer), `Shutdown`, the timer callback, the tip messages, show/hide timing; owner claimed: `cGZWinToolTipMgr`: specified +859 bytes (+12 functions); the 9 manager functions in the `cGZWinCOMDirector` owner are specified there too | none new |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 12 owner functions and the 9 manager functions attributed to `cGZWinCOMDirector` were read in the decompile; the
vtable `0x00AE434C` was read; the timer slots were matched with timers.md. Written on 2026-10-24.

### What changed

- widget-tooltip.md: new §6.

### Quirks found

- Messages `0x22C010D5`/`0x22C010D6` forward the tip's ID where the tooltip window expects a colour.
- `get_tip_win` returns a borrowed pointer after releasing its query.

### Coverage

- tools/coverage-map.tsv: new row `cGZWinToolTipMgr` → widget-tooltip.md.
- Specified bytes 4,677,075 → 4,677,934 (+859, 83.8%, `cov_local.py`).

### Not covered

- The message server's slots, cited by number.

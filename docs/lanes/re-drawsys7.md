| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-drawsys7: the draw system service (context registry, idle flush, tick), draw-system.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `4aa46040`, after re-smallhelpers7) | **ready** (docs only). One owner claimed: specified +1,259 bytes (+17 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 17 functions were read in the decompile; the interface table `0x00AC1668`, `OnTick`, `Shutdown` and the two flush
bodies in the disassembly. The context-creation slots and `Init` stay with span-render-4444.md and span-render-8.md,
which are referenced, not changed. `cGZWinCommandSystemSupport` (the `GZWin…` scripted UI test commands) was looked
at and left out as a test-automation tool. Written on 2026-10-24.

### What changed

- New docs/specs/draw-system.md.

### Quirks found

- `FlushIdleContexts` ignores its argument; the five-tick threshold is a constant.

### Coverage

- tools/coverage-map.tsv: `cGZDrawSystem` → draw-system.md, span-render-4444.md (before `cGZDraw*`).
- Specified bytes 4,927,953 → 4,929,212 (+1,259, 88.3%, `cov_local.py`).

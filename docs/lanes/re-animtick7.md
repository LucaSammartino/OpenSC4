| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-animtick7: the animation tick manager (`cSC4AnimationTickManager`), animation-tick.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `14e0416c`, after re-thumbs7) | **ready** (docs only). Priority classes and their rates, the 17-slot interface, the tick, deferred requests, pause/step/auto-step, save and load; owner claimed: specified +4,052 bytes (+37 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 37 functions were read in the decompile; the vtable `0x00A86760`, the constructor's frame masks and the constants
`0x00A867A4` (0.001), `0x00A867A8` (0.125) and `0x00A867AC` (1,000,000) were read. Written on 2026-10-24.

### What changed

- New docs/specs/animation-tick.md.

### Quirks found

- Reading a save discards the tickables it reads; receivers re-register themselves.
- An unbalanced `PauseAnimationSystem(false)` leaves the pause count negative.
- Removals during a tick only hide the target for the rest of that tick.

### Coverage

- tools/coverage-map.tsv: `cSC4AnimationTickManager` → animation-tick.md.
- Specified bytes 4,842,853 → 4,846,905 (+4,052, 86.8%, `cov_local.py`).

### Not covered

- The service's own `OnTick` (its `+0xC` vtable) that calls `DoAnimationTick`.

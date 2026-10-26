| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-streamops7: the shared save-stream helpers (`fn:operator<<`, `fn:operator>>`), stream-helpers.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `14e0416c`, after re-animtick7) | **ready** (docs only). The value types and containers with their users, the timer, generator, response-curve and grid readers' quirks, the particle description writer, the attractor destructor; owners claimed: specified +5,839 bytes (+45 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 45 functions were read in the decompile; the timer, generator, response-curve and grid readers, the particle
description writer's string write and the effects resource's section header were read in the disassembly, and every
uncited helper's callers were found with the cross-reference tool. Written on 2026-10-24.

### What changed

- New docs/specs/stream-helpers.md.

### Quirks found

- The generator reader and the response-curve reader read a value into the stack slot of the stream argument: on a
  failed stream the generator's cached-Gaussian flag, and the curve's point count, become the stream's address.
- The timer and grid readers read into uninitialised temporaries.
- The weather's sea mask is read by the bool-grid reader, so any non-zero byte loads as 1.
- The particle description writer writes flags B and C whole; reader B masks them.

### Coverage

- tools/coverage-map.tsv: `fn:operator>>` and `fn:operator<<` → stream-helpers.md.
- Specified bytes 4,846,905 → 4,852,744 (+5,839, 86.9%, `cov_local.py`).

### Not covered

- The effects resource's own array writers called by the particle description writer (`0x005AFC10`, `0x005AFC90`,
  `0x005AFD00`, owner `cSC4EffectsResource`).

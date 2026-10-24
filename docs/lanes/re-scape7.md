| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-scape7: the audio scape manager `cSC4AudioScapeManager` and the fire handler `cSC4AudioEventHandlerDisasterFire`, new spec audio-scape.md (RE spec lane) | `re-batch7` branch (outside contributor checkout, fork amarret/OpenSC4; on main `5c0ff782`, after re-scrubber7) | **ready** (docs only). audio-scape.md §1–§11: identity and the 19-slot table, lifetime, the manager/scape/cluster/definition layouts, scapes, emanations and clustering, the sound-band choice, the posted events and the per-tick fade, the three position queries, callers, the fire handler; questions SCP1–SCP4; both owners claimed: specified +11,250 bytes (+77 functions) | SCP1–SCP4 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`)
and the Ghidra export of the two owners; every function of the scape manager was disassembled. Written on 2026-10-23.
No upstream lane covers either owner. The audio event handler's calls into slots 13–15 are upstream's audio.md §7.3,
and this spec's slot table matches them.

### What changed

- New docs/specs/audio-scape.md.
- Byte-checked: the three vtables; `Init`/`Shutdown`; the scape, cluster and definition layouts; `CreateScape`'s
  auto sound.
- Behaviour read on the bytes: the nearest-cluster rule, the band test on the cluster's member count, and the four
  posted events with their words.
- The fade step is `trunc(d / 7)`, or `d` when that is 0. Its constant was checked at `0x004F6967`; from 0 it
  reaches 1024 on the 37th tick.
- The grid query's integer cell sizes and sparse output; the fire handler's scape and bands (`0x004F0830`–
  `0x004F08C3`).
- docs/specs/questions/audio-scape.md: SCP1–SCP4 (prefix new); QUESTIONS.md regenerated.

### Quirks found

- `RemoveScape` clears the flags of every scape it walks past.
- Removing a source divides the remaining sum by the old count.
- A moved source stays in its old cluster's average.
- `MoveEmanation` ignores its scape id.
- Slot 13's fourth argument has no effect.
- `RemoveSound` leaves copies playing.
- Slot 8 never ends without a sound system.
- The grid query writes sparse output, lets the right edge spill into the next row, and writes outside the grid
  for a zero cell size.
- The fade counts frames, not time.
- The fire handler's failed `Init` keeps its subscriptions, and a second `Init` fails on the existing "Fires" scape.

### Coverage

- tools/coverage-map.tsv: `cSC4AudioScapeManager` and `cSC4AudioEventHandlerDisasterFire` → audio-scape.md,
  before the `cSC4Audio*` catch-all.
- Specified bytes 4,381,762 → 4,393,012 (+11,250, 78.5% → 78.7%, `cov_local.py`, on `5c0ff782` + the two lanes
  before).

### Not covered

How the sound manager executes the posted events (audio.md §8); the role of the third vtable (SCP3).

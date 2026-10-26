| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audiofilters8: the 83 ambience filter classes of `cSC4AudioCOMDirector` and `cAmbienceOccupantFilterIsNature`, audio-filters.md (RE spec lane) | `re-batch9` branch (outside contributor checkout, fork amarret/OpenSC4; on main `626043df`, after re-videoprefs8) | **ready** (docs only). Two owners claimed: specified +15,115 bytes (+181 functions); AU47 archived | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
Each of the 83 factories of audio.md §11.2 was read for its vtable, and slot 5 of that vtable in the PE; every test
body was read in the decompile, the irregular ones (building profiles, property read, growth stage, network double
call, lot bounds) in the disassembly. Foreign slot names come from the existing specs and, where marked, from the
public gzcom-dll headers (lot, simulator, zone developer, lot configuration, building development simulator).
Written on 2026-10-25.

### What changed

- New docs/specs/audio-filters.md: the three filter families (global, occupant, cell) and their caches, and the test
  of every class ID (night, zoom, view mode, pause and speed, U-Drive-It; occupant groups, type, property, nature,
  building purpose, lot condition and wealth; zone types, empty cell, networks, water, lot state, wealth, condition,
  building, growth stage, crime and land value bands), quirks and conformance tests.
- docs/specs/audio.md §11.1: the open "role of the 83" paragraph now points to audio-filters.md and
  audio-ambience.md §8.3–§8.4.
- AU47 archived (already answered by audio.md §11.3); QUESTIONS.md regenerated.

### Coverage

- tools/coverage-map.tsv: new row `cSC4AudioCOMDirector` → audio.md, audio-filters.md (above `cSC4Audio*`); the
  `cAmbienceOccupantFilterIsNature` row filled.
- Specified bytes 5,016,747 → 5,031,862 (+15,115, 89.8% → 90.1%, `cov_local.py` with owners-c1).

### Why now

Earlier batches skipped audio as active upstream (re-audio9, core-audio-close13, 2026-10-24). Upstream main
`626043df` has STOP set and no lane on these owners.

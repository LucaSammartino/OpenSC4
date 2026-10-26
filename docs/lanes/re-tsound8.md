| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-tsound8: the sound instance `tSound` (linked averaging, volume and pan, pauses, Resume) and `cSourceFileDebugInfo`, audio-owner-closure.md §7–§8 (RE spec lane) | `re-batch9` branch (outside contributor checkout, fork amarret/OpenSC4; on main `626043df`, after re-audioclose8) | **ready** (docs only). Two owners claimed: specified +2,911 bytes (+21 functions); AU50 partly answered (noted in place) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
The eleven `tSound` bodies audio.md left open and the eight debug-info functions were read instruction by
instruction; the averaging constants (1.0, 1/1024, FLT_MAX) were read in the PE. Written on 2026-10-25.

### What changed

- docs/specs/audio-owner-closure.md §7: `tSound` GetPosition, SetVolume, UpdatePosition by emanation model, the two
  volume/pan entry points, `UpdateAveragePosition` (the weighted average of an instance and its children, then the
  outward move), ShouldPauseForDistance, PauseForDistance, the distance and simulation pause policies and Resume with
  its holder and buffer calls. §8: the HIT debug info (`cSourceFileDebugInfo`) line/offset maps.
- docs/specs/questions/audio.md: AU50 annotated as partly answered (the manager-side gates stay open).
  QUESTIONS.md regenerated.

### Coverage

- tools/coverage-map.tsv: `tSound` and `cSourceFileDebugInfo` rows filled.
- Specified bytes 5,058,611 → 5,061,522 (+2,911, 90.6%, `cov_local.py` with owners-c1).

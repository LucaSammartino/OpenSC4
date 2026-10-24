| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-introvid7: the intro video screen `cSC4WinIntroVideoScreen`, new spec intro-video.md (RE spec lane) | `re-batch7` branch (outside contributor checkout, fork amarret/OpenSC4; on main `5c0ff782`, after re-advlist7) | **ready** (docs only). intro-video.md §1–§5: fields, `Init` and `Shutdown`, the boot sequence (two movies, the Maxis logo, the splash picture, the region screen) with its messages and timings, the skip rule, the player functions reached only from here (notification, movie queue, destructor), four conformance cases; questions IV1–IV2; owner claimed: `cSC4WinIntroVideoScreen`: specified +3,040 bytes (+22 functions) | IV1–IV2 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 22 claimed functions were disassembled: the screen's 12, and 10 `cSC4MadCowPlayerWin` functions that the
coverage tool assigns here by reach. The two movie keys were read from the data section (`0x00B0DB60`,
`0x00B0DB48`). Written on 2026-10-23. No upstream lane covers this owner; framework-gz-services.md GZ4 already cited
its deferred posts.

### What changed

- New docs/specs/intro-video.md.
- Read on the bytes:
  - the player's size and placement (768 × 384, centred) and its sound setting from preference byte `+0x69`;
  - the movie keys `{0x0A8B0E70, 1, 2}` (flags 3) then `{0x0A8B0E70, 1, 1}` (flags 0);
  - the five messages that chain the screens;
  - the logo (fade 1000/1000, hidden after 5 s, deleted after 6 s) and the splash (fade 2000/0, region screen
    after 2 s);
  - the self-repainting countdown;
  - Esc or a click skipping only the first movie;
  - the player's notification setter, its movie queue (`std::deque` of `{flags, object}`) and destructor.
- docs/specs/questions/intro-video.md: IV1–IV2 (new prefix IV); QUESTIONS.md regenerated.

### Quirks found

- The splash branch posts `0xCAAC6A52` on every paint after 2 s, until the screen is gone.
- Only the first movie can be skipped.

### Coverage

- tools/coverage-map.tsv: `cSC4WinIntroVideoScreen` → intro-video.md, before the `cSC4Win*` catch-all.
- Specified bytes 4,536,510 → 4,539,550 (+3,040, 81.2% → 81.3%, `cov_local.py`, on `5c0ff782` + the sixteen lanes
  before).

### Not covered

- The MAD decoder and the rest of `cSC4MadCowPlayerWin`; IV1–IV2.

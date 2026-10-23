| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-sndsys: the framework sound system, sound manager and the sound director's classes (data-source creator, property and instance caches, default event handler, XA/WAV/UTalk data sources), new spec sound-framework.md (RE spec lane) | `re-batch6` branch (outside contributor checkout, fork amarret/OpenSC4; written on main `a4f059a7`, rebased on `15c9ce9e`, after re-radio) | **ready** (docs and RE tables only). sound-framework.md §1–§7: the director's eight classes, the sound thread (20 ms callback timer), event queue and handlers, fast/slow targets (250 ms), renderers and listener, extension map, the sound manager's registries and creators read on the bytes (its export fails), the XA ADPCM decoder with its tables, the WAV and UTalk headers; sound-system.md §8 corrected (PreAppShutdown); audio.md AU8 and AU9 answered; questions SY1–SY4; 20 names (15 new); `cGZSnd*` and `cGZSound*` claimed: specified +28,987 bytes (+340 functions, measured on `a4f059a7`) | SY1–SY4 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`),
the Ghidra export of the `cGZSnd*`/`cGZSound*` owners and the SDK headers; `cGZSndMgr` was read entirely on the
bytes (33 methods) because every method but `QueryInterface` fails to decompile. Written on 2026-10-22.

### What changed

- New docs/specs/sound-framework.md. Byte-checked: the `cIGZSndSys` table (`0x00AD7180`, 51 slots); the sound
  manager's constructor, field offsets, service ID `0x1AF85BFB`, creator ordering (front insert) and
  `GetSound`'s name form calling `nRZHash::CRC24` (`0x00932F76`); the XA decoder's coefficient, nibble and rounding
  constants (`0x00B170E8`, `0x00B170F8`, `0x00B17108`, `0x00A84D2C`, `0x00AD74A8`).
- radio.md §2.1 corrected: the radio's control events are dispatched at once (sound-system slots 7 and 5 are the
  dispatch-now helpers), not queued.
- audio.md: §2 framework row points at sound-system.md and sound-framework.md; AU8 (Hit, freshness, radio) and AU9
  (the `0xFF000000` tag) marked answered.
- docs/specs/questions/sound-framework.md: SY1–SY4 (prefix new); QUESTIONS.md regenerated.

### Rebase onto `15c9ce9e` (2026-10-23)

Upstream landed its own docs/specs/sound-system.md (lane `re-sound-hit5`), the interface contract for `cGZSndSys`,
`cGZSndMgr`, the three directors, five HIT classes, three freshness classes and `cGZPlayList`. This lane's spec was
therefore renamed **sound-framework.md** (questions file likewise; prefix SY kept, upstream's is SS) and now states
its relation to sound-system.md. One disagreement was re-read on the bytes and fixed in sound-system.md §8:
`cGZSoundCOMDirector::PreAppShutdown` (`0x0096653F`) calls `cIGZSndSys` slot 34 (`call [edx+0x88]` at `0x00966551`,
`DisableUpdates`), not `Panic` (slot 47); the table tail is 48 `SetTestMode`, 49 `GetTestMode`, 50
`GetSoundSystemThreadId`. Upstream's specific rows (`cGZSndSys`, `cGZSndMgr`, `cGZSoundCOMDirector`) stay on
sound-system.md; this lane keeps the `cGZSnd*` and `cGZSound*` catch-alls.

### Names and coverage

- re/symbols/sound-framework.tsv: 20 names (15 new: the timer callback and message handler of the sound system, the
  segment data-source creator, the property cache, the default event handler, the XA and WAV file data sources;
  5 confirm auto names). Delta (−5 +20) identical to `merge_names.py`'s.
- tools/coverage-map.tsv: `cGZSnd*` and `cGZSound*` → sound-framework.md. Specified bytes 3,563,529 → 3,592,516
  (63.8% → 64.3%, `cov_local.py`, on `a4f059a7`); audio 63.0%.

### Questions

- New: SY1 (timer thread call), SY2 (listener factors), SY3 (instance cache), SY4 (XA header bytes).

### Not covered

The MP3 data source (`0x3CEC2B47`, not in these owners), the EA SNDSYS renderer (next lane), `tSound` (the SC4
audio lane).

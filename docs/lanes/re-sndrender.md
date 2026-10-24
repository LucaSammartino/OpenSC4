| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-sndrender: the DirectSound back end registered by `GZSndSysMacCOMDirector` (renderer, listener, 2D and 3D sounds, 3D helper), new spec sound-renderer.md (RE spec lane) | `re-batch6` branch (outside contributor checkout, fork amarret/OpenSC4; written on main `a4f059a7`, rebased on `15c9ce9e`, after re-sndsys) | **ready** (docs and RE tables only). sound-renderer.md §1–§8: the five classes, renderer Init (device enumeration and choice, primary buffer 16-bit/22050 Hz/stereo, cooperative level 3→2→1), the listener, the 2D sound's static-buffer load, buffer flags by priority, the volume (square-root curve to −60 dB, −100 dB at zero), pan and frequency conversions, fades, the 3D distance table; questions SRN1–SRN3; no names (naming split the owner, see Report); the `GZSndSysMacCOMDirector` owner claimed: specified +16,829 bytes (+246 functions, measured on `a4f059a7`) | SRN1–SRN3 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`),
the Ghidra export of the `GZSndSysMacCOMDirector` owner (`0x008747B0`–`0x008792D0`), the public DirectSound API and
the SDK headers. Written on 2026-10-22.

### What changed

- New docs/specs/sound-renderer.md. Byte-checked: the director's five CLSIDs and sizes; the volume path
  (`0x00876894`: clamp, ×1/1024 at `0x00AC2C10`, `sqrt` `0x009F1D04`, ×1024 at `0x00AC2C08`, master scaling, then
  `0x00875FFF`: −10000 / 0 / `w × 6000 / 1024 − 6000`); the pan formula; the frequency clamp; the distance table at
  `0x00B11DE0` and the ×100 at `0x00A80AA0`.
- audio.md §2: the framework row also points at sound-renderer.md.
- docs/specs/questions/sound-renderer.md: SRN1–SRN3 (prefix new); QUESTIONS.md regenerated.

### Names and coverage

- No symbol table. Giving the back end's functions class names (renderer, listener, 2D, 3D, helper) split the
  owner and left 65 functions unattributed by the coverage owner rules (neighbour/reach), so the names were dropped
  and the owner is claimed whole.
- tools/coverage-map.tsv: `GZSndSys*` → sound-renderer.md. Specified bytes 3,592,516 → 3,609,345 (64.3% → 64.6%,
  `cov_local.py`, on `a4f059a7`); audio 70.4%.
- Rebase onto `15c9ce9e` (2026-10-23): upstream's coverage map gained an exact `GZSndSysMacCOMDirector` row on
  sound-system.md, whose §8 covers only the director's constructor, ID and `OnStart`. That row now points at this
  spec, which covers the whole owner (the director in §1, the back end in §2–§8); `GZSndSys*` is back to `-`.

### Questions

- New: SRN1 (the 1–6 level), SRN2 (the 1/127-scaled 3D value), SRN3 (the play-position formula).

### Not covered

Streaming (every 2D sound is loaded whole into a static buffer here); the 3D helper's DirectSound listener calls in
detail; the EA SNDSYS/AEMS library proper (`0x00864000`–`0x00874000`, other owners).

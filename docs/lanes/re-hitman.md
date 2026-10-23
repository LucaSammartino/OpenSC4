| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-hitman: the Hit sequenced-sound engine (HitMan, track players, channels, TKDT tracks, Hit lists, sems, the Hit INI), new spec hit-music.md (RE spec lane) | `re-batch6` branch (outside contributor checkout, fork amarret/OpenSC4; on main `a4f059a7`, after re-batch5) | **ready** (docs and RE tables only). New docs/specs/hit-music.md (§1–§14): identity, the INI, the HitMan service and its 47-slot table, the resource factory, TKDT text/binary codecs and the 27 fields, the default track and channel tables, the Hit list shuffle bag, registers and the 80 aliases, the track player (Play, start, tick, fades, pitch spread, dependents, Stop/Kill), the 97-entry bytecode, the channel, the sems, creators and debug service; questions HM1–HM8; 45 names (29 new); `cHit*` claimed: specified +48,181 bytes (+459 functions) | HM1–HM8 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`),
the Ghidra export of the 17 `cHit*` owners and the SDK headers. Written on 2026-10-22 as the first of the audio-backlog
lanes; [audio.md](../specs/audio.md) named Hit as boundary only (AU8).

### What changed

- New docs/specs/hit-music.md. Byte-checked: the director's ten classes and the service/interface IDs; the
  `cIHitMan` vtable (`0x00AD2968`); the 54- and 29-pair default tables (`0x00B16B38`, `0x00B16CE8`); the 80-entry
  register alias table (`0x00B169F8`); the `DoCommand` jump table (`0x0093DC8B`) with every opcode's operand widths,
  and the opcodes whose export was wrong or ambiguous (`And` = `Greater`, `Or`/`Xor` bitwise, `Div` = `idiv`,
  `SetTargetLocal` reading the local set, the no-op and illegal slots, `CheckedRegId`); `NoteOn` ignoring its
  argument; the pitch ratio `2^(1/12)` and the random spread; the Hit list's add/remove/choose/window rules; the 3D
  sem's duplicated test; the channel's 100000.0 distance default.
- audio.md §2: the Hit row points at hit-music.md.
- docs/specs/questions/hit-music.md: HM1–HM8 (prefix new); QUESTIONS.md regenerated.

### Names and coverage

- re/symbols/hit-music.tsv: 45 names (29 new: the default error handler, the control-group ctor, the player's
  compare/register/enumeration helpers, the channel's register setters, the sems' Apply/Setup, eleven `cHitList`
  methods; 16 confirm auto names). The `names_windows.tsv` delta (−16 +45) is identical to `merge_names.py`'s.
- tools/coverage-map.tsv: `cHit*` → hit-music.md. Specified bytes 3,478,316 → 3,526,497 (62.3% → 63.1%,
  `cov_local.py`); the audio area is now 33.9% specified.

### Rebase onto `15c9ce9e` (2026-10-23)

Upstream landed docs/specs/sound-system.md (lane `re-sound-hit5`), whose §3–§5 are the interface contract for five
of the 17 owners (`cHitTrackPlayer`, `cHitMan`, `cHitChannel`, `cHitIniFile`, `cHitPropertySetTrackResource`) with
their own coverage rows; `cHit*` → hit-music.md now claims the other twelve. hit-music.md states the relation. Two
overlaps were re-read on the bytes: `Play`'s retrigger calls `Kill` (sound slot 9, `0x0093E16E`; hit-music.md §6.2
said `Stop`, fixed) and writes registers 0–3 silently (register slot 3 with argument 1, `0x0093B346`;
sound-system.md §3.2 said "with notification", fixed). The numbers above were measured on `a4f059a7`.

### Questions

- New: HM1 (control-group gate), HM2 (tick registration and units), HM3 (the resume path), HM4 (three TKDT fields
  with no Hit reader), HM5 (unused HitMan pause slots), HM6 (default error handler), HM7 (the shipped INI and `.hit`
  layout), HM8 (pitch-spread compounding).

### Not covered

The `.hit` file layout beyond the entry-point table, the shipped INI's contents, the framework sound objects' slot
semantics (sound-system spec), the radio and freshness layers.

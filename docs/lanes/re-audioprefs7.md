| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audioprefs7: the audio options dialog `cSC4AudioPreferencesWinProc`, new spec audio-preferences.md (RE spec lane) | `re-batch7` branch (outside contributor checkout, fork amarret/OpenSC4; on main `5c0ff782`, after re-modelmaker7) | **ready** (docs only). audio-preferences.md §1–§10: identity and lifetime, the statics and song list records, the audio preference record, the song tables (load, defaults, file test, custom tunes, enabled test, exclusion lists), the volume mappings, opening and closing, the message and control tables, quirks, nine conformance cases; questions APW1–APW3; owner claimed: specified +11,812 bytes (+26 functions) | APW1–APW3 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 26 functions of the owner and the shared song-table loader `0x004F2910` were disassembled. The tuning exemplar
"Audio UI Settings" and the two song tables were read from `SimCity_1.dat` with `tools/dbpf.py`. Written on
2026-10-23. No upstream lane covers this owner. The receiving side of its messages is upstream's audio.md §7.2.1, and
this spec's message words match it (data1 1 → `krgn`, 0 → `kmay`).

### What changed

- New docs/specs/audio-preferences.md.
- Read on the bytes:
  - the 31-ID song checkbox ranges `0x2A5BA8A7`/`0x3A5BA8A7`;
  - the jump table of `HandleControlMessage` (`0x004F4248`, IDs `0x8A550C50`–`0x8A550C5A`);
  - the slider mapping with the binary32 −0.001 constant (`0x00A94260`) and its inverse with 1000.0 (`0x00A8FAF0`);
  - the floor test that stores 0;
  - the folder strings, from the static constructors `0x00A72160`–`0x00A724F1`;
  - the song entry layout from `LoadSongTable`.
- Shipped data: music floor 450 and effects floor 173 (ceilings 1024, fade 4000); 10 Region songs (one off by
  default) and 25 Mayor songs.
- docs/specs/questions/audio-preferences.md: APW1–APW3 (new prefix APW); QUESTIONS.md regenerated.

### Quirks found

- `LoadSongTable` carries a song's "enabled" value over to the next song that has no third value.
- A slider at the floor mutes.
- The open-time slider position compares the volume with 1000.
- A volume below the floor zeroes the saved copy, so Cancel stores 0.
- The song checkbox ranges are 31 wide, and ID 30 writes a neighbouring byte.
- The callers ignore `ShowDialog`'s result.
- Cancel does not rebuild the radio's exclusion lists.
- `EndDialog` runs twice per close and repeats the preview-end post.

### Coverage

- tools/coverage-map.tsv: `cSC4AudioPreferencesWinProc` → audio-preferences.md, before the `cSC4Audio*` catch-all
  (which matched it first).
- Specified bytes 4,433,407 → 4,445,219 (+11,812, 79.4% → 79.6%, `cov_local.py`, on `5c0ff782` + the four lanes
  before).

### Not covered

- The volume scale (APW1).
- The reader of the list record's `+4` word (APW2).
- The song buttons' style calls (APW3).
- The radio's own handling of the posted messages (audio.md §7.2.1).

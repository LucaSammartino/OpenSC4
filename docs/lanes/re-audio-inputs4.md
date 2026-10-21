| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audio-inputs4: answer audio-ambience AA5-AA9 (alarm defaults, margins/layer ids/prefs/zoom, filter iteration, instance ids, message results) | `re-audio-inputs4` / (spec lane) | in progress | - |

## Handoff
- Goal: answer AA5-AA9 in docs/specs/audio-ambience.md (sections 2.2, 3.1, 3.2, 3.5, 3.6, 4, 5, 6, 7), move to answered/, questions.py index.
- Done: lane file; all RE for AA5-AA9 gathered (facts below), spec edits not yet written.
- Next: write the facts into the spec sections, move AA5-AA9 to answered/, questions.py index, finish.
- Facts: AA5 DIFFERS: last := t at creation (0x004EA757-0x004EA786, same hour+min/60 formula), on=0, kind=arg; gate IsFlagSet(5) && !IsFlagSet(0x10) (0x004EA6AB/BB) else return 0; null clock crashes; entry appended at tail.
- AA6: margins/ids are std::vectors (ctor null); GetPropertyValue 0x005FFCC0/0x005FF510 absent -> untouched, present -> exactly n values; missing+empty vector -> null write (crash); prefs/zoom globals not null-checked (crash); zoom unbounded read.
- AA7: iterator at +0xB4 prime mode (bytes state1,mode1,dirty1; current +0xDC=0, step +0xE0=1), Reset(0) no rand -> current carries over; rect = current>>1 (SAR) inclusive; weather.md 6.7. HandleCell: cell-outer, layer-inner.
- AA8: (a) yes loop0; (b) DIFFERS move set-pos instance = occupant ptr (0x004E8DCE, 0x004E8CB2), 6A36AFAC tested via holder slot +0xC; (c) nothing; (d) occ+reread value via _AddOccupantSndInstance(loop1), absent nothing; (e) DIFFERS instance = automaton iface ptr, flag 1 (0x004E8A64..73); _PlayOccupantSound start flag 1 (0x004E87C8); (f) no preview check in automaton handlers but _AddOccupantSndInstance itself checks flags 5 + preview (0x004E8AE1-0x004E8B14).
- AA9: focus 0x004E8CF0 returns 1, stage := flag?1:4, Pause(!flag) slot +0x70; 0x2A2C0384 QIs the event itself, fail -> return 1; master vol returns 0 (0x004E9C04); release 0x004E9660 returns 1, layers only (Shutdown+Release), alarms kept.

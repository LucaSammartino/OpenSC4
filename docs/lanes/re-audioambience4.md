| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audioambience4 (RE spec: audio ambience AA2/AA3, AA1 most) | `re-audioambience4` / (RE lane) | **ready** (spec only, no engine/ABI change): audio-ambience.md 3.4 (AA2 answered), 3.6 (AA3 answered), 8.1-8.5 and 8.7 layer Init/tuning/filters/filtering/group layers/Update/loops, tests 7-14 | AA1 rest: _UpdateAmbienceScores 0x004E5920 (8.6), ease step 0x004F8650, director +0x54/+0x11C; AA4 class of 0xC7CC75F1 |

## Report

- REA was unavailable both rounds ("No app is open"); everything came from re_query.py asm and decomp/,
  numerics checked on the instructions; constants read from bin/SimCity4.exe .rdata/.data with pefile
  (0x00A81228 = 1.0, 0x00B09818 = 1.0, 0x00A81054 = 0.0, 0x00A92480 = 0.33333334, 0x00A9332C = 0.0001;
  0x00B2194C is bss and never written, so 0.0).
- Round 1-2 (71582dec, 917e998f, f296193c, f05b37e9, 51785d04): AA2 answered (DIFFERS, lot condition-change
  sounds), AA3 answered (MATCHES), 8.1 layer Init, 8.2 tuning, 8.3 filter creation, tests 7-10.
- Round 3: 8.4 (4a549f1f) filter interfaces 0xEA40074F global / 0xCA400757 cell / 0xCA400753 occupant, all at
  slot 5; kinds 1 all / 2 none / 3 any; CanHandle cached per cycle behind the audio-preference byte;
  HandleCell/HandleOccupant counters +0x124..+0x134 and group insertion; cGroupLayer (radius, strict distance,
  count/peak, Clear/ClearEmptyGroups); _CreateLayers roles of +0x20/+0x24/+0x34. 8.5 (07372d1c) Update:
  groups by +0x34 incl. Intersect, score value +0xAC by +0x38 (6 modes), 3x3 sample grid, weighted average
  position. 8.7 (64754fd9) loops: target volume from curve +0x1C0, ease object +0xE8, start/stop/volume/
  position events per loop entry. Tests 11-14 and the AA1 note in the questions file in the final commit.
- Still open (AA1 stays open): 8.6 _UpdateAmbienceScores 0x004E5920 (+ _UpdateFreshnessParameters
  0x004E4AA0), ease time step 0x004F8650, director +0x54 member field +0x11C. Owner coverage is not complete.
- AA4 not pursued this round (budget). No ABI rows proposed. No game runs (spec lane).

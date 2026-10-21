| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audioambience4 (RE spec: audio ambience AA1-AA3) | `re-audioambience4` / (RE lane) | **ready** (spec only, no engine/ABI change): audio-ambience.md 3.4 (AA2), 3.6 (AA3), 8.1-8.7 complete incl. scores/freshness, ease step, mode 6 (AA1 answered); tests 7-18 | AA4: name the class of type 0xC7CC75F1 (ctor 0x005628F0, callers all unnamed) |

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
- Superseded by round 4: still open (AA1 stays open): 8.6 _UpdateAmbienceScores 0x004E5920 (+ _UpdateFreshnessParameters
  0x004E4AA0), ease time step 0x004F8650, director +0x54 member field +0x11C. Owner coverage is not complete.
- AA4 not pursued this round (budget). No ABI rows proposed. No game runs (spec lane).
- Round 4 (final, 4529dba3): 8.6 _UpdateAmbienceScores 0x004E5920 + _UpdateFreshnessParameters 0x004E4AA0
  (curve x +0x1E8 >> 10 logical; play percent scaled 50/75% by +0x204; per-score start/stop/divisions/volume/
  percent; d = u32(+0x1F0) x 0.01f x Z(0x00B21A30, listener zoom-5 scale) stored f32, data2 x16.0). 8.7 ease:
  +0x108/+0x10C are per-ms rates; Step 0x004F8650 moves min(remaining, t x rate), vector normalised (mixed
  f32/extended rounding cited), scalar integer, sets flag +0x38. 8.5 mode 6: director 0x00B21978 +0x54 is only
  zeroed (ctor 0x004EC72D), never written -> original faults reading 0x11C. AA1 moved to answered; tests 15-18.
- All from re_query.py asm (no REA used); constants 0x00A823A4=0.01f, 0x00A8D45C=16.0, 0x00B098C4=1.0,
  0x00A80AA8=2^32 read with pefile.
- AA4 still open (not cheap: all 10 callers of 0x005628F0 unnamed, vtable has no slot refs).

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audioambience4 (RE spec: audio ambience AA2/AA3, AA1 part) | `re-audioambience4` / (RE lane) | in progress (round 3/3) (spec only, no engine/ABI change): audio-ambience.md 3.4 condition-change table (AA2 answered), 3.6 alarm `last` = t (AA3 answered), new 8.1-8.3 layer Init/tuning/filters, tests 7-10 | AA1 rest: CanHandle 0x004E5110, HandleCell/HandleOccupant, _UpdateAmbienceScores 0x004E5920, _UpdateAmbienceLoops 0x004E5D50, Update 0x004E6340; AA4 class of 0xC7CC75F1 |

## Handoff

- Goal (round 3/3): finish AA1 in audio-ambience.md: CanHandle 0x004E5110, HandleCell 0x004E5370,
  HandleOccupant 0x004E5670, _UpdateAmbienceScores 0x004E5920, _UpdateAmbienceLoops 0x004E5D50,
  Update 0x004E6340; field roles +0x20/+0x24, +0x1E0-+0x204; AA4 class 0x005628F0 / vtable 0x00A9D488 if cheap.
- Done earlier: 71582dec, 917e998f, f296193c, f05b37e9, 51785d04 (AA2/AA3 answered, 8.1-8.3, tests 7-10).
- Next: 8.4 filtering (4a549f1f) and 8.5 Update done; now 8.6 _UpdateAmbienceScores 0x004E5920, 8.7 _UpdateAmbienceLoops 0x004E5D50 via asm (REA down).
- Facts: REA was unavailable in round 2; use re_query.py asm --bytes for numerics.

## Report

- REA was unavailable ("No app is open"); everything came from re_query.py asm and decomp/, numerics checked on
  the instructions (stack depths traced by hand; build-runs/stk.py is a crude helper, not trusted alone).
- **AA2 answered** (DIFFERS, no engine rule was stated): `_UpdateLotOccupantsForConditionChange` 0x004E9DA0,
  args (lot, old = msg data1, new = msg data2) from cSC4Lot::SetCondition 0x006C26D0; insert/init call it with
  (-1, -1) so new = lot.GetCondition(). Per building/prop occupant of the lot: new 0/5/6 stop 0x0A4C13CB, start
  0xCA19D7CA; 2/3 the reverse; 1/4 stop both; new 0 with old 1 also starts 0x4A4C132E (never stopped, not
  even on remove).
- **AA3 answered** (MATCHES): the copied slot at 0x004E91B0 is t.
- **AA1 partly specified**: 8.1 layer Init order, 8.2 tuning (exemplar 0x6534284A/0xCA25875D/layer id, 22 keys
  with reader type, destination, absent value; ease times unsigned x 0.001), 8.3 filter creation (3 kinds,
  process-wide cache, interface counters, per-kind lists). Field roles beyond that remain AA1.
- **AA4 narrowed**: 0xA823821E = props; 0xC7CC75F1 written at +0x28 by the cSC4TrafficAutomaton-derived ctor
  0x005628F0; class name open.
- No ABI rows proposed. Commits: 71582dec, 917e998f, f296193c, and the final one.

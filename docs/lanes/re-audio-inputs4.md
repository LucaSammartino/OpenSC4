| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audio-inputs4: answer audio-ambience AA5-AA9 (alarm defaults, margins/layer ids/prefs/zoom, block iterator, instance ids, message results) | `re-audio-inputs4` / (spec lane) | ready: spec commit f868d3fe; AA5-AA9 answered, all DIFFERS from core-audioambience4's engine assumptions; no ABI, no game run; AA4 left open | 1. An engine lane conforms the ambience owner to AA5-AA9 (alarm `last` at creation, vectors, prime-step iterator order, move/railway instance ids, flag 1 starts, focus stage 4, master-volume result 0, alarms kept at shutdown). 2. AA4 (type `0xC7CC75F1` class) still open. |

## Report

Spec: docs/specs/audio-ambience.md §2.2, §2.3, §3 (intro: `_AddOccupantSndInstance`/`_Remove...` in full),
§3.1, §3.2, §3.5, §3.6 (alarm creation), §4.1, §4.2, §5 (iterator, cell-outer HandleCell, worked example),
§6, §9 row, §10 tests 19-24. Questions AA5-AA9 moved to questions/answered/audio-ambience.md; QUESTIONS.md
regenerated. Sources: re_query.py asm (export) and decomp/; REA was not needed.

Findings (all DIFFERS from the engine's current choices):
- AA5: `_AddAlarmOccupant` (`0x004EA690`) sets `last` = current hour (+ unsigned min/60), `on` = 0; it refuses
  occupants without flags 1 and 4 or with flag 0x10; appends at the tail; null clock is not checked.
- AA6: margins/ids are std::vectors; absent property leaves them unchanged, present gives exactly n values;
  the 70.0 fill writes through the begin pointer without resizing (null on first read); prefs global, view
  global and zoom index unchecked. Replacement defaults marked [choice].
- AA7: prime-step RandomIterator, inclusive halved bounds, `Reset(0)` no rand, current 0 at construction and
  carried over; step from weather.md §6.7; example count 20 → step 7, first block (2, 1).
- AA8: (a) yes, loop 0; (b) move set-positions use the bare occupant address (quirk: moved sounds do not
  follow); (c) nothing; (d) occupant + re-read value, absent → nothing; (e) instance = automaton interface
  address, flag 1; (f) no preview check in automaton handlers; automaton/railway starts pass flag 1.
- AA9: focus returns 1, stage := 1/4, Pause(!flag); `0x2A2C0384` QIs the event itself, non-answering → 1;
  `0x2A5B69C0` returns 0; `0x004E9660` returns 1, empties layers only (alarms survive).

No open questions filed.

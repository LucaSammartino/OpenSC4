| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audio-au50 (audio owner contracts) | Codex / `re-audio-au50` | in progress | Answer AU45–AU49 and AA14; doccheck |

## Handoff

- Goal: close core-audio-close11's six RE questions with prose contracts and conformance cases.
- Done: bb6273807 initial handoff; debbb740c AU45 vector correction and source-question archive.
- Current: radius/play corrections and director 11.3 written; AU47 archived on 2026-10-24.
- AU46/AU49/AA14 patches did not land (heading mismatch, then guard denial).
- Next: review/complete 11.3, archive AU47 from core-audio-close11, then finish remaining four
  questions. Regenerate tools/questions.py index; run python tools/doccheck.py check (NOT run).
- Sound facts: reset QI BB1BE367 then +30 then QI FD584467, rewrite property15 unchanged.
  Enumeration payload+80 QI EA4FC425, callback+C(instance), false stops but returns true.
  Volume wrapper 4F9D20 reads payload+38 volume/+24 ID; inner4F9AF0 signed volume*gain,
  truncation, property15(value,0), flag10. Completion registration QI E9D00A9A +C(manager+8/null).
- Radius: 4F9FCA signed FILD then *f32 .01 *f32 100; FST first radius then *4 FSTP second.
  Play4FA037 sets property10=3 with third arg0; holder play uses manager master volume.
  SetPosition4FA260 found ID stores xyz, updates self/master, distance pause, flag2; missing false.
  UpdatePosition4F9050 writes trunc(x/y/z) as property20/21/22 with flags1/1/0.
- Layer: failed first Init retains flag/count, repeated Init succeeds without retry; Shutdown
  balances. CreateFilters resets interface counters/bits but retains pointer lists/kind counts.
  Destructor4E76E0 does not Shutdown. CanHandleOccupant5DEB40 always false. RemoveLoop no stop;
  bulk loop removal sends stops. Freshness loading takes group/instance pairs, ignores odd tail.
- AEMS: Init latchB21938 set before work; negative backend stages call Shutdown. Shutdown clears
  latch/stops/removes banks, critical removesystem, restore, clears work vector/unregisters.
  Destructor4E1920 no Shutdown. AddBank backend0 means success. Bank4E2450 frees rawbuffer/heap.
- All six question bodies read via git show core-audio-close11. No merge queue: incomplete.

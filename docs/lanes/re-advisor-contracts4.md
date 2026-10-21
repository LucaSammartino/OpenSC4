| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-advisor-contracts4: answer advisors AD1-AD3 (window procs, private global consumers, map consumers) | re-advisor-contracts4 / .claude/worktrees/re-advisor-contracts4 | ready: AD1-AD3 answered in advisors.md, commit 5006679d | glue-advisors next round: decide window route (run original ctor+Init vs no window) |

## Report

What changed (docs/specs/advisors.md, commit 5006679d):
- §4.1 step 10 and §4.6: the exact window creation / removal calls (own-vtable slot 24 at 0x41B7DD, slot 28 at 0x41A423).
- §12.1.1 (new): bodies of slots 23-28. System member +0x5C holds the cSC4AdvisorUIAdvisorWinProc (0x70 bytes).
  Slot 24 allocates it with the game heap and calls ctor 0x41DB50 directly (no CLSID, single caller), then Init
  0x41E320 (script window 0x96A006B0/0xCBC905CD). 25/26 show/hide via 0x41D650 / 0x41D710 (byte +0x56), 27 returns
  +0x56, 28 tears down (0x41D830) and releases. Callers outside the system: View3D only (slots 25-27, five functions).
  Literal "no window" (null/no-op/false) is NOT the original: AD1 answered DIFFERS.
- §12.2: container layout (SGI _Rb_tree: map {header*, count}; node {color, parent, left, right, key +0x10,
  value +0x14}; advisor map at system +0x4C, active map at advisor +0x70) and the four read-only outside consumers
  (MySimAgentSimulator::Init 0x42EC30; advisor win proc GetMessageList 0x41E000 / get_advisor_mood 0x41DC20;
  MySimWinProc::GetMessageList 0x76F260). No outside insert/erase/operator[]: AD3 MATCHES read-only.
- §14.1 UI row: traced slots per consumer. §14.2: byte scan of 0x00B207C0 = 66 accesses (3 writes, 63 reads), all
  in the family + Lua extensions; none in UI/popup/citysit/MySim/tutorial/View3D/HTML; reads call slots only. AD2.
- Questions AD1-AD3 moved to docs/specs/questions/answered/advisors.md; QUESTIONS.md reindexed.

Method / evidence: REA unavailable (batch_decompile: "No app is open"); used re_query.py, decomp/, a byte scan of
bin/SimCity4.exe (export xrefs list 0 rows for 0x00B207C0) and a register trace from every CALL [R+0x144]
(cISC4City::GetAdvisorSystem). Scripts: build-runs/scan.py, use207.py, track2.py, cooc.py (outputs *.txt there).

Limits kept precise: the trace sees only consumers that fetch the system through the city getter; a consumer caching
the pointer elsewhere would be missed (none calls the tree helpers either). 4 of 63 global reads were not traced past
the load (0x40A906, 0x41429B, 0x4169EB, 0x421ABC; all family code).

Next: glue-advisors round 2 can plan without blockers; the window choice (run original 0x41DB50 + 0x41E320 from
the shim, or accept an empty View3D advisor panel) is a [choice] for the glue lane.

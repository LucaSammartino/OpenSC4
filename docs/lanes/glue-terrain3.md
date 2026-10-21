| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-terrain3 (enable = terrain glue over the TR21 3D view contract, terrain.md §15.5) | `glue-terrain3` / (engine lane) | in progress | see Handoff |

## Handoff

- Round 2 committed implementation: c1b06da3 view/raw-record contract and 25a4ec76 slot-96 terrain Load. Code was committed; the no-code parking condition does not apply.
- Full gate PASS (cargo/clippy/build and 62/62 ctests). gt3-repl-4 reached the real game after queue-environment repair but is corrupt before a game date, foreign_exceptions=1, ref_warnings=0.
- NOT ready for merge or replacement coverage; no regress claimed. Next: diagnose gt3-repl-4 foreign exception; finish Save and unresolved TR33-TR37 contract details without guessing.
- The CLI killed this round after its 10-minute background wait ceiling, without a final lane result; do not treat the absence of a result as a guard permit or a successful validation. Coordinator disabled the CLI ceiling for future launches.

## Prior Handoff
- Goal: `enable = terrain` clean to 2032 (turbo) per terrain.md §15.5.1/.4/.5/.7/.8/.11; then a regress-style run with terrain added; coverage evidence.
- Done: worktree from main ea097498, lane file d98dc80d. Baseline `qbuild all` PASS: cargo 3576/0, clippy 0, ctest 62/62. Round 2: plan steps 1-4 done in c1b06da3 (Direct::TerrainView3DInit 0x0075C420 + direct-calls.md; T+0x6C/W+0x18 0x24-byte mirrors synced at outermost Leave after HostPost; DoPostCityInit per §15.5.1; GetView = base+0x0C; questions TR33-TR37 filed). qbuild quick PASS.
- Cause of glue18's crash (run glue18-repl-9, eip 0x41800000), found by reading cFC4Terrain.cpp DoPostCityInit (~line 846): view made with GetClassObject(CLSID, GZIID_cIGZUnknown) and GetView (slot 66, line ~541) returns that pointer; spec says IID 0x6771477D, store out-0x0C as base in f_.view, slot 66 returns base+0x0C (§15.5.6). View Init (0x0075C420) never called. f_.vertices (T+0x6C) is never filled (null). Water W+0x18 array missing.
- Next: (a) plan step 5, slot 96 Load / 97 Save via the DB segment (keys below, pattern cFC4Flora.cpp ~700-760, fc4_terrain_load / fc4_terrain_write_record); (b) game run gt3-repl-N (enable = terrain, turbo, end_date 2032-01-01): gt3-repl-1 and -2 were harness-error (tools/ingame_run.ps1:326 Get-FileHash not recognized; infra, not ours) - resend; (c) regress-style run, qbuild all, queue.
- Keys: misc 0x8A91E7E3/0xE98F9525/3, alt 0xA9DD6FF4/0xE98F9525/1, props 0x8A91E7E0/0xE98F9525/2; edges 0x64+side.
- Not done by choice (questions): §15.5.1 step 7 -> whole-city redisplay (TR33); +0x100 slot 5 (TR34); view Shutdown (TR35); record +0x16 texture = 0 (TR36); SetTriangleIndicesForCell not called (TR37). QUESTIONS.md next-free for terrain must become TR38 (coordinator; I may not edit it).
- Regress: tools/ is off-limits; coordinator must add `terrain` to the regress replace list and a coverage map replace key.

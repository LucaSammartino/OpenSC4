| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netrules-impl4: wire fc4_network_rules into our network tool | fc4-lane / .claude/worktrees/glue-netrules-impl4 | in progress (context budget; restart) | see Handoff |

## Handoff

Goal: per network-rules.md 2.4 (NR13), run fc4_network_rules_* as a component of our network tool
(engine/plugin/FC4NetToolHost.cpp, active under `enable = netmgr, nettool`), no netrules key; prove in game.

Done:
- e33c545c lane file. Baseline `qbuild all`: test 3896 passed, clippy 0, build 186/186, ctest 63/63 PASS.
- 9772084d SolvePieces (after fc4_netgeom_place in ValidateAndSolve: geom cell infos pad-2 -> rules grid, S = geom
  solved cells, fc4_network_rules_solve(type, resolve=false), result logged not adopted) + Rules() loader (OpenDBRecord/
  CloseDBRecord: tiles per type, INI 0/8A5971C5/8A5993B9, RUL1, RUL2, RUL0) + InsertOccupantsForSolvedCells logs ours vs
  occupant PieceId()/GetRotationAndFlip() readback (no public piece setter -> question NR15, NOT yet filed).
- Game run nrimpl4-a1 (verdict exited, code -1; DO NOT reuse label): solver worked (rules loaded tile_files=24
  missing=0 tile_rules=6295 ini=1 rul=3/3 intersection_rules=249; road (4,4)..(14,4): end piece 0x200/03, straight
  0x2600/03; street 0x05000200 / 0x05002600; occupant readback 0/0) but AV after insert and at Shutdown: piece id used
  as pointer. Cause: I had grown NetToolContext in FC4NetToolHost.h; cFC4NetworkTool.cpp.obj was NOT recompiled (no
  header dep tracking), so solvedPieces overflowed into cFC4NetworkTool::host_/core_.
- Fix in the commit with this Handoff (UNBUILT): header reverted to main; solved pieces kept in file statics
  g_pieces/g_pieceCount/g_piecesOwner in FC4NetToolHost.cpp.

Round 2: aedfd10d filed NR14 (next free ID was NR14, not NR15). 0d7918d3 static_assert layout pin in
FC4NetToolHost.h (both cFC4NetworkTool.cpp.obj and FC4NetToolHost.cpp.obj rebuilt 20:35:31 after header 20:35:20;
quick PASS 148/148), solved pieces cleared after same-context insert.
Next: game request nrimpl4-a2 queued; qwait (no --fresh) on build-runs/game-results/nrimpl4-a2.verdict, grep
`netrules`; then qbuild all, final report (partial: choice logged, not applied, NR14), merge-queue json.
Facts: native-vs-ours comparison not available (native tool runs only when nettool off; no hook into its solve).

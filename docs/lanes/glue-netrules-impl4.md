| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netrules-impl4: wire fc4_network_rules into our network tool | fc4-lane / .claude/worktrees/glue-netrules-impl4 | wip (partial: solver runs in game, choice logged not applied; NR14 open) | answer NR14, then apply pieces |

## Report

Goal: per network-rules.md 2.4 (NR13), run fc4_network_rules_* as a component of our network tool
(engine/plugin/FC4NetToolHost.cpp, active under `enable = netmgr, nettool`), no netrules key; prove it in game.

What changed (engine/plugin only, no Rust, no ABI):
- 9772084d SolvePieces after fc4_netgeom_place in ValidateAndSolve (geom cell infos -> rules grid, S = solved cells,
  fc4_network_rules_solve(type, resolve=false)); Rules() loader through OpenDBRecord/CloseDBRecord (tiles per type,
  INI, RUL1, RUL2, RUL0); InsertOccupantsForSolvedCells logs ours vs occupant PieceId()/GetRotationAndFlip() readback.
- 1efec3bd + 0d7918d3: the solved pieces live in file statics (g_pieces/g_pieceCount/g_piecesOwner), not in
  NetToolContext. Its layout is pinned with static_asserts in FC4NetToolHost.h (size 4 ptr + 6 u32, offsets of self
  and connections), so both users (cFC4NetworkTool.cpp, FC4NetToolHost.cpp) check it at compile time. Both objs were
  rebuilt after the header (20:35:31 > 20:35:20). Lifetime: the records are cleared at the start of each solve and
  after the same-context insert consumes them, so another context never reads them. Main thread only (inside
  PlaceNetworkOccupants), never read by the getters on the second thread.
- c28d527d OpenDBRecord (slot 37, eax-or-void) bool is not trusted; the out record decides (ret_channel_check).
- aedfd10d question NR14 filed in docs/specs/questions/network-rules.md.

Measured:
- nrimpl4-a1 (old build): AV after insert and at Shutdown. Cause: the context grew but cFC4NetworkTool.cpp.obj was
  stale. Kept as negative evidence; fixed by the layout fix above.
- nrimpl4-a2 and nrimpl4-a3 (fixed build, turbo, end 2026-05-01, netmgr+nettool): verdict clean, ref_warnings=0, no
  exceptions, normal shutdown. Rules loaded tile_files=24 missing=0 tile_rules=6295 ini=1 rul=3/3
  intersection_rules=249. Solves: #1 road (4,4)..(14,4) 11/11 solved (end 0x200/03, straight 0x2600/03); #2 road
  (9,1)..(9,8) 8/8 (end 0x200/00 and /02, straight 0x2600/00, also at the crossing cell (9,4), S held only the new
  line); #3 street 11/11 (0x05000200, 0x05002600). The occupants read back 0/0 (same_as_occupant=0).
- Gate (qbuild all): cargo test 3896 passed, clippy 0, build 63/63, ctest 63/63 PASS.

Status: PARTIAL. The real solver picks pieces in game, but the choice is only logged: there is no published contract
for writing a piece id or orientation onto a game-created occupant (NR14). No replacement credit and no native
fallback presented as ours.

Questions: NR14 (network-rules): which public slot applies a solved (piece, rotation, flip) to an occupant.
Next: when NR14 is answered, apply the pieces in InsertOccupantsForSolvedCells and compare them with the native tool
on the same script. Also check how a crossing cell gets an intersection piece (S including existing neighbours).

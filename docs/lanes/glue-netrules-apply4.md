| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netrules-apply4: apply solved piece id/orientation via occupant slot 30 SetPiece before InsertOccupant (NR14) | fc4-lane / .claude/worktrees/glue-netrules-apply4 (engine lane) | ready. No ABI change. Gate: cargo 3980 passed, clippy 0, ctest 65/65. Game nrapply4-g1 clean, ref_warnings=0: road 11/11, road 8/8, street 8/8 applied, readback = ours 27/27. NR15 filed. | 1. crossing cell gets intersection piece (S incl. existing neighbours) 2. answer NR15 (Pipe/Subway) |

## Report

Goal: network-rules.md §11.2 (NR14), test 30/31: the solved piece goes onto the new occupant through slot 30
before the occupant manager's InsertOccupant.

What changed (engine/plugin only, no Rust, no ABI):
- compat (FC4Compat.h/.cpp): new table field `networkOccupantSetPieceVA` = 0x0061B490 (spec §11.2), appended at the
  end of ExecutableVersion and of the 1.1.641 entry. `compat::NetworkOccupantSetPiece(occ, id, orient)` calls it
  only when the executable is identified, the VA passes CheckCode once (logged), and the occupant's own vtable[30]
  equals that VA. It calls through a typed `void(__thiscall*)(void*, uint32_t, uint32_t)` with the orientation
  zero-extended (the SDK's `SetQuadDraw(uint32_t, uint8_t)` name is not used). Any other vtable: not called, false.
- FC4NetToolHost.cpp InsertOccupantsForSolvedCells: for each cell that has a solved record, slots 23/26 are read
  (pre), SetPiece is applied, they are read again (set), then PlaceOccupant (type flag, position, InsertOccupant), then
  read once more (occ). [choice] Pipe (4) and Subway (7) are not applied (underground, no contract) and are counted
  `unsupported`; a connection node (flag 0x200000) is also refused. The solved-records statics are unchanged
  (per-run, owner-checked, cleared after use), and the NetToolContext layout is unchanged.
- Commits: e8cf6484 (code + NR15), then the lane files.

Measured (nrapply4-g1: turbo, end 2026-05-01, enable = netmgr, nettool, script build-runs/nrapply4.script): verdict
clean, ref_warnings=0, 0 foreign exceptions, 0 Rust panics.
- road (4,4)..(14,4): chosen=11 fresh_zero=11 applied=11 unsupported=0 set_before_insert=11 same_as_occupant=11
  (end 00000200/03, straight 00002600/03).
- road (9,1)..(9,8): 8/8 every count (end 00000200/00 and /02, straight 00002600/00).
- street (20,4)..(27,4): 8/8 every count (05000200/03, 05002600/03, 05000200/01).
- The compat check logged slot 30 at 0x0061B490 as executable code inside the image.
- Gate (qbuild all): cargo test 3980 passed, clippy 0 diagnostics, build PASS, ctest 65/65.

Questions: NR15 (network-rules): do Pipe/Subway occupants take their pieces through the same slot 30 before insert?

Next steps:
1. The crossing cell (9,4) still gets a straight piece (S holds only the new run). Include the existing neighbours in S
   so an intersection piece is chosen (network-rules.md §6/§8).
2. When NR15 is answered, apply pieces for Pipe/Subway.

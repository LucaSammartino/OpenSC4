| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-netrules-apply4: answer NR14 (how a solved piece id/orientation reaches an occupant) | fc4-lane / .claude/worktrees/re-netrules-apply4 | ready (NR14 answered: occupant slot 30 before insert) | glue: call slot 30 on our tool's new occupant before InsertOccupant; run tests 30-31 |

## Report

Static only: REA had no app open (target_unavailable), so the answer is from re_query.py asm and decomp/.

**Answer (NR14, MATCHES).** `cISC4NetworkOccupant` slot 30 (`+0x78`, `0x0061B490`, docs/abi name
`SetQuadDraw`) is SetPiece(u32 piece id, orientation byte zero-extended), thiscall `ret 8`. It sets
`0x2000000`/`0x4000000` unless the occupant is a connection node (`0x200000`), releases the path info (slot 50),
clears `0x2000`, writes `+0xFC` and `+0x104`, turns the quad's texture coordinates, sets the vertex alphas to 0xFF.
It posts no message. `InsertNetworkOccupant` (slot 72) calls it at `0x0062797F` on the occupant that slot 54
`CreateNetworkOccupant` has just made, using the 12-byte solved record `{+0 id, +4 orientation, +8 key}`. That call
comes before `AddSupportStructures`, `ClearNetworkOccsFromCell` and `occupantManager.InsertOccupant`
(`0x00627A38`). Slot 54 never writes the two fields, which explains the 0/0 readback in run nrimpl4-a1. Other
callers: the connection-node branch of `InsertOccupantsForSolvedCells` (`0x00625A5D`, then remove and re-insert),
`InsertHighwayIntersectionPiece` (`0x00627757`) and `InsertBridgePiece`. Slots 24 and 25 are the rotation and flip
readbacks.

**Changed:** docs/specs/network-rules.md §11 (record layout), new §11.2, §15 tests 30-31; NR14 moved to
docs/specs/questions/answered/network-rules.md; QUESTIONS.md regenerated (questions.py check ok).

**Proposed abi rows:** build-runs/abi-rows-re-netrules-apply4.tsv (slot 30 behaviour note; slots 24 and 25
named). Not committed, following the build-runs convention.

**Open:** the Mac name of slot 30 (the auto-match `SetQuadDraw` is medium confidence). The underground override
(`0x00659747`) also calls `+0x78` with the record's id and byte on its own occupant class; I did not verify that
class's slot. Neither blocks the glue work.

**Next:** glue-netrules lane: on our tool's newly created occupant, call slot 30 `(id, orientation)` before
`InsertOccupant`, then check §15 tests 30-31 in game (slot 23/26 readback and the order of the `0x99EF1142`
listener).

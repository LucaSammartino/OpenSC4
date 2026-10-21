| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-disasters46-4: answer DS46 (special-building vector at record +0x50) | `re-disasters46-4` / .claude/worktrees/re-disasters46-4 | in progress | see Handoff |

## Handoff

- Goal: answer DS46 (docs/specs/questions/disasters.md): layout of layer +0xA0 (record +0x50) vector<tSpecialBldgInfo>, element, owners, readers.
- Done: lane file.
- Next: asm of DoOnOccupantRemoval 0x00578BA0, push_back, clear, FUN_00578760, dtor; xrefs to GetDisasterCost readers of +0x50.
- Facts: decomp names it std::vector<cISC4DisasterLayer::tSpecialBldgInfo>; element = cRZString name + int64 cost.

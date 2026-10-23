| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-mysim-dispatch5: dispatch and pool owner closure | Codex; core-mysim-dispatch5 | in progress | Map specification, close owners, validate full gate |

## Handoff
- Goal: close cSC4MySimDispatch and cSC4MySimPool in the existing core module; no ABI/plugin changes.
- Done: reused reserved detached worktree; mapped spec sections 4.7.1, 4.12, 12.21-22 to existing dispatch/pool and conformance runner.
- Next: implement settled MS13 arithmetic and MS14 import branches, add core tests and conformance vectors, document owner closure.
- Findings: stale MS13/MS14 comments; floor/saturating conversion, cell-only setter, missing gender/zodiac accepted, short listed filename stem wrong.
- Constraints: clean room; do not edit street module; all changes via apply_patch.
- Game requests: none.

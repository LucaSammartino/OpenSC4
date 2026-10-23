| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-mysim-dispatch5: dispatch and pool owner closure | Codex; core-mysim-dispatch5 | in progress | Map specification, close owners, validate full gate |

## Handoff
- Goal: close cSC4MySimDispatch and cSC4MySimPool in the existing core module; no ABI/plugin changes.
- Done: reused reserved detached worktree; mapped spec sections 4.7.1, 4.12, 12.21-22 to existing dispatch/pool and conformance runner.
- Done: MS13 arithmetic/cell world setters; MS14 required reads, short names, signed formatting and nonterminating UINT_MAX loop; core tests and conformance vectors 29/30; owner closure README; MS17 filed for explicitly out-of-scope dispatch behavior/record.
- Next: focused core/conformance tests, explicit fmt, then final qbuild all.
- Focused initial run failed E0689 (written counter type ambiguous), fixed with u32; failed logs retained under build-runs/q.
- Findings: stale MS13/MS14 comments; floor/saturating conversion, cell-only setter, missing gender/zodiac accepted, short listed filename stem wrong.
- Constraints: clean room; do not edit street module; all changes via apply_patch.
- Game requests: none.

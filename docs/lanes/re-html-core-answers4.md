| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-html-core-answers4 (answer HD5/HD6 in docs/specs/html-document.md) | `re-html-core-answers4` / `.claude/worktrees/re-html-core-answers4` | in progress | see Handoff |

## Handoff
- Goal: answer HD5 (ParseSGMLElements reset of 0x194/0x28) and HD6 (a-d return values, FONT size, RZConvertToUint32A) from the binary; write rules into html-document.md §5.1/§7.x; move answered entries.
- Done: worktree created.
- Next: read asm 00908122 (ParseSGMLElements), ParseSELECT, ParseSCRIPT, FONT size code, RZConvertToUint32A.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-scres-scr10: physical file-line observation contract | Codex, `re-scres-scr10` (reserved branch, detached HEAD) | in progress | Verify ReadLine and open/path association; answer SCR10 and run doccheck. |

## Handoff

- Goal: close SCR10 for physical file-parser observation with prose and conformance vectors.
- Done: section 11 and SCR-V81–92 close SCR10; answered entry copied from native6's unpublished question.
- Facts: this base has no SCR10 entry; source question is in `C:/Users/tlori/FreeCity4/.claude/worktrees/glue-scres-native6/docs/specs/questions/sc-resources.md`.
- Native6 report has no Owner closure table or `open:` rows; its Open scope section identifies ReadLine ABI as the blocker.
- Next: regenerate questions index, run doccheck, review diff and finalize report/queue.
- Verified instructions: ReadLine is complete file ECX + S reference, AL bool, ret 4; constructor/path/open map reads to nested DoParseFile calls.
- Quirks: CRLF and LFCR collapse; later chunk failure returns true prefix; wide append uses byte count as unit count, potentially reading uninitialized/beyond-buffer data.
- No game runs, ABI rows or engine changes requested.

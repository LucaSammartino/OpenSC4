| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lotmodels-contracts4: LM1–LM8 native contracts | Codex / `re-lotmodels-contracts4` (RE) | in progress | Trace native owners; publish contracts and exact residuals. |

## Handoff

- Goal: resolve only LM1–LM8 from primary native evidence; no engine reads/edits.
- Worktree reserved, detached HEAD reused; no ref changes.
- Read lot-model spec target sections, open questions, lane/merge format and short handoff.
- Done: `81d9d85b` binding-failure and cell corner/placement contract.
- Added foundation buffer order, exact step arithmetic, FSH groups and 11 independent vectors; LM3/LM5/LM6 answered.
- Native proof: missing zero key disables foundation/wall binding; fallback only after nonzero failure.
- Cell x/z unsigned bytes, four-record strip and full orientation table verified in instructions/data.
- Next: trace LM1/LM7 draw modes, wall geometry and LM2 lifecycle/inventory; retain exact residuals if not closed.
- q.py cannot launch Bash (E_ACCESSDENIED); use bounded PowerShell output and re_query directly.
- No game runs, ABI changes or runtime claims planned.

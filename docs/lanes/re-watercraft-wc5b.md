| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-watercraft-wc5b (watercraft remaining contracts) | Codex / `re-watercraft-wc5b` (RE) | in progress | Audit WC3, WC5 and WC4 residuals; preserve answered WC1/WC2 and WC6–WC8. |

## Handoff

- Goal: finish the remaining WC3–WC5 behavior contracts and conformance vectors, with an honest residual if unresolved.
- Read prior wc5 report and Handoff, rules, question list and landing format. WC1/WC2 already answered; do not redo.
- Worktree is reserved detached HEAD; commit normally without changing refs.
- Next: inspect section 10.2 inventory and source for remaining owner methods, then crash transform and AI.
- Main evidence: `C:/Users/tlori/FreeCity4/tools/re_query.py`, `decomp/cSC4Watercraft.c`, `decomp/region_00560000.c` (read-only).
- q.py Bash wrapper fails CreateFileMapping access denied; use native PowerShell with bounded output.
- No engine files accessed, ABI changes, game requests or REA calls.

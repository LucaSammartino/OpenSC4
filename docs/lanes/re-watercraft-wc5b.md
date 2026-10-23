| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-watercraft-wc5b (watercraft remaining contracts) | Codex / `re-watercraft-wc5b` (RE) | in progress | Audit WC3, WC5 and WC4 residuals; preserve answered WC1/WC2 and WC6–WC8. |

## Handoff

- Goal: finish the remaining WC3–WC5 behavior contracts and conformance vectors, with an honest residual if unresolved.
- Read prior wc5 report and Handoff, rules, question list and landing format. WC1/WC2 already answered; do not redo.
- Worktree is reserved detached HEAD; commit normally without changing refs.
- Done: added §3.3 helper signatures/failures and §5.1.4 inherited animation arithmetic/dispatch, unconditional normal true result and wake request ordering; eight vectors (124 total). Doccheck and diff check pass.
- Next: crash transform at 0056A440/0056AF90, then AI target selection/far approach. WC3/WC4/WC5 remain open; archive none yet.
- Evidence correction: primary table is 00A9DBB8 from 0056D9BB. Initial scratch lookup at 00A9DB88 was wrong and discarded; no spec claims use it. PE b22a80 is runtime-initialized, not a file constant.
- Main evidence: `C:/Users/tlori/FreeCity4/tools/re_query.py`, `decomp/cSC4Watercraft.c`, `decomp/region_00560000.c` (read-only).
- q.py Bash wrapper fails CreateFileMapping access denied; use native PowerShell with bounded output.
- No engine files accessed, ABI changes, game requests or REA calls.

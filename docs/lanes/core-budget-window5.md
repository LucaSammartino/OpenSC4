| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-budget-window5: sub-budget owner closure | Codex / core-budget-window5 | in progress | Read existing module and owner specs; implement and verify closure |

## Handoff
- Goal: close cSC4SubBudgetWindow in the existing budget_window core module; no plugin/C ABI edits.
- Done: d24445c6 initial handoff; raw ordinance/neighbour population, combo edits, individual list/reference acquisition and bounded group-A refresh implemented; same-owner callbacks removed.
- Tests: focused r4 passed 43/43 (before latest property and refresh-counter tests); r2 failed 33/35, corrected; logs build-runs/focused-budget-window-r*.log. Initial q.py shell startup failed (log build-runs/q/20261023-153415-17724.log).
- Next: finish owner README table; formatting; focused tests; one final qbuild all.
- Open: BW17 settled slices implemented, remaining explicitly Unsupported; BW19 invalid-input safe divergence and BW20 initial group temporary references appended in questions/budget-window.md.
- Gate/game: full gate not run; core only, no game wiring or request. Manifest: main build-runs/run4-core-budget-window5-r1-manifest.json.

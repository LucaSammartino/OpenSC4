| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-dispatch-complete4 (dispatch replacement completion) | `Codex` / `glue-dispatch-complete4` (engine lane) | in progress | Complete published dispatch host contracts and measure native/replacement exercise |

## Handoff
- Goal: finish supported dispatch replacement, including vehicle views and independent source/pylon comparisons; no completion credit with missing contracts.
- Reused reserved detached worktree; initial HEAD equals main, clean status.
- Baseline qbuild all running (session 80140).
- q.py cannot start Bash (E_ACCESSDENIED); use bounded native PowerShell commands.
- Next: inspect glue17 report, exact dispatch implementation and required spec/ABI sections, then implement supported gaps.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-dispatch-complete4-r1-manifest.json.
- No game request outstanding; no ABI changes; no questions yet.

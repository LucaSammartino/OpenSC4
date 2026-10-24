| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| audit-shadow-methods15 (method-level credit for seven partial shadows) | `Codex` / `audit-shadow-methods15` (RE docs lane) | in progress | (1) Audit measured original-function comparisons; (2) resolve coverage names and bytes; (3) doccheck and enqueue. |

## Handoff

- Goal: propose exact original-function coverage credit for viewutil, querytool, legacyctl, gdriver, renderprops, app and modelinst shadows.
- Done: reused the reserved detached worktree; confirmed clean status and read lane/rule formats.
- Next: inspect the named lane reports and branch sources, separating game-path comparisons from fixtures and blocked questions.
- Facts: read-only engine source access is explicitly authorized for this attribution audit; no engine/tool changes or ABI changes are permitted.
- Outputs: `docs/specs/attribution/proposed-shadow-methods.tsv`, this report, and local merge queue JSON.
- Guard: `C:/Users/tlori/FreeCity4/build-runs/run4-audit-shadow-methods15-r1-manifest.json`; budget 150000, 150 calls between commits.

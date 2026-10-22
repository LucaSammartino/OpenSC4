| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-filebrowser-contracts4: browser ownership and remaining contracts | Codex / `re-filebrowser-contracts4` (RE spec lane, detached worktree) | in progress; docs only; FBQ1-FBQ6 audit | 1. Publish lifecycle/control contracts. 2. Audit 174 candidates and exact residual bytes. 3. Validate and park final round. |

## Handoff

- Goal: highest-byte remaining browser contracts, prioritizing lifecycle and ownership; no complete-owner claim without full owned-method coverage.
- Worktree reused at reserved detached HEAD; no branch/ref changes permitted.
- Done: lifecycle contracts in sections 8.5/8.6; constructor 009A57C4 (710 B) absent from 174 candidates; FBQ3 answered.
- Facts: two secondary tables; constructor installs 00ADCF30. Shutdown releases 29 fields, omits primary +0x280; actual destructor never calls browser Shutdown. Init sets actual 396 x 236, not minima.
- Next: verify highest-byte control/property methods; reconcile inventory with explicit residual byte totals before final parking.
- Evidence roots: `C:/Users/tlori/FreeCity4/{ghidra/export/windows,decomp,re}` (read only).
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-filebrowser-contracts4-r1-manifest.json`.
- No game requests, ABI proposals, runtime changes or subagents.
- This is the final logical third round: preserve and park residuals; no restart.

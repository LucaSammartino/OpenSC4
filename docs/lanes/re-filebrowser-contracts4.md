| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-filebrowser-contracts4: browser ownership and remaining contracts | Codex / `re-filebrowser-contracts4` (RE spec lane, detached worktree) | in progress; docs only; FBQ1-FBQ6 audit | 1. Publish lifecycle/control contracts. 2. Audit 174 candidates and exact residual bytes. 3. Validate and park final round. |

## Handoff

- Goal: highest-byte remaining browser contracts, prioritizing lifecycle and ownership; no complete-owner claim without full owned-method coverage.
- Worktree reused at reserved detached HEAD; no branch/ref changes permitted.
- Done: a3dbc38c published lifecycle contracts before call 30; FBQ3 answered. Constructor 009A57C4 (710 B) absent from 174 candidates.
- Facts: two secondary tables; constructor installs 00ADCF30. Shutdown releases 29 fields, omits primary +0x280; actual destructor never calls browser Shutdown. Init sets actual 396 x 236, not minima.
- Done: EnableChildControl, selected primary slots/pending copy+append, full default comparator and string fitting specified. FinalRelease verified to call virtual browser Shutdown before deletion (qualifies direct-destructor finding).
- Inventory: 174/28907 B exact; 60/7029 B specified incl. wrappers/helpers; 114/21878 B residual. Constructor adds 1/710 B. 42/3823 B remain unattributed.
- Next: run questions index/check and whitespace; commit publication, final report, queue; park with no fourth round.
- Evidence roots: `C:/Users/tlori/FreeCity4/{ghidra/export/windows,decomp,re}` (read only).
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-filebrowser-contracts4-r1-manifest.json`.
- No game requests, ABI proposals, runtime changes or subagents.
- This is the final logical third round: preserve and park residuals; no restart.

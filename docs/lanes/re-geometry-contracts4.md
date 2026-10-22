| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-geometry-contracts4: GU2–GU6 native contracts | Codex / `re-geometry-contracts4` (RE spec lane) | in progress; native evidence audit | 1. Reconcile actual owner inventory. 2. Prove GU2–GU6 and publish independent vectors. |

## Handoff
- Goal: resolve only GU2–GU6 using original export/instructions/decomp/data/callers; spec text only.
- Worktree reused at detached HEAD; initial status clean; no commits ahead of main.
- Milestone 1: native instruction proof added in §§6.1/6.4.1; Reset/AddVertex do not dirty, colour uses float bits with double lighting constants, failure epilogue mutates outputs and restores/releases in order.
- Export finds actual 37 bodies (20 geometry, 9 builder, 8 nS3D); no separately exported builder destructor/EndPrimitive or general transform/sort.
- Bash q.py fails with WSL E_ACCESSDENIED; use local PowerShell with bounded output.
- No engine reads, foreign owner methods, ABI-table edits, runtime claims, or subagents.
- Evidence: ignored `build-runs/geometry-evidence/<VA>.asm.txt` collected with re_query asm --bytes, 56 instructions per page. Data helper reads main bin/SimCity4.exe; no engine reads.
- Next: finish numeric/branch/clip proof, surface pitch/compression, reconcile 37 body sizes, publish checklist and independent vectors; resolve question entries only within proved scope.
- Guard manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-geometry-contracts4-r1-manifest.json`.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-ripples-contracts4: exact ripple and selection contracts | Codex / `re-ripples-contracts4` (RE spec lane) | in progress; WR4-WR10 native proof | 1. Prove shoreline/strip bodies. 2. Resolve animation, drawing, lifecycle and selection boundaries. |

## Handoff
- Goal: resolve WR4-WR10 from native export/decomp/instructions; spec text only.
- Done: 3b970a37 answers WR5; native WR4/WR6/WR9/WR10 contracts now in §§1.2,1.5,2.2, with vectors 12-15.
- Next: finish WR7 geometry/bounds/state proof and WR8 owner lifecycle/inventory; retain precise residuals if capped.
- Evidence roots: C:/Users/tlori/FreeCity4/{ghidra/export/windows,decomp,bin/SimCity4.exe}.
- Questions: docs/specs/questions/water-ripples.md; target docs/specs/water-ripples.md.
- Constraints: no engine reads, no ABI table edits, no runtime/coverage claims, no subagents.
- q.py Bash wrapper fails with WSL E_ACCESSDENIED; use bounded PowerShell commands directly.
- REA batch 0074b570 failed target_unavailable (No app is open); no session changes, export fallback.
- Native scan: ripple range 0075fb20..00762100 = 43 bodies/9374 B; selection 007505c0..00751c80 = 24/5635 B; reconcile owner attribution later.
- WR5 differences: partial outer survives inner spacing rejection; lower edge equal accepted; 1002 total pairs maximum; fade denominator configured count.
- New differences: period bounds [122499,227499); raw 64-bit RNG product participates; single frame step/discard overshoot; strict >2M delay; all clear-mark overloads refresh preference; packed b3=low colour byte.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-ripples-contracts4-r1-manifest.json.

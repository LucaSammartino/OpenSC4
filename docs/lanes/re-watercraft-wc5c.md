| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-watercraft-wc5c (remaining watercraft contracts) | Codex / `re-watercraft-wc5c` (RE, detached HEAD) | in progress | Investigate WC3–WC5, add vectors, validate and enqueue. |

## Handoff

- Goal: narrow or answer remaining WC3 method/failure, WC4 AI/motion and WC5 crash/bump contracts; specs only.
- Prior wc5/wc5b reports read; WC1/WC2 and WC6–WC8 already answered and excluded.
- Done: §3.4 land/tow/angular/quantization signatures, side effects and failure boundaries; §5.3.2 far-docking entry effects, distance/time and call ordering; eight vectors (132 total). WC3/WC4 residual lists narrowed.
- Done: §5.7.2 sinking basis from previous heading, independent roll/pitch sign selection, current-heading feedback and publication; four additional vectors (136 total). Grounding/heel 0056AF90 remains WC5.
- Next: validate sinking sign/basis arithmetic, then remaining tuning/debug signatures if budget permits; stop answer work at measured 100000 and reserve report/validation budget.
- Verified fixed data: 00B0A154=2 degrees, 00B0A18C=1, 00B0A148=7; PE constants read with ignored build-runs/read-pe.py. Far approach uses occupant 3D distance and strict >p[37], always runs AI/movement/collision even at zero dt, and returns true.
- Evidence main checkout: `C:/Users/tlori/FreeCity4`; write only this worktree.
- Reserved detached HEAD: commit normally, never update refs. No engine access, ABI changes or game runs planned.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-watercraft-wc5c-r1-manifest.json`.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-fontanswers4: answer drawing-fonts FN1-FN6 | `re-fontanswers4` / (RE spec lane) | in progress | see Handoff |

## Handoff
- Goal: resolve FN1-FN6 (docs/specs/questions/drawing-fonts.md) from instructions; write rules into docs/specs/drawing-fonts.md sections 1, 8.2, 10, 11, 12.
- Done: FN1 (log10, k=(s+1)^3.3219), FN3 (scoring walk), FN4 (aa slots 5/7/8/9, +0x64 map, pre-Init name) written in spec sections 10 and 12 (commit "spec: FN1 FN3 FN4"). Questions not yet moved to answered/.
- Next: FN5 (0x008968EB, 0x008973D4), FN6 (0x00898A5A), FN2 (0x00899665); then move FN entries to answered/drawing-fonts.md and run tools/questions.py index.
- Tool: build-runs/xdis.py <hex> [n] | --dump N (capstone on bin/SimCity4.exe, for bytes outside Ghidra functions).

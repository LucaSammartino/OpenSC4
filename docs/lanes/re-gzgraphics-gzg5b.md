| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzgraphics-gzg5b: GZG1–GZG5 behavior closure | Codex / `re-gzgraphics-gzg5b` (detached HEAD) | in progress | Verify export evidence, extend spec and vectors, answer questions, doccheck |

## Handoff

- Goal: answer GZG1–GZG5 in gz-graphics.md with behavior and vectors; no engine edits or reads.
- Done: GZG4 milestone `475e79821`; §§7.1–7.4, answered question, regenerated index; final review adds deleting-slot correction and vector V78.
- Owner closure: core lane report mirrors open rows for §§2–7; actual table is in engine/, forbidden by task, so use mirrored list.
- Next: GZG1 remaining outline subdivision, fixed-point diagonal ties/clipping and polygon rasterization; then GZG2/GZG3/GZG5.
- Evidence: main checkout C:/Users/tlori/FreeCity4; re_query.py first, then decomp/; REA unavailable by brief.
- No game runs, ABI edits or proposed rows.
- Canvas corrections: primary 7/8 suspend/resume; 15 repaint event; 18 keyboard filter. Native listeners register at front (reverse registration order).
- Canvas quirks: realized byte set on failed slot-4 realization; keyboard filter consumes an uncounted 1025th record.
- Final review: slot 21 invokes deleting destructor via complete +24, NOT a framework exit; fixed before final report.

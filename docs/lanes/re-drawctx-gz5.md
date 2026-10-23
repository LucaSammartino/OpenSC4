| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-drawctx-gz5 (GZ graphics contracts) | Codex / `re-drawctx-gz5` (RE lane) | in progress | (1) inventory exported owners; (2) specify contracts and vectors; (3) doccheck and report |

## Handoff

- Goal: write `docs/specs/gz-graphics.md` for GZ context, graphics system, alpha blitter, W32 canvas and render targets; no engine edits.
- Worktree is intentionally detached; commit HEAD normally, do not change refs.
- Done: read operating rules, spec policy and lane record format; located all named decomp files in main checkout.
- Next: query export method/vtable inventory, read related spec outlines and relevant decomp functions.
- Evidence roots: `C:/Users/tlori/FreeCity4/decomp/` and `ghidra/export/windows`; query with main `tools/re_query.py` first.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-drawctx-gz5-r1-manifest.json`.
- No game runs, ABI changes or question resolutions yet.

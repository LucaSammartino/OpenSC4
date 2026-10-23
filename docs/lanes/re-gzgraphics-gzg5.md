| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzgraphics-gzg5: GZG1–GZG6 graphics contracts | Codex, `re-gzgraphics-gzg5` (detached HEAD) | in progress | Trace export, extend spec and vectors, close questions, doccheck |

## Handoff

- Goal: answer GZG1–GZG6 in gz-graphics.md from Windows 1.1.641 export.
- Read core-gzgraphics5 lane report's open section list; engine files are forbidden.
- No engine, plugin, capi, ABI or coverage-map edits authorized.
- Done: GZG6 differs; destructor explicitly shuts engine down before releases.
- Done: partial GZG1 geometry/blit/gradient, GZG3 getters, GZG5 clipping/loaders;
  declarative conformance vectors GZG-V45–V61. These are export observations,
  not game measurements. GZG1–GZG5 remain open; do not mark them answered.
- Next: regenerate question index, doccheck, commit this bounded evidence.
- Research reached the instructed two-thirds context allocation; no further RE
  in this round. Final report must candidly state incomplete task (wip).
- Export/main: C:/Users/tlori/FreeCity4; worktree is this directory.
- No game runs requested; no REA binary open, use re_query.py and decomp.

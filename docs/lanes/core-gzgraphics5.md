| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-gzgraphics5: GZ graphics owners | Codex, `core-gzgraphics5` | in progress | Implement spec owners and conformance tests; run full gate |

## Handoff
- Goal: core-only closure of docs/specs/gz-graphics.md; no C ABI or plugin changes.
- Done: spec sections 1-9 and SC4DrawContext boundary reviewed; sibling gz_graphics module added.
- Done: context, buffer/HW/SimGL state, graphics/alpha/canvas services, and all 44 vector fixtures; focused Cargo 21/21 passed.
- Next: review lifecycle and foreign boundaries, add additional failure-path tests and owner closure README, format, full gate.
- Evidence: q.py could not launch Bash (CreateFileMapping Win32 error 5), build-runs/q/20261023-203322-13380.log; direct Cargo succeeded.
- Foreign boundaries: drivers, surfaces, OS window; no real-game wiring requested.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-gzgraphics5-r1-manifest.json.

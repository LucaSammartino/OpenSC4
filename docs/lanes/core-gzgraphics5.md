| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-gzgraphics5: GZ graphics owners | Codex, `core-gzgraphics5` | in progress | Implement spec owners and conformance tests; run full gate |

## Handoff
- Goal: core-only closure of docs/specs/gz-graphics.md; no C ABI or plugin changes.
- Done: spec sections 1-9 and SC4DrawContext boundary reviewed; sibling gz_graphics module added.
- Done: d35b3fbe context, buffer/HW/SimGL state, graphics/alpha/canvas services, all 44 vector fixtures; first focused Cargo 21/21 passed.
- Done: additional lifecycle, existing-span-kernel and Send+Sync tests; focused Cargo 27/27 passed; owned Rust formatting passed (5 files).
- Done: module README owner closure table and explicit GZG1-GZG5 boundaries; GZG6 filed for context destructor Shutdown ambiguity.
- Next: collect the required full qbuild all gate on stable source; diagnose only actual failures, then final report and queue.
- Evidence: q.py could not launch Bash (CreateFileMapping Win32 error 5), build-runs/q/20261023-203322-13380.log; direct Cargo succeeded.
- Foreign boundaries: drivers, surfaces, OS window; no real-game wiring requested.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-gzgraphics5-r1-manifest.json.

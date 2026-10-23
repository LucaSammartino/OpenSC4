| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-legacy-control5: legacy control owner | Codex / core-legacy-control5 | in progress | Inspect existing owner modules and implement spec closure |

## Handoff
- Goal: implement the complete cGZLegacyControl core owner, foreign window/script/renderer traits, conformance tests and owner closure table; no ABI or plugin wiring.
- Done: implementation milestone 71d76437; all new published type passes, layout facts and conformance tests; full README closure matrix and existing-interpreter audit complete. Named-font LC13 stays explicit Unsupported::Open for §9.9 branches; older explicitly specified text metadata preserved.
- Next: run final qbuild all on current stable source, collect its terminal result, resolve any diagnosed errors, then final report/queue.
- Evidence: q.py wrapper failed before cargo (Git Bash CreateFileMapping error). Direct focused cargo terminal: 31 passed / 1 fixture failure (BMP expected wrong dimensions); fixed fixture to spec's 40x20. Added further tests since this result. fmt passed on 8 owned files before final test additions; recheck needed. No game requests (core-only, no plugin wiring).
- Constraints: clean-room permitted sources only; final qbuild all required; manifest C:/Users/tlori/FreeCity4/build-runs/run4-core-legacy-control5-r1-manifest.json.

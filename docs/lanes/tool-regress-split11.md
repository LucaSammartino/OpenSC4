| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| tool-regress-split11 (parallel house shadows) | `tool-regress-split11` / tooling lane | in progress | Implement balanced shadow groups and measure split/combined parity. |

## Handoff
- Goal: split regression shadows across queue slots; combined every fourth train.
- Done: implemented longest-first grouped queue runs, concatenated summary and fail-closed worst verdict;
  merge-train default 3/every-fourth combined cadence, state and JSON run record; ops documented.
- Tests: 17 regression-profile tests passed on PowerShell 5.1, including grouping and summed summaries.
- Next: native verify (also builds main-current engine DLL here); wait for train197 then split/combined game runs.
- Cost source: train196 timestamp gaps before shadow log lines; residential 198.290 s, development 66.596 s,
  demand 59.856 s. Cost proxies, not CPU profiling; measured run will validate balancing.
- Coordinator train197 is active; integration DLL was overwritten since train196, so build our engine for main.
- Validation: current main DLL, house profile turbo to 2032; split and combined counts must match.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-tool-regress-split11-r1-manifest.json.

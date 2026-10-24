| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| tool-regress-split11 (parallel house shadows) | `tool-regress-split11` / tooling lane | in progress | Implement balanced shadow groups and measure split/combined parity. |

## Handoff
- Goal: split regression shadows across queue slots; combined every fourth train.
- Done: implemented longest-first grouped queue runs, concatenated summary and fail-closed worst verdict;
  merge-train default 3/every-fourth combined cadence, state and JSON run record; ops documented.
- Tests: 17 regression-profile tests passed on PowerShell 5.1, including grouping and summed summaries.
- Native gate: qbuild verify PASS, build 262/262 edges, ctest 81/81; 353.2 s (queued behind train197).
- Broader tooling tests: 141/147 passed; six untouched test_laneguard WorkflowDryRun tests fail on gate ERROR.
- Next: run build-runs/validate-split.py in foreground; compare split/combined games and document measured parity.
- Cost source: train196 timestamp gaps before shadow log lines; residential 198.290 s, development 66.596 s,
  demand 59.856 s. Cost proxies, not CPU profiling; measured run will validate balancing.
- Coordinator train197 completed clean (885.953 s game time); own DLL built from unchanged main engine at ffb8f38c2.
- Implementation commit: 3ffd15df6. Native evidence: build-runs/fc4verify-{build,ctest}.log.
- Validation: current main DLL, house profile turbo to 2032; split and combined counts must match.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-tool-regress-split11-r1-manifest.json.

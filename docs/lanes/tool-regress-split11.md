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
- First real run split11-s3 exposed PS hashtable Sort-Object property lookup bug: all keys in group 3;
  empty groups stalled on shutdown. Failed evidence retained, not a timing claim.
- Fixed with explicit sort expressions; all 17 tests pass, now asserting nonempty balanced groups (<250 s proxy).
- Measured corrected split11-s3-fixed PASS, zero problems, wall 664 s (target <420 s unmet).
  Groups completed 85/174/660 s after submission; timestamp proxies did not balance real runtime.
- Initial split11-s3 FAIL, 868 s, group 3 contained every switch; retained as failed evidence only.
- Next: collect original validator (session 27531), waiting for train198 before split11-c1 combined;
  compare fixed split against combined only. No further implementation work this round (two-thirds budget).
- A short isolated-cost probe refused submission because train198 is active; no probe games ran.
- Latest source preserves actual nonclean queue verdict names; 17 regression tests still pass.
- Cost source: train196 timestamp gaps before shadow log lines; residential 198.290 s, development 66.596 s,
  demand 59.856 s. Cost proxies, not CPU profiling; measured run will validate balancing.
- Coordinator train197 completed clean (885.953 s game time); own DLL built from unchanged main engine at ffb8f38c2.
- Implementation commit: 3ffd15df6. Native evidence: build-runs/fc4verify-{build,ctest}.log.
- Validation: current main DLL, house profile turbo to 2032; split and combined counts must match.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-tool-regress-split11-r1-manifest.json.

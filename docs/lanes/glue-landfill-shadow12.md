| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-landfill-shadow12: landfill owner shadows | Codex / glue-landfill-shadow12 | in progress | Inspect owner probes, implement supported comparisons, gate and run to 2032 |

## Handoff
- Goal: measure exact shadows for all three landfill owners in the house city through 2032.
- Done: implemented LG33 good-stream holder-rejection handling; focused reader tests 2/2 pass (build-runs/focused-reader.log). Extended occupant probe to 11 cases and 7 reloads; corrected LG32 void SetPosition call.
- Next: collect SAME final qbuild all (session 56030, native qbuild-20261024-145728-build-ctest), then finalize report and ready queue for measured work. No outstanding game request.
- Clean room: tools/ tables and regress script are forbidden by the overriding clean-room rule; coordinator must handle their credit/profile updates.
- Existing LG31 explicitly parks native zone verification: no public callable contract. Do not claim zone verification through developer serialization.
- No game request outstanding. q.py failed because Git Bash CreateFileMapping is denied; capture helper uses direct subprocess instead. Initial fmt used wrong relative root; applied absolute-path fmt suggestions.
- Milestone 2c4b2946f: source fix/probe extension. First full gate Rust tests 591 passed/1 stale C ABI test failed; clippy passed. Corrected the stale test to LG33 (holder rejection succeeds, checks position words/bounds); fmt passed. Native result still pending on original operation (session 73444).
- Milestone 8db352b74: C ABI test correction. First gate completed FAIL solely Rust test; native build PASS 262/262, CTest 81/81, with tree-changed annotation (test corrected during gate). Original logs preserved as build-runs/gate1-*.log; native verdict qbuild-20261024-144548-build-ctest.verdict. Final full gate now running on corrected source.
- Final gate Rust 7214/7214 passed; clippy 0 diagnostics. Native pending. Source stable; documentation milestone commits may change HEAD during native gate.
- Game glue-landfill-shadow12-2032-20261024a clean through 2032-01-13, ref_warnings=0; landfill developer 73/73 exact, resynced=0; occupant probe 11/11 exact, created/destroyed=18/18, reloads=7/7. Run record docs/runs/glue-landfill-shadow12-2032-20261024a-511.md. Zone verification remains parked LG31.

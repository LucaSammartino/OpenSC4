| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-landfill-shadow12: landfill owner shadows | Codex / glue-landfill-shadow12 | in progress | Inspect owner probes, implement supported comparisons, gate and run to 2032 |

## Handoff
- Goal: measure exact shadows for all three landfill owners in the house city through 2032.
- Done: implemented LG33 good-stream holder-rejection handling; focused reader tests 2/2 pass (build-runs/focused-reader.log). Extended occupant probe to 11 cases and 7 reloads; corrected LG32 void SetPosition call.
- Next: collect SAME running qbuild all (native queue qbuild-20261024-144548-build-ctest), preserve its logs with build-runs/preserve-gate.py, then rerun full gate for corrected C ABI test, house city to 2032.
- Clean room: tools/ tables and regress script are forbidden by the overriding clean-room rule; coordinator must handle their credit/profile updates.
- Existing LG31 explicitly parks native zone verification: no public callable contract. Do not claim zone verification through developer serialization.
- No game request outstanding. q.py failed because Git Bash CreateFileMapping is denied; capture helper uses direct subprocess instead. Initial fmt used wrong relative root; applied absolute-path fmt suggestions.
- Milestone 2c4b2946f: source fix/probe extension. First full gate Rust tests 591 passed/1 stale C ABI test failed; clippy passed. Corrected the stale test to LG33 (holder rejection succeeds, checks position words/bounds); fmt passed. Native result still pending on original operation (session 73444).

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-map-view5: map view owner closure | Codex / core-map-view5 | in progress | Audit existing module against map-view spec and implement all settled owner behavior. |

## Handoff
- Goal: close cSC4WinMapView in the existing core module; no plugin/capi/ABI changes.
- Worktree reserved at detached HEAD; clean at start, no lane commits above main.
- Implementation milestone: b6e06bb2, foreign-trait owner execution for selection, drawing, init, input, legends and ticks.
- Added exact-integer owner ramps and explicit QV8 refusal outside that domain.
- Added README owner table covering every section, including all four section 10 vectors.
- Focused tests: existing map_view core tests 24/24; new conformance interface tests 14/14.
- Filed QV12 (interface/summary edges) and QV13 (hotspot effects) in questions/map-view.md.
- q.py focused attempt failed before execution (Git Bash CreateFileMapping error); native PowerShell cargo tests passed.
- fmt initial relative paths failed; use absolute paths. Formatting applied through apply_patch from checked rustfmt diffs.
- Final-source fmt: PASS, six explicit owned files, diff_lines=0.
- Full qbuild all is STILL RUNNING: collect exec session 28866, do not restart it.
- Its cargo log is complete: 6085 passed, 0 failed/ignored across 30 result lines (build-runs/qbuild-test.log).
- Its clippy log is complete: Finished dev profile, no errors/warnings (build-runs/qbuild-clippy.log).
- Native CMake/CTest phase remains outstanding; full gate has no terminal verdict yet.
- Next: collect session 28866 using write_stdin with 30-60 second waits; inspect final PASS/FAIL and logs.
- No game requests outstanding; core-only task.

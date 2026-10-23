| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-map-view5: map view owner closure | Codex / core-map-view5 | in progress | Audit existing module against map-view spec and implement all settled owner behavior. |

## Handoff
- Goal: close cSC4WinMapView in the existing core module; no plugin/capi/ABI changes.
- Worktree reserved at detached HEAD; clean at start, no lane commits above main.
- Added foreign-trait owner execution for selection, drawing, init, input, legends and ticks.
- Added exact-integer owner ramps and explicit QV8 refusal outside that domain.
- Added README owner table covering every section, including all four section 10 vectors.
- Focused tests: existing map_view core tests 24/24; new conformance interface tests 14/14.
- Filed QV12 (interface/summary edges) and QV13 (hotspot effects) in questions/map-view.md.
- q.py focused attempt failed before execution (Git Bash CreateFileMapping error); native PowerShell cargo tests passed.
- fmt initial relative paths failed; use absolute paths. Formatting applied through apply_patch from checked rustfmt diffs.
- Next: explicit-file fmt, final qbuild all on stable source; inspect actual terminal result and named logs.
- No game requests outstanding; core-only task.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-camera-close5b (camera CA7–CA10 closure) | `core-camera-close5b` / engine lane | in progress | Implement resolved owner contracts; focused tests; full gate |

## Handoff
Goal: close every CA7–CA10 owner-table row using the revised camera spec.
Done: 8a363a3a implements CA7 live conversions and eye/pixel/history caches; CA8 four commands; CA9 raw-kind updates and all command dispatch; CA10 QI plus stable fallible allocation/release/destruction.
Tests: 64 camera tests passed (12 new); explicit-file fmt passed after applying formatter check diffs with apply_patch; contract review complete, no owner-table openings.
Next: run the single final qbuild all and collect its terminal result.
Facts: no ABI/plugin wiring; no game request outstanding; no questions filed; core source forbids unsafe and targets Rust 1.75.
Evidence: q.py Bash startup failed (Win32 error 5); direct native Cargo focused test passed. Initial fmt invocation used wrong engine-relative paths, corrected to core/src; formatting diff fixed before gate.

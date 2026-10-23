| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-vehicles5 (vehicle owner closure) | Codex / `core-vehicles5` (engine lane) | in progress | Read contracts, implement owners, run final gate |

## Handoff
Goal: close Vehicle, TrafficAutomaton, Train and TrainCar core owners; no ABI/plugin changes.
Done: inspected automata/watercraft; existing codecs and AutomatonBase state stack will be reused.
Read: vehicles sections 0-5/9-11; automata 6.1-6.5, 7.5, 14; managers 1/5; dispatch 4.2.1/4.3.
Done: implemented sibling vehicles core, reused records/stack; owner coverage table added.
Questions appended: VH3-VH9 (train layout, init, train algorithms, damage/blur,
route semantics, plummet/pull-over reconstruction, inherited state/dispatch gaps).
Focused attempt 1: 13 passed, 1 failed (occupant Read sets status=2); test corrected,
same-source attempt 2 pending session 94549. Logs build-runs/vehicles-focused-{1,2}.log.
Formatting applied through apply_patch using rustfmt stdin (UTF-8, no filename headers).
Next: collect focused attempt 2, run focused conformance and fmt, then final qbuild all.
Closure incomplete; do not claim ready: module README gives exact open sections.
Facts: no game requests outstanding; manifest is main build-runs/run4-core-vehicles5-r1-manifest.json.

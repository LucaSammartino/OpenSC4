| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-vehicles5 (vehicle owner closure) | Codex / `core-vehicles5` (engine lane) | in progress | Read contracts, implement owners, run final gate |

## Handoff
Goal: close Vehicle, TrafficAutomaton, Train and TrainCar core owners; no ABI/plugin changes.
Done: inspected automata/watercraft; existing codecs and AutomatonBase state stack will be reused.
Read: vehicles sections 0-5/9-11; automata 6.1-6.5, 7.5, 14; managers 1/5; dispatch 4.2.1/4.3.
Next: implement sibling vehicles module and focused core/conformance tests.
Gaps: VH1 player physics, VH2 neighbour ranking already open. Train record layout, constructor flags,
train composition/following and damage/blur details need new questions; do not invent layouts.
Facts: no game requests outstanding; manifest is main build-runs/run4-core-vehicles5-r1-manifest.json.

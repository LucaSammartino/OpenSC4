| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-vehicles-vh5 (VH1–VH9 vehicle contracts) | Codex / `re-vehicles-vh5` | in progress | Resolve vehicle owner questions from export; doccheck and queue |

## Handoff

- Goal: answer VH1–VH9 in vehicles.md, move questions, regenerate index.
- Read core-vehicles5 report closure inventory; its detailed owner table is under engine/ and prohibited to this RE lane, so use the report's explicit VH closure rows.
- Evidence: main checkout decomp/cSC4{TrafficAutomaton,Vehicle,Train,TrainCar,Tank,Automaton}.c and tools/re_query.py.
- Next: query exact function exports, read bounded decomp sections, add normative behavior and conformance vectors.
- No engine, ABI, plugin, capi, or coverage-map changes authorized.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-vehicles-vh5 (VH1–VH9 vehicle contracts) | Codex / `re-vehicles-vh5` | in progress | Resolve vehicle owner questions from export; doccheck and queue |

## Handoff

- Goal: answer VH1–VH9 in vehicles.md, move questions, regenerate index.
- Read core-vehicles5 report closure inventory; its detailed owner table is under engine/ and prohibited to this RE lane, so use the report's explicit VH closure rows.
- Evidence: main checkout decomp/cSC4{TrafficAutomaton,Vehicle,Train,TrainCar,Tank,Automaton}.c and tools/re_query.py.
- Added vehicles.md §12.1 (VH3 wire formats) and §12.2 (VH8 plummet/pull-over), moved those questions to answered/vehicles.md.
- Next: regenerate index, doccheck, commit; report partial closure and leave VH1/VH2/VH4/VH5/VH6/VH7/VH9 open.
- Research stopped at the requested two-thirds measured budget (guard last measured 104847/150000 after writing the answers; prior research checkpoint 95920/150000).
- Native init, train generation/following and driving evidence is partly read but not sufficient to close remaining questions; do not infer answers from this partial read.
- No engine, ABI, plugin, capi, or coverage-map changes authorized.

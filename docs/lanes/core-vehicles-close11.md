| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-vehicles-close11: vehicle owner closure | Codex / core-vehicles-close11 | in progress | Audit specified owners, implement missing contracts, run gates |

## Handoff
- Goal: close cSC4Vehicle, cSC4Train, cSC4TrainCar, cSC4TrafficAutomaton in core vehicles only; no ABI/glue changes.
- Done: 6eb5508cf adds clarified constructors/Init, Train/TrainCar wire, collision/damage/blur, plummet/PullOver, countdown/hop/attach, retained-node routing.
- Current milestone: all 27 focused vehicle tests PASS (Cargo session 59224 collected terminal exit 0). Includes 13 new clarification-vector tests.
- Shared codec fix: VehicleRecord::read now rejects version !=2 immediately; nested Train failure vector exposed it.
- Format fixes applied with apply_patch from rustfmt --check diffs; ignored build-runs/format-owned.py generates patches without stdout-to-source writes.
- Next: audit remaining native owner obligations, update stale vehicles README closure table, then qbuild fmt/all/queued verify as required.
- Important unfinished audit: InitWorld::load_drive_parameters and initialize_route are owner-slot hooks, not established foreign services; replace with core logic or explicit open-contract result (VH7/VH9, possibly new question).
- Other audit points: blur release during shutdown; runtime cars vs serialized car identities; traffic facade base-handler semantics and state lifecycles; owner/helper coverage beyond clarified vectors.
- README rewrite attempt failed atomically (duplicate delete/add target); README is still the OLD stale closure table and must be corrected.
- Source: new init.rs/effects.rs/routing.rs/states.rs, tests/closure.rs + tests/routing.rs; Unsupported now only VH1/VH5/VH7/VH9.
- Earlier diagnostic failures preserved in tool evidence: RefValue Default compile; 25/27 then 26/27 assertions corrected; missing reference registry fixed with actual SaveRegistry/RefResolver.
- Clean-room: only permitted specs/core/docs; no analysis-side sources.
- Outstanding game requests: none; core-only changes planned.
- Gate: full gate not yet run (source audit still underway). q.py wrapper failed pre-Cargo (Git Bash CreateFileMapping); direct Cargo works. Workspace Rust MSRV 1.75; SPEED_UNIT uses exact decimal const instead of const from_bits.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-vehicles-close11-r1-manifest.json

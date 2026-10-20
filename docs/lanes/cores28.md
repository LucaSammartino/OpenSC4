| cores28 (the cSC4MySimPool and cSC4MySimDispatch cores: the two mysims owners still coreless) | `cores28` / (engine lane) | in progress | baseline gate running; then the two modules, tests, questions MS8+, gate all |

## Handoff

Goal: implement the two mysims owners the coverage map shows coreless (`cSC4MySimPool`, `cSC4MySimDispatch*`) in `engine/core/src/mysims/`, following the house pattern; unit tests; MS8+ questions; gate `qbuild all`; abi=false.

Done:
- worktree created from main `a2e6fdf5` (branch cores28).

Next: baseline `qbuild all`; then `dispatch.rs` (the §4.6/§4.7/§3.3 simulation-side contract: the trip snapshot, find_new_job / change_job / check_commute, the notification sub-IDs, the immediate-arrival choice of §10) and `pool.rs` (the Sim templates of the pick-sim UI: GetMySimTemplates / Import / FindTheSimsInstallation per docs/abi/vtables.tsv slots +0x18/+0x1C/+0x20).

Facts:
- mysims.md §1.1 marks both classes out of scope (dispatch = automata, pool = app/UI); the spec's named contract is what the modules model; everything else is [choice]/[question].
- §12 lists no vector for either class (21 ms12_* vectors exist, none names them) — no new vectors.
- pathfinder.md §8 names the MySim dispatch's pathfinder setup; clock24.md names `DispatchCommute` ((17−hour)×60 for hours 8–17, else 5) and that the dispatch's Init only takes and drops a clock reference.
- Existing MySim core reads the dispatch through MySimHost (dispatch/find_new_job/change_job/job_destination/commute_time/dispatch_command/read_dispatch_inline); the new core is the game-side object a glue lane delegates those to. ScriptedMySimHost stays untouched (the 21 vectors must keep passing).
- Questions next free ID: MS8 (QUESTIONS.md row mysims: 7 open, next MS8).

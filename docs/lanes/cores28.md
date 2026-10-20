| cores28 (the cSC4MySimPool and cSC4MySimDispatch cores: the two mysims owners still coreless) | `cores28` / (engine lane) | in progress | modules + tests landed, quick gate PASS; full gate running; then final report + queue file |

## Handoff

Goal: implement the two mysims owners the coverage map shows coreless (`cSC4MySimPool`, `cSC4MySimDispatch*`) in `engine/core/src/mysims/`; unit tests; MS8-MS10; gate `qbuild all`; abi=false.

Done:
- `dispatch.rs` (4798848a): MySimDispatch over a NetworkModel + PathfinderHost — find_new_job / change_job / check_commute through the §4.7 pathfinder (commuter mask 0x1CF [inferred MS8], filter 3, flags 1|2, route deque), job_destination, the §3.3 notifications (trip states, flags, arrival, sim states), check_flags (§4.1 step 3), reset_commute; DispatchHost trait (home/job/neighbor rects, candidates, notify).
- `pool.rs` (4798848a): MySimPool — templates list, Import / GetMySimTemplates / FindTheSimsInstallation over a PoolHost; the vtable's other slots are base-class (QI/AddRef/Release/dtor/RemoveRef/ToChar per docs/abi/vtables.tsv 00a85b14).
- `dispatch_tests.rs` (t30-t37): find_new_job with a real street city, change_job, check_commute, neighbour commute, notifications+flags, IDs, pool. Real traffic NetworkModel + street cells (§11 preamble shape).
- sim.rs: SetResidence's else-branch now asks `dispatch_check_flags(0x100)` (§4.1 step 3, the tracking flag; not stored, UI only); new optional MySimHost hook `dispatch_check_flags` (default false keeps the stub).
- Questions MS8 (the 0x1CF mask is a travel-type mask, not a destination mask), MS9 (one FindPath vs per-candidate runs), MS10 (pool template layout), appended to docs/specs/questions/mysims.md; QUESTIONS.md regeneration pending (questions.py check must run on the merged tree).
- Quick gate PASS: cargo 1742/0 (baseline 3535/0 workspace), fmt clean on the 4 touched files.

Next: full gate `qbuild all` (running); then finish: report replaces Handoff, merge-queue/cores28.json (abi=false, no game runs), `python tools/questions.py index`+`check` on the worktree.

Facts:
- mysims.md marks both classes out of scope (dispatch = automata, pool = app/UI); the modules implement only the named simulation-side contract, everything else [choice]/[question].
- §12 lists no vector naming either class (21 ms12_* exist, none new) — no new vectors, conformance untouched, all 21 still pass.
- No C ABI change: core-internal, capi unchanged.
- The `0x2BA06B99` sub-ID direction: TRIP_STATE_B carries (old,new) as (data2,data3) [inferred]; the state-1/2 notifications carry the new MySim state in data2 [inferred].

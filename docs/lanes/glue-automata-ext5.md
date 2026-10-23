| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-automata-ext5: remaining automata owner shadows | Codex / glue-automata-ext5 | in progress | Audit documented observation contracts, extend shadow where specified, verify through 2032. |

## Handoff
- Goal: compare AutomataManager, Automaton, TrafficAutomataManager, AutomataController and ConstructionOccupant state at load/month.
- Done: initial handoff 98cb12128; audited automata §§0,3.5,4,5.1,6.1,11, manager §2.1, construction lot-records §9.
- Implemented default-off automata_owners_shadow: inherited manager records, recursive automaton/controller records, indexed construction records via occupant-manager Write references.
- Added one C export for construction record roundtrip, with tracked reference cleanup and explicit same-input live crane presence seeding (not a lifecycle comparison).
- Focused test passes 1/1 (crane/scalars/refs preserved; failed reads release references); first fake-stream test failed because fake error callback never latched failure, corrected test to mimic CaptureIn.
- Done: implementation 8d2297802; final fmt exit=0 files=2 diff_lines=0.
- Full qbuild all completed PASS (session 38951 terminal exit=0).
- Full Rust gate: test exit=0 passed=6673 failed=0 ignored=0; clippy exit=0 diagnostics=0.
- Native request qbuild-20261023-233932-build-ctest PASS: build 220/220 edges; CTest 69/69; tree-change note is documentation-only commit 53b9bcb86 during wait, engine source unchanged.
- Next: collect unique game request automata-ext5-r1-2032-20261023-a with build-runs/wait-verdict.ps1 (event-based wait, no qwait), then inspect owner lines and ref warnings.
- Game request being submitted: automata-ext5-r1-2032-20261023-a, turbo, end_date=2032-01-01, Konradshohe. ABI exports changed; ABI version unchanged.
- q.py cannot start Git Bash in sandbox (CreateFileMapping denied); direct native cargo works. fmt needs absolute filenames.
- No guessed native signatures. Unknown referenced automaton/controller classes fail rather than being silently skipped.
- Manager fields: version/max count/active/time-of-day flag/automaton refs/source-building counts, embedded in each concrete record.
- Traffic fields: version, embedded base, default path type, layer cursor and complete incident list, on vehicle/pedestrian routes.
- Automaton fields: complete derived records including occupant base; vectors/radius/default speed/flags/dispatch/attractor/leader/state stack/model keys/callbacks/tweens/priority/group.
- Controller fields: version/occupant/template/position/lifetime scalars/flags/controlled refs; generator periods/group/count or attractor curve.
- Construction fields: occupant base/lot/building/transform/bounds/crane presence and position/prop and effect refs/all progress and crane scalars/crew tuples.

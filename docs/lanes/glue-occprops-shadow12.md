| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-occprops-shadow12 (occupants and props shadows) | Codex / glue-occprops-shadow12 | in progress | Inventory implemented owners, add contract-backed shadows, gate and measure to 2032 |

## Handoff
- Goal: default-off live shadows for implemented occupants and props owners, largest first.
- Worktree reused at detached HEAD f5b1805; initial tree clean.
- Coverage map owners: construction occupant; prop occupant; prop visual effect; prop wildcard; signpost; text decal; occupant wildcard; occupant filter wildcard; broad occupant wildcard; 3D placeable object; LinearRandomIterator.
- Done 2c2420ff7: default-off construction_shadow, typed live iteration, class/type checks and Rust position from independently observed native bounds (LR14/T79).
- Never calls native construction Write, Init, Shutdown, Draw or tick; stale effect path LR14 remains excluded.
- Map has only one explicitly implemented owner, construction (so byte-order selection is unambiguous). Subsystem native total in coverage-report is 69,634 bytes, not an owner-specific measurement.
- Focused Rust test passed 1/1; fmt passed on absolute owned path. Evidence build-runs/focused-construction.log (17m compile under shared load).
- q.py cannot start Git Bash here (CreateFileMapping access denied); focused Python runner logs subprocess output locally instead.
- Workspace test stage passed: 7,294 passed, 0 failed over 48 result groups (build-runs/qbuild-test.log).
- Final qbuild all PASS: workspace 7,294/0, clippy 0 diagnostics; queued native qbuild-20261024-160226-build-ctest passed build 263/263 and CTest 81/81. Operation 41598 collected to terminal exit 0.
- Native evidence: build-runs/fc4verify-build.log and fc4verify-ctest.log; Rust evidence qbuild-test.log and qbuild-clippy.log.
- Next: collect game wait operation exec session 10849 to terminal, then analyze with python build-runs/summarize-construction.py occprops12-t79-20261024-a.
- Existing open LD39 covers missing texture scales; do not duplicate it. Full lifecycle/render comparison requires passive tracing; this lane measures only getters.
- Game request outstanding: occprops12-t79-20261024-a, turbo to 2032-01-01, construction_shadow=1, refaudit=1. Run 512 reserved in docs/runs/occprops12-t79-20261024-a-512.md.
- Clean-room exclusions apply. Only task-named coverage-map.tsv and regress.ps1 are exceptions under tools; no tools scripts run.

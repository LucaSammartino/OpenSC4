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
- Next: collect existing final all operation (exec session 41598), currently clippy; do not duplicate it. Then queued native verify if all does not provide it, then game to 2032.
- Existing open LD39 covers missing texture scales; do not duplicate it. Full lifecycle/render comparison requires passive tracing; this lane measures only getters.
- No game request outstanding. Run 512 reserved in docs/runs/occprops12-t79-20261024-a-512.md; request label occprops12-t79-20261024-a.
- Clean-room exclusions apply. Only task-named coverage-map.tsv and regress.ps1 are exceptions under tools; no tools scripts run.

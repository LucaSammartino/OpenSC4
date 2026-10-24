| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-occprops-shadow12 (occupants and props shadows) | Codex / glue-occprops-shadow12 | in progress | Inventory implemented owners, add contract-backed shadows, gate and measure to 2032 |

## Handoff
- Goal: default-off live shadows for implemented occupants and props owners, largest first.
- Worktree reused at detached HEAD f5b1805; initial tree clean.
- Coverage map owners: construction occupant; prop occupant; prop visual effect; prop wildcard; signpost; text decal; occupant wildcard; occupant filter wildcard; broad occupant wildcard; 3D placeable object; LinearRandomIterator.
- Added default-off construction_shadow: typed live occupant iteration, class/type checks and Rust position from independently observed native bounds (LR14/T79).
- Never calls native construction Write, Init, Shutdown, Draw or tick; stale effect path LR14 remains excluded.
- Map has only one explicitly implemented owner, construction (so byte-order selection is unambiguous). Subsystem native total in coverage-report is 69,634 bytes, not an owner-specific measurement.
- Next: collect focused Rust test, run final all plus queued native verify, then game to 2032.
- Existing open LD39 covers missing texture scales; do not duplicate it. Full lifecycle/render comparison requires passive tracing; this lane measures only getters.
- No game request outstanding; no source changes yet.
- Clean-room exclusions apply. Only task-named coverage-map.tsv and regress.ps1 are exceptions under tools; no tools scripts run.

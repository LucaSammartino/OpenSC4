| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-simgrid-lots-shadow5: sim grid and lot record shadows | Codex, `.claude/worktrees/glue-simgrid-lots-shadow5` (reserved detached HEAD) | in progress | Inspect documented read contracts and shadow templates. |

## Handoff
- Goal: default-off simgrid_shadow and lotrec_shadow, full comparisons at city load and monthly through 2032.
- Done: reused reserved clean detached worktree; inspected simgrid sections 1-6, lot-records sections 1/3.1/3.2, SDK city/lot-manager interfaces, existing grid shadow/probe, WriteCapture, and zone-developer shadow.
- Finding: existing simgrids_shadow verifies constructed/natural operations; simgrids_probe observes only FreeCity4 stream reads, not a complete original-grid inventory.
- Question: SG3 appended to docs/specs/questions/simgrid.md; full live-grid discovery and its coverage boundary are unspecified.
- Next: obtain SG3 inventory/enumeration contract or explicit persisted-grid scope, then implement the two requested record shadows.
- Constraints: clean room; no tools/ access; no native signature guessing; missing contract means question and wip.
- No builds or game requests started.

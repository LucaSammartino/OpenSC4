# Lane glue-propdev4 (finish/validate the cFC4PropDeveloper replacement, enable=propdev)

| Lane | Worktree | State at stop | Next steps |
|---|---|---|---|
| glue-propdev4: validate cFC4PropDeveloper (enable=propdev) for replace credit | .claude/worktrees/glue-propdev4 | in progress | see Handoff |

## Handoff
- goal: existing cFC4PropDeveloper (glue12b1) -> trace Read/Init/Write/Shutdown/ticks/prop requests, clean game run ref0, shadow, full gate; find why replace credit absent.
- done: worktree at main 66415952; baseline qbuild all PASS (cargo 3652, clippy 0, ctest 62/62). Game glue-propdev4-r1 (enable=propdev, turbo to 2028, save 2027-03-01): clean ref0; Read/Init/AddAgent x2/13 subs/Write(save)/RemovePropRequester/Shutdown/destroy all traced, 24 month lines.
- next: wait glue-propdev4-sh1 (propdev_shadow=1, no enable; queued) with qwait (no --fresh); judge mismatches.
- facts: replace credit absent because no landed run ever enabled propdev (regress logs: 'propdev: off by default'; apply15b run predates glue12b1). r1 lists: Read power=466 tod=1175 dated=19 -> month 2026-02 power=2 tod=97 dated=14 (check vs shadow).

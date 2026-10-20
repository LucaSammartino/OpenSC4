# mysims1 — the mysims shadow (a same-process differential of the MySims)

| lane | base | tip | state | abi | game runs | summary |
|---|---|---|---|---|---|---|
| mysims1 | main | - | in progress | no | - | the mysims shadow: FC4MySimsShadow + regress shadows list, in-game sh-1/sh-2 |

## Handoff

- Goal: add the mysims shadow (like the other shadows), wire it into the regress
  shadows list; exact to 2032 on the test city; in-game runs mysims1-sh-1 (shadows
  on, original classes) and mysims1-sh-2 clean with ref_warnings=0.
- Done:
  - worktree C:/Users/tlori/FreeCity4/.claude/worktrees/mysims1, branch mysims1,
    cut from main (7e042544); submodule repointed to this worktree and updated.
- Facts:
  - The mysims core lives in engine/core/src/mysims/ (agent.rs, mod.rs, sim.rs,
    scripted.rs, tests.rs); the C ABI in engine/capi/src/advisors.rs
    (fc4_mysims_*, fc4_mysim_*; fc4_mysim_state 240 bytes, fc4_mysims_agent_state
    40 bytes in fc4.h).
  - The plugin has NO MySim glue yet: nothing in engine/plugin/ references
    fc4_advisors; the game's own cSC4MySimAgentSimulator is the original (the
    shadow compares against it). City slot for the agent: the agent simulator is
    a city simulator (vtable 0x00A85544 in docs/abi/interfaces.tsv:3605).
  - Shadow pattern (FC4Clock24Shadow.cpp / FC4CivicShadow.cpp): fc4_* shadow core
    via a C-ABI handle; OriginalOf(city) via the city getter + IsOurObject;
    HistoryShadow/CivicShadowDeliver style wrap of the original's delivery in
    cFC4Simulator::HostDeliver; a message target for SimNewMonth month lines;
    SettingInt("mysims_shadow", ...) in FC4Config.cpp kOtherKeys (additive, end
    of list); regress.ps1 $runs.shadows string (additive).
- Next: read mysims.md 4/6/7 detail + how the original's agent is reached from
  the city (cISC4City slot for the MySim agent simulator); design the comparison.

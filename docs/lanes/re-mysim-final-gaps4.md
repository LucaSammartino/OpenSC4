# re-mysim-final-gaps4 — answer MS13/MS14/MS15 (mysims §4.6/§4.7/§4.7.1/§4.12/§7.2)

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-mysim-final-gaps4 (answer MS13-MS15 mysims location/import/commute contracts) | `re-mysim-final-gaps4` / `.claude/worktrees/re-mysim-final-gaps4` | in progress | see Handoff |

## Handoff
- Goal: audit preserved WIP and resolve only public MS13–MS15 with byte/decompile evidence; docs only.
- Done: audited scope of 4c714151 (27 spec lines, not yet verified); targeted specs have no other delta against main. Existing build-runs/rd.py modification is a read-only PE constant reader, not evidence of native execution.
- Done this round: verified MS13 location bytes and corrected rounding/local-copy wording; proved MS14 zero-initialized family count, signed key formatting, short-name whole-stem and missing text skip branches. Native-derived examples are arithmetic/formatting checks, not game measurements.
- Next: finish MS15 spec and question moves. CheckCommute uses building slot 3 → occupant slot 11 GetBoundingCityCells, NOT lots; calls pathfinder slots 5(1),9(3),18(1),19(home),20(job or stored f0/f4),23; only result 1 succeeds, slot26→_ftol2→low32 at +154. Early missing references/null factory output returns false without FailCommute. Release follows success/failure handler.
- MS15 trip proof: active = state nonzero (004277AB→00449550); subject uses new state at 00427980; force zeros local location world AND cell at 00427C60; congestion PositionToTract uses local world; ratio local cell. Detailed static asm files in build-runs/ms15-trip-asm.txt and ms14-import-asm.txt (do not commit exports).
- Facts: detached HEAD is intentional; do not update refs. q.py cannot start Bash (E_ACCESSDENIED), so use capped PowerShell/local Python. Static export contains CheckCommute, but its pseudocode has broken variable recovery; use bytes for arguments/order. No REA request needed so far.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-mysim-final-gaps4-r2-manifest.json; no engine/tools/ABI edits, runtime tests or subagents.


## Coordinator quota pause (2026-10-22)
Provider weekly limit; resets 18:00 Europe/London. This round stopped. Existing work is preserved as WIP, not ready or verified. Resume only after the provider limit clears and fresh slots/check --resume pass; the round still counts.

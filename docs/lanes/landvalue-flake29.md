| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| landvalue-flake29: first-pass input diagnosis; kind engine | Codex / .claude/worktrees/landvalue-flake29 / detached HEAD (reserved branch landvalue-flake29) | WIP; ABI unchanged; no seam reservations; quick queued; game runs pending; questions none | 1. Compare scripted and load-only Tegel/Spandau. 2. Sample census groups and house. 3. Gate and report remaining capture limitation. |

Early handoff: census pass 1 does not imply unedited inputs.
Tegel script edits terrain (4,4)..(54,54) on 2029-08-23, before pass 1 on 08-26.
That rectangle intersects exactly 49 slope tracts, matching census slope mismatches.
Spandau script edits the same rectangle on 2001-09-12, before pass 1 on 09-15.
Sources: engine/tests/scenarios/scripts/berlin-{tegel,spandau}.script.
Adding opt-in city-init/day/pass-end slope/altitude observations and an explicit
observed-change pass counter; these are diagnostics, never native capture/replay inputs.
Native capture remains disabled under landvalue sections 6.1.5-6 (LV9/LV10).
No core formula, tolerance, output selection or acceptance change.
Next: build, paired controlled runs, then measure census/house and gate.

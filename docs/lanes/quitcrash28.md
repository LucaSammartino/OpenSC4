| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| quitcrash28: construction teardown; kind engine | Codex / quitcrash28 / detached HEAD | WIP; c7d5b60e9 one-action reduction, fc69c58e0 default-off posted quit; ABI unchanged; no questions | 1. Compare posted and tick-owned quit. 2. Pause/save/clear controls. 3. Gate and enqueue. |

Handoff (resumed round; measurements below are retained round-1 results):
- Prior handoff was absent in both worktree and main; resumed from commits and build-runs/suite artifacts.
- Minimal script: engine/tests/scenarios/scripts/quit-construction28.script, one airport at (8,11), 2026-01-30; end 2026-02-01.
- qc28-port2 plain (DLL SHA256 624dd491...): one accepted action, target reached, clean-exit-crash at 0x005D0F94; three-month wait and dezone also fault.
- qc28-pausesave pause control: two accepted actions, target reached, same teardown fault; save control stalled and is inconclusive.
- Earlier qc28-portplain had rejected actions and clean exits: not a valid reproduction; qc28-fullplain had stale DLL/failed actions: not a passing control.
- Current source includes autotest_quit_post, script_port_instant, script_port_probe, all off by default; native command delivery comparison pending.
- Cause not yet assigned. re-nm28 finding supplied by coordinator: ConstructionOccupant +0x13C dust effect is null when Init was not followed by successful SetLot (or effect was released).
- qc28r2-paths-legacy: accepted 1/1, end reached, same 0x005D0F94. qc28r2-paths-post and qc28r2-controls-* returned queued=0 and terminated before shutdown: INVALID controls, not clean-exit evidence.
- Suite now rejects that false-clean fallback and recognizes accepted posted-command exit crashes; focused suite tests passed 21/21 before the added failed-dispatch test.
- Next: compile pinned current DLL; shared queue, plain profile, parallel=1 before [ini], at most three in flight; run both quit paths before other variants.

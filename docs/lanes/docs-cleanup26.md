| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| docs-cleanup26 (instruction docs cleanup) | `Codex` / `docs-cleanup26` (tooling lane) | WIP: changes committed through `9539c6b03`; doccheck/check, generated tables and four prompt renders pass; pytest 165 passed, 6 failed (Node child-process EPERM); ABI unchanged; no game runs/questions | (1) Rerun full tools suite where Node child processes work; (2) mark queue ready after pass |

Moved five histories; consolidated 15 instruction docs against the canonical prompt; added 12 STALE rows and regression coverage, including question README scanning.
Per-round doccheck bytes before→after: engine 50098→29852 (-40.4%); re 46955→27407 (-41.6%); tool 45173→26099 (-42.2%). Prompt bytes unchanged.
Raw file bytes before→after below (Python equivalent of wc -c; Git Bash/wc fail with Win32 signal-pipe error 5 here).
- docs/ops/rules.md 2187→1474; docs/ops/conventions.md 3060→1691.
- docs/ops/lessons.md 2222→1791; docs/ops/running-the-game.md 13370→9471.
- docs/ops/lane-providers.md 9640→8147; docs/ops/coordinating.md 31788→20321.
- docs/harness.md 9984→5625; docs/lanes/README.md 9791→4624.
- docs/lane-tools.md 9788→6178; docs/re-workspace.md 9960→5845.
- docs/merge-train.md 6614→2283; docs/specs/questions/README.md 3342→1721.
- CONTRIBUTING.md 4955→3117; .claude/agents/fc4-lane.md 1007→367; .claude/agents/fc4-lane-re.md 1067→443.
- docs/ops/run4-recovery.md→docs/history/run4-recovery.md 3870→3877 (provider link fixed).
- docs/ops/acceptance-incident-train121.md→docs/history/acceptance-incident-train121.md 3294→3294.
- docs/ops/forge-pilot.md→docs/history/forge-pilot.md 12499→12499; docs/ops/efficiency-log.md→docs/history/efficiency-log.md 13482→13482.
- docs/token-ledger.md→docs/history/token-ledger.md 6857→6857; 80-byte pointer retained for the protected HANDOFF link.
Measurements saved in build-runs/docs-{bytes,files}-{before,after}.log; raw bytes include CRLF, doccheck normalizes CRLF.
Real-tree checks: `python tools/doccheck.py check` printed `doccheck check: ok`; `python tools/gen_handoff_tables.py --check` printed `up to date (948 lanes, 570 runs)` before this final record.
`node tools/lane_prompt.mjs` rendered via build-runs/doc-measure.py: engine/re/tool/mech PASS (no workflow/model calls).
`python -m pytest tools/tests/test_doccheck.py -q`: 9 passed in 0.12s; `python -m pytest tools/tests -q`: 6 failed, 165 passed in 29.90s (build-runs/docs-pytest-final.log).
All six failures are unchanged WorkflowDryRun tests whose preflight gets ERROR; diagnostic node build-runs/doc-test-spawn.mjs prints spawnSync cmd.exe EPERM and spawnSync python EPERM (build-runs/docs-spawn-diagnostic.log).
Preserved schemas/defaults, coordinator launch/resume/landing/credit procedures and lessons §5.1/§5.1b. Uncertain paragraph deletions: none. Engine untouched; qbuild/game checks n/a.
Next: coordinator reruns the required full suite with Node child-process execution available; queue remains wip until it passes.

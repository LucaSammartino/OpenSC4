| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| docs-cleanup26 (instruction docs cleanup) | `Codex` / `docs-cleanup26` / reserved branch `docs-cleanup26` (tooling lane, detached HEAD) | READY per coordinator: main merged at `2a186a749`; doccheck and four prompt renders pass; pytest 231 passed, 7 failed (six known EPERM, one timeout that passed alone); ABI unchanged; no game runs/questions | (1) Coordinator reviews/enqueues coverage credit; (2) regenerate lane index at landing; known sandbox failures accepted by coordinator's outside-sandbox run |

Moved five histories; consolidated 15 instruction docs; added 12 STALE rows/tests, including question README scanning. Main merged with Codex retry/time limits and coverage credit retained; added review enqueue and regress summary ordering.
Per-round doccheck bytes before→after: engine 50098→29922 (-40.3%); re 46955→27477 (-41.5%); tool 45173→26169 (-42.1%). Prompt bytes unchanged.
Raw file bytes before→after below (Python equivalent of wc -c; Git Bash/wc fail with Win32 signal-pipe error 5 here).
- docs/ops/rules.md 2187→1474; docs/ops/conventions.md 3060→1691.
- docs/ops/lessons.md 2222→1791; docs/ops/running-the-game.md 13370→9471.
- docs/ops/lane-providers.md 9640→8147; docs/ops/coordinating.md 31788→22246.
- docs/harness.md 9984→5625; docs/lanes/README.md 9791→4695.
- docs/lane-tools.md 9788→6178; docs/re-workspace.md 9960→5845.
- docs/merge-train.md 6614→2283; docs/specs/questions/README.md 3342→1721.
- CONTRIBUTING.md 4955→3117; .claude/agents/fc4-lane.md 1007→367; .claude/agents/fc4-lane-re.md 1067→443.
- docs/ops/run4-recovery.md→docs/history/run4-recovery.md 3870→3877 (provider link fixed).
- docs/ops/acceptance-incident-train121.md→docs/history/acceptance-incident-train121.md 3294→3294.
- docs/ops/forge-pilot.md→docs/history/forge-pilot.md 12499→12499; docs/ops/efficiency-log.md→docs/history/efficiency-log.md 13482→13482.
- docs/token-ledger.md→docs/history/token-ledger.md 6857→6857; 80-byte pointer retained for the protected HANDOFF link.
Baseline: build-runs/docs-{bytes,files}-before.log; final: build-runs/docs-final-{bytes,files}.log (earlier after logs retained). Raw bytes include CRLF; doccheck normalizes CRLF.
Real-tree `python tools/doccheck.py check` printed `doccheck check: ok`; `bytes` printed the kind totals above.
`python tools/gen_handoff_tables.py --check` printed `out of date: docs\lanes\INDEX.md: lanes`; coordinator regenerates at landing, as directed.
`node tools/lane_prompt.mjs` via build-runs/doc-measure.py rendered engine/re/tool/mech: PASS (no workflow/model calls).
`python -m pytest tools/tests -q`: `7 failed, 231 passed in 241.22s`; six known WorkflowDryRun EPERM failures left alone as directed, plus a PowerShell city-fixture timeout (20s).
Retry `python -m pytest tools/tests/test_city_fixture.py::CityFixtureTests::test_real_copy_preserves_source_documents_and_strips_harness_key -q`: `1 passed in 0.43s`. Full-suite failure remains recorded separately in build-runs/docs-final-pytest.log.
Preserved schemas/defaults, coordinator procedures and lessons §5.1/§5.1b. Uncertain paragraph deletions: none. Engine untouched; cargo/clippy/ctest/qbuild/game n/a.
Next: coordinator reviews the ready queue with enqueue (attaching credit) and regenerates the index at landing; no further lane work.

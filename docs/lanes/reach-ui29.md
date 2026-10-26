| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| reach-ui29 UI action drivers; kind engine | Codex / reach-ui29 / reserved branch reach-ui29 (detached HEAD) | WIP; code commits 66a255efe, d2887bd5e, 1965848a5, aaeaa96a2; ABI unchanged, no seam reservation; cargo 7901, clippy 0 diagnostics, CTest 125/125; reach-ui29-p1 pending, no verdict/ref/shadow measurements; questions none | 1. Service shared queue and collect p1. 2. Qualify all verbs and census rows. 3. Complete missing owner paths and measure bytes before/after; enqueue engine only after qualification. |

- Seven verbs implemented: uicommand, uimouse, uicell, uicontrol, uikey, uitext, uiwait; documented in docs/ops/ui-drivers.md.
- Command IDs from installed accelerator data; budget/news button IDs from installed UI data. No extracted game data committed.
- Tick delays and modal input pumping preserve ordered scripts; command execution uses verified slot 20 and separate retained sets.
- Census candidates: ui29-house (Konradshohe), ui29-weburg (West Weburg), ui29-bevopolis (Bevopolis); suite.tsv group ui29-candidates, excluded from full.
- Qualification: 0/7 verbs proved in game; all three census rows unqualified. No additional reach bytes claimed.
- Startup: reach_startup=1 arms in director construction; distinct 1999 startup / 2000 load windows. Actual startup gain is unmeasured.
- Final qbuild all: 7901 passed, 0 failed; clippy 0 diagnostics; CMake 44/44; CTest 125/125, 79.3s.
- Gate logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log.
- Earlier all: 124/125 (ret_channel_check); fixed documented return use. A subsequent pass had a citation edit during its run; final run above had no tree-change caveat.
- Focused grammar executable passed; focused return-channel check: 0 flagged, 103 reviewed.
- Game request: C:/Users/tlori/FreeCity4/build-runs/game-queue/reach-ui29-p1.req, still unconsumed after two qwait timeouts; no running marker, verdict or game log. One request outstanding.
- Request DLL is this worktree's engine/build-x86/FreeCity4.dll; plain/default shadows, visible client, 60-second runtime; no Documents or real Plugins writes.
- Before bytes (run7/reachgap.txt); every after value pending: Query 0/75807; SubBudget 269/73268; TextEdit 270/19999; TreeView 32/16893.
- Before / after pending: WWW 783/34049; HTTP 0/15133; Nation 355/10945; Volcano 0/10422; App 7846/85964.
- Startup before / after pending: ConfigRuleManager 0/15096; EffectsResource 0/28498; RenderProperties 38/22115; effects parser commands 0/42078 (task baseline).
- Missing: budget subpanels, map layers, tree controls, web/HTTP, nation/volcano scenarios; actual lot query, road placement and text-edit owner entry; startup parser-command roster and all owner byte deltas.
- Ignored helpers/data: build-runs/ui-contracts.tsv (767 entries), ui-reach-contracts.py, read-ui-data.py; the reduced roster currently excludes effects parser command owners.
- Next: collect p1 before another request; enable ui_command_catalog for later ID diagnosis, calibrate control/cell targets, replay three cities with clean verdict/ref_warnings=0 and exact or explained shadows.
- Not ready and not enqueued for landing; a WIP engine merge-queue record preserves the full tip for the supervisor. Questions: none.

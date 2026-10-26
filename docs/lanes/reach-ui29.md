| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| reach-ui29 UI action drivers; kind engine | Codex / reach-ui29 / reserved reach-ui29 (detached HEAD) | Ready qualified delivery; code 09c9830cf, e1de80a37, c90ad57fc; suite included; ABI unchanged, no seam reservation; cargo 7959, clippy clean, CTest 129/129; p18/p20/p22/p23 clean/ref=0, 102/102 actions, shadow 40/40 exact; UI29-HTTP and UI29-TREE open, cell query unqualified | 1. Coordinator attach coverage credit and land queued tip. 2. Answer HTTP/tree questions and qualify cell targeting in follow-up work. |

- Implementation: command/input grammar, early startup tracing, explicit-cell volcano; end-date quit drains pending UI and the final delay (e1de80a37). c90ad57fc settles Bevopolis startup UI and narrows the tree question.
- Final qbuild all PASS, run once this round: cargo 7959/7959, clippy 0 diagnostics, CMake 172/172 edges, CTest 129/129; logs build-runs/qbuild-{test,clippy}.log and fc4verify-{build,ctest}.log. Focused harness test passed directly.
- Native gate verdict: build-runs/build-results/qbuild-20261026-180508-build-ctest.verdict; fc4verify PASS in 138.0s. Subsequent tracked edits are suite/docs metadata only.
- Final DLL SHA256 436a575c5c41852f9ae5ec9de0571b7661113660aa7945ae9146d4fb345b5f26; all four final runs match the rebuilt artifact.
- Evidence: C:/Users/tlori/FreeCity4/build-runs/game-results/reach-ui29-pN.{verdict,log}; traces build-runs/ui29-pN.reach.tsv; report build-runs/ui29-evidence.py (N=18,22,20,23).
- p18 house 50/50, end 2027-07-05; p22 West Weburg 22/22, end 2197-06-01; p20 Bevopolis 27/27, end 2258-01-11. All clean/ref=0; requested end dates crossed while UI drained.
- p23 volcano 3/3, clean/ref=0, end 2026-11-10, 55.797 loaded seconds; lava 26/28 methods / 7306 B; explicit cell 63,63 -> world 1016,1016, 55s real-time wait completed.
- gzfw_shadow exact: p18 18/18, p22 3/3, p20 9/9, p23 10/10 = 40/40; every sample buffer 140, command 75, differences=0 (the four logs).
- docs/ops/ui-drivers.md documents all seven UI verbs and volcano cells; engine/tests/scenarios/suite.tsv has all four full,ui29 rows, including the house and two Maxis cities.
- p18 Query OnMouseDownL 004d4d70 / DescribeBuilding 004ccf60, 17 sub-budget methods and text insertion entered; map ShowWindow and both grid display methods entered (20165 B).
- p22 road down/up plus PlaceNetwork 0063b830 / PlaceNetworkOccupants 00629210 entered; no saved-network assertion. p20 news PurgeInactive 00772ee0 first enters window 7 containing the 2257-08-03 button, distinct from periodic flipper display.
- Baseline -> new-run union, named-function entered bytes: Query 0 -> 28452; SubBudget 269 -> 19677; TextEdit 270 -> 9791 (report and four traces).
- Baseline -> new-run union: TreeView 32 -> 0; WWW 783 -> 649; HTTP 0 -> 0; Nation 355 -> 3351; Volcano 0 -> 7306; App 7846 -> 21945. No HTTP/tree gain claimed.
- Startup owners baseline -> new-run union: ConfigRuleManager 0 -> 7647; EffectsResource 0 -> 4196; RenderProperties 38 -> 19918; parser commands 0 -> 3381 (four traces).
- Startup window 1 arms before subsystem/application init; Config/parser/Render/Nation/App enter there, EffectsResource at load window 2. Earlier city-only arming missed this reach (reach lifecycle/window lines).
- Roster: 1006 ABI entries, adding four unnamed news bodies assigned by news-ticker.md. These are new-run unions, not baseline-plus-full-suite union; coordinator owns aggregate credit.
- Cell input remains delivery-only: p20 height 0 posts 478,354 with no query mouse handler, as height 250 did at 478,198 in p15. Bevopolis qualifies news, not cell targeting or terrain intersection.
- UI29-TREE, docs/specs/questions/ui-windows.md: slot 4 element-ID lookup reconciled with ui-script-service.md; cleanup/import initialization still open. Corrupt p7 candidate remains removed; no tree driver reinstated.
- UI29-HTTP, docs/specs/questions/web-network.md: ordered public LoadDocument/string/target signature unresolved; no guessed binding, network request or web coverage claim.
- Failed evidence stays distinct: p19 stalled before script without dialog closer; p22 with suite [ini] close_dialogs=1 closed two dialogs and passed. p21 rejected before launch for misplaced setting, corrected under new labels.
- Ready queue: build-runs/merge-queue/reach-ui29.json uses the final full HEAD; shared game queue only, no requests outstanding. Remaining HTTP/tree/cell acceptance is explicitly deferred; Next: coordinator attach credit and land the qualified delivery.

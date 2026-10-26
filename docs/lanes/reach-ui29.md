| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| reach-ui29 UI action drivers; kind engine | Codex / reach-ui29 / reserved reach-ui29 (detached HEAD) | WIP partial delivery; merged main 77be16561; code 09c9830cf, e1de80a37; suite aeb81b104; ABI unchanged, no seam reservation; cargo 7959, clippy clean, CTest 129/129; p12-p15 clean/ref=0, shadow 33/33 exact; UI29-HTTP and UI29-TREE filed | 1. Resolve HTTP/tree questions. 2. Qualify cell and causal news input. 3. Complete owner coverage and enqueue engine ready. |

- Code: explicit-cell volcano manager action; end-date quit now drains queued/active UI actions and final input delay while preserving the wall deadline.
- qbuild all PASS: cargo 7959/7959, clippy 0 diagnostics, CMake 29/29, CTest 129/129 (218.5s). Final later edits only suite/docs metadata; focused harness test passed again.
- Gate logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log; native verdict build-runs/build-results/qbuild-20261026-174040-build-ctest.verdict.
- Evidence root: C:/Users/tlori/FreeCity4/build-runs/game-results/reach-ui29-pN.{verdict,log}; traces: this worktree build-runs/ui29-pN.reach.tsv; report build-runs/ui29-evidence.py.
- Final DLL SHA256 03895f4b875211465fb961101d1c1c703eb769be74a9a011e95487fd8d925b98; p12/p13/p14/p15 all match.
- p13 house: clean/ref=0, 50/50 actions, end 2027-07-12 after requested 2026-04-01; one harness popup closure. p14 West Weburg: clean/ref=0, 22/22, end 2197-07-04 after requested 2197-06-01.
- p15 Bevopolis: clean/ref=0, 23/23, end 2257-08-26 after requested 2257-07-01. p12 volcano: clean/ref=0, 3/3, end 2026-08-24, 70.859 loaded seconds; 55s wall wait completed.
- End-date overshoot is intentional while UI drains; p8 was clean but incomplete because graceful quit bypassed pending actions. e1de80a37 fixes this; p9-p11 also clean/complete on earlier DLL afb3f2fd.
- gzfw_shadow p13 18/18, p14 4/4, p15 4/4, p12 7/7 exact; every sample buffer 140/140, command 75/75, differences=0.
- Query p13: OnMouseDownL 004d4d70 and DescribeBuilding 004ccf60; sub-budget 17 methods; text insertion entered. Road p14: mouse down/up and PlaceNetwork 0063b830 / PlaceNetworkOccupants 00629210; no saved-network assertion.
- Map p13: ShowWindow 007a6290, DisplayStandardUint8Grid 007a2380, DisplayStandardSint16Grid 007a2c70; 20165 entered bytes. News periodic display 00773980 entered (6754 B owner); causal button path remains unqualified.
- Volcano p12: 26/28 traced cSTEVolcanoLava methods, 7306 B entered; cell 63,63 maps to world 1016,1016; emergency pause ended and requested date was crossed.
- Baseline -> newly observed union of p12-p15, entered named-function bytes (not total suite union): Query 0 -> 28452; SubBudget 269 -> 19677; TextEdit 270 -> 9791.
- Baseline -> newly observed: TreeView 32 -> 0; WWW 783 -> 649; HTTP 0 -> 0; Nation 355 -> 3351; Volcano 0 -> 7306; App 7846 -> 21945. No HTTP/tree gain claimed.
- Startup owners baseline -> newly observed: ConfigRuleManager 0 -> 7647; EffectsResource 0 -> 4196; RenderProperties 38 -> 19918; parser commands 0 -> 3381.
- Startup window 1 first-enters Config 7647 B, parser commands 3381 B, RenderProperties 1178 B, Nation 3028 B, App 18197 B; EffectsResource first enters load window 2 (4196 B). Earlier city-only arming missed these.
- The roster now has 1002 ABI entries including road/map/news/volcano managers. Baselines are sh28 census values; coordinator owns full-suite union/credit, which was not recomputed here.
- Full/ ui29 rows: house, West Weburg and volcano. Bevopolis stays ui29-candidates: projection posts at 478,198 but Query enters only lifecycle, no mouse handler; no qualified uicell action.
- Tree template p7 returned null but left a visible child; Down caused AV 009d1676, corrupt/ref=0, no completed trace. Driver removed. UI29-TREE: docs/specs/questions/ui-windows.md.
- Public WWW LoadDocument argument order/string type remains unqualified; no guessed binding or external-browser coverage. UI29-HTTP: docs/specs/questions/web-network.md.
- No requests outstanding; shared queue only, one at a time. Missing tree/HTTP and cell/causal-news acceptance; WIP local queue record only, no ready enqueue. Next: answer questions and finish these drivers.

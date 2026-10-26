| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| reach-ui29 UI action drivers; kind engine | Codex / reach-ui29 / reserved branch reach-ui29 (detached HEAD) | WIP partial delivery; code 1a9d05e44, 6570fca0b, 6d46b332b; suite/docs d020a11e2; ABI unchanged, no seam reservation; cargo 7901, clippy clean, CTest 125/125; p1-p6 clean/ref_warnings=0; p3-p6 framework shadows exact; questions none | 1. Remove refused close from candidate. 2. Qualify cell/road/news targets and missing owners. 3. Replay all three census rows, then enqueue engine. |

- Seven documented verbs retained; host input now uses SDK cIGZFrameWorkW32::GetMainHWND; hidden/disabled control ancestors are refused.
- Final source gate: cargo 7901/7901, clippy 0 diagnostics, CMake 46/46, CTest 125/125 (83.1s); final all has no tree-change caveat; later edits only docs/suite metadata.
- Gate logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log. Earlier all with tree-change caveat is superseded.
- Evidence root: C:/Users/tlori/FreeCity4/build-runs/game-results/reach-ui29-pN.{verdict,log}; traces: this worktree build-runs/ui29-pN.reach.tsv.
- p1 reused, never resubmitted: clean/ref=0, 36s dialog hold; input skipped because host class lookup failed. p2 fixes that; all 32 original actions complete.
- p3 West Weburg: clean/ref=0, all 20 actions complete, end 2200-01-09; road placement itself unproved.
- p4 Bevopolis: clean/ref=0, all 21 actions complete; cell 63,63,250 posts at 478,198; query handler unproved; emergency pause at 2258-04-14 misses end date.
- p5: clean/ref=0, but hidden sub-budget buttons enter no owner; ViewWebBrowser status=2 (documented failure), no HTTP reach. Superseded by p6 for budget.
- p6 final DLL: clean/ref=0, end 2029-01-09; 43/44 actions delivered, one already-hidden close refused; harness also closed one popup. Candidate is not a full-suite pass.
- p3/p4/p5/p6 gzfw_shadow: each monthly sample buffer 140/140, command 75/75, differences=0; p1/p2 plain, no shadow comparison claim.
- Query action proof p2/p6: OnMouseMove 004cbc70, OnMouseDownL 004d4d70; text proof InsertText 009bf5b7 and InsertTextHighLevel 009c07e1.
- Sub-budget proof p6: 17 entered methods, including CreateSubBudgetWindow, SetupWindowBig, initOrdinanceBudget, on_init/on_shutdown and DoWinMsg.
- Before -> observed p6 bytes (not union/suite credit): Query 0 -> 28583; SubBudget 269 -> 19677; TextEdit 270 -> 9791; TreeView 32 -> 0.
- Before -> observed p6 bytes: WWW 783 -> 649; HTTP 0 -> 0; Nation 355 -> 3351; Volcano 0 -> 0; App 7846 -> 21927 (p3 observes 21945).
- Startup owners before -> p6: ConfigRuleManager 0 -> 7647; EffectsResource 0 -> 4196; RenderProperties 38 -> 19918; parser commands 0 -> 3381.
- Startup window 1999-01-01 first-enters ConfigRuleManager 7647 B, parser commands 3381 B, RenderProperties 1178 B, App 18197 B, Nation 3028 B; earlier city-only arming missed these.
- EffectsResource 4196 B first enters load window 2000-01-01, not startup. These are entered function sizes from docs/abi, not measured exclusive execution bytes.
- Roster expanded 767 -> 847 to include 80 anonymous command entries; reports from ignored ui-reach-report.py/ui-methods.py. No extracted game data committed.
- All three suite rows remain ui29-candidates, outside full; end dates now allow wall-time UI actions to finish. Verbs/visible budget IDs documented in docs/ops/ui-drivers.md.
- Missing acceptance: map layers, tree controls, HTTP/web, volcano, actual lot result and road placement, qualified cell/news targets, all-three-city complete census; no total suite-credit claim.
- No requests outstanding; shared queue only, at most one at a time; no user Documents/real Plugins writes. No ready enqueue; WIP engine queue record preserves tip.
- Next: qualify remaining owner paths and candidate action refusals, replay census, then enqueue engine. Questions none; no ABI exports changed.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| records-cities29: citywide record shadows (engine) | Codex / .claude/worktrees/records-cities29 / reserved branch records-cities29, detached HEAD | wip; recovered code tip 77e3eaf136cb2f0ff568ae201f7c40e685c08f67; ABI unchanged; suite12 collected 13 clean/exact, 1 timeout, 3 awaiting collection | 1. Merge main. 2. Collect existing suite12 requests without duplication; investigate South Weburg timeout. 3. Final gate, refresh measurements below and enqueue engine on final HEAD. |
Recovery at 77e3eaf1: app target captures mode-message inputs before modal delivery; native-save continuation reacquires interrupted DBPF work; capture-time pause keeps every promised sample at its original date. Prior gate below predates these changes; final qbuild all remains required.
Evidence root R=C:/Users/tlori/FreeCity4/build-runs; suite12=R/suite/records-cities29-12 (DLL SHA256 5352911079fed4dfa8980128de68dc50e1af41e10ace95ff33928eba9e721ec6). Its summary.tsv: 13 clean/end=1/ref_warnings=0, all seven requested keys exact; South Weburg timeout/end=0, house Tegel/growth/views not yet collected. Existing growth/views queue requests remain outstanding; do not duplicate them.
Historical measurements below are suite6/7, not suite12: before=R/run7/census3 (reach29 baselines R/run7/reach29); after=R/suite/records-cities29-{6,7}/<scenario>.log. D=B (dbpf and dbpf_bounds separately identical), C(citysit), R(dispatch), Z(zones), A(app), S(app-startup). Prior gate: cargo 7893, clippy 0 diagnostics, CTest 125/125.
- 6/london-kensington: D=B 4/5->5/5; C 5/5->5/5; R 5/5->5/5; Z 20/20->20/20; A 4/4->4/4; S 1/1->1/1.
- 6/timbuktu-getting-started: D=B 0/4->4/4; C 4/4->4/4; R 4/4->4/4; Z 16/16->16/16; A 3/3->3/3; S 1/1->1/1.
- 6/gen27-konradshohe-s0-default: D=B 19/25->25/25; C 25/25->26/26; R 25/25->26/26; Z 100/100->104/104; A 24/24->25/25; S 1/1->1/1; verdict is corrupt/clean-exit-crash, not a pass.
- 6/maxisland-aquaton: D=B 0/3->3/3; C 0/3->4/4; R 3/3->4/4; Z 12/12->16/16; A 2/2->3/3; S 1/1->1/1.
- 6/maxisland-banktonoplankton: D=B 1/3->3/3; C 0/3->4/4; R 3/3->4/4; Z 12/12->16/16; A 2/2->3/3; S 1/1->1/1.
- 6/maxisland-bendburger: D=B 0/3->3/3; C 0/3->3/3; R 3/3->3/3; Z 12/12->12/12; A 2/2->2/2; S 1/1->1/1.
- 6/maxisland-bevopolis: D=B 3/3->3/3; C 0/3->3/3; R 3/3->3/3; Z 12/12->12/12; A 1/2->0/2; S 0/1->0/1.
- 6/maxisland-norte-delano: D=B 3/3->3/3; C 0/3->3/3; R 2/3->3/3; Z 12/12->12/12; A 2/2->1/2; S 0/1->0/1.
- 6/maxisland-san-indersty: D=B 3/3->3/3; C 0/3->3/3; R 3/3->3/3; Z 9/12->12/12; A 2/2->2/2; S 1/1->1/1.
- 6/maxisland-south-pennoli: D=B 3/3->3/3; C 0/3->3/3; R 3/3->3/3; Z 12/12->12/12; A 2/2->0/2; S 0/1->0/1.
- 6/maxisland-south-weburg: D=B 0/3->3/3; C 0/3->3/3; R 3/3->3/3; Z 12/12->12/12; A 2/2->0/2; S 0/1->0/1.
- 6/lua-lots-empty: D=B 3/3->3/3; C 3/3->3/3; R 3/3->3/3; Z 12/12->12/12; A 2/2->2/2; S 1/1->1/1.
- 6/lua-lots-grown-qualified: D=B 2/3->0/3; C 0/3->4/4; R 3/3->4/4; Z 12/12->16/16; A 2/2->3/3; S 1/1->1/1.
- 6/reach29-house: D=B 0/2->2/2; C 2/2->3/3; R 2/2->3/3; Z 8/8->12/12; A 1/1->2/2; S 1/1->1/1.
- 6/reach29-tegel: D=B 0/2->2/2; C 2/2->3/3; R 2/2->3/3; Z 8/8->12/12; A 1/1->2/2; S 1/1->1/1.
- 6/reach29-growth: D=B 2/4->4/4; C 4/4->5/5; R 4/4->5/5; Z 16/16->20/20; A 3/3->4/4; S 1/1->1/1.
- 6/reach29-views: D=B 2/2->2/2; C 0/2->3/3; R 2/2->3/3; Z 8/8->12/12; A 1/1->2/2; S 1/1->1/1.
- 7/gen27-spandau-s0-edge: D=B 25/25->25/25; C 25/25->25/25; R 25/25->25/25; Z 100/100->100/100; A 24/24->24/24; S 1/1->1/1.
- 7/maxisland-caramel-island: D=B 1/3->3/3; C 0/3->3/3; R 3/3->3/3; Z 12/12->12/12; A 2/2->2/2; S 1/1->1/1.
EX58 (docs/specs/questions/data-formats.md): suite12 Lua empty and grown-qualified now D=B 3/3; native-save continuation and capture-time pause retain full-file checks. Final merged-tip verification pending; refusals still count as mismatches.
AP19 (docs/specs/questions/application.md): suite12 Bevopolis/Norte Delano/South Pennoli now app 2/2 and startup 1/1; South Weburg startup 1/1 before timeout. Input capture occurs at app target; no output seeding or sample deletion.
Native quit diagnostic: R/suite/records-cities29-exit-baseline/gen27-konradshohe-s0-default.log reproduces 0x005D0F94, corrupt after target, ref_warnings=0 with every shadow disabled; both runs have 444 successful/500 refused actions and identical action-script SHA256 e01c041fc71b54bd14c67e5dcf9bc00a5c0c9829ae3c788c070161d44f82912e. This is not claimed as clean or as a shadow mismatch fix.

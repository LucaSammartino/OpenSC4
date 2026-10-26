| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| records-cities29: citywide record shadows (engine) | Codex / .claude/worktrees/records-cities29 / reserved branch records-cities29, detached HEAD | wip; code 83839ddb5..e6a1007cf; ABI unchanged; cargo 7893 passed, clippy 0 diagnostics, CTest 125/125; suite records-cities29-6/-7: 18 clean + 1 native exit crash, shadows 957/974 exact; AP19, EX58 | 1. Resolve AP19/EX58 observation boundaries. 2. Retest modal/native-save cases; keep native quit failure separate. 3. Enqueue engine only after remaining acceptance criteria pass; census credit stays unflipped. |
Gate: qbuild all PASS once; build-runs/qbuild-test.log (7893/0), qbuild-clippy.log (0 diagnostics), fc4verify-build.log (43/43), fc4verify-ctest.log (125/125); owned dispatch/tests.rs fmt PASS.
Evidence root R=C:/Users/tlori/FreeCity4/build-runs; before=R/run7/census3 (reach29 baselines R/run7/reach29); after logs=R/suite/records-cities29-{6,7}/<scenario>.log. Below: exact/total before->after in order D=B (dbpf and dbpf_bounds separately identical), C(citysit), R(dispatch), Z(zones), A(app), S(app-startup).
Measured: 19 scenarios, 18 clean/end=1 + Konradshohe native exit crash/end=1; ref_warnings=0 in all 19; 14/19 all-key exact, 13 clean/all-key exact across region/gen27/Lua; four house scenarios clean/exact. DLL SHA256 d92003e7767fbc0318219dfc5865863906494b1fa9ee27c83fcc2aa525c016a2. Changes: ordered-map comparison, saved zone counts, dispatch Init view inputs, sliced-read save/quit deferral and balanced hidden pause; no ABI exports changed.
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
EX58 (docs/specs/questions/data-formats.md): native SaveCity interrupts acquired readers; PostSave is still a writing phase. Latest grown-qualified has D=B 0/3 (9,2,2 mismatches), while latest empty is 3/3; no safe independent restart boundary specified. Refusals remain mismatches; no exclusion or tolerance change.
AP19 (docs/specs/questions/application.md): modal loops advance dates while native mode is already 2 but this target has not received the mode message; blocks startup in Bevopolis/Norte Delano/South Pennoli/South Weburg and their affected months. No mode seeding or deferred sample deletion applied.
Native quit diagnostic: R/suite/records-cities29-exit-baseline/gen27-konradshohe-s0-default.log reproduces 0x005D0F94, corrupt after target, ref_warnings=0 with every shadow disabled; both runs have 444 successful/500 refused actions and identical action-script SHA256 e01c041fc71b54bd14c67e5dcf9bc00a5c0c9829ae3c788c070161d44f82912e. This is not claimed as clean or as a shadow mismatch fix.

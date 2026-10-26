| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| records-cities29: citywide record shadows (engine) | Codex / .claude/worktrees/records-cities29 / reserved branch records-cities29, detached HEAD | ready; recovered code 77e3eaf1 + main merge 8c3563e72; ABI unchanged; cargo 8075, clippy 0 diagnostics, CTest 135/135; suites12/13/14 latest 18 clean/end=1/ref_warnings=0, seven keys 712/712 exact; AP19/EX58 addressed | - |
Gate: final qbuild all PASS once after merge; build-runs/qbuild-test.log (8075/0), qbuild-clippy.log (0 diagnostics), fc4verify-build.log (220/220), fc4verify-ctest.log (135/135). No Rust files changed this round.
Changed: capture app-target mode inputs before modal loops; trace native slot-15 save continuations and replay whole DBPF acquisitions; hold the promised date from capture with a balanced hidden pause. Earlier citysit map-order, saved zone counts and dispatch Init-input fixes are retained.
Evidence R=C:/Users/tlori/FreeCity4/build-runs; before=R/run7/census3 (house=R/run7/reach29); after=R/suite/records-cities29-<number>/<scenario>.log and summary.tsv. Below: D=B (dbpf/dbpf_bounds separately identical), C(citysit), R(dispatch), Z(zones), A(app), S(app-startup), exact/total before->after.
Measured: suite12 broad sample has 15 clean/exact across region/gen/Lua; final suites13/14 add 8 clean on merged source, including all four house cases. Latest 18 unique: D=B 76/76, C=R 80/80, Z 320/320, A 62/62, S 18/18; all ref_warnings=0. Final DLL SHA256 e15920fdc604916c876a0ec65df1ac5de63f2febeb68547cfb36aaf99d5bfe43 (suite12 was 53529110).
Questions: AP19 (application.md 6.2.1) and EX58 (data-formats.md 8.3.2) addressed by independent input/return observation; none unresolved or newly filed. No output seeding, omitted promised files, tolerance changes or refusal forgiveness.
- 12/london-kensington: D=B 4/5->5/5; C 5/5->5/5; R 5/5->5/5; Z 20/20->20/20; A 4/4->4/4; S 1/1->1/1.
- 12/timbuktu-getting-started: D=B 0/4->4/4; C 4/4->4/4; R 4/4->4/4; Z 16/16->16/16; A 3/3->3/3; S 1/1->1/1.
- 12/gen-tegel-s0-default: D=B 0/2->2/2; C 2/2->2/2; R 2/2->2/2; Z 8/8->8/8; A 1/1->1/1; S 1/1->1/1.
- 12/maxisland-aquaton: D=B 0/3->3/3; C 0/3->3/3; R 3/3->3/3; Z 12/12->12/12; A 2/2->2/2; S 1/1->1/1.
- 12/maxisland-banktonoplankton: D=B 1/3->3/3; C 0/3->3/3; R 3/3->3/3; Z 12/12->12/12; A 2/2->2/2; S 1/1->1/1.
- 12/maxisland-bendburger: D=B 0/3->3/3; C 0/3->3/3; R 3/3->3/3; Z 12/12->12/12; A 2/2->2/2; S 1/1->1/1.
- 14/maxisland-bevopolis: D=B 3/3->3/3; C 0/3->3/3; R 3/3->3/3; Z 12/12->12/12; A 1/2->2/2; S 0/1->1/1.
- 12/maxisland-norte-delano: D=B 3/3->3/3; C 0/3->3/3; R 2/3->3/3; Z 12/12->12/12; A 2/2->2/2; S 0/1->1/1.
- 12/maxisland-san-indersty: D=B 3/3->3/3; C 0/3->3/3; R 3/3->3/3; Z 9/12->12/12; A 2/2->2/2; S 1/1->1/1.
- 12/maxisland-south-pennoli: D=B 3/3->3/3; C 0/3->3/3; R 3/3->3/3; Z 12/12->12/12; A 2/2->2/2; S 0/1->1/1.
- 14/maxisland-south-weburg: D=B 0/3->3/3; C 0/3->3/3; R 3/3->3/3; Z 12/12->12/12; A 2/2->2/2; S 0/1->1/1.
- 12/lua-lots-empty: D=B 3/3->3/3; C 3/3->3/3; R 3/3->3/3; Z 12/12->12/12; A 2/2->2/2; S 1/1->1/1.
- 14/lua-lots-grown-qualified: D=B 2/3->3/3; C 0/3->4/4; R 3/3->4/4; Z 12/12->16/16; A 2/2->3/3; S 1/1->1/1.
- 14/reach29-house: D=B 0/2->2/2; C 2/2->3/3; R 2/2->3/3; Z 8/8->12/12; A 1/1->2/2; S 1/1->1/1.
- 14/reach29-tegel: D=B 0/2->2/2; C 2/2->3/3; R 2/2->3/3; Z 8/8->12/12; A 1/1->2/2; S 1/1->1/1.
- 13/reach29-growth: D=B 2/4->4/4; C 4/4->4/4; R 4/4->4/4; Z 16/16->16/16; A 3/3->3/3; S 1/1->1/1.
- 14/reach29-views: D=B 2/2->2/2; C 0/2->3/3; R 2/2->3/3; Z 8/8->12/12; A 1/1->2/2; S 1/1->1/1.
- 14/gen27-spandau-s0-edge: D=B 25/25->25/25; C 25/25->25/25; R 25/25->25/25; Z 100/100->100/100; A 24/24->24/24; S 1/1->1/1.
Incomplete results retained: suite12 South Weburg/growth timed out mid-full-pass; new labels14/13 respectively finish clean with unchanged comparisons and more time. Prior Konradshohe native exit crash reproduces with every shadow disabled in R/suite/records-cities29-exit-baseline/gen27-konradshohe-s0-default.log; excluded from the 18 clean scenarios above, not claimed fixed.
Limits: action refusals remain (suite14 Spandau 227, Bevopolis 8, South Weburg 5, Lua 43; summary.tsv); suite14 process exit=1 is not claimed as an overall action-suite pass. All seven lane keys are exact in the measured sample; full-census credit remains the coordinator's decision.

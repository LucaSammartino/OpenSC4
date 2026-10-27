| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| records-cities29: citywide record shadows (engine), round 6 | Codex / .claude/worktrees/records-cities29 / reserved records-cities29, detached HEAD | ready: authorized guard fallback; main merge 9cc0d4c4f + code 2598f495e; ABI unchanged; cargo 8253, clippy 0, CTest 142/142; final house traffic 2/2 exact, refs 0 | Coordinator: land guarded fallback; default Lua DBPF refusals remain, so do not claim suite-wide default DBPF credit |
Changed: only blanket acquisition date holds now require dbpf_shadow_hold_date=1 (default off). App-target input capture, actual native-save return/replay, citysit map order, saved zone counts and dispatch Init-input fixes remain.
Cause isolation: baseline regress-records-cities29-r6-before-shadows.log reproduces traffic 0/2 exact after a 104 s acquisition HiddenPause, with no native save entry recorded. HiddenPause suppresses idle agents (simulator.md 5.1/5.4); disabling only the blanket hold restores traffic. Its coexistence with traffic is not repaired; the intrusive hold is explicitly opt-in.
Gate: qbuild all ran once, PASS: cargo 8253/0, clippy 0 diagnostics, CMake 29/29 edges, CTest 142/142; WT/build-runs/{qbuild-test.log,qbuild-clippy.log,fc4verify-build.log,fc4verify-ctest.log}. No Rust files changed; git diff --check clean.
Traffic: unchanged house profile r6-guard clean/end=1/ref_warnings=0; final records-cities29-r6-traffic clean/end=1/ref_warnings=0, traffic 2/2 exact, 47/47 deliveries clean, record mismatches=0, resyncs=0. Jan28 is within the first 10/10 clean window; clean individual days are not printed. Final house seven lane keys 27/27 exact. Logs: R/game-results/<label>.log and .verdict.
Records: latest 13 unique clean/end=1/ref_warnings=0, 581/581 exact: D=B 63/63, C=R 65/65, Z 260/260, A 52/52, S 13/13. Eleven use default hold=0; both Lua reruns explicitly use hold=1. Same code 2598f495e throughout; broad DLL 7563d397, gate-built Lua/traffic DLL 9362dded.
Fallback limit: default r6-broad Lua empty D=B 2/3 and grown 0/3; other five keys exact. EX58 correctly refuses replay after promised dates advance (2451545->2451548; 2523577->2523638 first refusal). Those results remain in their logs; no refusal is forgiven, tolerance widened or comparison omitted. Opt-in Lua reruns below recover record coverage while retaining the known intrusive hold.
Paths: R=C:/Users/tlori/FreeCity4/build-runs; before=R/run7/census3/<scenario>.shadows.txt; after=R/suite/records-cities29-r6-<broad or lua>/<scenario>.log and summary.tsv. Below D(dbpf)=B(dbpf_bounds), C(citysit), R(dispatch), Z(zones), A(app), S(app-startup), exact/total before->after.
- broad/london-kensington: D=B 4/5->5/5; C 5/5->5/5; R 5/5->5/5; Z 20/20->20/20; A 4/4->4/4; S 1/1->1/1.
- broad/timbuktu-getting-started: D=B 0/4->4/4; C 4/4->4/4; R 4/4->4/4; Z 16/16->16/16; A 3/3->3/3; S 1/1->1/1.
- broad/gen-tegel-s0-default: D=B 0/2->2/2; C 2/2->2/2; R 2/2->2/2; Z 8/8->8/8; A 1/1->1/1; S 1/1->1/1.
- broad/gen27-spandau-s0-edge: D=B 25/25->25/25; C 25/25->25/25; R 25/25->25/25; Z 100/100->100/100; A 24/24->24/24; S 1/1->1/1.
- broad/maxisland-aquaton: D=B 0/3->3/3; C 0/3->4/4; R 3/3->4/4; Z 12/12->16/16; A 2/2->3/3; S 1/1->1/1.
- broad/maxisland-bendburger: D=B 0/3->3/3; C 0/3->3/3; R 3/3->3/3; Z 12/12->12/12; A 2/2->2/2; S 1/1->1/1.
- broad/maxisland-bevopolis: D=B 3/3->3/3; C 0/3->3/3; R 3/3->3/3; Z 12/12->12/12; A 1/2->2/2; S 0/1->1/1.
- broad/maxisland-norte-delano: D=B 3/3->3/3; C 0/3->3/3; R 2/3->3/3; Z 12/12->12/12; A 2/2->2/2; S 0/1->1/1.
- broad/maxisland-san-indersty: D=B 3/3->3/3; C 0/3->3/3; R 3/3->3/3; Z 9/12->12/12; A 2/2->2/2; S 1/1->1/1.
- broad/maxisland-south-pennoli: D=B 3/3->3/3; C 0/3->3/3; R 3/3->3/3; Z 12/12->12/12; A 2/2->2/2; S 0/1->1/1.
- broad/maxisland-south-weburg: D=B 0/3->3/3; C 0/3->3/3; R 3/3->3/3; Z 12/12->12/12; A 2/2->2/2; S 0/1->1/1.
- lua/lua-lots-empty (hold=1): D=B 3/3->3/3; C 3/3->3/3; R 3/3->3/3; Z 12/12->12/12; A 2/2->2/2; S 1/1->1/1.
- lua/lua-lots-grown-qualified (hold=1): D=B 2/3->3/3; C 0/3->4/4; R 3/3->4/4; Z 12/12->16/16; A 2/2->3/3; S 1/1->1/1.
Not accepted: initial r6-suite --keys invocation selected only Kensington/San Indersty with the full profile; both exited before target, retained separately. Selected-key broad runs above are clean. Action refusals remain (e.g. Spandau 227, Lua 15/43); full 179-scenario census not rerun, no suite-wide credit claim.
Questions: none filed; AP19/EX58 observation/replay work retained. Final gate-built DLL SHA256 9362ddedd901aa7cbf5e5dcf35c6563590215c92ab0a4744dc36020f26df52cb. No C ABI exports changed.

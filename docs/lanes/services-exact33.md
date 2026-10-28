| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| services-exact33 engine: budget and parks replacement exactness | Codex / services-exact33 / reserved services-exact33 (detached HEAD) | ready for partial landing; budget 2/2 and parks 1/1 exact; code f36fba2fc, ff0d8b4ab; tests 5dfb85736, a083b2a14; ABI exports unchanged, existing seam 59; cargo 8409, clippy clean, CTest 146/146; clean/ref0 game runs; shadow n/a; PL-SE33-1/2 and LV-SE33-1 | 1. Land these two exact owners. 2. Resolve the three spec questions. 3. Continue the nine outside owners from their first differences below. |

Code: budget department Write sorts local holder/purpose references by original holder address order, fixing the first shifted object IDs without changing funding distribution order.
Code: parks reads the distress tuning with the documented float32 percent conversion, 25 to 0.25; fully funded parks remain normal. Updated core, C ABI and conformance boundary tests.
Prior terrain fixes restore seven records before Init, bound draw snapshots to change rectangles, return a retained body-map SimGrid, and retain drawing state through city teardown (476b3db53).
Final gate: qbuild all PASS on committed code; cargo 8409 passed/0 failed, clippy 0 diagnostics, CMake 10/10 edges, CTest 146/146. Owned Rust fmt checks pass. Focused parks core 14/14, ABI 8/8 and vectors 8/8 pass.
Gate history: pre-parks all passed; two later all runs failed obsolete ABI then core threshold assertions. Those assertions were corrected; the final all run above passed with the source unchanged. The q.py focused-test attempt failed to start Bash; native PowerShell cargo runs passed.
EXACT budget: qcheck-8 exit 0, maxisland-wrenton and reach29-growth (2/2). Wrenton's former DepartmentBudget E990BFFC reference-order difference is eliminated.
EXACT parks: qcheck-13 exit 0, paradis-zoe (1/1). qcheck-11's four changed records resulted from threshold 25 marking 54 fully funded park lots run down; percent conversion eliminates them.
Controls: qcheck-14 replace none exit 0 on maxisland-wrenton, reach29-growth and paradis-zoe (3/3). qcheck-10 none also exact on ui29-weburg; qcheck-6 none exact on both terrain and both pollution scenarios.
Game labels: budget golden-26aacbfd7bc645f0a26478f93d85a71c and golden-3dd5fccf20744b668d55901091554831; parks golden-2ef71a1e00d44b53998995765ec6a27f. All clean/ref_warnings=0. Manual terrain services33-terrain-load-5 also clean/ref0 through full shutdown.
NOT EXACT landvalue: qcheck-9 ui29-weburg, only 8BA98EDA helicopter object 636 differs, seven payload bytes in position/motion floats; delivery rand states, DIGESTs and land-value records match. None is exact; timer-read hypothesis remains unproven (LV-SE33-1).
NOT EXACT pollution: qcheck-5 ui29-bevopolis first A9BD882D (1 record), paradis-stohess first 8A22BEBE (3 records). Equal usage values have signed/unsigned property tags; two landfill holders gain maintenance properties absent from the reference (PL-SE33-1/2).
NOT EXACT terrain, qcheck-4 paradis-jinae: delivery 2047-04-06 #26 vt 00AB127C; DIGEST 2047-05-01 income/expense; 114 records, 59 relabel-only, IDs shift at 519/class 896E75AF; first content 096E75F5. Startup/shutdown errors are resolved, simulation difference remains.
NOT EXACT terrain, qcheck-4 ui29-volcano: delivery 2026-01-28 #95 vt 00AB2214; DIGEST 2026-02-01 income/expense; 117 records, 46 relabel-only, IDs shift at 7/class A98F9D28; first content 098F964D (occmgr). Both terrain qcheck verdicts clean/ref0.
The six owners below were not rerun this round; these are the coordinator's supplied starting differences, not fresh measurements.
NOT EXACT power: paradis-berner delivery 2039-10-07 #5 vt 00AB2214; first content 098F964D (occmgr), 11 records (4 relabel-only), IDs shift from 72945/class 49C05C8F.
NOT EXACT ordinance: reach29-growth, 20 records, first 00D0723D (CommunityCPRTraining).
NOT EXACT clock24: timbuktu-rush-hour, 5 records (1 relabel-only), IDs shift at 1359/class C9C05C6E; first content 6A0F82B2.
NOT EXACT demolition: gen28-tegel-s2-default delivery 2029-08-30 #21 vt 00AB1B88; DIGEST 2029-09-01 funds/income/expense; 120 records (42 relabel-only), IDs shift at 382/class 0A134F24; first content 096E75F5.
NOT EXACT dispatch: maxisland-norte-delano first content 098F964D, 100 records (75 relabel-only), IDs shift at 1375/class E990BFFC; ui29-volcano first content 2977AA48, 4 records (1 relabel-only), IDs shift at 1466/class C9C05C6E; timbuktu-big-city first 896E75AF, 1 record.
NOT EXACT tutorial: timbuktu-rush-hour delivery 2010-05-03 #5 vt 00AB2214, 100 records (56 relabel-only), IDs shift at 95/class 896E75AF, first content 098F964D; ui29-weburg was exact in the supplied baseline.
Questions: PL-SE33-1 and PL-SE33-2 in docs/specs/questions/pollution.md (a69cc7a40); LV-SE33-1 in docs/specs/questions/landvalue.md (ebd177d84). No behavior spec or C ABI export changed.
Next: nine owners remain outside; only budget and parks receive new exactness credit. Follow the first difference, check none controls, and request missing behavior contracts rather than guessing.

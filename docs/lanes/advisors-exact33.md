| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| advisors-exact33; engine; saved advisor identity | Codex; advisors-exact33 worktree/reserved branch; detached HEAD; code ae97762f6..cb3113fca | WIP; no owner exact; cargo 8410 passed, clippy 0 diagnostics, CTest 146/146; C exports unchanged; questions none | 1. Fix Rivertner's first shifted ID/class. 2. Implement standalone mysims activation. 3. Wire citysit dependencies and rerun exact checks. |

Registered 13 implemented advisor/advice/subject child factories alongside the system so the class store loads saved counters, advice order and subjects into our code instead of dropping native references and recreating advisors from Lua.
Retained other owners' native advisors (MySim/MyStreetSim) with acquired-reference ownership, interface Init/Shutdown/Simulate forwarding, and native serialization identity. Added a core regression and exercised every child factory in the registration test.
Appended one callback to fc4_advisor_host; synchronized Rust/C++/C layout assertions (272 bytes x86, 544 x64). No C export added/removed; no ABI-version bump.
Final qbuild all PASS: cargo 8410/0, clippy 0 diagnostics, native build 5/5 edges, CTest 146/146. Earlier attempts failed on layout assertions, registration count and return-channel review; those failures are corrected. Logs: build-runs/qbuild-{test,clippy}.log and fc4verify-{build,ctest}.log.
Core fmt: five owned files pass. capi/src/advisors.rs has 205 lines of pre-existing formatting differences outside the changed blocks; left them unchanged.
advisors NOT EXACT: qcheck-4.log, exit 1, 0/4. Rivertner 72 -> 70 differing records; Mitras 70 -> 63; no exact credit.
- maxisland-rivertner: shift now ID 44235/class 49B9E602 (previously 42438/CA2C2A2F); 51 only relabelled; first content 2A134F42/299B2D1B/0 (cSC4NeighborDeal), replacing the previous MyStreetSim first content difference.
- paradis-mitras: shift ID 195187/class 49B9E602; 46 only relabelled; first content 2A134F42/299B2D1B/0.
- timbuktu-rush-hour: first rand delivery 2010-05-03 #5, reference vt 00AB2214; 101 different/51 relabelled; shift 4017/2977AA47; first content 098F964D/299B2D1B/0 (occmgr).
- ui29-bevopolis: 71 different/45 relabelled; shift 417/896E75AF; first content 0A413481/299B2D1B/0 (civic).
citysit NOT EXACT: qcheck-5.log, exit 1, 0/4; no source changes to this owner.
- maxisland-east-weburg: 7 different; paradis-jinae: 3 different; both first 096E75F5/299B2D1B/0 (watercraft).
- gen28-konradshohe-s2-churn: 57 different/43 relabelled; shift 12/6A3848DB; first content civic 0A413481/299B2D1B/0.
- timbuktu-rush-hour: 10 different/2 relabelled; shift 1465/C9C05C6E; first content occmgr 098F964D/299B2D1B/0.
mysims NOT EXACT/NOT ACTIVATED: qcheck-6.log, exit 2, both gen28-tegel-s0-churn and ui29-house rejected: expected mysims+simulator, logged only simulator. The existing dependency guard disables mysims without advisors. These are invalid replacement checks, not save-difference evidence.
Baseline checks: qcheck-3 none/maxisland-rivertner exact (1/1); qcheck-7 none/maxisland-east-weburg,paradis-jinae exact (2/2). Do not attribute those owners' observed differences to baseline drift.
All eight valid replacement runs verdict clean, ref_warnings=0. watchdog_stalls=1 despite clean on advisors/Rivertner and citysit/Jinae; others 0. Both invalid mysims runs also clean/ref_warnings=0, but owner was off. No shadow exactness claimed.
Advisors labels: golden-2ad62b05d5984ad996f7aeb6ebdd9ed1, golden-10fe79ebb00c41a883196161c0e28c69, golden-ebe66770b0114ceebf7371726c635298, golden-b5e1dd8f7cd6428c86f907d4bccf18d4.
Citysit labels: golden-80e41f0c4e7d4eeda74042f10f7d34c6, golden-356c1dc39886449e93780c0d1ada8566, golden-7ea1728a9f2746a5bad3bd0c31eb9055, golden-a8e2a2e793804b2c9c382f7aa4d0a31c.
Run directories and exact comparison details are in qcheck-4/5/6/7.log; each run's game.log was checked for initialization, activation and verdict counters. Initial qcheck-1 lacked references; later checks have references and supersede it.
Next: fix the first shifted 49B9E602 object ID before interpreting later neighbor-deal bytes; inspect the missing system GetAdvisors/AddAdvisor/event interface plumbing; make mysims replace independently without silently replacing advisors; then investigate citysit tracking/Lua callbacks from its spec. No ready entry: no newly exact owner.

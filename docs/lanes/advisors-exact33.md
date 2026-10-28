| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| advisors-exact33; engine; citysit tracking and subscriptions | Codex; advisors-exact33 worktree/reserved branch; detached HEAD; code 14f975842, 7db24ce97 | WIP; no newly exact owner; cargo 8410 passed, clippy 0 diagnostics, CTest 146/146; C exports unchanged; CS16 | 1. Check advisor-29 map walk and Lua publication. 2. Fix advisors' first shifted ID. 3. Implement standalone mysims activation. |

This round resolves the city at Init, loads max-CSI tuning (5 in these runs), enumerates advisor 29 and group candidates, wires native dispatch indicators and automaton callbacks, and normalizes notification pointers. Init remains idempotent.
Fixed an existing dead gate: Server() returned null because msgServerHeld was never set, silently skipping all ten subscriptions. The constructor already held the server reference; Server() now returns it.
Native indicator calls are version checked in compat; original citysit implementations are not forwarded. Candidate creation-order stamps remain unavailable to this host; pointer order is marked SPEC-GAP.
Final qbuild all PASS on 7db24ce97: cargo 8410/0, clippy 0 diagnostics, native build 6/6 edges, CTest 146/146. Logs: build-runs/qbuild-{test,clippy}.log and fc4verify-{build,ctest}.log. No Rust files changed; no fmt needed.
Earlier gate failed only ret_channel_check; the CheckAutomataFlag return-channel citation fixes it. The final gate supersedes that failure.
citysit NOT EXACT: final qcheck-14.log, exit 1, 0/4, with the server fix and diagnostics compiled. Differences are unchanged from the previous round:
- maxisland-east-weburg: 7 different; paradis-jinae: 3 different; first 096E75F5/299B2D1B/0 (watercraft) in both.
- gen28-konradshohe-s2-churn: 57 different/43 relabelled; first shifted ID 12/class 6A3848DB; first content 0A413481/299B2D1B/0 (civic).
- timbuktu-rush-hour: 10 different/2 relabelled; first shifted ID 1465/class C9C05C6E; first content 098F964D/299B2D1B/0 (occmgr).
Final diagnostics: PostCityInit now reaches the enumeration callback in all four cities, which reports 0 advices; all final track_entries/tracked counts are 0. Candidate/indicator branches remain unexercised. Check WalkGameMap status/count before concluding the native active map is empty, then inspect the missing sitmgr publication (existing CS13).
Baseline qcheck-13.log: --replace none on those same four scenarios exits 0, 4/4 exact. It used the pre-server-fix DLL with citysit disabled; do not blame baseline drift for the unchanged observed differences.
Final four citysit verdicts clean: foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0. No shadow exactness claimed.
Final labels in scenario order above (churn, east-weburg, jinae, rush-hour): golden-5aa2a54bd2e44a96a22ea8be85b2a172, golden-d10cb0311ee64449b52da0c7733d5c3c, golden-4f291bbabbb24c0ebc5e23e42e6bc2e9, golden-a55eb03cca954b80b5b4c48bd504f2d0.
qcheck-8/10/11 are earlier Jinae probes (3 different). qcheck-9 was rejected before gameplay because a proposed schedule accessor changed the pinned source hash; that accessor was removed completely.
qcheck-12 was the full pre-server-fix check (0/4); east-weburg had watchdog_stalls=1 despite clean. Its zero tracking counts led to the server fix. qcheck-14 supersedes it.
advisors NOT EXACT, unchanged/not rerun this round: qcheck-4.log exit 1, 0/4; inherited child-factory/native-advisor fixes remain on HEAD.
- Rivertner: 70 different/51 relabelled, shift 44235/49B9E602; Mitras: 63 different/46 relabelled, shift 195187/49B9E602; first content neighbor deal 2A134F42/299B2D1B/0 in both.
- Rush Hour: first rand delivery 2010-05-03 #5, reference vt 00AB2214; 101 different/51 relabelled, shift 4017/2977AA47, first content occmgr. Bevopolis: 71 different/45 relabelled, shift 417/896E75AF, first content civic.
mysims NOT EXACT/NOT ACTIVATED, unchanged/not rerun: qcheck-6.log exit 2 on gen28-tegel-s0-churn and ui29-house. Resolve disables mysims without advisors; factories require CurrentAdvisorCore. Enabling the flag alone cannot make a valid standalone check.
Filed CS16 in docs/specs/questions/city-situation.md for indicator number/label/untrack arguments. CS17 draft was withdrawn after finding the dead server flag. C ABI exports unchanged; no ABI-version bump.
Next: validate advisor-29 map walking and Lua table publication; rerun citysit from its first difference. Then fix advisors' first shifted simgrid ID and build a native-advisor-compatible MySims host. No owner earned exact credit; queue WIP.

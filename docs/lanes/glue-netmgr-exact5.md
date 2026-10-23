| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netmgr-exact5: network-lot record 11 shadow | Codex, `.claude/worktrees/glue-netmgr-exact5` | in progress | Continue: supply missing shadow object arrays/ID callback, then verify and rerun. |

## Handoff
- Round 2 resumed on detached HEAD e806aeeb. Added decoded shadow objects and SDK family-ID expansion; focused RNG test added, gate pending.
- Focused object tests PASS 2/2, including RNG reload and one-ID prop draw. Operations 63326/18266 collected; no outstanding test. q.py Bash startup failed (CreateFileMapping access denied); use direct PowerShell with bounded output. fmt paths are relative to engine/.
- Round 2 source ready for required all gate: object arrays decoded by Rust at detail 3, stable nonzero config tokens, SDK records uniquely owned, public family expansion, SDK payload cleanup at city shutdown. R1 game record saved as docs/runs/netmgr-exact5-r1-2032-322.md.
- Goal: fix netmgr record 11 mismatch and investigate 19 resynced lines; real-city shadow exact to 2032.
- Done: initial handoff commit; diagnosed missing (20,103) as absent north-column walk (§6.3 NM10/NM11).
- Implemented origin walk, SGI pending order (§8.5 NM13), captured RNG input (§17.3), exact config comparison; removed NM10 suppression and record resyncs.
- Focused tests: first 39/40 (expected-order assertion corrected), second 40/40; new north-column/RNG reload test awaits gate.
- Evidence copied locally: `build-runs/regress-train149-shadows.log` and `.out`; original had 72/73, 19 resyncs, 10 un-compared config choices.
- Implementation milestone: `7812f988`; source unchanged while first native queue job is pending.
- Full `qbuild all` started on that source: Rust stopped at 740 passed/1 failed (conformance nm16_23 expected old event order), clippy failed `int_plus_one` in ordered.rs. Failed logs preserved as `build-runs/qbuild-{test,clippy}-failed-r1.log`.
- First all terminal: native build PASS 217/217 edges; CTest 67/68 (cargo_test failed). Queue label `qbuild-20261023-195549-build-ctest`, collected session 70839 to exit 1. No game request submitted.
- Applied diagnostic corrections: pending comparison, conformance event order, formatting, fake-city coordinates. Native failed logs preserved with `-failed-r1` suffix.
- Corrected source milestone `cb48b856`: owned fmt PASS (6 files); workspace tests PASS 6231/0/0; clippy PASS, zero diagnostics.
- Corrected native verdict PASS: build 50/50 edges, CTest 68/68, session 67115 collected to exit 0. Queue `qbuild-20261023-202733-build-ctest`. Tree-changed note is handoff-only commit f3c49406; engine source stayed cb48b856.
- Real-city turbo run COMPLETED: `netmgr-exact5-r1-2032 2/73`, 0 resynced lines, verdict clean, ref_warnings=0, reached 2032-01-04 (target 2032-01-01). DLL SHA256 18B8D1ACEA857D6B0E424F910B659C1A2B6CE2298E9BA0B75C2B1E55DFEDF518. No outstanding operations/requests.
- Root cause of remaining failures FOUND: shadow StartLot calls fc4_netlot_add_configuration_props with token=0, objects=nullptr/count=0 for ALL configs; MakeNetLotReadOnlyHost has NO object_ids callback. Native draws for prop/flora objects are omitted. March first post-message divergence next-word 322/318; April 327/319; later configurations differ. Original April lot-count gap is gone (527/527 in this run).
- EXACT NEXT: read fc4_lotdev_cfg_object fields at engine/capi/include/fc4.h:5904 and use EXISTING fc4_lot_config_object_array(props,3,nullptr,callback,ctx) (header:8826, capi/src/lot_config.rs:498) to decode kept objects in Rust, avoiding new C++ simulation logic. Give each shadow config a nonzero stable token; retain fc4_netlot_object values plus owned SDK cSC4LotConfigurationObject records/ID data for that token.
- Wire shadow g_lhost.object_ids to city's GetLotConfigurationManager()->GetObjectIDArray(nativeRecord, SC4Vector<uint32_t>&), SDK slot 27; emit returned IDs through callback. This uses the game's public interface and expands families correctly. No native signature guesses needed. The SDK cSC4LotConfigurationObject fields/layout were read (52 bytes), SC4Vector nested data owns allocation: avoid value copies/reallocation double frees (use vector<unique_ptr<record>> or existing wrapper).
- References already read: prop-lotconfig.md B3.3 lines478-510 says shared reader d=3 for network configs, all shipped version2; network-manager §8.6 says one draw per eligible prop/flora nonempty ID list. Existing replacement loader engine/plugin/cFC4NetworkLotManager.cpp:594-650 builds arrays manually; its header:60-110 explains SC4Vector ownership, but prefer Rust converter above. SDK cISC4LotConfigurationManager.h:72 and cISC4City.h:153 expose required methods.
- Preserve/record first game run in docs/runs (NOT yet done); highest existing run number measured 321, so next free was 322 (recheck). It is clean but shadow NOT exact; do not credit coverage. Need normal-speed run if a remaining difference proves turbo-only; known empty-object bug affects either pace.
- After object fix add focused test for config + mandatory prop/flora draw advance; gate and fresh unique-label turbo run to 2032. Do not reuse r1 label or discard its completed verdict. No questions filed, no spec blocker, ABI exports changed (3 additions), no merge queue written.
- Game wait helper: `python build-runs/wait-netmgr-result.py build-runs/game-results/<label>.verdict`, collect same operation; no qwait. Summary helper `build-runs/summarize-netmgr.py <label>` prints exact clean/total and resync count.
- Constraints: no tools/ access; no native signature guesses; game requests unique, wait with same exec session (no qwait).
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-glue-netmgr-exact5-r2-manifest.json`.

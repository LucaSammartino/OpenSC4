| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-region27 / RegionalCity replacement (engine) | Codex / replace-region27 / detached HEAD | Continue: main merge 07ffb56e4; native hosts 47e43b854; ABI unchanged; cargo 664/0; native link FAIL; replacement acceptance unmeasured; RY9 answered | 1. Replace SDK cell-map construction with ZoneRegionBuffer and pass quick. 2. Register default-off regionalcity factory. 3. Measure (a)-(c), qbuild all, enqueue engine. |

## Handoff
Round 3 stopped by measured context guard; no merge-queue entry and no outstanding new game request.
Merged main first (07ffb56e4); rc23-wip and prior lane commits were already included. RY9 is answered on merged source.
47e43b854 adds cFC4RegionalCity (74-slot primary including destructor, message/serializable views), cache/setup/read hosts and native connection callbacks.
Private Rust callbacks run reconciliation outside the state lock, preserving getter/setter re-entry. New private symbols do not change C ABI exports.
RY9 demolition uses true, masked region, privilege 0, flags 1, dezone false, layer filter, null outputs/clicked, zero final coordinates.
Filter classifier implements occupant-manager 13.2.4 interface/property/group precedence; type/property-holder admissions are unrestricted.
Added regionalcity_border_contract native test: eleven arguments, layer precedence, masked west strip, callback getter re-entry, edge word, queue drain and ConnectionsReady.
Focused quick qbuild-20261026-050435-quick: cargo 664/0; native failed Repair alias collision and missing cRZAutoRefCount include; both fixed.
Latest quick qbuild-20261026-051225-quick: cargo 664/0; native 52/53 edges, LINK FAIL with exactly three unresolved cRZCellMap constructor/destructor/SetValue symbols.
Exact next fix: FC4RegionalCityConnections.cpp Cells() constructs SDK SC4CellRegion, whose cRZCellMap implementation is not linked. Reuse fc4::ZoneRegionBuffer from cFC4ZoneManager.h:138 (Build(fc4_zone_region), Game()) instead; preserve holes and empty masks.
Read cFC4ZoneManager.h:129 onward and cFC4ZoneManager.cpp ZoneRegionBuffer::Build before editing; fc4_zone_region is fc4.h:4753.
Logs: build-runs/fc4verify-build.log and build-runs/qbuild-test.log; latest build has only linker failures. Native contract executable has not been run.
Formatting: qbuild fmt passed all three owned Rust files after module ordering fix. A temporary formatting helper in build-runs is obsolete; do not reuse it.
Review remaining glue: records 5.1/5.2 disagree about PostCityShutdown recipients; current native handler uses application's object identity. No new question filed.
Registration still absent: append CreateFC4RegionalCity declaration/include, CLSID 0x6A037C24 ClassEntry and default-off regionalcity Subsystem in FC4Subsystems.cpp.
(a) House list + regionalcity through 2032: NOT RUN. Include mysims and all keys from task; simulator/budget remain default-enabled.
(b) Replacement regionalcity shadow 73/73: NOT RUN. Baseline region27-r2-records2032b was clean to 2032-01-02, ref_warnings=0, shadow 73/73; MAIN/build-runs/game-results/region27-r2-records2032b.verdict and .log.
(c) Berlin neighbour interaction plus save in replacement: NOT RUN. Existing berlin-spandau/tegel and generated edge scripts stay inside tile; private network script explicitly makes no regional connection. Need a real neighbour-deal or border-connection script path.
All new requests need top-level parallel=1 and shared MAIN/build-runs/game-queue; no request currently pending. Do not reuse the pre-implementation DLL.
Next: fix mask linkage, run quick/native focused test, commit milestone and guard; register key, build and measure (a)-(c); qbuild all ONCE when code done, handoff <=25 lines, enqueue kind engine using actual HEAD. regionalcity is not ready for the replace list.

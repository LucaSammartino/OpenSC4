| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-dispatch-complete4 (dispatch host and independent source comparisons) | `Codex` / `glue-dispatch-complete4` (engine lane, detached reservation) | WIP: cargo4239/0, clippy clean, ctest67/67; original a3 and replacement a1 clean/ref0 to2032; shadow1694/1696, two DP10 differences; DP10-DP13 filed; no ABI exports changed | Publish missing contracts before completing replacement; no promotion |

## Report

Replacement remains incomplete. No complete-owner, supported replacement, default or regression-list promotion is justified. The native exercise created vehicles; the identical replacement exercise returned ID0. This is measured missing behavior, not an idle-counter success.

Changes: independent source acquisition from city fire/police interfaces and their serialized air-source references; actual source registration in the shadow core; manager slot15 identity and slot21 availability comparisons for all five queried types. The output being tested is never used to seed source registration. Station states come from real source slots6/7, with no fabricated provider data. Added real automaton group, Remove(false), and ReturnToSourceOccupant callbacks. View, vehicle creation and pylon construction/placement remain unwired under DP10-DP12.

The shadow now starts each sample empty (Read appends), maps typed object identities for nonempty records, compares target coordinates, and releases captured references before native cleanup. Core destruction releases registered sources and Read-before-Init pylon references; a drop ownership regression test covers source cleanup. State getters use mutex-backed storage instead of RefCell; the token is atomic. Script action `dispatch type` uses a real station and performs BeginTool, Dispatch, live record capture, optional independent shadow, RevokeByID and EndTool. It only reports completed when a vehicle exists.

Validation: initial baseline all passed (cargo4238/0, clippy clean, ctest67/67). Final qbuild all passed (cargo4239/0, clippy zero diagnostics, CMake2/2, ctest67/67). Quick passed2892/0 with dependent crates compiled and build6/6. Absolute-path qbuild fmt passed all three touched Rust files. Code tip c5488378; later commits only document measurements. No C ABI exports or ABI tables changed.

Real game, identical DLL SHA prefix3e86daf4 and script build-runs/dispatch-exercise.txt:
- gdc4-original-20261022-a3: clean/ref0, 2032-01-07. Fire ID2/call1/vehicle+pylon/view1/79-byte record; police ID3/call1/vehicle+pylon/view2/95-byte record. Both revoked and ended successfully. Free units fire2->1->2, police4->3->4.
- Its independent shadow:77 samples,75 exact lines,1694/1696 exact values; every saved record byte-exact and every source identity/free-unit comparison exact. The two differences are the unsaved vehicle-view handles1/2 vs0, explicitly tagged DP10. They arise from the absent view contract, not turbo pacing.
- gdc4-replace-20261022-a1: clean/ref0, 2032-01-08. Both CanDispatch and BeginTool true; both Dispatch results ID0, no record/vehicle/pylon,22-byte records. Free units restored2/4; Shutdown nextID3,2 posts. Both exercises explicitly INCOMPLETE (DP11/DP12).
- Earlier original a1/a2: corrupt/ref0 at first exercise. Root cause was this lane's double ground-reference release: SDK SC4List already releases its elements. Audited cleanup now clears nodes before list destruction. The prior replacement already balanced its ground station references; no original leak claim remains. DP13 has an appended evidence correction. Final a3 fixes both failures.

Questions: docs/specs/questions/dispatch.md DP10 vehicle-view creation/interface/renderer lifecycle; DP11 native pylon construction/slot prefix/placement; DP12 complete BeginDispatch vehicle/path arguments and ownership; DP13 air-source station-list ownership/outputs. Ground PR8 ownership is published; air acquisition is unspecified, so air nodes are conservatively cleared without an unproven Release. No same-owner native method delegation or guessed forwarder was added. Generated question index left for coordinator regeneration because tools/ is prohibited.

Next: obtain and reconcile DP10-DP13, then complete actual host/provider callbacks and remeasure original/replacement dispatch behavior within the remaining family-round limit. Coordinator must keep this lane WIP; union regression and promotion require full support. No request remains outstanding. No restart, merge or push was performed.

## Milestone record
- Goal: finish supported dispatch replacement, including vehicle views and independent source/pylon comparisons; no completion credit with missing contracts.
- Reused reserved detached worktree; initial HEAD equals main, clean status.
- Baseline qbuild all: cargo workspace and clippy logs clean; full ctest 67/67.
- q.py cannot start Bash (E_ACCESSDENIED); use bounded native PowerShell commands.
- Implemented independent ground/air source registration and slot 15/21 comparisons; real station callback releases; automaton despawn/go-home/group callbacks; active-record shadow identity/ref fixes.
- Added script dispatch type: real station, BeginTool/Dispatch/live record/shadow/Revoke/EndTool; not success unless a vehicle actually exists.
- Milestone 1a2fb6bb: independent sources/stations and scripted live dispatch; original a1 created fire vehicle (ID2, call1, pylon1, 79-byte exact record); unsaved handle differs under DP10.
- Original a1 corrupt/ref0 on air list release. PR8 ownership applies to ground only; conservative air no-release choice and missing contract filed DP13. No replacement credit.
- Fixed core destruction releasing captured references (including Read-before-Init pylons); shadow drops each sample before native cleanup. State getters now mutex-backed (no RefCell), token atomic; added drop ownership regression test.
- Interim all: cargo4238/0, clippy clean, ctest65/67; return-channel citations and script error-string expectation repaired. Quick cargo2372/0 PASS, build7/7 PASS; tree changed during this interim run. Final all next on committed tree.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-dispatch-complete4-r1-manifest.json.
- No game outstanding. Next run original gdc4-original-20261022-a2 to 2032 after final build, then replacement gdc4-replace-20261022-a1; never concurrently.
- a2 also corrupt/ref0; actual bug: SDK SC4List destructor releases elements, so explicit cleanup doubled ground releases. Nodes now cleared after audited cleanup; prior replacement already balanced ground references. DP13 has appended correction (failure does not establish air ownership).
- Gate at 8a99762a: cargo4239/0, clippy clean, ctest67/67. Revalidate after node cleanup; next original label gdc4-original-20261022-a3, replacement gdc4-replace-20261022-a1.
- c5488378 original a3 clean/ref0 through 2032-01-07: real fire ID2 (record79B) and police ID3 (95B), vehicle/pylon present; revoke/end successful; free counts 2->1->2 and 4->3->4.
- a3 shadow 75/77 exact lines, 1694/1696 exact values. Only 2 unsaved viewHandle differences (1/2 vs0), both DP10; all records byte-exact, independent source identities/free counts exact.
- Replacement gdc4-replace-20261022-a1 currently outstanding; qwait session81525. Final all session43546 (build2/2 PASS; ctest running).
- DP10-DP13 in docs/specs/questions/dispatch.md: missing view, pylon, BeginDispatch and air-list ownership contracts. Completion blocked; no ABI export changes.

## Coordinator phase review

Bounded preparation and diagnostic source publication is approved for train83; the worker result remains incomplete. DP10-DP13 remain unanswered owner contracts. No complete-owner, exact shadow, default or replacement credit is granted. The original worktree is parked awaiting answers, and its existing round budget is retained. The train must pass the full native gate and current default union before landing.

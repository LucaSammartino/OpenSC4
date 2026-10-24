| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-viewutil-apply12 (VU15/VU16 shadow contracts) | `glue-viewutil-apply12` / (engine lane, detached HEAD) | WIP: implementation 9189e7185, accounting b6672b1de; native build 263/263, CTest 81/81 PASS; focused view_utilities tests 47 passed; full cargo/clippy gate interrupted without a verdict. Runs 520/521 clean to 2032, ref_warnings=0; each has 33 families 73/73 exact, three 0/73, aggregate 0/73; 14/50 bodies unmeasured. VU17/VU18/VU19 filed; VU12/VU13 remain open. Public C ABI unchanged. | (1) Recover original full-gate evidence or arrange a replacement after reviewing the recovery diagnostic. (2) Review measured partial closure. (3) Answer VU12/VU13/VU17/VU18/VU19 before expanding credit. |

## Implementation

## Handoff

- Final round: full `qbuild all` is running as session 59094, output `build-runs/all-final-r3.log`; collect this operation only.
- Added `viewutil-subset` accounting for 33 bodies, explicitly excluding object-angle/camera-angles/city-width. Original all-36 and individual family lines remain intact.
- Rust source unchanged. Logger change made during Rust compilation, before native request; final native build must include it.
- Logger committed as f1b647211. At 18:20 BST, the same gate was still compiling core tests: worktree-specific rustc PID 28804 alive, 3063s elapsed, 106 CPU seconds, 4103 MiB resident. No terminal gate result; no game request outstanding.
- Final gate Rust stage now PASS: 7376 passed, 0 failed, 0 ignored; clippy exit=0, diagnostics=0. Same session 59094 has queued native build/CTest as `qbuild-20261024-182852-build-ctest`; collect it, do not queue another verify.
- Full gate completed PASS: native build 47/47 edges (2.7s), CTest 81/81 (89.9s), fc4verify PASS (94.7s), qbuild PASS. Session 59094 terminated exit=0. Next submit `viewutil12-r3-subset-20261024c` to 2032, using the newly built DLL.
- Next: collect gate, submit unique turbo run to 2032, wait once without qwait; record next free run (currently max 521); finish READY partial queue with full HEAD.
- No new contracts or questions. Existing runs 520/521 and VU12/VU13/VU17/VU18/VU19 remain as below.

## Prior implementation

Merged `glue-viewutil-shadow11` at d7befe3aa before implementing VU15/VU16 contracts. Commit 9189e7185 extends the default-off `viewutil_shadow` observer to 36/50 bodies: rotation, 17 local helpers, and 18 read-only cached-input operations. Commit b6672b1de retains a separate rotation-family summary and includes all compared fields in aggregate accounting. Engine source has remained unchanged since that commit.

Native calls check the specified PE identity and executable sections, use isolated local storage, and observe scalar returns through float32 stores. Private static-link transport adds no public C ABI exports. The cached-input adapter tests arithmetic using published native input fields; it does not validate camera or frustum construction.

## Validation

- Final-round `qbuild all` Rust stages passed: **7376/0/0** (passed/failed/ignored), clippy **0 diagnostics**. Logs: `build-runs/all-final-r3.log`, `qbuild-test.log`, `qbuild-clippy.log`. Native build/CTest is pending in request `qbuild-20261024-182852-build-ctest`, launched after the logger commit f1b647211. The prior interrupted gate below is preserved as historical evidence.
- Final native request completed: build **47/47**, CTest **81/81**, `fc4verify: PASS (94.7s)`, `qbuild: PASS`. Logs are `fc4verify-build.log`, `fc4verify-ctest.log`, and `all-final-r3.log`; matching request `qbuild-20261024-182852-build-ctest`. Engine source remained stable at f1b647211 throughout native validation; later commits changed only this report.

- Owned-file formatting passed. The focused check completed with 47 view_utilities tests passing; this filter does not establish the entire capi test suite.
- Completed native request `qbuild-20261024-163355-build-ctest`: build **263/263**, CTest **81/81**, `qbuild: PASS`, 755.8 seconds. Evidence: `build-runs/verify-1.log`, `fc4verify-build.log`, `fc4verify-ctest.log`, and the matching build-results verdict. The tree-change notice concerned handoff-only commit 0ca77e027; engine source was stable at b6672b1de.
- The required full `qbuild all` was started in the previous round, session **94098**, with `build-runs/all-1.log` and `qbuild-test.log`. On resume, collection returned **Unknown process id 94098**. Process metadata confirmed no remaining build operation for this worktree. `all-1.log` is empty; `qbuild-test.log` ends during workspace compilation, without any `test result:` lines or a terminal verdict. Full cargo totals and clippy status are therefore **unavailable**. No replacement or duplicate check was started under the explicit instruction to collect that operation only. Recovery observations are in `build-runs/full-gate-recovery-20261024.md`.
- The incomplete full gate prevents READY despite the separately passing native gate and measured game work. Incomplete evidence has been retained unchanged.

## Real-city measurements

DLL identity `9efd3ba0`, engine source b6672b1de, Konradshohe, `viewutil_shadow=1`; default remains off. Both verdicts have zero reference warnings, foreign exceptions, watchdog stalls, RTC failures, and overruns; resyncs=0 throughout.

- [Run 520](../runs/viewutil12-r1-turbo-20261024a-520.md): `viewutil12-r1-turbo-20261024a 0/73`; clean at **2032-01-02**, turbo, 30.562 seconds.
- [Run 521](../runs/viewutil12-r1-normal-20261024b-521.md): `viewutil12-r1-normal-20261024b 0/73`; clean at **2032-01-01**, normal speed, 109.343 seconds.

For **each** run, every family below is **73/73 exact**:

rotation, bounds, cross, min, max, ground-arithmetic, triangle-pick, ground-predicate, edge-vector, round-down, round-up, primitive-count, primitive-indices, vertex-copy, index-decode, clip2, units, zoom-rotation, view-viewport, fixed, view-box, city-corner, clip3, visible-edges, world-view, world-viewport, depth, world-viewport2, view-world, viewport-world, box-rectangle, view-point, viewport-fixed.

For **each** run, **object-angle**, **camera-angles**, and **city-width** are each **0/73 exact**. Thus 33 families pass every monthly line, three differ every month, and the aggregate remains **0/73**. Differences persist at normal speed. These results do not establish whole-owner verification, and generic numeric decode outside its checked bias range remains VU13 rather than whole-body credit.

## Open contracts and remaining work

Questions are recorded in [view-utilities questions](../specs/questions/view-utilities.md):

- **VU17:** complete private mesh resource layout is needed for isolated mesh/model picking and iterator select-group fixtures.
- **VU18:** exact draw-cache field offsets are needed for draw-frustum consumers.
- **VU19:** object-angle signed zero: native rotation=0 stores 0x80000000; current portable arithmetic gives 0x00000000.
- Existing **VU12** has measured camera-angle and city-width rounding evidence appended: camera yaw sample 3 gives native/core 0x408a3ae6/0x408a3ae7; city width sample 1 gives 0x42a73d74/0x42a73d75. Existing **VU13** limits generic numeric-decode coverage. No duplicate question was filed.

The 14 unmeasured bodies are constructor, draw-box, city-clipped-box, camera update, viewport ray/ground/corners, mesh/model pick, visible area, draw build, iterator delete/select/next. They remain uncredited; missing resource/cache contracts are not guessed. The differing arithmetic remains visible pending numeric answers. No coverage map or regression profile was edited, and no default-on switch was added.

No game request is outstanding. The immediate next step is coordinator recovery of the missing full-gate result; this queue entry is WIP until that gate is established. The measured partial work can then be reviewed independently of unanswered contracts.

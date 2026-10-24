| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-viewutil-apply12 (VU15/VU16 shadow contracts) | `glue-viewutil-apply12` / (engine lane, detached HEAD) | READY, measured partial: implementation 9189e7185, accounting b6672b1de, subset f1b647211; cargo 7376 passed/0 failed, clippy 0 diagnostics, native build 47/47, CTest 81/81 PASS. Run 522 clean to 2032-01-13, ref_warnings=0; viewutil-subset 73/73 exact (33 families), each family 73/73; three differing families and all-36 aggregate 0/73; 14/50 bodies unmeasured. Runs 520/521 confirm the same family counts at turbo/normal speed. Existing VU12/VU13/VU17/VU18/VU19 remain open. Public C ABI unchanged. | (1) Land measured partial work. (2) Answer existing questions before expanding coverage; whole-owner verification remains open. |

## Implementation

Merged `glue-viewutil-shadow11` at d7befe3aa before implementing VU15/VU16 contracts. Commit 9189e7185 extends the default-off `viewutil_shadow` observer to 36/50 bodies: rotation, 17 local helpers, and 18 read-only cached-input operations. Commit b6672b1de retains a separate rotation-family summary and includes all compared fields in aggregate accounting.

Final-round commit f1b647211 adds `viewutil-subset`: 33 bodies, explicitly excluding object-angle, camera-angles, and city-width. Their individual shadow lines and the original complete 36-body aggregate remain visible. The subset counts every comparison from the other 33 families, including binding/live-state failures; it does not filter observations according to whether they passed. Rust and native-call arithmetic are unchanged in this round. No new owner or question was started.

Native calls check the specified PE identity and executable sections, use isolated local storage, and observe scalar returns through float32 stores. Private static-link transport adds no public C ABI exports. The cached-input adapter tests arithmetic using published native input fields; it does not validate camera or frustum construction.

## Validation

- Final-round `qbuild all` completed **PASS** in one collected operation: cargo **7376 passed, 0 failed, 0 ignored**, clippy **0 diagnostics**, native build **47/47**, CTest **81/81**. Logs: `build-runs/all-final-r3.log`, `qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
- Native request `qbuild-20261024-182852-build-ctest` completed `fc4verify: PASS (94.7s)`; build 2.7s, CTest 89.9s. Its build log explicitly compiles `FC4ViewUtilShadow.cpp`. Rust source was unchanged throughout the full gate; the reporting-only C++ change was committed before this queued native request. Engine source remained stable at f1b647211 throughout native validation and the final game run.
- Earlier owned-file formatting and 47 focused view_utilities tests passed. Earlier native request `qbuild-20261024-163355-build-ctest` passed build **263/263**, CTest **81/81** in 755.8s; its summary remains in `build-runs/verify-1.log` and its build-result verdict. Shared native log names now contain the final gate.
- The prior round's full gate had no terminal verdict. Its empty `build-runs/all-1.log` and `build-runs/full-gate-recovery-20261024.md` remain as interruption evidence; shared `qbuild-test.log` now contains the authorized replacement gate. No missing session was queried in this round. The passing replacement resolves the former gate blocker.

## Real-city measurements

Konradshohe, `viewutil_shadow=1`; default remains off. Runs 520/521 used DLL `9efd3ba0`, engine source b6672b1de; final run 522 used DLL `4a19a8f0`, source f1b647211. All three verdicts have zero reference warnings, foreign exceptions, watchdog stalls, RTC failures, and overruns; resyncs=0 throughout.

- [Run 520](../runs/viewutil12-r1-turbo-20261024a-636.md): `viewutil12-r1-turbo-20261024a 0/73`; clean at **2032-01-02**, turbo, 30.562 seconds.
- [Run 521](../runs/viewutil12-r1-normal-20261024b-637.md): `viewutil12-r1-normal-20261024b 0/73`; clean at **2032-01-01**, normal speed, 109.343 seconds.
- [Run 522](../runs/viewutil12-r3-subset-20261024c-638.md): **`viewutil12-r3-subset-20261024c 73/73`** for `viewutil-subset`; clean at **2032-01-13**, turbo, 27.906 seconds. All 73 monthly subset lines have `mismatches=0`; the complete `viewutil` aggregate remains **0/73**. Evidence: `build-runs/game-results/viewutil12-r3-subset-20261024c.verdict` and `.log`.

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

No build or game request is outstanding. The completed final verdict was accepted after the filesystem-event wait missed its notification; only that wait helper was stopped, and the game was not repeated. The READY queue entry lands the measured partial work. Next: review it, then answer VU12/VU13/VU17/VU18/VU19 before implementing or crediting the remaining contracts. Whole-owner verification remains open.

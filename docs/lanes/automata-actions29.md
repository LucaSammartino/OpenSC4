| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| automata-actions29: native construction worker slots (engine) | Codex / automata-actions29 / detached HEAD, reserved branch automata-actions29; commits d9aaac58a, 6bd7ce714, 8bce367da, ecf5be204, 83b011895; C ABI +2 exports, no seam reservation | wip; cargo 7883/7883, clippy 0 diagnostics, CTest 125/125; shadow 73/73 exact; s0 216/216, s1/s2 incomplete; AT25 open | 1. Answer AT25. 2. Finish worker continuation and rerun s1/s2 grow. 3. Enqueue kind engine after acceptance passes. |

Game logs below are in `C:/Users/tlori/FreeCity4/build-runs/game-results/`; build logs are this worktree's `build-runs/`.
Fix: native worker slots 30/14 had zero-argument placeholders leaving 8/4 stack bytes; now typed, with Rust flag/orientation stores. Scaffold slots 7/8 now copy positions; getter uses a locked C++ snapshot.
NM17: merged re-nm29 (handoff 0a9991f61) via 8bce367da; no concrete automaton factory registered and no native internal creation/removal body reused.
Reproduction: `aa29-base-auto-a` and `aa29-base-netmgr-a` corrupt 5/216 at 2026-01-28; `aa29-base-house-s1-a` corrupt 26/216 at 2026-01-30, heap target 0x29A5D1EC; logs with those labels.
Isolation: `aa29-base-nettool-a` corrupt 26/216; first exception +0x9F0FA is distinct. `aa29-trace-auto-a.log` records slots 30 then 14 before the original heap-target fault.
Regression: `engine/tests/automata_construction_test.cpp` checks both managers, native slot cleanup/x87 and roof (12.5,7,34.5) -> (12.5,9,34.5); focused CTest 1/1, final gate includes it.
Gate PASS: `qbuild-test.log` 7883 passed/0 failed/0 ignored; `qbuild-clippy.log` exit 0/0 diagnostics; `fc4verify-build.log` 44/44 edges; `fc4verify-ctest.log` 125/125. Full all invoked once; final owned-Rust fmt passes.
s0 grow PASS execution/end: `aa29-fix-house-s0-a.log`, 216/216, complete marker/end date 2028-01-28; corrupt verdict only at excluded native quit +0x1D0F94 (2028-01-30), ref0.
s1 grow FAIL: `aa29-fix-house-s1-a.log` 27/216 then other-thread ucrtbase+0x973FE; repeat `aa29-fix-house-s1-b.log` 27/216 then ntdll+0x4E728; ref0, no original heap-target fault.
s2 grow FAIL: `aa29-fix-house-s2-a.log` 75/216 then native scaffold +0x1CC3B3 on 2026-07-29; repeat `aa29-fix-house-s2-b.log` 84/216 then +0x3529C0 on 2026-09-01; ref0.
Control PASS: `aa29-control-noauto-a.log`, same DLL/full house minus automata, s1 216/216, clean quit 2028-01-29, ref0.
Shadow PASS: `aa29-shadow-a.log`, clean through 2032-01-13, shutdown aggregate 73/73 samples exact, 438 component records/730783 bytes, 0 mismatches, ref0 (sample total not added to aggregate again).
Unchanged house PASS: `aa29-house-a.log`, clean, target 2032-01-01 reached, ref0; verdict additionally reports a 58 s hidden-dialog hold after target at 2032-01-02.
All listed game runs used the shared main queue, parallel=1, at most three outstanding. Post-fix DLL sha256 prefix 451d3caf; later commits change tests/questions/handoff only.
AT25 filed in `docs/specs/questions/automata.md`: remaining worker continuation, return-register and concrete-view contract is unresolved. Renamed from the accidentally reused AT24 (already assigned to airport ownership); no new construction answer found on resume. No guessed layout/return fix; this item is stopped as instructed.
Queue: wip only; not submitted for landing because required s1/s2 acceptance fails despite a passing gate.
Next: spec writers answer AT25; implement that boundary, rerun s1/s2 and required gate after any fix, then enqueue kind engine with the actual detached HEAD tip.

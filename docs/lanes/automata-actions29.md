| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| automata-actions29: construction worker ABI (engine, r3) | Codex / automata-actions29 / detached HEAD; reserved branch automata-actions29; code 6e4dcad3e, da8c350d0; ABI true, no version bump | wip; gate 7894 Rust / clippy 0 / CTest 125/125; s0/s1 216/216; s2 84/216 with and without automata; shadow 73/73 exact; NM20 open | 1. Answer NM20 and regenerate question index. 2. Resolve s2 consumer and rerun acceptance. 3. Enqueue kind engine after all criteria pass. |

Game logs: `C:/Users/tlori/FreeCity4/build-runs/game-results/`; gate logs: this worktree's `build-runs/`. All labels below have matching `.log`/`.verdict` files.
Fix: primary worker slots 30/14 now pop 8/4 argument bytes and store flags/orientation; slots 7/8 copy positions; GetPosition returns the caller buffer in EAX through a locked C++ snapshot.
Contract: re-nm29/NM17 already merged via 8bce367da; AT25 answer via main merge 6d932fcdc. No concrete worker factory or reused native internal creation/removal body.
Original isolation (r1): `aa29-base-auto-a`, `aa29-base-netmgr-a` fail 5/216; `aa29-base-nettool-a`, `aa29-base-house-s1-a` fail 26/216; `aa29-trace-auto-a.log` records slots 30/14 before the heap-target fault.
Build correction: inherited Ninja deps watched replace-netlot28 headers; allocator object predated the enlarged worker. Cleaned 320 outputs, rebuilt 318 edges; current deps reference this worktree. Earlier r1 heap-corruption runs are not current acceptance.
Regression PASS: `automata_construction_test` 1/1 focused and in final gate; both managers, D0-D8 heights, EAX, stack/x87, guarded buffers, stable primary vtables and both list-drain orders. Test now isolates/restores its ini.
Gate PASS: `qbuild-test.log` 7894 passed/0 failed/0 ignored; `qbuild-clippy.log` exit 0/0 diagnostics; `fc4verify-build.log` 43/43; `fc4verify-ctest.log` 125/125. `all` once; three owned Rust files fmt PASS.
DLL attribution: s0/s1/s2-a/shadow/house use SHA256 prefix `6c4bb917`; final gate relink and paired s2-b controls use `afb59058`. Engine source is unchanged between these builds.
s0 PASS actions/end: `aa29-at25-house-s0-a.log`, 216/216, complete marker and 2028-01-28 target; corrupt only at excluded native quit +0x1D0F94 on 2028-01-29, ref0.
s1 PASS: `aa29-at25-house-s1-a.log`, 216/216, complete marker, 2028-01-28 target, clean quit, ref0; original heap-target fault absent.
s2 FAIL: `aa29-at25-house-s2-a.log` and `aa29-at25-house-s2-b.log`, both 84/216; native +0x3529C0 reads null+0x6C on 2026-08-31 / 2026-09-01; ref0.
s2 control: `aa29-at25-s2-noauto-b.log`, automata explicitly off, same final DLL, same 84/216 and native +0x3529C0/null+0x6C on 2026-08-31; this failure does not require replacement automata.
Incomplete control: `aa29-at25-s2-noauto-a.log`, 0 actions, load fault +0x10A6CD; not used as an in-city comparison.
Shadow PASS: `aa29-at25-shadow-a.log`, clean through 2032-01-07; shutdown 73/73 samples exact, 438 components/731467 bytes, 0 mismatches, ref0.
Unchanged house PASS: `aa29-at25-house-a.log`, clean, 2032-01-01 target reached, ref0.
Queue discipline: shared main queue, parallel=1, at most three requests outstanding; no private queue or direct game starts.
NM20 filed in `docs/specs/questions/network-manager.md`: identify the later native consumer/view after s2's type-5 network action; stop that item rather than guess. Coordinator must regenerate the question index.
Not enqueued for landing: s2 end criterion remains unmet; local merge entry is wip despite the passing gate.
Next: answer/route NM20, fix its specified boundary, rerun s2 and final artifact acceptance, then enqueue kind engine.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| close-gdriver29 / renderer-driver, engine | Codex / .claude/worktrees/close-gdriver29 / detached HEAD; branch reserved close-gdriver29 | ready for partial landing; code 0e6b755a6, e75bc6af8, b4280f930, c09a497ca, 919d83b8a; ABI unchanged; cargo 7888/0, clippy 0, CTest 125/125; H/A clean, each 72/72 exact; questions none | 1. Credit only complete method evidence below. 2. Complete remaining 62 slots and matched DX7 timing before whole-row credit. |

Changed: bounded device-call replay with foreign results, scalar/out-byte comparisons, per-method counters, public probes and explicit default-off startup DX7 selection; final fix registers its config key.
Gate (final source): qbuild all PASS, Rust 7888/0/0, clippy diagnostics=0, native 47/47 edges, CTest 125/125; build-runs/build-results/qbuild-20261026-120314-build-ctest.verdict and qbuild-{test,clippy}.log / fc4verify-{build,ctest}.log. Owned Rust fmt and final diff check passed.
Final DLL SHA256 a8992448e49d6ecbe251ec8368cf7f0ef549a4d3b3b5d18250294598f7a574c3; both runs use gdriver_shadow=1, gdriver_shadow_select=1, turbo=1, end_date=2032-01-01, minimize=0, parallel=1; active DX7 device, 8 stages.
H = gdriver29-r4-house-2032-20261026a: clean, 2032-01-10, 66.172s, ref_warnings=0, other warnings=0; 72/72 exact, 222 samples, mismatches=0, refused=0. MAIN/build-runs/game-results/H.log:14549 and H.verdict (substitute label).
A = gdriver29-r4-actions-2032-20261026a: clean, 2032-01-05, 66.062s, ref_warnings=0, other warnings=0; 72/72 exact, 264 samples, mismatches=0, refused=0. MAIN/build-runs/game-results/A.log:14620 and A.verdict (substitute label).
AC1 PARTIAL: device stream / return / captured state compared; texture and surface creation/lock formats, primitive queue prestate, host memory and lifecycle effects remain unmeasured. Do not credit the whole nSGLDX7* row.
AC2 PARTIAL: 52/83 slots hooked. Startup public probe runs in H and A; A also executes all 7 script lines (log:1667-5823), lighting hours 6/18/23 and dated probes. Slot 14 is exercised but incomplete; 26,46,48,54-57 have complete calls. Zoom/rotate/screenshot/display and unreachable-slot proofs remain missing.
AC3: 21 slots have cumulative complete>0, mismatches=0, resyncs=0 monthly lines in BOTH 2032 runs; H.log:14462-14513, A.log:14554-14605. Credit those method lines only; shared addresses may deduplicate function credit.

| Slots (all 83 accounted for) | H complete per slot | A complete per slot | Scope |
|---|---|---|---|
| 6,8-10,26,47-49,54-57,82 | 1 | 4 | complete sampled outputs |
| 28,40,44-46,51,81 | 9 | 9 | complete sampled outputs; 44/45 only caps 5/7 |
| 50 | 1 | 1 | complete sampled outputs |
| 11-23,27,41-43,52-53,62,67-76,78 | 0 | 0 | hooked; partial or unexercised |
| 0-5,7,24-25,29-39,58-61,63-66,77,79-80 | 0 | 0 | not hooked |
H/A complete-call totals 77/116; per-slot coverage at H.log:14550-14632 and A.log:14621-14703; 18-20,23,68-73,78 had zero observed calls in both runs, without an unreachable claim.
AC4 PARTIAL: sampling bounded to one/slot/32 frames and 128 device calls; measured H/A times above. A matched active-DX7 control was not run; <=1.5x overhead is NOT established.
Intermediate r4-startup-2032-20261026a used DLL 91029f73: 218 samples, 72/72 exact, 3 config warnings fixed by 919d83b8a; not final-source evidence. Earlier r2 in-city selection exit -1 and inactive-software runs remain excluded.
Questions none: no unexplained native/core differences; RD1/RD3/RD4 remain open. Public C ABI/version unchanged (private static-link transport only); no coverage map, generated ABI entries or shared Git refs edited. No request outstanding.
Next: land the finished partial oracle and 21-slot evidence; finish texture/surface/queue/remaining slot outputs and missing actions, then repeat both 2032 runs and a matched timing control before whole-row credit.

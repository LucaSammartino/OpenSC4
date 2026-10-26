| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-gzfw29 - GZ command server/buffer; kind engine | Codex; .claude/worktrees/replace-gzfw29; detached HEAD, reserved branch replace-gzfw29 | BLOCKED: owners 80671b63d/1d060a436; fixes b23a1e54e/d38c8fd9d/c2defe7c5; ABI unchanged; seam 0x475A0001; all PASS 7889 Rust, clippy 0, CTest 125/125; off shadow 25/25 exact; GZF12/GZF6 | 1. Specify remaining buffer/cache contracts. 2. Qualify current DLL against house profile, cross-loads and command stream. 3. Enqueue engine only after acceptance. |

gzfw is NOT ready for the replace list. WIP queue entry only; no merge-train enqueue.
Code: default-off buffer factory and command service; synchronized Rust registry/storage; native dispatcher/parameter-set helpers through interfaces.
Resume fixes: default conversion slots no longer call notification generators; parsed words now use copied 0x800E arrays instead of the SDK's empty SetValString method.
Native tests cover replay/retention, conversion outputs, 256-word cap, copied string type/count, full dispatch status, parameter lifetime and cdecl formatting.
(a) House list plus gzfw to 2032: NOT RUN; qualification stopped at the prior minimal-profile key-on startup failure below.
(b) Fresh key-off shadow: 25/25 monthly samples exact, all buffer/command differences zero, ref_warnings=0; main build-runs/game-results/gzfw29-s0-off-r3.log and .verdict.
(b) Save on/load off and reverse: NOT RUN; no qualified key-on city load or on-shadow samples.
(c) Fresh minimal-profile gen27 s0-grow off: all 216 actions ran, final date 2028-02-01, corrupt=1 foreign exception, ref_warnings=0 (same r3 log/verdict).
Fault 0x005D0F94 is within the native cSC4ConstructionOccupant::Shutdown ABI range, matching the task's known quit-time fault.
(c) Prior minimal-profile s0-grow on: no-load after 900 s; main build-runs/game-results/gzfw29-s0-on-r1.log and .verdict (DLL f966786a).
On log reached uncovered buffer slots 21/31/37/38; broken region UI: main build-runs/q-main-gzfw29-s0-on-r1-end.png. This does not isolate the cause.
(c) House-profile s0/s1 parity: NOT QUALIFIED; s1 not run. Minimal-profile results give no house-list credit.
Command stream: 0 execution lines in fresh off r3 and prior on r1; no dispatch-stream credit. A script invoking GZ commands is still needed.
Game r3 used DLL b70111f1 before c2defe7c5; the last argument fix is native-tested only. Buffer and key-off paths are unchanged; final key-on is unqualified.
(d) Full all PASS on final code: Rust 7889/0; clippy 0 diagnostics; build 46/46; CTest 125/125, including gzfw_native_owners and compare_dispatch_modes.
Gate logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log. Owned Rust fmt: 2 files, 0 differences.
Earlier fixture null-QI fault was fixed by d38c8fd9d; two subsequent string assertions failed before c2defe7c5 and pass in the final full gate.
Old session 65368 is unavailable; its completed native log had 123/124 passing. The final full gate supersedes that result; no checks remain running.
GZF12 remains in docs/specs/questions/gz-framework-core.md: missing hardware-cache identity/ownership/invalidation contract; GZF6 also leaves other buffer operations unspecified.
Next: resolve GZF6/GZF12, then obtain the current house list and collect (a)-(c), nonempty command-stream and cross-load evidence before ready/enqueue.

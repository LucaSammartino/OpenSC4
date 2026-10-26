| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-gzfw29 - GZ command server/buffer; kind engine | Codex; .claude/worktrees/replace-gzfw29; detached HEAD, reserved branch replace-gzfw29 | WIP/BLOCKED GZF13; code 93eb6c585,9b0612dc1,904cef175,d60676e32,a9ad3d2d4; ABI unchanged; seam 0x475A0001; all PASS 7962 Rust, clippy 0, CTest 130/130; off shadow 3/3 exact; on corrupt | 1. Specify the buffer secondary drawing face. 2. Requalify (a)-(c) with the current house list. |

gzfw is NOT ready for the replace list. Do not flip cGZCommandServer/cGZBuffer replacement credit (17,269 B); no ready enqueue.
Merged main first (89d036278); restored the in-flight default-off command service and buffer factory.
Implemented the 47 primary buffer entries, packed colors/alpha quirks, fills, clipping, lines, conversion/overlap, disposal/reinit and hardware-cache ownership/invalidation.
Added native GZB worked cases/cache callbacks, all-36-converter checks and an explicit dated gzfwprobe action for a nonempty command stream.
(d) Final qbuild all PASS: Rust 7962/0, clippy 0 diagnostics, CMake 30/30, CTest 130/130 including gzfw_native_owners and ret_channel_check.
Gate logs in this worktree: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log; owned Rust fmt 5 files/0 differences.
Gate/runtime code d60676e32; subsequent changes only mark/document GZF13. Game DLL SHA256 prefix 240183d7.
Game paths below are <MAIN>/build-runs/game-results/; requests used the shared queue, at most two outstanding.
(a) House list plus gzfw to 2032: NOT QUALIFIED. Minimal key-on already fails before the first day: gzfw29-min-on-r4e.log/.verdict, corrupt, foreign_exceptions=1, RTC=0, ref_warnings=0.
(b) Key-off control: gzfw29-min-off-r4e.log/.verdict clean through 2026-04-03, ref_warnings=0; 3/3 monthly shadows exact (420 buffer + 225 command comparisons).
(b) Cross-loads NOT QUALIFIED: key-on cannot reach its scripted save. Key-off r4d exported build-runs/suite/states/gzfw29/off.sc4 successfully; no key-on save exists.
(c) House-profile gen27 s0-grow/s1-grow parity NOT RUN: stopped at the earlier key-on startup failure; no house-profile or known quit-time-fault exemption claimed.
Command stream: r4e off has 6 execution lines, all status 0; on has 0. No off/on or gen27 stream-equality credit. Prepared gen27 probe script remains in build-runs/gzfw29-s0-probe.script.
GZF13 filed in docs/specs/questions/gz-framework-core.md: secondary IID 0x86D72B57 lacks drawing-slot inventory/signatures; no inferred slots implemented.
Evidence: r4d buffer created at 0522EF60 (lines 1047-1050), fault ECX=0522EF64 (4088), invalid target 6F6E6160; first return address 008D889F lies in ABI-listed RZDrawBltEdge.
Earlier r4a RTC=488 was fixed with the SDK's full pixel-format descriptor; r4d/r4e RTC=0 but the independent secondary-face fault persists.
Earlier r4b requests were rejected (export key placement), r4c failed before launch (export path), r4d on export failed after the corrupt game; none is a passing qualification run.
Current house replace list was requested asynchronously but not supplied. WIP merge-queue entry only; no merge-train enqueue or credit assertion.
Next: answer GZF13, implement/test that face, then obtain the house list and collect (a)-(c), bidirectional cross-loads and an exact nonempty gen27 command-stream comparison.

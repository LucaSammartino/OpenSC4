| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-gzfw29 - GZ command server/buffer; kind engine | Codex; .claude/worktrees/replace-gzfw29; detached HEAD, reserved branch replace-gzfw29 | READY; code 40057b1cd, merge 9a99aee86; ABI unchanged; seam 0x475A0001; Rust 8078/0, clippy 0, CTest 136/136; house clean 2032; key-off shadow 3/3 exact; cross-loads clean; gen27 parity; no new questions, GZF13 resolved | 1. Add gzfw to the house replace list. 2. Flip cGZCommandServer and cGZBuffer replacement credit (17,269 B). 3. Collect optional long shadow control gzfw29-shadow-off-r5a. |

gzfw is ready for the replace list: flip cGZCommandServer and cGZBuffer (17,269 B, engine/core/src/gz_framework).
Merged main first; retained main's UI actions/effects-resource registrations and this lane's default-off gzfw key.
Implemented secondary slots 3-4 from gz-framework-core section 6.7: truncated RGBA metadata, all-255 absence, byte stores, ordered aliasing, shared lifetime, no pixel/cache changes.
Native GZB9 and GZC75-GZC80 pass, including secondary ECX/stack cleanup and disposal independence; gzfw_native_owners failures=0; ret_channel_check passes.
Game evidence below is <MAIN>/build-runs/game-results/<label>.log and .verdict; all completed acceptance runs have ref_warnings=0 and rtc=0.
(a) gzfw29-house-on-r5a: house list plus gzfw CLEAN through 2032-01-01; foreign_exceptions=0; 72/72 GZ shadows exact (10,080 buffer + 5,400 command comparisons).
House list came from game-queue/done/regress-train282-replace.req; the complete key-off shadow ini block came from done/regress-train282-shadows.req.
(b) gzfw29-shadow-off-r5b: full train282 shadow settings, gzfw OFF, CLEAN through 2026-04-01; 3/3 exact (420 buffer + 225 command), zero mismatches; both recorded actions succeed.
(b) gzfw29-load-on-off-r5a: key-on save loads with key OFF, CLEAN through 2032-02-01; 2/2 exact. Source on-r5.sc4 SHA256 0cbdb2fd3b4cbb0c75b60c8bfd83a5e967d6db9d87ce0aa1c390cbf84d39509f.
(b) gzfw29-load-off-on-r5a: native key-off save loads with key ON, CLEAN through 2026-06-01; 3/3 exact. Source off.sc4 SHA256 2ffb6664d73964a6f00ef539001b33c6c36922df1a55dc041362bcdd0c4358d0, exported by the earlier CLEAN gzfw29-min-off-r4d.
The unchanged owner's shadow also has 25/25 exact samples in each s0/s1 run, with the key both off and on (3,500 buffer + 1,875 command comparisons per run).
(c) gzfw29-s0-off-r5a / gzfw29-s0-on-r5a: both reach 2028-02-02; both CORRUPT only at native construction occupant Shutdown, one null-read AV at 0x005D0F94, the explicitly exempted original-game quit fault.
(c) s0 pair: same 162 recorded action results and same 28 refused actions; no on-only refusals. All 12 ordered command-stream payloads match exactly, all status=0.
(c) gzfw29-s1-off-r5a / gzfw29-s1-on-r5a: both CLEAN through 2028-02-01, zero foreign exceptions; same 159 recorded action results and same 106 refused actions; no on-only refusals.
The s0 pair uses the same gen27 s0-grow script with two dated gzfwprobe actions; s1 uses the unchanged gen27 s1-grow script. No new failure with gzfw is observed.
(d) qbuild all PASS once after the implementation: Rust 8078/0, clippy 0 diagnostics, CMake 31/31, CTest 136/136. Owned Rust fmt: 2 files, 0 differences.
Gate logs in this worktree: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log.
Game DLL d43490e5 was built from the final source before its commit; full gate relinked c8081936 without tracked code changes (19 C++ cache hits, one native-test miss). Both cross-loads and the bounded key-off control use c8081936.
Additional gzfw29-shadow-off-r5a is still running toward 2032 with every train shadow enabled; last observed 2028-11-01 in slot13/Plugins/FreeCity4.log. It is not used for acceptance; the bounded r5b control above is final and clean.
Requests used the shared queue with parallel=1 and never more than three outstanding. No game files, coverage records, regress lists or ABI exports were changed.
Next: coordinator flips the two named rows and adds gzfw to the replace list; collect the optional long control separately.

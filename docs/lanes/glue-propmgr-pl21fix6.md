| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-propmgr-pl21fix6: deterministic prop shadow, PL21 | Codex; glue-propmgr-pl21fix6 (detached HEAD) | in progress | Apply PL21, gate, three house repeats |

## Handoff
- Goal: correct intermittent GetLotProps shadow list differences using PL21; three exact 220/220 house repeats through 2032.
- Done: PL21 A7/A8/T17 reviewed; core already implements whole-list retention and grid-only appends. Added focused PL21 tests and per-entry mismatch evidence (39766bb9c).
- Done: lot shadow now captures dependency slot-40 results before native pruning, checks native filter predicates independently, replays ordered inputs/positions to Rust, and compares complete outputs. No native GetLotProps output is replayed. Replay/argument/filter failures remain mismatches. SEH cleanup restores the hook and releases retained inputs.
- Done: final repeat 1 pl21fix6-house-r1-20261024-b 220/220; clean2032/ref0, 649.265s, all 73 queries inputErrors=0, 1694 captures/64293 filter checks, lotcfg 293/293; run 387.
- Done: final repeat 2 pl21fix6-house-r2-20261024-c 220/220; clean2032/ref0, 719.844s, all 73 queries inputErrors=0, 1694 captures/65928 filter checks, lotcfg 293/293; run 388.
- Next: collect final repeat 3 pl21fix6-house-r3-20261024-d via build-runs/wait-verdict.ps1. Final DLL SHA256 f86ce9002955b95e10fe2658e67fed08daa72d81db7760b517e7ad35f63b1a59 stays unchanged. Next record 389 if still free locally. After exact verdict, final report/commit/READY merge entry.
- Facts: repeat 3 request outstanding; no ABI changes. Diagnostic 39766bb9c DLL SHA prefix 9b041156 finished clean, ref0 at 2032 in 671.110s: pl21fix6-diag-20261024-a 220/220, lotcfg 293/293; no intermittent failure reproduced. Record run 386.
- Required gate: qbuild all once on final source; qbuild verify uses coordinator queue.
- House profile read from main build-runs/regress-train172-shadows.ini; reproduce all settings in request.
- Diagnostic verify PASS: 233/233 edges, CTest 72/72, 338.6s. Tree-changed warning: focused tests were added during diagnostic build; final all still required.
- Focused PL21 cargo tests 2/2 PASS; absolute-path fmt PASS. Initial q wrapper failed to start Bash (Win32 error 5); initial relative fmt path failed, corrected to absolute.
- Full stable-source qbuild all PASS: cargo 6891/6891, clippy zero diagnostics, CMake 91/91 edges, CTest 72/72 (77.7s), native total 86.2s; verdict build-results/qbuild-20261024-032946-build-ctest.verdict. No repeat needed unless source changes.
- Diagnostic game first 9-minute wait timed out; same unique label completed and was retained. Wait helper now allows 20 minutes per directory-change event; no qwait used.

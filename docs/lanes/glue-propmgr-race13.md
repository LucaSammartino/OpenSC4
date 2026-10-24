| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-propmgr-race13: prop query capture race | Codex / glue-propmgr-race13 | in progress | Trace capture, fix, gate, two house runs to 2032 |

## Handoff
- Goal: eliminate intermittent propmgr lot-list mismatch without comparison tolerance.
- Done: initial handoff committed 611ca106e; source now captures native slot-8 positions rather than reading them early after slot 40. Strict sequence/count validation; comparison unchanged.
- Evidence: train200 failure native extra at pos Z=624; train195 extra Z=976; train190 extra Z=592 (all exact cell boundaries). This suggests checking A7.2 upper-Z contract as well as timing; do not change the core without a spec answer.
- Next: collect qbuild all terminal result (exec session 35875, started 16:35 local); test step aborted, clippy still active at 17:24. Do not launch a duplicate. Then queued verify if all does not already include it; game runs and boundary diagnostics.
- Required: two house runs through 2032, exact 220/220 and no resyncs if possible; otherwise numbered question and measured partial result.
- No game request outstanding. build-runs/wait-game.ps1 waits on filesystem notifications without qwait.
- Focused cargo command through q.py failed before execution: Git Bash CreateFileMapping Win32 error 5, evidence build-runs/q/20261024-163426-34100.log. Full qbuild uses its own supported path.
- Full test evidence: build-runs/qbuild-test.log, pollution_tests::getters_answer_from_another_thread_during_a_main_thread_call hit its 20-second receive timeout, causing non-unwinding panic / 0xc0000409. No Rust changes in this lane. Same built test binary passes alone with --exact --test-threads=1 in 0.00s (build-runs/pollution-getters-focused.log). If repeating full gate, preserve first logs with apply_patch Move and change RUST_TEST_THREADS=1; this is a diagnosed input change, not a redundant rerun.
- Prior failure excerpts preserved at build-runs/propmgr-prior-failures.md. First request prepared as build-runs/propmgr-race13-a.pending; NOT queued, no DLL verified yet.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-propmgr-race13: prop query capture race | Codex / glue-propmgr-race13 | in progress | Trace capture, fix, gate, two house runs to 2032 |

## Handoff
- Goal: eliminate intermittent propmgr lot-list mismatch without comparison tolerance.
- Done: initial handoff committed 611ca106e; source now captures native slot-8 positions rather than reading them early after slot 40. Strict sequence/count validation; comparison unchanged.
- Evidence: train200 failure native extra at pos Z=624; train195 extra Z=976; train190 extra Z=592 (all exact cell boundaries). This suggests checking A7.2 upper-Z contract as well as timing; do not change the core without a spec answer.
- Initial qbuild all TERMINATED exit 1: workspace test aborted in pollution getter timeout; clippy passed; queued native build passed 262/262 edges and full CTest 81/81. Original logs moved to qbuild-initial-{test,clippy}.log and fc4verify-initial-{build,ctest}.log under build-runs.
- Current serial-input qbuild all: RUST_TEST_THREADS=1, exec session 42745. Collect this operation to terminal; no duplicate. It repeats only because the timeout diagnostic passed alone (changed scheduling input).
- Games TERMINATED: propmgr-race13-house2032-a 219/220 (turbo), propmgr-race13-house2032-b-normal 219/220 (normal). Both clean through 2032, ref_warnings=0, inputErrors=0, no propmgr resync lines. Records 521/522 committed with this milestone. No outstanding game request.
- Cause measured: native retained upper-X boundary 144 in a and upper-Z boundary 944 in b even with identical observed slot-8 values and all ordered reads present. Not a position timing race for these failures. PL24 filed in docs/specs/questions/prop-manager.md requesting corrected A7.2 bounds contract; core remains literal half-open, no tolerance. Exact 220/220 target NOT achieved; measured instrumentation is ready once gate passes, contract fix remains open/default-off.
- Required: two house runs through 2032, exact 220/220 and no resyncs if possible; otherwise numbered question and measured partial result.
- build-runs/wait-game.ps1 waits on filesystem notifications without qwait.
- Focused cargo command through q.py failed before execution: Git Bash CreateFileMapping Win32 error 5, evidence build-runs/q/20261024-163426-34100.log. Full qbuild uses its own supported path.
- Full initial test evidence: build-runs/qbuild-initial-test.log, pollution_tests::getters_answer_from_another_thread_during_a_main_thread_call hit its 20-second receive timeout, causing non-unwinding panic / 0xc0000409. No Rust changes in this lane. Same built test binary passes alone with --exact --test-threads=1 in 0.00s (build-runs/pollution-getters-focused.log). Serial gate input change already applied; collect session 42745 to terminal before final reporting.
- Prior failure excerpts preserved at build-runs/propmgr-prior-failures.md. Verified DLL is engine/build-x86/FreeCity4.dll from source 88730ab68.

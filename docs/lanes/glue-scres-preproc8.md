| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-scres-preproc8 physical preprocessing | Codex / glue-scres-preproc8 | in progress | Restore merged replay, implement SCR11, gate and measure 2032 raw replay |

## Handoff
- Goal: SCR11 physical file preprocessing, 16 vectors, full natural scres replay exact to 2032.
- Merged requested branches at 866a2a718; octopus emitted packed-refs permission diagnostic but committed.
- Merge tree unexpectedly removes replay7 files; inspect branch diff and restore only replay work before implementation.
- Restored replay files and SCR11 spec/answered entry using apply_patch; preserved current ABI93 and overlay configuration.
- Open questions file already has no SCR11 entry. Read replay7 report and SCR11 section12 with all 16 vectors.
- Implemented production SCR11 filter with invocation-local flag, removed foreign preprocessing replay and stale SCR11 difference label.
- Focused core SCR-V93–V108:16/16; capi raw replay:3/3 passed. First wrapper attempt failed before execution (Git Bash mapping denial), direct Cargo worked.
- Applied rustfmt check diffs via apply_patch to the owned tests; final format check and full gate next.
- Implementation committed at f90a7edfc. Final owned formatting PASS, zero diffs.
- qbuild all Rust gate PASS: 6936 tests passed, 0 failed/ignored; clippy 0 diagnostics.
- qbuild all terminal PASS: native build240/240 (93.5s), CTest74/74 (143.2s), native240.4s; queue qbuild-20261024-052626-build-ctest collected. Session59001 complete.
- Verifier tree-change note is docs-only checkpoints: engine diff f90a7edfc..HEAD is empty.
- Prepared event-driven build-runs/wait-verdict.py for the required single blocking game wait without qwait; summarize-scres.py counts dated exact lines.
- Predecessor run records remain in merged replay7 branch: their 403–406 IDs collide with current main records; do not re-add them with duplicate IDs.
- Run449 scres8-raw-2032-maple-449 terminal clean to2032-01-05, all problem counts0, ref_warnings0; record committed with this checkpoint.
- Exact counted line: scres149/149 (raw_file_parser_replay73/73, existing76/76); raw mismatches0/resyncs0 on every dated line. No game outstanding at this checkpoint.
- Next: submit default-off scres8-default-2032-oak-450 (run450 checked free), collect once, record result, finalize READY report/queue.
- DLL SHA256 0f40f2a589fad1fe2ca25f3389b52974f723ad88a593044c957c4472a3f45a94. No public C ABI exports changed.
- Worktree detached by supervisor; commit normally, do not update refs.

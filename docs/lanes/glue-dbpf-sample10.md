| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-dbpf-sample10: rotate packed DBPF shadow coverage | Codex / glue-dbpf-sample10 | in progress | Implement rotation; gate and measure house to 2032 |

## Handoff
- Goal: full city-load packed-file sweep, deterministic monthly subsets covering all records yearly; house runtime under 800 seconds, dbpf 73/73 exact.
- Done: prerequisite merge c17f37d71; retained both additive fc4.h declaration groups.
- Implemented full load sweep, sorted-path monthly ceil(files/12) rotation, completed/cumulative and yearly coverage counters; unfinished samples remain non-exact.
- Added native portable test checking all rolling twelve-month windows for 1..257 files and 72 house months.
- Implementation commit: 30cdc0adf. Final qbuild all PASS: Rust 6988 passed/0 failed/0 ignored, clippy 0 diagnostics.
- Native verify job qbuild-20261024-090545-build-ctest completed PASS: build 255/255 (109.7 s), CTest 80/80 (185.8 s), total 299.7 s. Original session 29432 collected to exit 0.
- Outstanding game request: game-queue/gdbs10-r1-house2032-20261024a.req; submitted after passing gate. Never resubmit this label or discard an existing verdict.
- Next: collect existing blocking wait exec session 6676, then summarize with build-runs/summarize-dbpf.py gdbs10-r1-house2032-20261024a.
- DLL SHA256: 1BA53DB4F8BE377D45F3B34651FF3CC566B3B03BB59BB962BA2EE0BF383046BE.
- Game request: train187 house ini plus dbpf_shadow=1; Konradshohe, turbo to 2032-01-01, seconds=1800/timeout=2100.
- Wait for game via build-runs/wait-dbpf-game.ps1, a single filesystem event wait without qwait or polling loops.
- No new spec ambiguity: rotation is explicitly requested shadow policy. Existing EX56 remains open.
- Clean-room restriction forbids opening/editing tools/regress.ps1; coordinator must append the profile key after measured exact run.
- HEAD is intentionally detached; never update shared refs. EX56 remains open; resource-manager packed files only.
- Gate logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log and build-results/qbuild-20261024-090545-build-ctest.verdict.

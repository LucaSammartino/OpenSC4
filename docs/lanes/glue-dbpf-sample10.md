| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-dbpf-sample10: rotate packed DBPF shadow coverage | Codex / glue-dbpf-sample10 | in progress | Implement rotation; gate and measure house to 2032 |

## Handoff
- Goal: full city-load packed-file sweep, deterministic monthly subsets covering all records yearly; house runtime under 800 seconds, dbpf 73/73 exact.
- Done: prerequisite merge c17f37d71; retained both additive fc4.h declaration groups.
- Implemented full load sweep, sorted-path monthly ceil(files/12) rotation, completed/cumulative and yearly coverage counters; unfinished samples remain non-exact.
- Added native portable test checking all rolling twelve-month windows for 1..257 files and 72 house months.
- Implementation commit: 30cdc0adf. Final qbuild all in progress: Rust 6988 passed/0 failed/0 ignored, clippy 0 diagnostics.
- Native verify job qbuild-20261024-090545-build-ctest is queued; collect existing exec session 29432, never duplicate the build.
- Next: collect gate, then submit build-runs/gdbs10-r1-house2032-20261024a.draft to game queue (not yet submitted).
- Wait for game via build-runs/wait-dbpf-game.ps1, a single filesystem event wait without qwait or polling loops.
- No new spec ambiguity: rotation is explicitly requested shadow policy. Existing EX56 remains open.
- Clean-room restriction forbids opening/editing tools/regress.ps1; coordinator must append the profile key after measured exact run.
- HEAD is intentionally detached; never update shared refs. EX56 remains open; resource-manager packed files only.
- No game request outstanding.

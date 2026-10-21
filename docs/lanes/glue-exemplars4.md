| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-exemplars4 (exemplar/cohort resource-factory replacement, data-formats §8.1) | `glue-exemplars4` / (engine lane) | in progress | - |

## Handoff
- Goal: plug-in replacement of the exemplar (0x6534284A, class 0x453429B3) and cohort (0x05342861, class 0x053429C8) factories per data-formats §8.1, behind enable=exemplars; formats_shadow keeps comparing ORIGINAL vs Rust.
- Done: 79540b2f capi fc4_exrec_* (+unit test); 1ea955e9 plugin FC4ExemplarFactory.{h,cpp} (takeover PostAppInit, restore PreAppShutdown), formats_shadow logs "game side = ORIGINAL/REPLACEMENT"; questions EX46-EX49 appended (QUESTIONS.md not regenerated: tools/ off limits).
- Baseline gate (with capi change): clippy 0, ctest 61/62 only because my new test was wrong (fixed); quick PASS after plugin.
- Outstanding game request: gex4-r1-turbo (enable=exemplars, formats_shadow=1, turbo to 2032-01-01). Wait: qwait build-runs/game-results/gex4-r1-turbo.verdict (no --fresh).
- Next: read verdict + grep log for "exemplars:" and "formats shadow"; then normal-speed run gex4-r2-normal; then qbuild all; then finish (report, merge-queue json).
- Plan: shadow with replacement = ours-vs-core consistency, NOT native; native exactness = an enable-off formats_shadow run (gex4-r3-orig) for the candidate union.

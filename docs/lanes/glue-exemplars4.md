| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-exemplars4 (exemplar/cohort resource-factory replacement, data-formats §8.1) | `glue-exemplars4` / (engine lane) | in progress | - |

## Handoff
- Goal: plug-in replacement of the exemplar (0x6534284A, class 0x453429B3) and cohort (0x05342861, class 0x053429C8) factories per data-formats §8.1, behind enable=exemplars; formats_shadow keeps comparing ORIGINAL vs Rust.
- Done: 79540b2f capi fc4_exrec_* (+unit test); 1ea955e9 plugin FC4ExemplarFactory.{h,cpp} (takeover PostAppInit, restore PreAppShutdown), formats_shadow logs "game side = ORIGINAL/REPLACEMENT"; questions EX46-EX49 appended (QUESTIONS.md not regenerated: tools/ off limits).
- Baseline gate (with capi change): clippy 0, ctest 61/62 only because my new test was wrong (fixed); quick PASS after plugin.
- Game gex4-r1-turbo DONE (verdict clean, ref_warnings=0, to 2032-01-05, 45 s wall): takeover of both types logged ("unregistered original: yes"); formats_shadow game side = REPLACEMENT: per-file 8957/8957 exact, store sets exemplar own/effective 8959/8959, cohort own/effective 388/388, EXACT (ours vs core, NOT native); summary reads=9541 failed=0 parent_failed=0 cycle=0 created=9541 property_lists=0 message_adds=0 unsupported=0; restore at shutdown both "yes/yes".
- Game gex4-r2-normal DONE (normal speed, 150 s, verdict clean, ref_warnings=0, to 2034-04-14): REPLACEMENT shadow EXACT; reads=10006 failed=0 unsupported=0 property_lists=0.
- Next (in order): (2) native run gex4-r3-orig (no enable, formats_shadow=1, turbo to 2032-01-01) - its "game side = ORIGINAL" sets are the native verification for the candidate union; (3) qbuild all (cargo/clippy/ctest); (4) finish: report replaces Handoff, merge-queue json (kind engine/glue, abi true: additive fc4_exrec_*, no version bump), state ready.
- Notes: GetPropertyList never called in r1 (property_lists=0) so the EX49 workaround is unexercised; occupant EnumProperties paths may need a building-heavy script to exercise.

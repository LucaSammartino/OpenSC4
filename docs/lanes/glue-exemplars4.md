| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-exemplars4 (exemplar/cohort resource-factory replacement, data-formats §8.1) | `glue-exemplars4` / (engine lane) | in progress | - |

## Handoff
- Goal: plug-in replacement of the exemplar (0x6534284A, class 0x453429B3) and cohort (0x05342861, class 0x053429C8) factories per data-formats §8.1, behind enable=exemplars; formats_shadow keeps comparing ORIGINAL vs Rust.
- Done: 79540b2f capi fc4_exrec_* (+unit test); 1ea955e9 plugin FC4ExemplarFactory.{h,cpp} (takeover PostAppInit, restore PreAppShutdown), formats_shadow logs "game side = ORIGINAL/REPLACEMENT"; questions EX46-EX49 appended (QUESTIONS.md not regenerated: tools/ off limits).
- Baseline gate (with capi change): clippy 0, ctest 61/62 only because my new test was wrong (fixed); quick PASS after plugin.
- Game gex4-r1-turbo DONE (verdict clean, ref_warnings=0, to 2032-01-05, 45 s wall): takeover of both types logged ("unregistered original: yes"); formats_shadow game side = REPLACEMENT: per-file 8957/8957 exact, store sets exemplar own/effective 8959/8959, cohort own/effective 388/388, EXACT (ours vs core, NOT native); summary reads=9541 failed=0 parent_failed=0 cycle=0 created=9541 property_lists=0 message_adds=0 unsupported=0; restore at shutdown both "yes/yes".
- Game gex4-r2-normal DONE (normal speed, 150 s, verdict clean, ref_warnings=0, to 2034-04-14): REPLACEMENT shadow EXACT; reads=10006 failed=0 unsupported=0 property_lists=0.
- Gate round 3: qbuild all found 2 ctest failures from my calls (abi_conformance: FindObjectFactory name scan voids slot-22 waiver; ret_channel: 5 unreviewed bool reads). Fixed in plugin commit (member pointer to the slot-21 overload, [abi-ret] notes from §8.1/§4.4); both tests pass; cargo 3677/0, clippy 0.
- Game gex4-r3-orig and gex4-r3-orig-b: harness-error ("requested operation requires elevation" at game Start, no plugin log); environment, not the DLL.
- Next: rerun qbuild all; native control gex4-r3-orig-c (no enable, formats_shadow=1, turbo to 2032-01-01); building trace gex4-r4-build (enable=exemplars, script build-runs/gex4-build.script) to exercise GetPropertyList; then finish (report, merge-queue json, abi true additive fc4_exrec_*).
- Notes: GetPropertyList never called in r1 (property_lists=0) so the EX49 workaround is unexercised; occupant EnumProperties paths may need a building-heavy script to exercise.

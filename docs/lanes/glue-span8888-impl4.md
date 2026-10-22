| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-span8888-impl4: opt-in 8888 native bridge | `glue-span8888-impl4` / engine lane | blocked; uncommitted round2 over 59014b7b; cargo4182/clippy0/ctest67/67; two clean2032/ref0 runs; direct198/198 and native254/254 per type per run | Repair Git metadata access; supply current39 list; run union; commit reviewed changes before enqueue |

## Changes

- Resumed existing commits 46459ffd (four C ABI exports) and 59014b7b (unverified bridge). Audited partial code; current main has no changes to the relevant 8888/shared-context specs. No main merge or unrelated main code copied.
- Connected the five thiscall-compatible forwarders to the build, subsystem key, PostAppInit and PreAppShutdown. Repaired the nonexistent compatibility-helper call by using the published slot-entry checks. The original renderer constructor/destructor remain responsible for allocation and lifetime.
- Added only the 8888 renderer to existing shared-context dispatch. Target types 8 and 9 use the real draw-system factory, Init and slot22; all seven texture formats include keyed/unkeyed sources. Shutdown/Release and matching destination0x8040/texture0x800 locks are audited.
- Direct fixtures cover all168 blit kernels plus solid/alpha/gradient methods, padded stride140, explicit outer guards and untouched bytes. Corrected copied 16-bit comments and format-specific key sampling/masks.
- Prevented overlapping Rust source/destination slices and shadow comparisons against a mutated aliased source. Such calls retain the original renderer; added an overlap/no-write regression test. Refusal diagnostics remain distinct from exact comparisons. No core pixel math changed.
- Added four C ABI exports in this lane, without changing FC4_ABI_VERSION. No default promotion and no implementation of other renderers.

## Verification

- Baseline qbuild all: PASS, including ctest67/67. Final qbuild all: PASS, Rust4182 passed/0 failed/0 ignored, clippy0 diagnostics, CMake build PASS, ctest67/67 (92.59s). qbuild fmt passes for both C ABI files. Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log.
- Quick: PASS, capi519 tests. One intermediate quick link failed with LNK1168 while a concurrent CTest held the DLL; the sequential retry and final full gate passed. Do not overlap quick/all in this worktree.
- `span8888-impl4-r2-native-20261022-01`: clean, ref_warnings0, 2032-01-12; DLLc0104f5d. Direct198/198,58608 compared dwords, no guard corruptions. Native-context type8:254/254; type9:254/254. Shipped188 calls:25 solid+163 blit;185 exact shadow comparisons,3 original fallbacks,0 differing pixels.
- `span8888-impl4-r2-shadow-20261022-02`: clean, ref_warnings0, 2032-01-06; DLL6d5c15b8. Direct198/198,58608 compared dwords, no guard corruptions. Native-context type8:254/254; type9:254/254. Shipped372 calls:111 solid+261 blit;369 exact shadow comparisons,3 original fallbacks,0 differing pixels.
- In each run/type, native arenas compared235712 bytes; no faults, refused fixture calls, route mismatches, missing renderer calls or moved texture lock counts. Shutdown clears renderer; destination lock count/flags return0/0; context and destination Release both0. Constructed native calls508 per run are counted separately from shipped calls. Direct fixtures bypass hooks and have their own totals.
- Second-run fallbacks: two RenderBlt core status2 rejections (counts22/5,mode0); one memory-window/overlap refusal (count22). They are not exactness or replacement credit. SR8888-6 remains open: shipped entry calls now observed, but the target-producing set-target caller is unidentified.
- Both unique verdicts and logs remain under build-runs/game-results/. Both qwait calls completed. No request is outstanding. No observed difference requires a normal-speed follow-up. No new spec question IDs filed.

## Blockers and exact next steps

1. Restore write access to C:/Users/tlori/FreeCity4/.git/worktrees/glue-span8888-impl4. Git add and a direct write check both receive Permission denied despite the declared writable root. No commit in this round succeeded; tip remains59014b7b. q.py also fails Bash/Service/CreateInstance/E_ACCESSDENIED. Do not treat the branch tip as containing this round's work.
2. Supply the current39 enable list. The brief requests deriving it from tools/regress.ps1, but the absolute clean-room prohibition on reading tools/ prevents this lane from doing so. Async clarification requested; no answer received. Both completed runs enable only span8888, not the required current39 union. Do not relabel them as union runs.
3. Run a uniquely labelled current39+span8888 request through2032/ref0, inspect shadows/refusals, and retain opt-in status pending full evidence. New labels are mandatory; completed labels above must be retained.
4. Commit the audited working changes and this report using the prepared build-runs/msg-glue-span8888-impl4-2.txt, then write the merge queue only when its tip actually contains the work. No queue entry was written for an uncommitted tree. A recovery patch is saved as build-runs/span8888-r2-uncommitted.patch.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-span8888-impl4-r2-manifest.json. This is the single authorized round; no restart, subagents, merge, push or tools changes were used.




### Coordinator delivery recovery (2026-10-22)
Coordinator reviewed and committed the exact reported ten files after the worker exited on Git metadata denial. Two unique native/shadow runs already pass2032/ref0 as measured above; the integration train must independently gate and run the full current39+citydetail+span565+span8888 union before landing. No third lane round is needed merely to run that train regress. Four C ABI exports require the train ABI bump. Keep span8888 opt-in and withhold whole-owner replacement credit: documented refusals and SR8888-6 remain. Original result and recovery patch are preserved.

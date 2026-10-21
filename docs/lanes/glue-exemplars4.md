| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-exemplars4 (exemplar/cohort resource-factory replacement, data-formats §8.1) | `glue-exemplars4` / (engine lane) | ready: round-3 union foreign exception fixed (unread grid refs in Read), GetPropertyList ref audit fixed; union clean to 2032, ref 0; full gate PASS | coordinator final union; EX49/EX50 still block default promotion |

## Report (round 3)

Diagnosis
- Union failure: the native code at 0x0045E0DA did a null read, with return address FreeCity4.dll+0x8D6DE6. I symbolized it with the frozen PDB (evidence-run4) and MSVC llvm-symbolizer. The call path is FC4HostGrid.cpp:93 Get <- capi aura grid_stats <- fc4_aura_get_state <- cFC4AuraSimulator::Read -> LogState("Read").
- Aura Read had just resolved 9 grid references through ReadGZSerializable. foundations §2.7/§3.5: the store reads those objects later, in ascending class ID order, so a Read may only keep the pointer. Our Read-time status logs read grids that had not been read yet. 23 subsystems log their state in Read, and 18 of their get_state functions read grids.
- Why the failure showed only with exemplars: not proven. An unread grid has no specified state, and enable=exemplars only shifts what that state happens to be. The fix does not depend on the cause.

What changed
- 29c6c5c8: merged main.
- 5c2c7a92: FC4Streams ReadRef records every host reference it returns (NoteUnreadRef). FC4HostGrid treats a recorded grid as empty: geometry 0, Get 0, writes dropped, logged at most 4 times. The observer clears the set at PostLoad 0x26C63342 / PreCityInit 0x26D31EC0 (foundations §3.7) and at PostCityShutdown. Added seam op kFC4SeamUnreadRefs=69 and a plugin_load_test check (unmapped token never dereferenced, set cleared by both messages).
- deb65f0b: ExHolder::GetPropertyList was missing FC4_ACQUIRED for the reference CreateInstance returns. Every in-game call logged 2 ref warnings (114450 in gex4-r5-union-b).
- EX50 note: GetPropertyList now runs in game (57566 calls in the union run).

Measured
- Full gate at deb65f0b: cargo 3804 passed / 0 failed, clippy 0, ctest 63/63, build PASS.
- gex4-r5-union-b (42 enables incl. exemplars, turbo to 2032-01-15, formats_shadow=1) at 5c2c7a92: clean, foreign 0, reached 2032-01-17, ref_warnings 114450 (GetPropertyList, fixed in deb65f0b).
- gex4-r5-union-c, same request at deb65f0b: clean, foreign 0, reached 2032-01-15, ref_warnings 0. REPLACEMENT shadow EXACT 8957/8957 (148756 props). exemplars reads=9710 failed=0 property_lists=57566 list_failed=0. Restore at shutdown yes/yes. The store load released 21456 references at PostLoad; the unread-grid guard fired 4 times (log capped).
- gex4-r5-union-a was rejected (bad request format, my error) and is not a game result.

Open
- EX49 and EX50 still block default promotion. The shadow compares ours against ours (REPLACEMENT side), so it gives no native credit.

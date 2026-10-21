| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-exemplar-contractfix4 (exemplar replacement fixed to answered EX46-EX49, data-formats §8.2/§10.8; EX50 caller log; native-vs-ours contract test) | `glue-exemplar-contractfix4` / (engine lane) | ready: df33ed00, c422787d, dc5893f6; no ABI change; gate cargo 3980/0, clippy 0, ctest 65/65; gecf4-r2-turbo2032 clean to 2032, ref_warnings 0, contract native-vs-ours 3/4 MATCH (4th DIFF filed as EX51 e/f); EX51 filed, EX50 updated | 1. RE answers EX51 (cohort tag text, slot-14 name conversion, slot-13 effective list, return values, slot-15 dest layout, SerialWrite bytes). 2. Coordinator: union run with this DLL to split the EX50 GetPropertyList callers by module. 3. Default promotion stays the coordinator's call. |

## Report

What changed (engine/plugin/FC4ExemplarFactory.cpp only)
- GetPropertyList builds its own list object (cFC4UnknownList, the §8.2 slot table, QI 0x12E/1, count starts at 0, Release at 1 or 0 destroys). Properties are inserted at the front in ascending order, so the list reads in descending order. The game-factory workaround is gone.
- Slot 14 DebugInstanceTag(out) appends " Exemplar ", then "(Other)"/"(Unknown)" from 0x10, then " '<name>'" from 0x20, and returns true. Slot 15 rebuild(dest): null rebuilds from the parent holder's GetPropertyList with PopFront 0x452160EC, then the own list on top. A non-null dest is refused (EX51 a).
- Holder slots 22 (deleting destructor) and 23/24 (return false) are added. Serializable slots 3-5 are silent stubs (false/false/0). Factory slot 8 forwards to slot 3 with 0, 0.
- Removal changes the own list, then the parent's property is shown again or the entry is erased; false and no message when nothing was removed. SetCohort with the same parent does nothing. Get* return a borrowed pointer; Set* AddRef the new parent and release the old.
- Change messages with bool true: standard message 0xC53D10AA/0x853D11F1, data1/data2 = key group/instance, SetExtra(object), MessagePost(msg, false). Only holder slots 15/17/18 and SetCohort post (EX51 b).
- EX50: every GetPropertyList return address is logged (module+offset, thread, count).
- ContractTest at install runs the §10.8 scenarios on native objects (from the game's kept factories) and on ours, and logs MATCH/DIFF. Return values the spec does not give are logged separately and not compared.

Measured
- Gate at dc5893f6: cargo 3980 passed/0 failed, clippy 0, ctest 65/65 (baseline the same).
- gecf4-r1-turbo2032 (df33ed00): clean, but ref_warnings 9, all from the test's own releases of game-created objects; fixed in c422787d with FC4_RELEASE_CREATED. In that run the contract test showed 0/4 because of its setup, also fixed in c422787d.
- gecf4-r2-turbo2032 (c422787d, enable=exemplars, formats_shadow=1, turbo to 2032-01-15): clean, foreign 0, ref_warnings 0, date 2032-01-27. Contract native-vs-ours 3/4 MATCH (list_order, remove_reveals_parent, setcohort_refs). stubs_tag_rebuild DIFF: the native cohort tag is " Exemplar Cohort (Unknown)" and the native name from a uint 0x20 is '5' (ours '') -> EX51 e/f. Unspecified: native SetCohort/slot15/RemoveAll return garbage or 0, and a native slot-13 add does not update the effective list -> EX51 b/c evidence.
- Summary: reads 9579, failed 0, property_lists 3627, list_failed 0, messages_posted 0. EX50: all 3627 GetPropertyList calls came from FreeCity4.dll (+0xD72F5C 3624, the rebuild; +0xD7695E 3, the test). No game caller in an exemplars-only run.

Questions: EX51 filed (a-f) in docs/specs/questions/data-formats.md. EX50 got an update line. SerialWrite still fails (EX51 d).

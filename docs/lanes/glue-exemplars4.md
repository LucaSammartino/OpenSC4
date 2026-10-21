| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-exemplars4 (exemplar/cohort resource-factory replacement, data-formats §8.1) | `glue-exemplars4` / (engine lane) | in progress: round 3 union foreign-exception fix | gate, union game run |

## Handoff
- Goal (round 3/3): fix the union foreign exception (native 0x0045E0DA null read, ret FreeCity4.dll+0x8D6DE6) seen with enable=exemplars + other replacements; gate; combined run clean to 2032, ref0.
- Done: merged main (29c6c5c8). Diagnosis with evidence-run4 PDB (llvm-symbolizer, Hostx64/x86): +0x8D6DE6 = FC4HostGrid.cpp:93 Get <- capi aura grid_stats <- fc4_aura_get_state <- cFC4AuraSimulator::Read LogState("Read"). The aura Read had just resolved 9 grid refs (ReadGZSerializable); foundations §2.7/§3.5: their own Read runs later (ascending class ID), so a Read may only keep the pointer. Our Read-time LogState read unread grids. 23 subsystems log state in Read; 18 get_state functions read grids (build-runs/stategrids.py).
- Fix (generic): FC4Streams NoteUnreadRef set filled by ReadRef for host refs; FC4HostGrid treats noted grids as empty (geometry 0, Get 0, writes dropped); observer clears at PostLoad 0x26C63342 / PreCityInit 0x26D31EC0 (foundations §3.7) and PostCityShutdown. Seam op kFC4SeamUnreadRefs=69 + plugin_load_test check (==31).
- gex4-r5-union-b (42 enables incl exemplars, turbo to 2032-01-15, formats_shadow=1): clean, foreign 0, date 2032-01-17, REPLACEMENT EXACT 8957/8957; but ref_warnings=114450, all at ExHolder::GetPropertyList (now exercised in game): missing FC4_ACQUIRED after CreateInstance. Fixed (next commit).
- Next: gate all; union rerun gex4-r5-union-c; then report + merge-queue.
- EX dependence: likely heap state of unread grids (not specified); fix does not depend on it.

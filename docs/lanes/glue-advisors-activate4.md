| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-advisors-activate4: activate cFC4AdvisorSystem with AD7-AD10 corrections | `glue-advisors-activate4` / (engine lane) | in progress (round 1/3) | see Handoff |

## Handoff
- Goal: apply advisors.md §8.2/§12.1/§5.2/§4.2 (AD7-AD10), fill host callbacks + advisor/advice/subject classes, register factory opt-in (never default), game run sim_turbo to 2032 clean (ref_warnings 0).
- Baseline gate (main d68b254a): cargo 3980/0, clippy 0, ctest 65/65.
- Done 55ccc744: register_api walks the 47-entry §8.2 vector (random_chance/_dofile_from_resource/print = originals via new compat Directs 0x0040A810/0x005FC020/0x005FB850); slot 9 returns cRZRandom-shaped member (+4 mul, Seed(-1) form); slot 11 DoLink per §12.1; cheats via AddNotification2 + GetVoid2 text; reload_scripts host cb: _on_reload, app +0xD4(out,-1)/+0xDC dirs + "advisors\", cdecl UpdateResourcesFromDirectory(0x005FC510)/LoadPackageScripts(0x005FBA20) group 0x4A5E8EF6; dirs logged (AD11).
- Done 49281c4c: engine/plugin/FC4AdvisorApi.h (table, link parse, VCall, cheat payload, dirs) + tests/advisor_api_test.cpp (cases 23-27 plugin parts), quick build PASS.
- Not done (blocks activation): advisor/advice/subject GZCOM classes over fc4_advisors_advisor_object/advice_object (ObjectCreated kinds 2-4), AddAdvisor/RemoveAdvisor/Subscribe/PushContextAdvice/GetContextAdvice/SetLocalValue/Read/Write; host callbacks still unset: call_condition, detokenize, set_tutorial_lua, create_tags, register_detokenizer, popup_priority, set_local, city_established, difficulty, cell_counts, sim_date, start_year, is_any_paused, post_update_self, post_ticker, global_effect, rebuild_text, value, list, history/ordinance/occupant/deal/tutorial cbs.
- Gate at 49281c4c: qbuild all PASS (cargo 3980/0, clippy 0, ctest 66/66 incl. new advisor_api_test).
- Round 2 done 87dc9249: host cbs city_established/difficulty/cell_counts/sim_date (+0x1C GetSimDate obj)/start_year (+0x14)/is_any_paused, post_ticker, global_effect (View3D +0xBC/+0xC0).
- Round 2 done 489ff4de: cFC4AdviceSubject (12.4 14 slots incl. Clone, 11.4 via fc4_adv_obj_write/read) made in object_created kind 4; set_global/_string now write game[name] (7.1, was a Lua global); set_local + create_tags + tag-gc (8.1).
- Decision: wrappers never deleted on Release (core keeps tokens without refs, no C ABI drop callback); DestroyAll at system destruction.
- Next: qbuild all gate, one game run (label adv-act4-r2a), then advisor/advice classes (12.2/12.3), call_condition, detokenize, popup_priority, value/list.
- No game requests sent yet.

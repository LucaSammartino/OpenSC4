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
- Next (round 2): start the host callbacks (simple city/sim ones first), then classes.
- No game requests sent yet.

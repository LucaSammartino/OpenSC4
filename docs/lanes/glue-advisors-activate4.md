| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-advisors-activate4: activate cFC4AdvisorSystem with AD7-AD10 corrections | `glue-advisors-activate4` / (engine lane) | wip after round 2 (factory not registered; no activation credit) | advisor/advice GZCOM classes (12.2/12.3), remaining host cbs, then opt-in factory + 2032 run |

## Report (round 2)
Goal: apply advisors.md 8.2/12.1/5.2/4.2 (AD7-AD10), fill host callbacks + advisor/advice/subject classes, register the factory opt-in, game run sim_turbo to 2032 clean.

Done:
- 55ccc744, 49281c4c (round 1): AD7-AD10 in cFC4AdvisorSystem (47-entry 8.2 register_api, slot 9 shared RNG, slot 11 DoLink, cheats via message 2, reload_scripts with AD11 dirs logged); FC4AdvisorApi.h + advisor_api_test.
- 87dc9249: host callbacks city_established (+0x58), difficulty (+0x60), cell_counts (+0x174/+0x178), sim_date (simulator +0x1C GetSimDate object), start_year (+0x14 into a local cFC4Date), is_any_paused (+0x48), post_ticker (0xCA5501F2, 10), global_effect (View3D +0xBC/+0xC0, 6.9).
- 489ff4de: engine/plugin/cFC4AdviceSubject.{h,cpp}: cSC4AdviceSubject 0x6A26D049 / IID 0x6A26D050, all 14 slots (Clone added; SetData1/2 return true per 12.4), cIGZSerializable (11.4 via fc4_adv_obj_write/read), cIFC4CoreObject; built in object_created kind 4. set_global/set_global_string now write game[name] (7.1; they wrote a Lua global before). set_local (game[name], game[name.."_subject"] = subject-tag userdata, AddRef'd), create_tags + tag-gc (8.1, 4.1 step 3).
- e0b38242: [abi-ret] notes; gate PASS.

Measured: qbuild all PASS (cargo 3980/0, clippy 0, ctest 66/66). Game adv-act4-r2a (turbo to 2027-01-01, default ini): clean, ref_warnings 0. This run does not exercise the advisor code: CreateFC4AdvisorSystem is still not registered anywhere, so it only shows the DLL is not regressed.

Decision ([choice], in cFC4AdviceSubject.h): wrappers are never deleted on Release, because the core keeps their tokens without GZCOM references and the C ABI has no "core object dropped" callback. DestroyAll runs when the system is destroyed. A C ABI weak handle or drop callback would fix this; that is a capi change, not a spec question.

Still missing before activation: advisor (12.2, 21 slots + serializable + message target) and advice (12.3, 31 slots) GZCOM classes over fc4_advisors_advisor_object/advice_object (object_created kinds 2/3 still return 0); AddAdvisor/RemoveAdvisor/Subscribe/PushContextAdvice/GetContextAdvice/SetLocalValue/Read/Write system slots; host cbs call_condition, detokenize, set_tutorial_lua, register_detokenizer, popup_priority, post_update_self, rebuild_text, value, list, history/ordinance/occupant/deal/tutorial/conditional cbs. Questions: none filed (AD11 dirs are passed opaque and logged).

Next: (1) advisor + advice classes, wired in object_created; (2) popup_priority (tuning_constants.ADVICE_POPUP_PRIORITIES_BY_SKILL_LEVEL), call_condition, register_detokenizer; (3) value/list sources (7.3-7.8); (4) opt-in factory switch, then a turbo run to 2032 with the advisor action trace.

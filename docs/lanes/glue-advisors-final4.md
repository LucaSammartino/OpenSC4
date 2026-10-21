| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-advisors-final4: advisor replacement (enable=advisors), round 2/3 | glue-advisors-final4 / .claude/worktrees/glue-advisors-final4 | wip: cFC4AdvisorSystem + host compiled, gate PASS (cargo 3803/0, clippy 0, ctest 64/64); not registered, no game run | round 3: advisor/advice/subject classes, then register + game run |

## Report (round 2/3, 2026-10-21)

**What changed** (code commits 5d265158 + ret-channel annotations):
- `engine/plugin/cFC4AdvisorSystem.{h,cpp}`: `cFC4ISC4AdvisorSystem` with slots 3-28 in the exe order (§12.1; distinct
  names for the 12/13 overload pair), cIGZMessageTarget2 (+4), cheat target (+8), cIGZSerializable (+0x10), QI per §1.1
  via `fc4_advisors_query_interface`, one shared refcount. Constructor publishes into `0x00B207C0` (§14.2), destructor
  withdraws. Slots 23-28 over `AdvisorWindowSlot` with `GameAdvisorWinProcOps`; slot 8 over `AdvisorGameMap` (empty mirror).
- Host: Lua table reads (`advisors`, `advices`, fields of §2.1/§2.2), trigger compile/ref/call (§4.5), set_global /
  set_global_string, GC, close_lua, debug flag, cheats, window start/remove, has_city, now_ms, system subscriptions,
  post/send (cFC4Message2Standard), refs, object_created (system only).
- register_api: 34 core names via `fc4_advisors_call_lua` trampolines (lua-host §1.3 host lookup, QI 0xEA10C4A2) +
  the 11 compat originals (AD5/AD6), `_tf_` pattern of lua-host §8.2.
- CMake: the two files (additive). Questions AD7-AD10 appended to docs/specs/questions/advisors.md (QUESTIONS.md
  index not regenerated: tools/ is off-limits to this lane).

**Not done.** Slots 5/6/16/17/18/20, SetLocalValue, Read/Write and object_created for advisors/advices/subjects need the
advisor/advice/subject GZCOM classes (round 3); they currently return the §4.3 null case / false / null. Host
callbacks not set yet: call_condition, detokenize, reload_scripts (AD10), set_tutorial_lua, create_tags (tag-gc §8.1),
register_detokenizer, popup_priority, set_local, city_established, difficulty, cell_counts, sim_date, start_year,
is_any_paused, post_update_self, post_ticker, global_effect, rebuild_text, value/list (§7), history/ordinance/occupant/
civic/deal callbacks. Because of these the class is NOT registered (no `enable = advisors` row); plugin behaviour unchanged.

**Measured.** qbuild all: cargo 3803 passed / 0 failed, clippy 0, build PASS, ctest 64/64 (after annotating cISCLua
bool returns for ret_channel_check). Game: none (family not enabled).

**Next (round 3):** advisor/advice/subject classes over fc4_advisors_advisor_object/advice_object (maps from
FC4AdvisorMaps.h), fill the remaining host callbacks (sim_date/is_any_paused via cFC4ISC4Simulator, §7 value/list),
reload_scripts once AD10 is answered, register `advisors` in FC4Subsystems.cpp (opt-in), then a sim_turbo 2032 run with a
unique label.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-mysims-replace16 (mysims replace key) | Claude / `glue-mysims-replace16` (engine lane) | Ready: `mysims` replace key (needs `advisors`); replace run clean to 2032. | Answer cISC4MySim on a MySim front; native agent's MySim factory calls; saves; Lua table; transport. |

## Handoff
- Replaced with `enable = advisors, mysims`: cSC4MySim 0x4A1DBBBF, cSC4MyStreetSim 0x0BF606A5, cSC4MySimDispatch 0xCBC14674 (FC4Subsystems kMySimsClasses). `mysims` without `advisors` logs unavailable and registers nothing.
- The FC4 advisor system's core gets an fc4_mysim_host (plugin/cFC4MySim.cpp) and runs the MySim classes and the MySim agent (fc4_mysims_agent_init/shutdown in its Init/Shutdown).
- Each core MySim's "create" command makes a cFC4MySimDispatch through GZCOM. That dispatch sits over the new fc4_mysim_dispatch C ABI (capi/src/mysim_dispatch.rs, core/src/mysims/dispatch). start_commute/update_residence set its home; leave_for_work/return_home run DispatchCommute (hours 8/17); shutdown releases it. The core reaches the dispatch, job search, change job and job destination through it. Owner notifications go to fc4_advisors_advisor_do_message.
- The city services moved from the shadow to FC4MySimGameHost.cpp, which the shadow and the replacement share.
- Tests: mysim_dispatch_tests.rs (10). plugin_mysims_replace_factory_test covers registration and the §7.2.3 constructor contract. The default, alone and mixed tests stay negative.
- Gate: qbuild verify PASS (build 179/179, CTest 105/105), cargo test all ok, clippy -D warnings clean.
- Build note: 130 objects in build-x86 had ninja deps pointing at ../train-integration, so header changes did not rebuild them. A stale FC4ExemplarFactory.obj crashed the neighbors test and the game. Fixed by touching those sources.
- Game: mysims-claude-2 (house list + mysims) clean to 2032-01-01, ref_warnings=0, foreign_exceptions=0. 6 FC4 dispatches were created; no native MySim or dispatch was constructed.
- Shadows: mysims-claude-2s (same DLL) is clean to 2032-01-03; mysims shadow 73/73 with every mismatch count 0, 0 resyncs. mysimdispatch_shadow is not part of that profile.
- Hazards: the native agent's 5 GZCOM MySim requests (before advisors Init) get no object, so its MySim slots are empty. cISC4MySim is not answered, so the UI cannot move in or show MySims. The advisor system Write/Read are still stubs, so MySims are not saved. There is no MySim Lua table (tuning uses code defaults). The network model is empty (no jobs found, no commute_time). There is no transport (MS18) and U-Drive-It is refused. The MySim state does not change after a commute (no ABI setter).

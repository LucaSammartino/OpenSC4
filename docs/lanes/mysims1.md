# mysims1 — the mysims shadow (a same-process differential of the MySims)

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| mysims1 (the mysims shadow: FC4MySimsShadow + regress shadows list) | `mysims1` / `.claude/worktrees/mysims1` (branch `mysims1`, engine lane) | in progress — round 3: main merged clean (tip `a1a6d6d3`, tree clean); full design research DONE including the run_summary/regress line-format contract and all host-callback sources; no shadow code written yet | see Handoff |

## Handoff

- Goal: add the mysims shadow (same-process differential like the other shadows), wire it
  into the regress shadows list (tools/regress.ps1 `$runs.shadows`, additive); exact to
  2032 on the test city; in-game runs mysims1-sh-1 (shadows on, original classes) and
  mysims1-sh-2 clean with ref_warnings=0.
- Done: worktree created (branch `mysims1` from main `7e042544`); submodule
  engine/third_party/gzcom-dll repointed to this worktree and populated; lane file
  committed `93c9ca3e`; baseline `qbuild all` PASS (build-runs/baseline.out).
- Design researched (facts a successor needs):
  - The plugin has NO MySim glue: nothing in engine/plugin/ references fc4_advisors.
    The game's own `cSC4MySimAgentSimulator` is reached via `cISC4City` slot 71
    `GetMySimAgentSimulator()` (SDK header engine/third_party/gzcom-dll/gzcom-dll/include/cISC4City.h:188;
    cISC4MySimAgentSimulator.h, cISC4MySim.h, cISC4Advisor.h, cISC4AdvisorSystem.h all in that SDK dir).
  - Original per-MySim state readable read-only: each MySim's cIGZSerializable record
    (version 17, mysims.md §6.2: rows 1-31 + advisor part §11.2 advisors.md) captured via
    WriteCapture (FC4WriteCapture.h); getters (GetSlot, GetResidence, GetJob, GetAge,
    GetSign, IsMale, GetWealth, IsActive, GetVehicleKey, GetLocation); agent getters
    GetActiveMySims/MySimLivesHere/GetActiveUIPanelSim. The advisor system's own record
    (§11.1 advisors.md) refs every advisor incl. MySims.
  - Shadow-core read path problem: CaptureIStream returns refs as HOST tokens, so
    `Advisor::read_record`'s advice refs would NOT resolve to core Advices (skipped), and
    a byte-exact record round trip of the MySim record is infeasible as-is. Working
    pattern from advisors_tests.rs:717-744 (subject_round_trip_and_core_handles): write a
    custom istream whose get_ref returns `core_handle` from fc4_advisors_create_object
    (class store creates core Advice/Subject/MySim objects) so refs resolve as core
    objects; then compare field-by-field (decode BOTH records with CaptureReader in C++
    and diff the decoded fields) instead of raw byte equality.
  - Shadow core = fc4_advisors handle created with a read-only fc4_advisor_host +
    fc4_mysim_host (all mutating callbacks NULL; resolve_ref maps capture object id →
    token), plus MySimAgent init; the shadow drives its own daily/monthly via
    fc4_advisors_advisor_simulate on the MySim ids after mirroring the original's steps
    (compare per mysims.md §2.2 saved fields + GetActiveMySims count; EQ/HQ recomputed
    equal only if the host feeds the same grid values).
  - Wiring points: FC4Config.cpp kOtherKeys end (add `mysims_shadow`); cFC4Director.cpp
    PostAppInit `MySimsShadowStart()` + PreAppShutdown Stop; FC4Observer.cpp PostCityInit
    `MySimsShadowCityLoaded(CurrentCity())` and kMsgSimNewMonth case `MySimsShadowMonth`;
    CMakeLists.txt plugin list (additive); tools/regress.ps1 line 170 shadows string
    (additive at end).
- Round 2 (2026-10-21, context OVER before code; design above re-verified, keep it):
  - main is now 37283fba and HAS cores28: engine/core/src/mysims/dispatch.rs + pool.rs (+
    tests), sim.rs +6 lines, MySimHost gains only the optional-default `dispatch_check_flags`;
    mysims.md itself unchanged; questions MS8-MS10 exist on main but NOT in this worktree.
    This branch cut at 7e042544 = 2 commits behind. First step of round 3:
    `git -C <WT> merge main` (the shadow touches no mysims core file; merge is clean).
  - The C++ side of the design holds: cISC4City.h:188 slot 71, MySim QI 0x0A1DA42F,
    AsAdvisor() = cISC4MySim slot 3 → cISC4Advisor slot 14 Simulate(kind)/slot 5 GetID; the
    agent's 6 slot ids come from the city's advisor system (slot 81 GetAdvisorSystem,
    slot 8 GetAdvisors map, id 30 = My Street at index 5). GetActiveMySims takes an
    SC4Vector<cISC4MySim*> (the game appends; use a local SC4Vector, plugin/FC4GameContainers.h
    PushTrivial for pointers).
  - Feeding the shadow core over the C ABI (fc4.h): `fc4_advisors_create(&h,&m,S)` with
    read-only hosts (all mutating callbacks NULL, resolve_ref returns the token,
    object_created returns 0), `fc4_mysims_agent_init`, then mirror §3.2 messages via
    `fc4_mysims_agent_do_message` (Target pattern, FC4HistoryShadow.cpp lines 655-700) and
    the day/month steps via `fc4_mysims_advisor_simulate(p, id, 0|1)` AFTER the original's
    own pass (ageing needs the shadow advisor's month_counter to advance — Simulate(1) does
    that itself, advisor.rs:436). `ui_blocked` callback NULL (reads false; the original's
    blocking-UI state is not readable read-only, question MS2).
  - Session seed: `fc4::SessionSeed()` (plugin/FC4Seed.h). Each core MySim's RNG derives
    seed = SessionSeed ^ (clsid::MYSIM ^ index) (sim.rs:105, rng.rs derive_seed), so with the
    same S the shadow core is draw-exact; the original's MySim RNG is wall-clock-seeded and
    unreadable — FindNewResidence's start position is the only uncomparable draw (only
    reached on home destruction): log it, do not compare.
  - Session-seed mismatch NOTE: the shadow core's per-MySim index is its creation order in
    the class store during Read, which may differ from the original's; if a start-position
    draw ever matters the diff will show it — name it and move on.
  - Compare DECODED fields, not bytes: original per-MySim record (WriteCapture on each
    MySim's Serializable, version 17 §6.2) decoded in C++ with CaptureReader; shadow state
    read with fc4_mysims_agent_get_state + fc4_mysim_get_state + fc4_mysim_text. Also compare
    GetActiveMySims count/ids and the getters (GetSlot/GetAge/GetSign/IsMale/GetWealth/
    IsActive/GetResidence/GetJob) against the shadow's state — getters are the surface the
    UI reads. EQ/HQ: the shadow's host must feed the same grid values for eq/hq to match:
    wire the mysim host's eq_by_wealth/hq_by_wealth/life_expectancy/average_age to the game
    (residential simulator, mysims.md §9.2) READ-ONLY, or expect those fields to differ and
    name them. Simpler first cut: wire grid+eq hosts to the game read-only; the shadow is
    then exact when the simulators are.
  - Wiring points with VERIFIED line numbers (this worktree): FC4Config.cpp kOtherKeys ends
    line 320 ("ports_shadow"}}; add `mysims_shadow` before the close); cFC4Director.cpp
    PostAppInit 341-346 (add MySimsShadowStart() after TerrainShadowStart()), PreAppShutdown
    354-364 (add Stop); FC4Observer.cpp PostCityInit case 574-601 (add CityLoaded after
    AutomataShadowCityLoaded line 601), kMsgSimNewMonth case 664-707 (add Month after
    PropDevShadowMonth ~line 701), PreCityShutdown 630-647 (add CityShutdown after
    AutomataShadowCityShutdown 647); FC4Subsystems.cpp Resolve() end (~line 629, after
    AutomataShadowStartup(enabledKey("automata"))): `MySimsShadowStartup(!SimulatorReplaced())`
    (no subsystem named mysims; the INACTIVE case is `enable` making the agent ours — there
    is no enable key for it, so the bool is always true and the check stays for symmetry);
    engine/CMakeLists.txt plugin list after FC4AutomataShadow (~line 470); tools/regress.ps1
    line 170 shadows string (append `mysims_shadow = 1` at the end); engine/README.md a
    "MySims shadow" bullet after the Automata shadow bullet (~line 1774, additive).
- Round 3 (2026-10-21, further research, NO code yet):
  - main MERGED clean (tip `a1a6d6d3`); questions MS8-MS10 now in this worktree
    (docs/specs/questions/mysims.md, next free MS11).
  - Line-format contract (run_summary.ps1:134-151): a shadow line counts as a "shadow line"
    only if it matches `\] ([a-z]+) shadow [^:]*\d{4}-\d\d-\d\d[^:]*: .*(mismatch(es|ed)[= ]\d|: clean\b)`
    — i.e. the log line must read `<name> shadow YYYY-MM-DD <event>: ... mismatches=N`. The
    LAST `; mismatches=N` on the line decides clean/differs. So the shadow's load/month lines
    must carry a date and end with `; mismatches=N` (civic/citysit shadows are the template).
  - Compare DECODED vs DECODED (final decision): capture each original MySim's record
    (WriteCapture), decode it in C++ with CaptureReader (version-17 layout mysims.md §6.2),
    and diff against the shadow core's `fc4_mysim_get_state` + `fc4_mysims_agent_get_state`
    + `fc4_mysim_text(bio=0/path=1)`. Plus agent getters via
    `fc4_mysims_active/lives_here/get_active_ui_panel_sim` against the game's
    GetActiveMySims/MySimLivesHere/GetActiveUIPanelSim (SC4Vector via plugin FC4GameContainers
    PushTrivial; std::map GetAdvisors avoided — use GetAdvisor(id) after reading the map via
    per-id probe is NOT possible: instead enumerate ids 1-5 + 30 by QI-ing each candidate
    `as->GetAdvisor(id)` for cISC4MySim 0x0A1DA42F).
  - ORIGINAL message mirroring (exact source): the agent's own cIGZMessageTarget2 (city
    component, obtained slot 71) receives the SAME §3.2 messages the game delivers to it —
    the shadow core must get copies. Route them from the shadow's own message-subscriptions:
    register the shadow target for the SAME 11 msg ids (order in agent.rs AGENT_SUBSCRIPTIONS:
    0x99EF1142, 0x99EF1145, 0x99EF1143, 0xA6B79602, 0xA6B79612, 0x44778230, 0x8A09FA6C,
    0xCA1B1E64, 0xEA1F0DCD, 0x0A36F1E9, 0xEA0D163E) and forward each into
    `fc4_mysims_agent_do_message` with fc4_adv_message{msg_type,data1..4,void1=token of the
    occupant,unknown=advisor id for 0x8A09FA6C}. Occupant token = canonical cISC4Occupant
    TokenOf (civic shadow's Canonical() helper pattern, cFC4Civic.cpp:157).
  - Daily/monthly drive: the ADVISOR TICKER (0xCA5501F2, advisors.md §5.1) is what calls
    Simulate(kind) on every advisor incl. MySims (monthly on day 28 BEFORE daily). The shadow
    core's ticker must be driven identically: subscribe the shadow target to 0xCA5501F2 +
    PostCityInit 0x26D31EC1 + CityEstablished 0x26D31EC4 and forward into
    `fc4_advisors_do_message` — the core's on_ticker implements §5.1 itself (system.rs:431)
    and calls Simulate on all advisors, so NO manual per-advisor simulate calls needed. The
    host `sim_date` callback must return the real game date (fc4_adv_date{day_number,year,
    month,day}) from cISC4Simulator slot 8 GetSimDateNumber + fc4_date_from_julian.
  - Lua/host wiring for the SHADOW core (all read-only, mapped to game): create_table=false,
    set_field=collector (fields go to a buffer, optionally diffed later), activate_lua=true
    (no-op), tuning=NULL (code defaults — the original reads the same adv_const defaults at
    Init), ui_blocked=false (MS2), register_agent_lua=no-op, cell_counts=city slots 93/94,
    cell_center=city slot 97, disaster_ongoing=disaster layer slot 10, disaster_locations=
    slots 16/17 + instance GetID() (manager type = instance id, disasters.md §1: GetID
    returns the MANAGER's type), building=occupant QI 0x87DDA3A9: GetBuildingType(4)/
    GetCompletionPercent(9, SC4Percentage* → float)/GetBuildingProfile(11: purpose,wealth)/
    GetPosition(occ slot 8)/name=GetBuildingName(occ 123)/groups=GetOccupantGroups(occ 68,
    std::set<uint32_t>), lot=lot mgr GetOccupantLot(14)+GetLocation(21)/HasRoadAccess(28)/
    GetCondition(35)/GetOccupantWealth(41)/lot rect from GetLot bounds, occupant_position=
    occ slot 8, buildings_in_box=occupant mgr slot 40 w/ filter... FIRST CUT: implement
    only what the daily/monthly path + saved fields need (eq/hq/average_age/life_expectancy,
    grids for UpdateVariables), leave trip/dispatch callbacks NULL and NAME the gap in the
    startup log — trip fields only diverge on dispatch activity, which the test city's idle
    MySims don't generate.
  - EQ/HQ feed: residential simulator (city slot 68): GetCellEQByWealth(37)/GetCellHQByWealth(38)
    take (x,z,WealthType), GetAverageAgeMap(41) → cISC4SimGrid<int32_t> GetCellValue,
    GetCellLifeExpectancy(29) returns int32_t AL; grids for UpdateVariables: crime sim
    (slot 33?) GetCrimeGrid, pollution GetPollutionGrid(0..3), aura GetAuraGrid (int8),
    congestion GetCongestionMap, desirability via bds GetDeveloper(0x1010/0x1020/0x1030)
    → GetDesirabilityMap; coverage: police/fire sim GetPolicePowerGrid/GetFireProtectionGrid
    + GetAverageValueInCellRect; stations: police GetPoliceStations(63)/fire GetFireStations
    (SC4List, ilist for fire) + GetStationFunding(occ, int32&, SC4Percentage&); powered
    (power sim IsOccupantPowered 31), watered (plumbing IsBuildingWatered 9).
  - Also verified: SDK headers exist for every sim used (cISC4SimGrid template in SDK dir);
    `SC4Percentage` = struct{float percentage} (SC4Percentage.h:24); cISC4MySim SDK header
    compiled-order caveat is IRRELEVANT for the shadow (we only CALL slots ≤ 20 which match);
    cISC4AdvisorSystem::GetAdvisors returns std::map (avoid; use GetAdvisor(id) probes).
  - Next: (1) write engine/plugin/FC4MySimsShadow.{h,cpp} (decode-and-diff pattern,
    hosts above); (2) wire: FC4Config.cpp kOtherKeys end (~line 320, add `mysims_shadow`),
    cFC4Director.cpp PostAppInit/PreAppShutdown, FC4Observer.cpp PostCityInit + kMsgSimNewMonth
    + PreCityShutdown, FC4Subsystems.cpp after AutomataShadowStartup (line 637), engine/
    CMakeLists.txt plugin list after FC4AutomataShadow (line 447-448), tools/regress.ps1
    line 170 shadows string append, engine/README.md automata bullet area (additive);
    (3) qbuild quick then all; (4) game runs mysims1-sh-1 (shadows) + mysims1-sh-2 clean
    ref_warnings=0; (5) report + build-runs/merge-queue/mysims1.json (abi=false).
4c8d19c4 mysims1: handoff — design researched, baseline gate PASS
93c9ca3e mysims1: lane file, handoff at the baseline

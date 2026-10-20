# mysims1 — the mysims shadow (a same-process differential of the MySims)

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| mysims1 (the mysims shadow: FC4MySimsShadow + regress shadows list) | `mysims1` / `.claude/worktrees/mysims1` (branch `mysims1`, engine lane) | in progress — baseline gate PASS (`qbuild all`: cargo 3568/0, clippy 0, CMake 174/174, ctest 62/62); design researched round 2 (C++ ABI surface verified); no shadow code written yet; branch is 2 commits behind main (cores28 merged) | see Handoff |

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
- Next: (1) merge main; (2) write engine/plugin/FC4MySimsShadow.{h,cpp} per the design above
  (decode-and-diff, not byte round trip; host eq/grid callbacks to the game read-only);
  (3) wire the points above; (4) qbuild quick, fix, then `qbuild all`; (5) game runs
  mysims1-sh-1 (shadows incl. mysims) and mysims1-sh-2 clean ref_warnings=0; (6) report +
  build-runs/merge-queue/mysims1.json (abi=false).
4c8d19c4 mysims1: handoff — design researched, baseline gate PASS
93c9ca3e mysims1: lane file, handoff at the baseline

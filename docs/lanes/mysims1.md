# mysims1 — the mysims shadow (a same-process differential of the MySims)

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| mysims1 (the mysims shadow: FC4MySimsShadow + regress shadows list) | `mysims1` / `.claude/worktrees/mysims1` (branch `mysims1`, engine lane) | in progress — baseline gate PASS (`qbuild all`: cargo 3568/0, clippy 0, CMake 174/174, ctest 62/62); design researched, no shadow code written yet | see Handoff |

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
- Facts: main has cores28 NOT merged yet (train16 integration/16 pending gate/regress at
  71e4cf36); mysims core on main already covers agent+sim (pool/dispatch land with
  train16). Questions MS1-MS7 open (MS8-MS10 pending train16 merge); file MS11+ only if
  the shadow reveals a spec gap.
- Next: (1) write engine/plugin/FC4MySimsShadow.{h,cpp} per the design above (decode-and-
  diff, not byte round trip); (2) wire FC4Config/Observer/Director/CMake/regress.ps1;
  (3) qbuild quick then all; (4) game runs mysims1-sh-1/sh-2 via game-queue; (5) finish
  report + merge-queue/mysims1.json (abi=false).

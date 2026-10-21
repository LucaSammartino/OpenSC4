| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-advisors-final4: advisor replacement (enable=advisors), final round 3/3 | glue-advisors-final4 / .claude/worktrees/glue-advisors-final4 | wip: window slots 23-28 + AD4/AD5/AD6 compat entries + private global landed-ready (gate PASS 64/64); system NOT enabled, no game run | a new multi-round program for the system/advisor/advice/subject classes and the 64-callback host |

## Report (run4, final round 3/3, 2026-10-21)

**What changed** (branch glue-advisors-final4, base main b5253abf, code commit be5b96d0):
- `engine/plugin/compat/FC4Compat.{h,cpp}` (additive, at the end of the lists): `Direct` entries for the five
  win proc bodies from the AD4 answer (advisors.md §12.1.1 "Direct-call ABI": `0x41DB50` ctor, `0x41E320` Init,
  `0x41D650` show, `0x41D710` hide, `0x41D830` teardown; thiscall, ECX = block base, pop 0) and for the 11 original
  Lua extensions a replacement registers unchanged (§8.3.1 rows 1-9 per AD5, §8.3.2 camera_jump `0x40AF90` /
  camera_zoom `0x40B1F0` per AD6). New field `advisorSystemGlobalVA` = `0x00B207C0` (§14.2) and
  `compat::AdvisorSystemGlobal()` (aligned, committed, writable image data, else null, logged once).
- `engine/plugin/FC4AdvisorWindow.h`: `AdvisorWindowSlot`, the system member `+0x5C` and slots 23-28 bodies
  (Start: allocate 0x70, construct, AddRef, Init, no null test as in the original; Show/Hide only with a window;
  IsVisible = byte `+0x56`; Remove = teardown, null, Release) over injected ops; `AdvisorSystemGlobalSlot` (Publish
  our primary `cISC4AdvisorSystem*`, Withdraw = unconditional null, the literal §14.2 reading, commented).
- `engine/plugin/FC4AdvisorWindowGame.cpp`: `GameAdvisorWinProcOps` (global heap allocate + the five direct calls
  through `__fastcall`, AddRef/Release via the block base's cIGZUnknown slots 1/2: docs/abi/vtables.tsv
  `cSC4AdvisorUIAdvisorWinProc::vftable` 0x00A83548 slots 1/2 = 0x4BDF00 / 0x41D460).
- `engine/tests/advisor_window_test.cpp` + CMake `advisor_window_test` (additive, end of list).

**Not done (and why).** `enable = advisors` is still not registered: the replacement system needs cFC4AdvisorSystem
(29 slots + message target + serializable), the advisor hierarchy (21), advice (31), subject (14), and the
`fc4_advisor_host` of 64 callbacks (Lua tables / package loader / triggers / tags / detokenizer on the app's
cISCLua, the §7 variable sources over every §13 subsystem, cheats, messages, refs). That did not fit one 150-call /
150k round (context 86k at call ~30 after reading the needed spec sections). Nothing was registered as a no-op or
behind a disabled path; the new code is not called by any registered class yet, so plugin behaviour is unchanged.

**Measured.** Baseline `qbuild all`: cargo 3803 passed / 0 failed, clippy 0, build PASS, ctest 63/63. Tip:
cargo 3803/0, clippy 0, build PASS, ctest 64/64 (incl. advisor_window_test). Coverage unchanged (advisors:
specified yes, implemented yes, shadow -, verified -, replaced absent).
**Game:** no request: no advisor callback can run before the family is enabled, so a 2032 trace would show nothing.

**Questions:** none new (AD4-AD6 answers were sufficient for this part).

**Next (new program, several rounds):**
1. cFC4AdvisorSystem over `fc4_advisors_*` with slot 8 -> AdvisorGameMap, slots 23-28 -> AdvisorWindowSlot,
   Publish into AdvisorSystemGlobal in the constructor / Withdraw in the destructor; slots 20/22 from the core.
2. register_api: 36 core entries via `fc4_advisors_call_lua` trampolines + the 11 compat originals in collector order.
3. Advisor / advice / subject classes (maps from FC4AdvisorMaps.h), then the host's §7 readers.
4. Game run sim_turbo to 2032 (unique label), then normal speed, then candidate union.

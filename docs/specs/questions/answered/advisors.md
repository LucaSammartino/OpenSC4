# advisors: answered questions (archive)
<!-- Archive for [advisors.md](../../advisors.md). Lanes do not read this file. Open questions are in ../advisors.md. -->

<!-- from: glue-advisors4 (2026-10-21) -->
- **AD1 - section 12.1 slots 23-28 (with section 4.1 step 10 and section 4.6):** the table gives only names for
  `GetAdvisorWinProc`, `StartAdvisorWindow`, `ShowAdvisorWindow`, `HideAdvisorWindow`, `IsAdvisorWindowVisible` and
  `RemoveWinProcs`. A replacement `cSC4AdvisorSystem` must answer them for the original advisor UI. What does each
  body do: does `StartAdvisorWindow` create the `cSC4AdvisorUIAdvisorWinProc` (by which route: a class ID the class
  registry can create, a window resource, or non-GZCOM code a plugin cannot reach), which member holds it, what do
  Show / Hide / IsVisible call on it, and what does `RemoveWinProcs` release? If the win proc cannot be created through
  any interface, please say so, so a replacement answers "no window" (null / false / no-op) as the literal reading.
  [literal reading planned: null, no-op, false]
  **ANSWERED 2026-10-21 (re-advisor-contracts4):** DIFFERS — section 12.1.1: slot 24 builds the win proc with the game heap and its constructor 0x41DB50 (no CLSID, one caller), stores it in system +0x5C and runs its Init 0x41E320 (script window 0x96A006B0/0xCBC905CD); 23 returns +0x5C, 25/26 show/hide its panels and set byte +0x56, 27 returns +0x56, 28 tears it down (0x41D830) and releases it. View3D calls 25-27 through the city getter, so null/no-op/false is not the literal original.
- **AD2 - section 14.2, "Blocker: private global":** the 60 call sites of `0x00B207C0` are named as being in the
  advisor classes, the advice classes, the neighbour-deal advice and the Lua extensions. Does any code that stays
  original when the whole family is replaced read that global: the advisor UI win procs, the popup tracker
  (section 6.8.1), the city-situation manager, MySim, the tutorial registry, the View3D vehicle menu or the HTML link
  handler? If yes, which ones and through which members (they would then need the shim's [choice] pointer write, and
  that pointer would have to be a layout-compatible object, not just any `cISC4AdvisorSystem`).
  **ANSWERED 2026-10-21 (re-advisor-contracts4):** MATCHES (no outside reader) — section 14.2: byte scan finds 66 accesses of 0x00B207C0, 3 writes (ctor 0x41A9B5, two destructors store null) and 63 reads, all in system/advisor/advice classes (incl. city-situation, MySim balloon, neighbour-deal, reward advices) and the Lua extensions; none in the UI win procs, popup tracker, city-situation manager, MySim, tutorial, View3D or HTML handler. Reads call interface slots only.
- **AD3 - section 12.1 slots 8 and 12.2 slots 10-11:** `GetAdvisors` and `GetCurrentAdvices` return references to the
  original MSVC `std::map`s. Which consumers read them and how (iterate only, insert, erase): the advisor UI, the
  city-situation manager, MySim? A replacement must lay out compatible containers (the plugin has a game-layout map
  helper); knowing whether any caller mutates them decides whether the core's state must be mirrored into that map
  once per change or kept only there.
  **ANSWERED 2026-10-21 (re-advisor-contracts4):** MATCHES (read-only) — section 12.2: consumers are cSC4MySimAgentSimulator::Init 0x42EC30 (iterates slot 8), the advisor win proc GetMessageList 0x41E000 / get_advisor_mood 0x41DC20 and cSC4MySimWinProc::GetMessageList 0x76F260 (copy slot 10/11 with the game tree helpers); no outside insert/erase; layout {header*, count}, node {color, parent, left, right, key +0x10, value +0x14}.
<!-- from: glue-advisors-impl4 (2026-10-21) -->
- **AD4 - section 12.1.1 (window route as direct calls):** a replacement system can keep the original window only by
  calling `0x41DB50`, `0x41E320`, `0x41D650`, `0x41D710` and `0x41D830` itself (FC4Compat, docs/abi/direct-calls.md
  format), but the spec gives only their addresses and bodies. For each, what are the calling convention, the stack
  arguments (count, types, any `this`-adjusted sub-object pointer), the bytes the callee pops and the return channel
  (AL / EAX / none)? In particular: (a) does the constructor `0x41DB50` take arguments besides `this` (the 0x70 block
  from `0x5E55E0`), and what reference count does it leave (0 or 1) before slot 24's AddRef? (b) is the pointer
  stored at system `+0x5C` the block base or an interface sub-object, and which pointer do Init / Show / Hide /
  teardown expect in ECX? (c) does `0x41E320` return a value slot 24 ignores, and does the win proc (Init or later)
  keep a reference to the system or the city that the teardown releases, so the replacement's Shutdown order matters?
  (d) does anything in the win proc read the system, advisor or advice objects other than through `cISC4AdvisorSystem`
  slot 7, `cISC4Advisor` slots 10/11 and interface slots of the listed advices (which ones)? [no direct call is made:
  the replacement system is not enabled until this is documented; null / no-op is ruled out by AD1]
  **ANSWERED 2026-10-21 (re-advisor-ui4):** MATCHES — section 12.1.1 "Direct-call ABI": all five are thiscall, ECX = block base (the 0x70 allocation, also what system +0x5C stores), no stack args, RET 0; ctor returns this in EAX with refcount 0 (slot 24 AddRef makes 1; Release 0x41D460 deletes at count <= 1); Init/show/hide return nothing, teardown returns its tail Release result; win proc holds no system/city ref, holds an advisor ref at +0x0C and message subscriptions until teardown; full slot list of advisor/advice calls given.
<!-- from: glue-advisors-impl4 (2026-10-21) -->
- **AD5 - sections 8.2 / 8.3 (the 9 UI Lua entries under a replacement system):** a replacement's `register_api`
  host callback must register all 47 entries of the two collectors, but §8.3 gives the UI group (`camera_jump`,
  `camera_zoom`, `tool_plop_*`, `tool_button`, `window_*`, `window_query`) only as "UI only". (a) What are the exact 9
  names, which are global and which go into `game`, and their arguments / results / behaviour (enough to reimplement
  them through public interfaces)? (b) Or, if the shim is to register the original C functions instead: their
  addresses, that each is a plain `lua_CFunction` (cdecl, one `lua_State*`, int result), and whether any of them
  reads the private global `0x00B207C0` or a system / advisor / advice member (so it would break with the original
  system absent). (c) Does anything other than the system's `register_api` (e.g. the app, the tutorial, MySim)
  register any of the 47 entries into the app's Lua context? [the replacement is not enabled until known; no entry
  is guessed or registered as a no-op]
  **ANSWERED 2026-10-21 (re-advisor-ui4):** DIFFERS — section 8.3.1: the 9 UI entries are tool_plop_network/zone/building/flora, tool_button, window_budget/map/graph, window_query (all in game, none global; camera_jump/zoom are simulation-collector entries, new AD6); each is a cdecl lua_CFunction returning 0; rows 5-9 call system slot 20 through 0x00B207C0 (row 9 also slot 22), rows 1-4 never; only register_api 0x418940 reads the shared collector vector 0x00B207CC.
<!-- from: re-advisor-ui4 (2026-10-21) -->
- **AD6 - section 8.3 (camera_jump / camera_zoom):** these two are simulation-collector entries (`0x40AF90`,
  `0x40B1F0`), not UI ones, and §8.3 gives no behaviour. What are their arguments, results, the 3D-view / camera
  calls they make and whether they read `0x00B207C0`? [found while answering AD5; a replacement registering all
  47 entries needs them]
  **ANSWERED 2026-10-21 (re-advisor-ui4):** DIFFERS — section 8.3.2: both are simulation-collector records 5 and 6 (game table, cdecl lua_CFunction, return 0, read index -1 only); both call 0x00B207C0 slot 20 (discarded, no null test). camera_jump(subject userdata, tag checked by slot 22) -> location: (CellWidth*x, GetAltitude, CellWidth*z), else object as occupant 0x26BB767E GetPosition -> camera control (renderer 0x00B43DD0 slot 16) 0x7CD810; camera_zoom(number) -> SetZoom 0x7CDC50 with f32(ToNumber), no range check; failures show a gAdviceMessageBox error box. Original addresses need a system with slots 20/22 in 0x00B207C0.
<!-- from: glue-advisors-final4 (2026-10-21) -->
- **AD7 - section 8.2:** in which order are the 38 simulation and 9 UI collector entries in the registration vector (which collector first, and the order inside each)? It decides the final `_tf_` global. [simulation collector first, §8.3 table order]
  **ANSWERED 2026-10-21 (re-advisor-api4):** DIFFERS — simulation collector first (CRT table 0x00B07030 before 0x00B07034), each in constructor order; the UI order ends window_query, window_graph, not the 8.3 order; final _tf_ is print after register_api(true) and window_graph after register_api(false). Table in advisors.md 8.2.
<!-- from: glue-advisors-final4 (2026-10-21) -->
- **AD8 - section 12.1 slots 9 and 11:** who calls `GetRandom()` and which `cRZRandom` methods/layout must the returned object provide; and what does system slot 11 `DoLink(const cIGZString&)` do (body `0x41B080`)? [slot 9 returns null, slot 11 returns false]
  **ANSWERED 2026-10-21 (re-advisor-api4):** DIFFERS — slot 9 is called only by game.random_chance (0x0040A810), which runs the non-virtual cRZRandom::RandomDoubleRangeUniform on the returned pointer and reads/writes the u32 state at +4: null faults. Slot 11 parses "%08x%08x" (false if not 2 fields), forwards to the advice DoLink (slot 22) or runs the trailing text as Lua, and returns true. advisors.md 12.1.
<!-- from: glue-advisors-final4 (2026-10-21) -->
- **AD9 - section 5.2:** where does a cheat notification (`cIGZMessageTarget::DoMessage(cGZMessage&)`) carry the typed cheat text (`Adv reload` etc.): which `cGZMessage` field and what type? [only the cheat ID is passed]
  **ANSWERED 2026-10-21 (re-advisor-api4):** DIFFERS — the text is message-2 data2 as a borrowed cIGZString* (GetVoid2 +0x3C, set by cGZCheatCodeManager::SendCheatNotifications SetVoid2), the whole typed line; data1 is the cheat ID. advisors.md 5.2.
<!-- from: glue-advisors-final4 (2026-10-21) -->
- **AD10 - section 4.2 / lua-host.md 7.3-7.5:** may a replacement call `UpdateResourcesFromDirectory` `0x005FC510`, `LoadPackageScripts` `0x005FBA20` and register `_dofile_from_resource` `0x005FC020` as they are (calling convention, argument types, who pops), like the AD5/AD6 Lua entries? Also: which directories are `<data directory>` and `<plugin directory>` (which `cISC4App` getters)? [not called yet; reload_scripts left unset]
  **ANSWERED 2026-10-21 (re-advisor-api4):** DIFFERS — all three may be called as they are: cdecl, caller pops (8, 12 bytes), _dofile_from_resource is a lua_CFunction on the game cSCLua state. The directories are cISC4App +0xD4 GetPluginDirectory(out,-1) then +0xDC GetSkuSpecificDirectory(out), not the data directory. advisors.md 4.2, lua-host.md 7.3.

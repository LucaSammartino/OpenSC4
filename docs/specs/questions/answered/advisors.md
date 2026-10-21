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

# advisors: open questions
<!-- Questions about [advisors.md](../advisors.md). Append new entries at the END; when one is answered, move it to answered/advisors.md. Rules: README.md. -->

<!-- from: glue-advisors4 (2026-10-21) -->
- **AD1 - section 12.1 slots 23-28 (with section 4.1 step 10 and section 4.6):** the table gives only names for
  `GetAdvisorWinProc`, `StartAdvisorWindow`, `ShowAdvisorWindow`, `HideAdvisorWindow`, `IsAdvisorWindowVisible` and
  `RemoveWinProcs`. A replacement `cSC4AdvisorSystem` must answer them for the original advisor UI. What does each
  body do: does `StartAdvisorWindow` create the `cSC4AdvisorUIAdvisorWinProc` (by which route: a class ID the class
  registry can create, a window resource, or non-GZCOM code a plugin cannot reach), which member holds it, what do
  Show / Hide / IsVisible call on it, and what does `RemoveWinProcs` release? If the win proc cannot be created through
  any interface, please say so, so a replacement answers "no window" (null / false / no-op) as the literal reading.
  [literal reading planned: null, no-op, false]
- **AD2 - section 14.2, "Blocker: private global":** the 60 call sites of `0x00B207C0` are named as being in the
  advisor classes, the advice classes, the neighbour-deal advice and the Lua extensions. Does any code that stays
  original when the whole family is replaced read that global: the advisor UI win procs, the popup tracker
  (section 6.8.1), the city-situation manager, MySim, the tutorial registry, the View3D vehicle menu or the HTML link
  handler? If yes, which ones and through which members (they would then need the shim's [choice] pointer write, and
  that pointer would have to be a layout-compatible object, not just any `cISC4AdvisorSystem`).
- **AD3 - section 12.1 slots 8 and 12.2 slots 10-11:** `GetAdvisors` and `GetCurrentAdvices` return references to the
  original MSVC `std::map`s. Which consumers read them and how (iterate only, insert, erase): the advisor UI, the
  city-situation manager, MySim? A replacement must lay out compatible containers (the plugin has a game-layout map
  helper); knowing whether any caller mutates them decides whether the core's state must be mirrored into that map
  once per change or kept only there.

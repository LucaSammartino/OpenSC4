| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-application4: application framework spec, cSC4App lifecycle/services/messages | `re-application4` / (RE spec lane) | ready: docs/specs/application.md (bf01cf8c), questions AP1-AP5 filed; partial owner spec (about 35 of 322 cSC4App functions); no ABI, no game runs | 1. coordinator: add the coverage-map spec cell for cSC4App (proposed below) if partial owner specs count. 2. next RE round: city request/load/save path (section 11) and AP1 command-line effects |

## Report

What changed:
- New spec `docs/specs/application.md` for `cSC4App` (the largest application owner by bytes).
  It covers object layout (7 vtables), the 15-slot primary vtable, the 71-slot `cISC4App` interface
  (getters, field offsets), startup order (PreFrameWorkInit, AddCOMDirectors, AddSystemServices service
  list, Init steps and its two failure returns, PostFrameWorkInit), Shutdown and Quit, both DoMessage
  tables (v1: quit request, plugin-dir change, `GP`/`CreateException`/`ForceIdle` cheats; v2: app mode
  0/1/2, preferences, unilateral quit, idle, mouse), PostSC4AppMessage send/post rule, OnIdle timers, focus
  pause, command-line option names, startup resolution rules with worked examples, and the user
  directory layout.
- Questions `docs/specs/questions/application.md`: AP1 (command-line option effects), AP2 (OnIdle
  300 ms gate and tail), AP3 (misnamed directory slots), AP4 (+0x08 face, primary slots 4/5),
  AP5 (approximate items in Init/Shutdown/Quit). QUESTIONS.md index regenerated.

Measured (tools/coverage.py --owners, this tree): application subsystem owners by bytes:
cSC4App 86902 B / 322 functions; cSC4AutomataCOMDirector 41602; cGZToolsCOMDirector 31303;
cSC4GameDataRegistry 5807; cSC4ResourceLibCOMDirector 5243; cSC4COMDirector 4679; cGZResManCOMDirector 4093;
cWinSC4App 2224; others under 1.5 KB. Of cSC4App's bytes 25675 are reach-assigned (non-cSC4App code).

Proposed coverage-map row (coordinator decides; the spec says partial, section 11 lists the gaps):
`cSC4App	application	framework	application.md	-	-	-`. Not claimed: cWinSC4App, the COM-director
wildcards, cSC4Preferences, cSC4GameDataRegistry, cSC4AutoGamePause.

Sources: REA was unavailable this round ("No app is open"); everything comes from re_query.py asm /
callees / xrefs / vtable plus a PE string reader (build-runs/cstr.py, not committed), decomp/ only for
IsStartupResolutionOK and FixStartupResolutionValuesIfNeeded (constants checked against the
decomp's comparisons). No ABI rows proposed.

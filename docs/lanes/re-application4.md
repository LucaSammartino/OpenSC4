| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-application4: application framework spec, cSC4App lifecycle/services/messages/city path | `re-application4` / (RE spec lane) | ready (round 2): docs/specs/application.md sections 7.1, 8, 11 extended; AP2 answered, AP1 narrowed; partial owner spec (~52 of 322 cSC4App functions); no ABI, no game runs | 1. next RE round: SaveCity(path) 0x00455E20, RequestSaveCity 0x00455AE0, RequestQuit 0x004546B0, RequestQuitFromRegion 0x00454800, DoMessage 0x00455060. 2. AP1 remainder (ExceptionHandling, l, UserDir, CPUPriority, AutoZoomChange) |

## Handoff (round 3, 2026-10-21)
Goal: final round: SaveCity(path,flag) 0x00455E20, RequestSaveCity, RequestQuit, RequestQuitFromRegion, DoMessage; AP1 remainder.
Done: c6228104 = sections 11.12-11.15 (RequestSaveCity, RequestQuit, RequestQuitFromRegion, 0x00455060 is the
load-height-field dialog, not a message handler). REA still "No app is open"; all from re_query asm.
Done: commit 2 = 11.16 SaveCity(path,flag) (temp file, backup `_`, rename), load messages named in 11.9.
Next: AP1 options in ReadCommandLine 0x0044EED0.

## Report (round 2, 2026-10-21)

What changed (docs/specs/application.md):
- Section 11 (new): city lifecycle. 11.1 RequestNewCity, 11.2 RequestCloseCity (confirm dialog, Alt-key save),
  11.3 RequestGoToRegionView (two scripted dialogs, result IDs), 11.4 CloseCity/PartialCloseCity (PreCityShutdown
  sent, PostCityShutdown queued, region reload of overlapping/touching cities with worked example), 11.5 city DB
  segment register/unregister, 11.6 DoCityObliteration (save to temp, close, delete, NewCity from template segment
  +0x5B8, save back, restore zoom/rotation), 11.7 city-init message sequence, 11.8 NewCity, 11.9 LoadCity (version
  62..65, load stream key, Read/Write record), 11.10 RequestLoadCity/DoLoadCityUI (stub: fixed `SavedCity` name),
  11.11 SaveCity(flag). Section 12 lists what stays open.
- Section 7.1 OnIdle fully traced (AP2 answered, DIFFERS: idle timer is set to threshold - +0x3AC, and the
  300 ms branch is always taken, so GameInIdleState(60) is queued about once per second).
- Section 8: per-option effects table for IME, r, f, w, g, d, audio/sound, radio/music, sim, RDD, CPUCount.
- Questions: AP2 moved to answered/application.md; AP1 narrowed to the untraced options; QUESTIONS.md regenerated.

Commits: acdec563 (11.1-11.6), f2e74553 (11.7-11.11), b9344ada (AP2), this report commit (section 8, AP1).

Sources: REA unavailable ("No app is open"); decomp/ fails on all city-path bodies (struct error). Everything is
from re_query.py asm/callers/vtable, ghidra/export/windows/instructions.asm (scan for +0x390 timer accesses), and
build-runs/cstr.py for short strings. Helper scripts (not committed): build-runs/asmf.py, build-runs/mvq.py.

Coverage: partial owner spec, about 52 of cSC4App's 322 functions; not complete. Not claimed: cWinSC4App,
COM directors, cSC4Preferences, cSC4GameDataRegistry, cSC4AutoGamePause. Proposed coverage-map row unchanged:
`cSC4App	application	framework	application.md	-	-	-` (coordinator decides). No ABI rows.

Open: SaveCity(path, flag) 0x00455E20 (2611 B), RequestSaveCity 0x00455AE0, RequestQuit 0x004546B0,
RequestQuitFromRegion 0x00454800, DoMessage 0x00455060, city vtable slot names (+0x1A4, +0x1B4, +0x5C), the
messages 0x26C63340-42 / 0x2B96B3EA / 0xAB96B05F names, AP1 remainder, AP3-AP5.

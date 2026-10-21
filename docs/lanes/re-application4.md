| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-application4: application framework spec, cSC4App lifecycle/services/messages/city path | `re-application4` / (RE spec lane) | ready (round 3, final): application.md 11.12-11.16 (RequestSaveCity, RequestQuit, RequestQuitFromRegion, height-field dialog, SaveCity(path,flag)), section 8 complete; AP1 answered; partial owner spec (~57 of 322 cSC4App functions); no ABI, no game runs | 1. AP3-AP5. 2. city vtable slot names (+0x1A4, +0x3C), SC4CreateSavePathForCity. 3. cWinSC4App, COM directors, cSC4Preferences, cSC4GameDataRegistry, cSC4AutoGamePause unspecified |

## Report (round 3, final, 2026-10-21)

What changed (docs/specs/application.md):
- 11.12 RequestSaveCity(notify, flag): blocked-city dialog, empty path, success dialog only when notify,
  failure dialog always, `text (file name)` suffix. 11.13 RequestQuit(ask, saveWhenNotAsking): dialogs,
  save-and-quit, UnilateralQuit queued to server 2 slot +0x1C (delivered later). 11.14 RequestQuitFromRegion.
- 11.15: `0x00455060` (export name DoMessage) is the load-height-field file picker (INI key
  `Settings/Terrain Load/Save Location`, centred modal dialog); its caller `0x004559E0` (+0x08 face slot
  +0x0C) copies the pick (<= 511 chars) into NewCitySpecification. Partial AP4 note added.
- 11.16 SaveCity(path, flag): app-side file handling around foundations.md 3.7 (temp file, `_` backup,
  rename/restore, old file removed on rename, city path set/restored, segment unregister/register,
  adjacent regional records). Two items marked [inferred].
- Section 8: every ReadCommandLine option traced; AP1 answered (DIFFERS) and archived; QUESTIONS.md regenerated.
- 11.9 load messages named PreLoad/Load/PostLoad (re/class_id_table.tsv); slot table and section 12 updated.

Commits: c6228104 (11.12-11.15), b7e1560a (11.16), 16aabf85 (AP1), this report commit.
Sources: REA "No app is open" again; all from re_query.py asm (helpers build-runs/asmf.py, cstr.py, mvq.py; dumps
a-rsave.txt, a-save.txt, a-cmd.txt, not committed).

Coverage, honestly: about 57 of cSC4App's 322 functions; not complete. Gaps: AP3-AP5, SetupWinCtrlMgr/FontSet/
Resources, AddCodePlugins, AddDynamicLibrary*, preferences read/save bodies, DB segment callbacks, LoadRegistry,
SC4CreateSavePathForCity rule, city slots +0x1A4/+0x3C/+0x30/+0x14, message 0x26C63346 name. Unspecified owner
classes: cWinSC4App, COM directors, cSC4Preferences, cSC4GameDataRegistry, cSC4AutoGamePause.
No ABI rows; no game runs. Proposed coverage-map row unchanged (`cSC4App	application	framework	application.md	-	-	-`).

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

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-app: the application's preferences, plugins, resources, message pumps and start-up details, folded into application.md (RE spec lane) | `re-batch5` branch (outside contributor checkout, fork amarret/OpenSC4; on main `f01d1ce4`, after re-modelmaker) | **ready** (docs and RE tables only). application.md §13 (preferences: the file, the save-time quirk, the fields, re-applying), §14 (code plugins, resources in summary, files) and §15 (the message pumps) added; the entry, the `Restart` wait, Init's driver fallback and v2 subscriptions, the service order with IDs, Shutdown's unregistrations, the `-d` and `CPUCount` rows corrected or completed; AP5 narrowed, AP10–AP11 filed; 101 names confirmed. `cSC4App` stays a partial owner (no coverage change) | AP5, AP6, AP7; the rest of §12 |

## Report

Static RE only: the instruction bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`), the
Ghidra export and the SDK headers. Written on 2026-10-21 as a separate `application.md` and held back when `main`
gained its own application.md (re-application4, three rounds; the city life cycle there is more detailed than the
held lane's); this fold adds what upstream's §12 listed as open and corrects what the bytes contradict.

### What changed (docs/specs/application.md)

- §13 (new): `SimCity 4.cfg` (one record, version 0x15, the 0x1734-byte block), `ReadPreferences`, `SavePreferences`
  with its quirk (the live language name is cleared and IME forced on before writing, `0x0044BDEE`–`0x0044BEBA`),
  the fields the app uses, `DoPreferencesChanged` and the video preferences in summary.
- §14 (new): `AddCodePlugins` and its `[Admin] Enabled` test (`false` or exactly `0`, `0x00451740`–`0x004518CB`),
  the two `AddDynamicLibrary` slots, `LoadRegistry` / `SetupResources` / fonts / cursors in summary, the ini file.
- §15 (new): `RunMessageServerPump` / `RunMessageServer2Pump` (`0x004499F0`, `0x00449A70`).
- §3: the entry (`SC4AppStart`, summary); the `Restart` wait (polls the instance object's slot 3, 200,000 between
  polls, up to 30 units of a unit-5 timer; `0x0045265D`–`0x004527AC`).
- §3.1: Init's hardware-driver fallback (`0x0044AC30`: the "Could not initialize the hardware driver" box, `+0xF13`
  and `+0xF0C`) and the nine message-server-2 subscriptions after `0xC7AA61C6` (`0x0044C679`–`0x0044C74F`).
- §4.2: the service order re-read with its class IDs and constructors; the earlier text had one unnamed service where
  the bytes have three before the font system and one after it (`0x20673960` is the font system).
- §5.1: Shutdown unregisters the three cheats (`+0x318`), the nine v2 messages and then `0xC7AA61C6` on message
  server 1 (`0x0044A5E1`–`0x0044A843`).
- §8, corrected: `-d` is a substring search for `opengl`, `soft`, `dx`, `directx` in that order, with no `gl` test
  (`0x0044F46D`–`0x0044F56B`, strings at `0x00A86E54`); `CPUCount` sets the process affinity to the n lowest
  processors (`0x005F8CD0`).
- §2, corrected: iface slots 19 and 20 are the app's own video-preference bodies (vtable words `0x00A86B3C`,
  `0x00A86B40`), not folded getters.
- §12 updated for what is now covered.
- Kept upstream where it is fuller: the city life cycle (§11), `OnIdle` (§7.1), the record (§11.9), the command-line
  rows other than `d` and `CPUCount`. The held lane's offsets from the `+0x2C` and `+0x10` faces were converted to
  object offsets (its `+0x61C` instance object is object `+0x648`, its `+0x308` cheat manager `+0x318`).

### Names and coverage

- re/symbols/application.tsv: 101 names, all confirming the export's names (two of the held lane's rows,
  `GetTutorialRegistry` and `GetLuaInterpreter`, are left to lua-host.tsv, which already names them). The
  `names_windows.tsv` delta (−101 +101) is identical to `merge_names.py`'s.
- No coverage row: `cSC4App` (322 functions) stays a partial owner spec, as upstream rules.

### Measured

`tools/coverage.py` on the local (Steam) export, measurement only: 3,478,182 bytes in 15,345 functions before and after
(no owner claimed). `docs/coverage-report.md` was not regenerated (the tool accepts only the GOG exe).

### Questions

- Narrowed: AP5 (the `Restart` wait and Shutdown's `RZ::MS` call are now specified; Init step 4 and Quit's
  `RZ::GS` call stay).
- Filed: AP10 (the unnamed system services), AP11 (`-sim`, `-RDD`, `-AutoZoomChange` targets).
- The held lane's AP1–AP3 used numbers upstream had already taken; their content is AP11, AP10, and (its tick value and
  `CloseCity`'s regional work) upstream's §7.1 and §11.4.

### Not covered

The rest of §12: the setup bodies beyond summary, the DB segment callbacks, `cWinSC4App`, `cSC4Preferences` as a
class, `cSC4GameDataRegistry`, `cSC4AutoGamePause`.

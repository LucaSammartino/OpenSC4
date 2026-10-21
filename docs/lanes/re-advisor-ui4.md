| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-advisor-ui4: advisors AD4 (window route direct-call ABI) and AD5 (9 UI Lua entries) | `re-advisor-ui4` / (RE lane) | ready, docs only, no ABI change; AD4 MATCHES, AD5 DIFFERS answered and archived; AD6 filed | 1. glue lane: use advisors.md §12.1.1 "Direct-call ABI" and §8.3.1 for the replacement's window route and UI Lua entries. 2. RE lane: answer AD6 (camera_jump / camera_zoom, 0x40AF90 / 0x40B1F0). |

## Report

**What changed (docs/specs only).**
- advisors.md §12.1.1: new "Direct-call ABI of the five bodies" table and answers (a)-(d) for `0x41DB50`, `0x41E320`,
  `0x41D650`, `0x41D710`, `0x41D830`: thiscall, ECX = block base (= system `+0x5C`), no stack args, RET 0; ctor returns
  this, refcount 0; slot 24 AddRef makes 1; Release `0x41D460` deletes at count <= 1; Init/show/hide return nothing;
  teardown returns its tail Release. The win proc holds no system/city ref (city `0x00B43CEC` +0x144 each use), holds
  an advisor ref at `+0x0C`, advices in its `+0x60` list, and 5 message subscriptions (server `0x00B43CCC`). Full list of
  advisor/advice slots it calls.
- advisors.md §8.3: the UI row now names the real 9 entries; `camera_jump`/`camera_zoom` are simulation-collector
  entries (correction). New §8.3.1: names, addresses, IDs, ABI (cdecl lua_CFunction, return 0), behaviour (messages to
  the 3D view `0x00B43DC4` via AsIGZWin/DoMessage, tool_button via QI `0xC6AE7085` slot 3, window_query via subject ->
  QI `0x26BB767E` -> DoQueryDialog `0x4D3C30`), which read `0x00B207C0` (rows 5-9: slot 20 result discarded; row 9 also
  slot 22), and that only `register_api` `0x418940` reads the shared collector vector `0x00B207CC` (byte scan).
- advisors.md §14.2: the global's reader list names the UI helpers.
- questions: AD4, AD5 moved to answered/advisors.md; AD6 filed (camera entries); QUESTIONS.md regenerated.

**Evidence.** re_query.py asm (export) and decomp/ only; REA returned "No app is open" (target unavailable), so no REA
pseudocode was used. Byte scan script: build-runs/scan.py (not committed; build-runs is scratch).

**Open.** AD6. The `+0x60` list holding advice references and its release by the destructor are marked *[inferred]*.

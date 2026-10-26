# Lua `game.*` extension functions and `SCLuaTableRef`: owner closure (behaviour spec)

Scope: the `LUAExtension_*` functions that tools/coverage.py assigns as separate `fn:` owners, and the
`SCLuaTableRef` table-iteration helper. advisors.md §8.2–§8.3 already gives the registration order, the
one-line behaviour of every `game.*` function and the full bodies of the camera, tool and window entries; this spec
adds the bodies that spec only summarises (`advice_info`, `ordinance_name`, `ordinance_description`,
`trigger_event`, `unlock_reward_building` and its two record helpers) and `SCLuaTableRef`, and maps each owner to
its section, so the owners are complete.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Each body below was read in the disassembly and its string literals in the PE.
Written 2026-10-25 (re-luaext8).

## 1. Owners and where they are specified

| Owner | Body | Lua name | Specified in |
|---|---|---|---|
| `fn:LUAExtension_combine_guids` | `0x0040AD10` | `_combine_guids` (global) | advisors.md §8.3 (globals), lua-host.md §8.3; §2 below |
| `fn:LUAExtension_advice_info` | `0x0040AE50` | `game.advice_info` | §3 below |
| `fn:LUAExtension_camera_jump` | `0x0040AF90` | `game.camera_jump` | advisors.md §8.3.2 |
| `fn:LUAExtension_camera_zoom` | `0x0040B1F0` | `game.camera_zoom` | advisors.md §8.3.2 |
| `fn:LUAExtension_ordinance_description` | `0x0040B330` | `game.ordinance_description` | §4 below |
| `fn:LUAExtension_ordinance_name` | `0x0040B470` | `game.ordinance_name` | §4 below |
| `fn:LUAExtension_trigger_event` | `0x0040B5B0` | `game.trigger_event` | §5 below |
| `fn:LUAExtension_unlock_reward_building` | `0x0040B760`; helpers `0x0040B6E0`, `0x0040B720` | `game.unlock_reward_building` | §6 below |
| `fn:LUAExtension_tool_plop_network`, `_zone`, `_building`, `_flora` | `0x0040C720`, `0x0040C740`, `0x0040C760`, `0x0040C780` | `game.tool_plop_*` | advisors.md §8.3.1 rows 1–4 |
| `fn:LUAExtension_window_map`, `_budget`, `_graph` | `0x0040C850`, `0x0040C870`, `0x0040C890` | `game.window_*` | advisors.md §8.3.1 rows 6–8 |
| `SCLuaTableRef` | `0x005FB9C0`, `0x005FCA70`, `0x005FB730`, `0x005FB770` | — | §7 below |

The seven tool and window owners are 18-byte stubs: load the `lua_State*` argument into EAX, push the item or window
ID of advisors.md §8.3.1, call the shared helper (`0x0040C680` for the tools, `0x0040C7A0` for the windows), pop the
ID and return the helper's EAX (0).

## 2. Common shape *[compat]*

Every function here is a Lua 5.0 `lua_CFunction` (cdecl, one stack argument `lua_State*`, caller pops, the result
count in EAX). Each takes the host from the 4 bytes before the state, QIs it for `cISCLua` `0xEA10C4A2` into a holder
without testing the result, does all Lua access through `cISCLua` slots (lua-host.md §1.3, §5) and releases the host
(slot 2) on every exit. "The system" is the advisor system pointer in `0x00B207C0` (advisors.md §12.1), never
null-tested. Numbers are read as `ToNumber(-1)` (slot 23) truncated toward zero by `_ftol2`.

The error box is the one of advisors.md §8.3.2 step 7: two `cRZString`s built from the literals' begin and end
pointers (`0x00407FA0`) and `gAdviceMessageBox(caption, text)` (`0x004179A0`), not gated on a debug setting.

`_combine_guids(a, b)` (`0x0040AD10`): with `GetTop() > 1`, `IsNumber(-1)` and `IsNumber(-2)`:
`PushNumber(double(gCombineGUIDs(trunc(top), trunc(second))))` (`0x004177B0`, XOR, so the order does not matter;
the int32 is converted signed by `FILD`); otherwise `PushNumber(−1.0)` (the double at `0x00A80AD0`). Returns 1.

## 3. `advice_info()` (`0x0040AE50`, 313 bytes) *[compat]*

A debug aid that shows which advice is the context advice. It reads no argument.

1. `ctx := system.GetContextAdvice()` (slot 20).
2. `text :=` "No context advice available...\n" (`0x00A80B10`).
3. If `ctx` is non-null: `title := ctx.GetTextTitle()` (advice slot 18), `advisor :=
   system.GetAdvisor(ctx.GetAdvisorID())` (system slot 7, advice slot 11), `name := advisor.GetName()` (advisor
   slot 6), then `text.Sprintf("\tAdvisor : %s\n\tAdvice title : %s\n ", name.ToChar(), title.ToChar())`
   (`0x00A80AEC`, `cRZString::Sprintf` `0x0090F574`; `ToChar` is string slot 5). Neither the advisor, its name nor
   the title is null-tested: an advice whose advisor ID is not registered crashes *[quirk]*.
4. Show the error box with caption "game.advice_info()" (`0x00A80AD8`) and `text`, free both strings, release the
   host, return **0** (nothing pushed).

## 4. `ordinance_name(id)` and `ordinance_description(id)` (`0x0040B470`, `0x0040B330`, 318 bytes each) *[compat]*

The two bodies are the same code except for the placeholder format and the ordinance getter:

| Function | Placeholder format | Getter |
|---|---|---|
| `ordinance_name` | "Ordinance Name #%08x" (`0x00A80BF4`) | `GetName()` (ordinance slot 7, `+0x1C`) |
| `ordinance_description` | "Ordinance Description #%08x" (`0x00A80BD4`) | `GetDescription()` (ordinance slot 8, `+0x20`) |

1. `system.GetContextAdvice()` (slot 20), result discarded.
2. Require, in order: the ordinance simulator `0x00B43D90` non-null; `GetTop() > 0`; `IsNumber(-1)`;
   `ord := simulator.GetOrdinanceByID(id = trunc(ToNumber(-1)))` (simulator slot 20, `+0x50`; SDK order,
   ordinance.md) non-null. Any failure: `PushNil()` (slot 30) and go to 5.
3. Make the result string: a heap `cRZString` (`GlobalHeapAlloc(0x14)`), copy-constructed (`0x0040AE00`) from a
   temporary string built from "" (`0x00A80BF0`) and formatted by its slot 27 `Sprintf(placeholder, id)`; the
   temporary is then destroyed. The new string is AddRef'd once (slot 1) and then **assigned** from the getter's
   string (string slot 14), so the placeholder text never reaches Lua *[quirk]*: it only fills the string until the
   copy. An allocation failure leaves a null string that is used anyway (crash).
4. `PushUserData(string, system.GetDataTag(1))` (`cISCLua` slot 69 with the string tag, advisors.md §8.1): a tag-1
   string userdata holding the one reference, released by the shared gc method.
5. Release the host and return **1** (one value: the string userdata or `nil`).

So `game.ordinance_name(id)` gives the name text of that ordinance, an unknown ID gives `nil`, and a call without a
city (no ordinance simulator) gives `nil`.

## 5. `trigger_event(guid)` (`0x0040B5B0`, 298 bytes) *[compat]*

1. Load the system pointer (it does **not** call `GetContextAdvice`, unlike the other advice functions).
2. If `GetTop() > 0` and `IsNumber(-1)`: `system.PushEvent(trunc(ToNumber(-1)), null)` (system slot 15, `+0x3C`;
   result ignored), release the host, return 0.
3. Otherwise the error box: text "'eventGUID' was not specified or was not a valid number." (`0x00A80C2C`), caption
   "LUAExtension_trigger_event()" (`0x00A80C0C`, without the `game.` prefix the camera boxes use); return 0.

## 6. `unlock_reward_building(guid)` (`0x0040B760`, 362 bytes) *[compat]*

The local record is a `ConditionalBuildingStatus` (civic-buildings.md §2: `{u8 status; u16 buildingCount;
cRZString itemToolTip}`, the string at `+4`).

1. If `GetTop() ≤ 0` or not `IsNumber(-1)`: the error box with text "'buildingGUID' was not specified or was not a
   valid number." (`0x00A80C90`) and caption "LUAExtension_unlock_reward_building()" (`0x00A80C68`); return 0.
2. `guid := trunc(ToNumber(-1))`. If the civic building simulator `0x00B43DB4` is null, return 0 with no effect (no
   box).
3. Construct the record (`0x0040B6E0`: an empty string at `+4`, its 8-byte buffer allocated and terminated; the
   status byte and count are **not** initialised).
4. `st := civic.GetConditionalBuildingStatus(guid)` (slot 6). If non-null, copy it into the record (`0x0040B720`:
   status byte, count word, then the text assigned from the source string's character range, skipped when source
   and destination are the same object); else `count := 0`.
5. `status := 2` (the whole byte: available, and the hidden and announced bits cleared), then
   `civic.UpdateConditionalBuildingStatus(guid, &record)` (slot 7, which inserts a missing key, civic-buildings.md
   §4.7). Free the record's string, release the host, return 0.

The function never reads `0x00B207C0`.

## 7. `SCLuaTableRef` *[compat]*

A stack object that walks one Lua table with `lua_next`; its users are the automata script system and the city
situation advice. Layout (0x20 bytes):

| Offset | Field |
|---|---|
| `+0x00` | `cISCLua* lua` (not AddRef'd) |
| `+0x04` | `cRZString name` (empty unless built from a name) |
| `+0x18` | `int32 index`: absolute stack index of the table, −1 when none |
| `+0x1C` | `bool done` |

- **`SCLuaTableRef(lua, int32 idx)`** (`0x005FB9C0`, pops 8): `index := idx`; a negative `idx` is made absolute,
  `idx + GetTop() + 1`; `done := false`. The value is not type-checked.
- **`SCLuaTableRef(lua, const char* name)`** (`0x005FCA70`, pops 8): `name := text`. If the name is non-empty
  (string slot 7 `Strlen`): `GetGlobal(name)` (slot 37); if `IsTable(-1)` (slot 72) the table stays on the stack,
  `index := GetTop()`, `done := false`; otherwise `index := −1`, `done := true` and the value is popped (`Pop(1)`).
  With an empty name the stack is untouched and **`index` and `done` are left uninitialised** *[quirk]*.
- **`begin()`** (`0x005FB730`): if `index ≥ 0`: `PushNil()`, then `Next(index)` (slot 63); non-zero → `done :=
  false`, return true (key at −2, value at −1). Otherwise, or with `index < 0`: `done := true`, return false.
- **`next()`** (`0x005FB770`): if `done`, return false. Else `Pop(1)` (the value), `Next(index)`; non-zero → return
  true; zero → `done := true`, return false (Lua has already popped the key).

The helper never pops the table it found by name; its caller does.

## 8. Conformance tests

1. `game.ordinance_name(id)` for a registered ordinance returns a string userdata whose text is that ordinance's
   `GetName()` text, never "Ordinance Name #…"; with a number that is not an ordinance ID it returns `nil`.
2. `game.trigger_event()` with no argument shows the box captioned "LUAExtension_trigger_event()" and pushes
   nothing.
3. `game.unlock_reward_building(k)` on a key with status 6 and count 3 leaves status 2, count 3 and the same text;
   on a missing key it inserts `{2, 0, ""}`.
4. A table `{a = 1, b = 2}` walked by `SCLuaTableRef(lua, "t")` with `begin()` / `next()` visits two pairs and then
   returns false with the table still on the stack; `SCLuaTableRef(lua, "notatable")` leaves the stack unchanged
   and `begin()` returns false.

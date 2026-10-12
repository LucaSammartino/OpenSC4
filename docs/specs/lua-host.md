# Lua host (`cSCLua`, `cGZLua5`, `cGZLua5Thread`, script resources, extension registration) — behaviour spec

Status: round 3, written from the Windows 1.1.641 executable. The Mac named pseudocode was used for names and
intent only. Every fact below was checked on Windows unless it is tagged *[inferred]*. The game's Lua scripts
(`SimCity_1.dat`, type `0xCA63E2A3`) are data. This spec names the functions and globals they use but does not copy
them.

Tags: *[compat]* observable, must match; *[choice]* FreeCity4 decision; *[inferred]* not proven;
*[quirk]* original behaviour that looks unintended but is reproduced (or that a replacement must survive).

Shared material cited and not repeated here: class store (foundations §3), city component table (§3.6), framework
slot tables (§17: resource manager §17.6, game-data registry §17.7), slot-order rule (§16), x87 precision (§18,
in particular §18.1). The advisor system's use of the host is in advisors.md §4.1, §4.2 and §8. The `game.*`
functions themselves are specified in advisors.md §8.3 and are not repeated here. The budget's registry entries
are in budget.md §4.4. The MySim registry entries are in mysims.md.

Scope: the generic Lua host only. Who calls it is listed (§11) but not specified. The Lua 5.0 core (`lua_*`,
`luaL_*`, `luaopen_*`) is the stock PUC-Rio library compiled into the exe. It is cited by its public API name and
not re-specified.

---

## 1. Role and identity

### 1.1 What the host is

The host is the game's embedding of **Lua 5.0** (version string `$Lua: Lua 5.0 Copyright (C) 1994-2003 Tecgraf,
PUC-Rio $`; 5.0 or 5.0.1 *[inferred]*) behind three GZCOM interfaces, plus a small set of free functions for loading
scripts from DBPF resources and registering C functions.

It is **not** a simulator. It has no agent, no messages, no city component slot, no tuning exemplar, no curves and no
random draws. It has no save record (§2.2).

### 1.2 Classes and interfaces

| Class | CLSID(s) (registered by `cSCLuaCOMDirector`) | Interfaces answered by QI | Size |
|---|---|---|---|
| `cSCLua` (derives from `cGZLua5`) | `0xEA10C4A2` and `0x4B2811AA`, both with factory `0x008800A0` (new object per call) | `0xEA10C4A2` `cISCLua`; `0x6B2811A6` `cIGZLua5`; `0xEB2810F1` `cIGZLua5Thread` (its main thread); `cIGZUnknown` | `0x58` |
| `cGZLua5Thread` | `0x2B2810F7` (constructor `0x008B8DD0`) | `0xEB2810F1` `cIGZLua5Thread`; `cIGZUnknown` | `0x1C` |
| `cGZLua5CommandGenerator` | `0x2BD13FB4` (constructor `0x008B6480`) | `0x0B90251F`, `0x2BD144DF` | — |

`cSCLuaCOMDirector` (constructor `0x008800F0`, getter `0x00880180`) registers the four class IDs. The factory for
`0xEA10C4A2` and `0x4B2811AA` constructs a `cSCLua` (constructor `0x008809E0`, base constructor `0x008B9670`).

A `cSCLua` object holds, as sub-objects: a primary `cIGZUnknown` (vtable `0x00AC4444`), its **main thread**
(`cIGZLua5Thread`, vtable `0x00AC42C0`), its `cIGZLua5` (vtable `0x00AC4280`), its `cISCLua` (vtable `0x00AC4148`)
and a game-data-registry notification target (vtable `0x00AC4138`, §9). Every one of them answers QI through
`cSCLua::QueryInterface` (`0x008806C0`): `0xEA10C4A2` → `cISCLua`; anything else → `cGZLua5::QueryInterface`
(`0x008B8CF0`): `0x6B2811A6` → `cIGZLua5`, `0xEB2810F1` → the main thread, otherwise `cRZUnknown`.

A **stand-alone** `cGZLua5Thread` (vtable `0x00AC8DD0`, used for coroutine threads created by
`cIGZLua5::CreateThread`) answers only `0xEB2810F1` and `cIGZUnknown`. It does **not** answer `0xEA10C4A2` (§14 Q-6).

### 1.3 How a C function finds its host *[compat]*

The Lua state is created with 4 bytes of extra space in front of it (Lua's `LUAI_EXTRASPACE`). The host stores there a
pointer to the **thread object** that owns the state. Every game C function called from Lua reads that pointer
from the 4 bytes before its `lua_State*` and then:

- the extension functions (`print`, `_dofile_from_resource`, every `game.*` function) **QI it for `0xEA10C4A2`**
  and use only `cISCLua` methods (no direct `lua_*` calls were found in any of the 47 registered functions or the
  advisor tag-gc method `0x00418B50`, scanned to their first return *[inferred]* for later paths);
- the registry-mirror closures (§9) use it directly as a `cIGZLua5Thread`.

Neither checks the QI result. On the main thread the QI succeeds (§1.2). On a coroutine thread it fails and the
function dereferences null *[quirk]*.

### 1.4 Instances in the running game

There is no single Lua VM. Each `CreateInstance` makes a new, independent state:

| # | Owner and creation | Open flags | Lifetime | Used for |
|---|---|---|---|---|
| 1 | `cSC4App::Init` (`0x0044C320`): `GZCOM CreateInstance(0xEA10C4A2, 0xEA10C4A2)` into an app member, then `Open(0x7E)` at `0x0044C925` | `0x7E` | the app's lifetime; reached through `cISC4App` slot `+0xBC` (`0x0044B670`) and cached by `SL::OnAppInit1` (`0x00602290`) in the global `0x00B43CE8` | advisors, city situations, tutorial, MySim (§11) |
| 2 | `cSC4AutomataScriptSystem::InitScripts` (`0x0051C4C0`): `CreateInstance(0xEA10C4A2, 0xEA10C4A2)`, `Open(0x7E)` | `0x7E` | created in `ExecuteScripts` (`0x00523B10`, from the automata system's `Init`), **closed and released** at the end of the same call | automata configuration tables (group `0x4A5E8F3F`) |
| 3 | `cGZLuaScriptServer::Init` (`0x008B0860`): `CreateInstance(0x4B2811AA, 0x6B2811A6)` (a `cSCLua` reached as `cIGZLua5`), `Open(0x7E)` | `0x7E` | the script server's lifetime (a framework system service added by `cSC4App::AddSystemServices` `0x00450350`) | console scripts run by `cGZMiscCommandSystemSupport::DoMessage` (`0x005C8B30`) through `cIGZLuaScriptServer` |

Instance 1 is opened by the app before anything else touches it. The advisor system's `Open(0x7E)` (advisors.md
§4.1 step 1) is therefore a no-op that returns true (§3.1).

### 1.5 Init order (instance 1)

1. `cSC4App::Init`: create, `Open(0x7E)` (§3.1). Opening also subscribes the instance to the game-data registry
   (§9), which from then on mirrors every registry entry into the global `sc4game`.
2. `SL::OnAppInit1`: cache the Lua (`0x00B43CE8`), the string detokenizer (`cISC4App` `+0xA8` → `0x00B43CDC`) and
   the resource manager (`0x00B43CA8`).
3. `cSC4AdvisorSystem::Init` (city load, advisors.md §4.1): `register_api(true)` (§8.2), two tags (§3.4), the
   detokenizer hook (§10.2), `reload_data` (advisors.md §4.2), which runs §7.3, §7.4 and `register_api(false)`.
4. Later users (city situations, tutorial, MySim) only call into the already loaded scripts.

---

## 2. State

### 2.1 State table

| State | Initial value | Persisted |
|---|---|---|
| Lua state (heap, globals, registry) | none until `Open` | no |
| "is open" flag | false; set to the result of the base `Open` | no |
| Error reporter | the default reporter (§4.3) | no |
| Two command parameter sets (input, output) used by registry call entries (§9.4) | created by `Open`, released by `Close` | no |
| Lua 4 tag counter (§3.4) | 0 at process start; **process-wide**, never reset, shared by all instances | no |
| `current_resource_group_id_ca636e36` (Lua global) | unset until §7.4 sets it | no |
| `_tf_` (Lua global) | unset until §8.2 | no |

### 2.2 Persistence

None. Nothing of the host is written to a save file. Every load rebuilds the Lua state from scripts: the advisor
system re-runs `reload_data` in its `Init`, and script-side state lives only in Lua globals. Advisor state that must
survive a save is saved by the advisor classes (advisors.md §11), not by the host.

---

## 3. The VM

### 3.1 `Open(flags)` *[compat]*

`cISCLua::Open` (`0x00880860`) and `cIGZLua5::Open` (the same function through vtable `0x00AC4280` slot 4):

- If the instance is already open: return the stored flag (true). Nothing else happens.
- Otherwise run the base open (`cGZLua5::Open`, `0x008B8890`) and store its result as the "is open" flag. Then,
  **whatever that result was**: create the two command parameter sets (§9.4); get the game-data registry service
  (`0xEBE0E860`, IID `0xE898ED03`) and add this instance's notification target (registry slot 25, `+0x64`). Return
  the flag.

The base open:

1. If a state already exists, return false.
2. `lua_open`. If it fails, return false. Initialise the thread registry (which stores the main thread in the
   4 bytes before the state, §1.3). If that fails, return false.
3. `luaopen_base` (always).
4. Per flag bit (the SDK `OpenFlags` values are correct):

| Bit | SDK name | Opens |
|---|---|---|
| `0x01` | — | nothing (unused) |
| `0x02` | `OpenFlagIO` | `luaopen_io` (the `io` and `os` tables) |
| `0x04` | `OpenFlagString` | `luaopen_string` |
| `0x08` | `OpenFlagMath` | `luaopen_math` |
| `0x10` | `OpenFlagDebug` | `luaopen_debug` (this also defines `_TRACEBACK`, used by §4.1) |
| `0x20` | `OpenFlagTable` | `luaopen_table` |
| `0x40` | `OpenFlagLua4Compat` | the Lua 4 compatibility library (§3.3) |

5. Always open the host's thread library (a `coroutine` table: `create`, `resume` and the other Lua 5.0 coroutine
   functions, routed through `cIGZLua5::CreateThread`/`ResumeThread`).
6. `settop(0)`. Return true.

All three instances use `0x7E`, which opens everything. The registry subscription happens even when the base open
failed *[quirk]*; a second `Open` after a failure then subscribes the instance a second time *[inferred]*. If the
registry service does not exist, the original dereferences null *[quirk]*; a replacement should skip the
subscription *[choice]*.

### 3.2 `Close()`

`0x00880950`: remove the notification target (registry slot 27, `+0x6C`), clear the "is open" flag, run the base
close (`lua_close` and destruction of the thread registry *[inferred]*), release both parameter sets. Returns
**true in AL** (the SDK says `void`).

### 3.3 The Lua 4 compatibility library (`nsGZLua5CompatWithLua4`, `OpenLib` `0x008BF160`)

1. Run an embedded script with `DoString` (4071 bytes at `0x00AC9798`; its content matches the `etc/compat.lua` file
   shipped with Lua 5.0 *[inferred]*). It defines the Lua 4 global names: `dostring`/`dofile` (in Lua through
   `loadstring`/`pcall`), the table, math (degree-based trigonometry), string, os and io globals, and
   `getglobal`/`setglobal`. Among them is the global `getn`, used by §5 slot 64 *[inferred]*. If `DoString` returns
   non-zero, `OpenLib` returns false. The caller ignores this.
2. Register seven C globals: `tag` (`0x008BE990`), `settag` (`0x008BEE20`), `settagmethod` (`0x008BEF00`),
   `gettagmethod` (`0x008BEC80`), `newtag` (`0x008BE930`), `copytagmethods` (`0x008BEFE0`), `globals`
   (`0x008BEA30`).

### 3.4 Tag emulation *[compat]*

Lua 4 tags are emulated with metatables.

- **Tag numbers.** `newtag()` returns `++counter`, where `counter` is one process-wide integer starting at 0. Tags
  are therefore 1, 2, 3, … in creation order **across all instances** (instance 2 and 3 tags consume numbers too).
- **Tag → metatable.** The Lua registry holds, at the numeric key `1262289877.0`, a table mapping tag → metatable.
  A tag's metatable is created on first use. It holds the field `[187693250.0] = tag`.
- `tag(v)`: if `v` has a metatable whose field `[187693250.0]` is a number, return that number; otherwise `−1`.
- `settag(v, t)`: `setmetatable(v, metatable(t))`. Only tables and userdata can carry a metatable in Lua 5.0, so for
  other types it raises a Lua error.
- `settagmethod(tag, event, fn)`: map `event` to a metamethod name (table below), store `metatable(tag)[name] = fn`
  and return the previous value. An unmapped event name becomes a `nil` key, and the raw set raises a Lua error.
- `gettagmethod(tag, event)`: return `metatable(tag)[name]`, or `nil`.
- `copytagmethods(dst, src)`: copy every string-keyed field of `metatable(src)` into `metatable(dst)`; return `dst`.
  The loop tests the type of the same stack slot twice. The effect is that only string keys are copied *[inferred]*.
- `globals()`: the Lua 4 `globals` function (returns the globals table) *[inferred]*.

| Lua 4 event | Lua 5 metamethod |
|---|---|
| `index`, `gettable` | `__index` |
| `newindex`, `settable` | `__newindex` |
| `gc` | `__gc` |
| `add`, `sub`, `mul`, `div`, `pow`, `unm` | `__add` … `__unm` |
| `le` | `__le` |
| `concat` | `__concat` |
| `function` | `__call` |
| `lt`, `eq`, `getglobal`, `setglobal`, anything else | unmapped (error) |

`__gc` only runs for **userdata** in Lua 5.0, so the advisor tag-gc method (advisors.md §8.1) runs only for the
userdata it is meant for.

---

## 4. Protected calls, errors and the do-functions

### 4.1 `cIGZLua5Thread::CallProtected(nargs, nresults, errfn, popError)` (`0x008B8F30`, pops 16, status in EAX)

1. If `errfn == 0` **and** the owning `cIGZLua5` has an error reporter: fetch the global `_TRACEBACK` with a raw get,
   insert it below the function, and `lua_pcall(nargs, nresults, <that index>)`; then remove it from the stack.
2. Otherwise `lua_pcall(nargs, nresults, errfn)`.
3. If the status is not 0: `msg = ToString(−1)`; call `reporter(thread, status, msg)`; if `popError`, pop the message.
4. Return the Lua status: 0 OK, 1 `LUA_ERRRUN`, 2 `LUA_ERRFILE`, 3 `LUA_ERRSYNTAX`, 4 `LUA_ERRMEM`, 5 `LUA_ERRERR`.

`cISCLua::CallProtected(nargs, nresults)` (slot 51) is `CallProtected(nargs, nresults, 0, true)`. The reporter always
exists (§4.3). So every protected call made through `cISCLua` has `_TRACEBACK` as its message handler and pops the
error message.

### 4.2 Errors outside a protected call *[compat]*

Any Lua API call that raises an error (a metamethod error in `GetTable`/`SetTable`/`Concat`/`Equal`/`LessThan`,
`Error`, a failing `RawCall`, …) when **no** `lua_pcall` is active reaches `luaD_throw` (`0x008BB830`) with no
error handler. Lua 5.0 then calls the panic function and **exits the process with code 1**. Whether the state's
panic function is Lua's default is not proven (§15). When the same call is made from inside a C function that Lua
called under a protected call, the error unwinds to that protected call instead.

This is what happens when `register_api(false)` runs and the global `game` is missing (§8.2).

### 4.3 The error reporter

- `cIGZLua5::SetErrorReporter(fn, useDefault)` (slot 13, `0x008B96D0`): a null `fn` with `useDefault` installs the
  default reporter. `GetErrorReporter` (slot 14) returns the current one.
- The **default reporter** (`0x008B9360`, installed by the constructor) formats a message of the form
  "GZLua5 script error", error type (`Run-time`, `File-related`, `Syntax`, `Allocation`, otherwise `Unknown`, from
  status 1–4), thread ID and the message text. No output call was found: the text is built and dropped
  *[inferred]*. The game never installs another reporter on instance 1 *[inferred]*.
- Each thread, when created, registers the global `_ALERT` (`0x008B8D70`). If the value on top of the stack is a
  string, it calls `reporter(thread, 1, that string)`.

FreeCity4 *[choice]*: route the reporter text to the engine log. The game does not show script errors to the player.

### 4.4 `DoFile`, `DoString`, `DoBuffer`

These are Lua 5.0's `lua_dofile`, `lua_dostring` and `lua_dobuffer` (lauxlib) under the precision guard:

1. Load the chunk (`luaL_loadfile`, or `luaL_loadbuffer` with the chunk name; `DoString` uses the string itself as
   the chunk name).
2. If loading succeeded, `lua_pcall(0, LUA_MULTRET, 0)`. **No `_TRACEBACK` handler** here.
3. On error, *callalert*: if the global `_ALERT` is a function, call it unprotected with the message, which reports
   it with status 1 (§4.3). Otherwise print the message to `stderr` and pop it.
4. Return the status (int, EAX). The SDK declares these three as `void`. The results of a successful chunk stay on
   the stack (`LUA_MULTRET`), so the caller is responsible for the stack top.

### 4.5 Precision guard (all wrappers)

Every `cGZLua5Thread`, `cGZLua5` and `cSCLua` wrapper that calls into Lua switches the x87 precision as described in
foundations §18.1: get precision (`0x00892A0A`); if it is 24-bit, set 53-bit (`0x008929E0`) for the call and set
24-bit back afterwards. **Exception:** `Unref` (slot 62) has no guard. Lua arithmetic in scripts therefore runs in
double precision while the simulation runs in single precision *[compat]*. The only Lua code that may run under
24-bit is a garbage collection triggered from `Unref` *[inferred]*.

### 4.6 GC and references

- GC (slots 56–58): Lua 5.0 `lua_getgccount`, `lua_getgcthreshold`, `lua_setgcthreshold` (kilobytes). The game never
  changes the threshold *[inferred]*.
- References use the Lua registry (`LUA_REGISTRYINDEX = −10000`). `Ref` = `luaL_ref(registry)` (pops the top value,
  returns the new reference, `−1` for `nil`). `GetRef(r)` = `rawgeti(registry, r)`. `Unref(r)` =
  `luaL_unref(registry, r)`.

---

## 5. `cISCLua` slot table (vtable `0x00AC4148`, 78 slots) *[compat]*

The compiled order matches the SDK declaration order. In the table, "thread" means the call is forwarded to the main
thread's `cGZLua5Thread` method (shown as the implementation address). "guard" is §4.5. Index arguments are Lua
stack indices (negative = from the top; pseudo-indices allowed). **SDK** marks every difference from the SDK header
(`cISCLua.h`).

| Slot | Off | Method (as the game expects it) | Impl. | Pops | Return | Behaviour |
|---|---|---|---|---|---|---|
| 0 | `+0x00` | `QueryInterface(iid, void**)` | `0x008806C0` | 8 | AL | §1.2 |
| 1 | `+0x04` | `AddRef()` | `0x00968B60` | 0 | EAX count | |
| 2 | `+0x08` | `Release()` | `0x0090D997` | 0 | EAX count | |
| 3 | `+0x0C` | `bool Open(int32 flags)` | `0x00880860` | 4 | AL | §3.1 |
| 4 | `+0x10` | `bool Close()` | `0x00880950` | 0 | AL = 1 | §3.2. **SDK:** declared `void` |
| 5 | `+0x14` | `cIGZLua5* AsIGZLua5()` | `0x00880A50` | 0 | EAX | this instance's `cIGZLua5`, **no AddRef** |
| 6 | `+0x18` | `int32 GetTop()` | thread `0x008B6DF0` | 0 | EAX | `lua_gettop` |
| 7 | `+0x1C` | `void SetTop(int32)` | thread `0x008B6E30` | 4 | — | `lua_settop` |
| 8 | `+0x20` | `void PushValue(int32)` | thread `0x008B6E80` | 4 | — | |
| 9 | `+0x24` | `void Remove(int32)` | thread `0x008B6ED0` | 4 | — | |
| 10 | `+0x28` | `void Insert(int32)` | thread `0x008B6F20` | 4 | — | |
| 11 | `+0x2C` | `void Pop(int32 n)` | thread `0x008B6F70` | 4 | — | `settop(−n−1)` |
| 12 | `+0x30` | `int32 Type(int32)` | thread `0x008B70B0` | 4 | EAX | `lua_type` (−1 none, 0 nil, 1 boolean, 2 lightuserdata, 3 number, 4 string, 5 table, 6 function, 7 userdata, 8 thread) |
| 13 | `+0x34` | `const char* TypeName(int32 type)` | thread `0x008B7100` | 4 | EAX | `lua_typename` (re/names calls it `_tag_error`; wrong) |
| 14 | `+0x38` | `bool IsNumber(int32)` | thread `0x008B7150` | 4 | AL | |
| 15 | `+0x3C` | `bool IsString(int32)` | thread `0x008B71A0` | 4 | AL | true for numbers too (Lua rule) |
| 16 | `+0x40` | `bool IsCFunction(int32)` | thread `0x008B71F0` | 4 | AL | |
| 17 | `+0x44` | `int32 Tag(int32)` | thread `0x008B8400` | 4 | EAX | Call the compat `tag` C function with a copy of the value, `CallProtected(1, 1, 0, true)`; result truncated (`_ftol`). Error or result 0 → `−1`. Untagged value → `−1` |
| 18 | `+0x48` | `bool IsBoolean(int32)` | thread `0x008B73B0` | 4 | AL | |
| 19 | `+0x4C` | `bool Equal(int32, int32)` | thread `0x008B7520` | 8 | AL | `lua_equal` (may raise, §4.2) |
| 20 | `+0x50` | `bool LessThan(int32, int32)` | thread `0x008B75E0` | 8 | AL | `lua_lessthan` (may raise) |
| 21 | `+0x54` | `uint32 ToUint32(int32)` | thread `0x008B7640` | 4 | EAX | `_ftol(lua_tonumber)`, low 32 bits (truncation toward zero). Non-number → 0 |
| 22 | `+0x58` | `int32 ToSint32(int32)` | thread `0x008B7640` | 4 | EAX | **the same function** as slot 21 |
| 23 | `+0x5C` | `double ToNumber(int32)` | thread `0x008B7690` | 4 | x87 ST0 | `lua_tonumber` (0 for non-numbers) |
| 24 | `+0x60` | `const char* ToString(int32)` | thread `0x008B7740` | 4 | EAX | `lua_tostring` (converts a number in place; null for other types) |
| 25 | `+0x64` | `size_t Strlen(int32)` | thread `0x008B7790` | 4 | EAX | |
| 26 | `+0x68` | `lua_CFunction ToCFunction(int32)` | thread `0x008B77E0` | 4 | EAX | |
| 27 | `+0x6C` | `void* ToUserData(int32)` | thread `0x008B7830` | 4 | EAX | full or light userdata pointer, else null |
| 28 | `+0x70` | `void* ToUserTag(int32)` | thread `0x008B87F0` | 4 | EAX | Returns `*(void**)ToUserData(idx)`: the pointer **boxed** in a userdata made by slot 35. For a non-userdata it dereferences null *[quirk]*. **SDK:** declared `int32` "tag" |
| 29 | `+0x74` | `bool ToBoolean(int32)` | thread `0x008B76F0` | 4 | AL | |
| 30 | `+0x78` | `void PushNil()` | thread `0x008B78D0` | 0 | — | |
| 31 | `+0x7C` | `void PushNumber(double)` | `0x00880340` → thread `0x008B7910` | 8 | — (void) | docs/abi's x87 return for this slot is wrong: the function only copies its argument |
| 32 | `+0x80` | `void PushLString(const char*, size_t)` | thread `0x008B7960` | 8 | — | |
| 33 | `+0x84` | `void PushString(const char*)` | thread `0x008B79B0` | 4 | — | |
| 34 | `+0x88` | `void PushCClosure(lua_CFunction, int32 n)` | thread `0x008B7A00` | 8 | — | |
| 35 | `+0x8C` | `void PushUserTag(void* p, int32 tag)` | thread `0x008B87A0` | 8 | — | `newuserdata(4)` holding `p`, then the compat `settag(tag)` on it (protected, as slot 61). The userdata stays on the stack |
| 36 | `+0x90` | `void PushBoolean(bool)` | thread `0x008B7A50` | 4 | — | |
| 37 | `+0x94` | `void GetGlobal(const char*)` | thread `0x008B8280` | 4 | — | pushes `globals[name]` (may raise via `__index`) |
| 38 | `+0x98` | `void GetTable(int32)` | thread `0x008B7B40` | 4 | — | `lua_gettable` |
| 39 | `+0x9C` | `void RawGet(int32)` | thread `0x008B7B90` | 4 | — | |
| 40 | `+0xA0` | `void RawGetI(int32, int32)` | thread `0x008B7BE0` | 8 | — | |
| 41 | `+0xA4` | `void GetGlobals()` | thread `0x008B7D60` | 0 | — | pushes the globals table |
| 42 | `+0xA8` | `void GetTagMethod(int32 tag, const char* event)` | thread `0x008B84B0` | 8 | — | compat `gettagmethod(tag, event)` under `CallProtected(2, 1, 0, true)`; pushes the result, or `nil` on error |
| 43 | `+0xAC` | `void GetRef(int32)` | thread `0x008B8390` | 4 | — | §4.6 |
| 44 | `+0xB0` | `void NewTable()` | thread `0x008B7C30` | 0 | none (EAX garbage) | `lua_newtable`. **SDK:** declared `int32`. A replacement may return anything |
| 45 | `+0xB4` | `void SetGlobal(const char*)` | thread `0x008B82E0` | 4 | — | value on top: `pushstring(name)`, `insert(−2)`, `settable(GLOBALS)` (may raise via `__newindex`) |
| 46 | `+0xB8` | `void SetTable(int32)` | thread `0x008B7DB0` | 4 | — | `lua_settable` (may raise; §8.2) |
| 47 | `+0xBC` | `void RawSet(int32)` | thread `0x008B7E00` | 4 | — | |
| 48 | `+0xC0` | `void RawSetI(int32, int32)` | thread `0x008B7E50` | 8 | — | |
| 49 | `+0xC4` | `void SetTagMethod(int32 tag, const char* event)` | thread `0x008B8540` | 8 | — | the function is on top. Calls compat `settagmethod(tag, event, fn)` under `CallProtected(3, 0, 0, true)`, then pops the function |
| 50 | `+0xC8` | `int32 Ref(int32 ignored)` | `0x00880480` | 4 | EAX | **argument ignored**; `luaL_ref(registry)` of the top value (popped). SDK has one `int32` parameter and does not say it is ignored |
| 51 | `+0xCC` | `int32 CallProtected(int32 nargs, int32 nresults)` | `0x00880490` | 8 | EAX status | §4.1 with `errfn = 0`, `popError = true` |
| 52 | `+0xD0` | `void RawCall(int32 nargs, int32 nresults)` | thread `0x008B7F40` | 8 | — | `lua_call` (unprotected, §4.2) |
| 53 | `+0xD4` | `int32 DoFile(const char*)` | thread `0x008B7F90` | 4 | EAX status | §4.4. **SDK:** declared `void` |
| 54 | `+0xD8` | `int32 DoString(const char*)` | thread `0x008B7FE0` | 4 | EAX status | §4.4. **SDK:** `void` |
| 55 | `+0xDC` | `int32 DoBuffer(const char*, int32 size, const char* name)` | thread `0x008B8030` | 12 | EAX status | §4.4. **SDK:** `void` |
| 56 | `+0xE0` | `int32 GetGCCount()` | `cGZLua5` `0x008B8C20` | 0 | EAX | §4.6 |
| 57 | `+0xE4` | `int32 GetGCThreshold()` | `cGZLua5` `0x008B8C60` | 0 | EAX | |
| 58 | `+0xE8` | `void SetGCThreshold(int32)` | `cGZLua5` `0x008B8CA0` | 4 | — | |
| 59 | `+0xEC` | `int32 NewTag()` | thread `0x008B85E0` | 0 | EAX | compat `newtag()` under `CallProtected(0, 1, 0, true)`, then reads the number at **absolute stack index 1** (not the top) and pops one value. ≤ 0, not a number or error → `−1`. Correct only when the stack was empty *[quirk]* |
| 60 | `+0xF0` | `int32 CopyTagMethods(int32 dst, int32 src)` | thread `0x008B86C0` | 8 | EAX | compat `copytagmethods` (protected); returns `dst` |
| 61 | `+0xF4` | `void SetTag(int32 tag)` | thread `0x008B8670` | 4 | — | compat `settag(top value, tag)` under protection; the value stays on the stack |
| 62 | `+0xF8` | `void Unref(int32)` | thread `0x008B83E0` | 4 | — | §4.6; **no precision guard** |
| 63 | `+0xFC` | `int32 Next(int32)` | thread `0x008B8130` | 4 | EAX | `lua_next` |
| 64 | `+0x100` | `int32 GetN(int32 idx)` | `0x00880570` | 4 | EAX | Not a table → 0. Otherwise call the **global** `getn` on the table under `CallProtected(1, 1, 0, true)` and return the truncated number. If `getn` is not a function, or the call fails: **return `idx`** *[quirk]*. The stack is restored |
| 65 | `+0x104` | `void Error(const char* ignored)` | thread `0x008B80F0` | 4 | does not return | **message ignored**: `lua_error` raises the value on top of the stack. SDK does not say so |
| 66 | `+0x108` | `void Concat(int32 n)` | thread `0x008B8180` | 4 | — | `lua_concat` (may raise) |
| 67 | `+0x10C` | `void* NewUserData(size_t)` | thread `0x008B7AF0` | 4 | EAX | `lua_newuserdata`; pushes it |
| 68 | `+0x110` | `void Register(lua_CFunction fn, const char* name)` | thread `0x008B81D0` | 8 | — | `globals[name] = fn` (order `(fn, name)` as in the SDK) |
| 69 | `+0x114` | `void PushUserData(void* p, int32 tag)` | thread `0x008B87A0` | 8 | — | **the same function as slot 35** |
| 70 | `+0x118` | `int32 CloneTag(int32 tag)` | thread `0x008B8750` | 4 | EAX | `NewTag()` then `CopyTagMethods(new, tag)`; returns the new tag. **SDK:** declared `void` |
| 71 | `+0x11C` | `bool IsFunction(int32)` | thread `0x008B7240` | 4 | AL | |
| 72 | `+0x120` | `bool IsTable(int32)` | thread `0x008B72A0` | 4 | AL | |
| 73 | `+0x124` | `bool IsUserData(int32)` | thread `0x008B7300` | 4 | AL | |
| 74 | `+0x128` | `bool IsNil(int32)` | thread `0x008B7410` | 4 | AL | |
| 75 | `+0x12C` | `cEntry* ToDataRegistryEntryID(cISC4GameDataRegistry*, int32 idx)` | `0x00880760` | 8 | EAX | If the value is a table: its raw field `_e` as userdata; returned if the registry says it is valid (`IsValidEntry`, slot 12), else null. Not a table → null. Stack restored. **SDK:** declared `uint32` (it is the entry pointer) |
| 76 | `+0x130` | `cEntry* ToDataRegistryNode(cISC4GameDataRegistry*, int32)` | `0x008807D0` | 8 | EAX | entry := slot 75; if the registry is non-null, the entry is valid and `IsNodeEntry` (slot 13): `GetNode(entry)` (slot 10); else null |
| 77 | `+0x134` | `cEntry* ToDataRegistryValue(cISC4GameDataRegistry*, int32)` | `0x00880820` | 8 | EAX | entry := slot 75; if valid: `GetValue(entry)` (slot 11) as a pointer; else null |

Slots 6–74 except 31, 50, 51 and 64 are thin forwards to the main thread. The only `cSCLua`-specific behaviour is in
slots 0, 3, 4, 5, 31 (copies the double), 50 and 51 (fixed arguments), 64 and 75–77.

### 5.1 SDK errors in `cISCLua.h` (checked with `abi_extract --check` and the epilogue)

| Slot | SDK | Exe | Address checked |
|---|---|---|---|
| 4 `Close` | `void` | `bool`, AL = 1 | `0x00880950` (`mov al, 1; ret`) |
| 28 `ToUserTag` | `int32` tag | `void*`, the boxed pointer; null deref for non-userdata | `0x008B87F0` (pops 4) |
| 35/69 `PushUserTag`/`PushUserData` | two methods | one function `(void*, int32 tag)` | `0x008B87A0` (pops 8) |
| 21/22 `ToUint32`/`ToSint32` | two methods | one function (`_ftol`, low 32 bits) | `0x008B7640` (pops 4) |
| 44 `NewTable` | `int32` | `void` (EAX garbage) | `0x008B7C30` (pops 0; no EAX set after the last call) |
| 50 `Ref` | `Ref(int32)` | argument ignored | `0x00880480` (pops 4) |
| 53/54/55 `DoFile`/`DoString`/`DoBuffer` | `void` | `int32` status | `0x008B7F90`, `0x008B7FE0` (`mov eax, esi; ret 4`), `0x008B8030` (pops 12) |
| 65 `Error` | message used | message ignored; raises the stack top | `0x008B80F0` (pops 4) |
| 70 `CloneTag` | `void` | `int32` new tag | `0x008B8750` (`mov eax, esi; ret 4`) |
| 75 `ToDataRegistryEntryID` | `uint32` | the `cEntry*` | `0x00880760` |

The same `void`-vs-`int32` differences apply to the identical methods of `cIGZLua5Thread` (§6.2).

---

## 6. `cIGZLua5` and `cIGZLua5Thread`

### 6.1 `cIGZLua5` (IID `0x6B2811A6`; `cGZLua5` vtable `0x00AC8F80`, 15 slots)

The compiled order matches the SDK:

| Slot | Method | `cGZLua5` impl. | Pops |
|---|---|---|---|
| 0–2 | QI / AddRef / Release | `0x008B8CF0`, … | 8 / 0 / 0 |
| 3 | `cIGZLua5Thread* AsIGZLua5Thread()` (main thread) | `0x00880A40` in `cSCLua`'s copy | 0 |
| 4 | `bool Open(flags)` | `0x008B8890` (in `cSCLua`'s copy `0x00AC4280`: `cSCLua::Open` `0x00880860`) | 4 |
| 5 | `void Close()` | base (in `cSCLua`'s copy: `0x00880950`, returns AL = 1) | 0 |
| 6 | `int32 CreateThread(cIGZLua5Thread**, int32, uint32)` | `0x008B8A00` | |
| 7 | `void DestroyThread(int32)` | `0x008B8AE0` | |
| 8 | `bool ResumeThread(int32, int32, int32*)` | `0x008B8B30` | |
| 9 | `cIGZLua5Thread* GetThread(int32)` | `0x008B8A80` | |
| 10–12 | `GetGCCount` / `GetGCThreshold` / `SetGCThreshold` | `0x008B8C20` / `0x008B8C60` / `0x008B8CA0` | 0 / 0 / 4 |
| 13 | `void SetErrorReporter(fn, bool useDefault)` | `0x008B96D0` | |
| 14 | `ErrorReporterCallback GetErrorReporter()` | `0x004ED900` (a shared getter that returns one member) | 0 |

`cSCLua` has its own copy of this vtable at `0x00AC4280` (15 slots), identical except that slots 4 and 5 are
`cSCLua`'s `Open`/`Close`. So opening an instance through `cIGZLua5` (instance 3) also subscribes it to the registry.

### 6.2 `cIGZLua5Thread` (IID `0xEB2810F1`; `cGZLua5Thread` vtable `0x00AC8DD0`, 97 slots)

The compiled order matches the SDK (`cIGZLua5Thread.h`, 94 declared methods plus the 3 `cIGZUnknown` slots). The
methods shared with `cISCLua` behave as in §5, with the same SDK errors. In addition: slot 82 (`+0x148`)
`GetUpValueIndex(i)` (used by §9), slot 83 `Ref`, 84 `GetRef`, 85 `Unref`, and the full
`CallProtected(nargs, nresults, errfn, popError)` (§4.1). The thread class is `0x1C` bytes.

There are other copies of this vtable: `0x00AC42C0` (`cSCLua`'s main thread, whose QI is `cSCLua::QueryInterface`)
and `0x00AC8FC0` (`cGZLua5`'s main thread, whose QI is `cGZLua5::QueryInterface`).

### 6.3 `cIGZLuaScriptServer` (vtable `0x00AC8710`, 25 slots)

`cGZLuaScriptServer` (constructor `0x008B07C0`) implements the SDK interface in SDK order: 3 Init, 4 Shutdown,
5 GetLua, 6 RunScriptString, 7 RunScriptFile, 8 KillScript, 9 PauseScript, 10 ResumeScript, 11–18 breakpoints,
19 EnumerateScriptIDs, 20 GetScriptInfo, 21 GetCurrentRunningScriptID, 22/23 Get/SetScriptAutoYieldEnabled,
24 GetLastCreatedScriptID (a shared one-member getter `0x006AAC50`). Slot 25 is the destructor (not part of the
interface). Its `Init` creates instance 3 (§1.4) and a `cGZLua5CommandGenerator` (`0x2BD13FB4`/`0x2BD144DF`). It
initialises the generator with the Lua, the string `sc4app` and the command system *[inferred]*. This server is the
developer/console scripting path. It is not part of the simulation and is described here only as a user of the host.

---

## 7. Script resources

### 7.1 Resource key *[compat]*

A Lua script is a DBPF resource of type `0xCA63E2A3`, read through the resource manager as interface `0xCA63D8E9`.

```
key(group, name) = { type 0xCA63E2A3, group, instance = 0xFF000000 | CRC24(basename(name)) }
```

- `SC4GetResourceIDFromString` (`0x005F8BB0`) = `CRC24(s, lowercase = false) | 0xFF000000`.
- `CRC24` (`nRZHash::CRC24`, `0x00932F76`) is the OpenPGP CRC-24 of RFC 4880: polynomial `0x864CFB` (table built
  from `0x1864CFB`), initial value `0xB704CE`, one byte at a time
  `crc = (crc << 8) XOR T[((crc >> 16) XOR byte) AND 0xFF]`, result `AND 0xFFFFFF`. No final XOR. Its second
  argument would lower-case each byte first; the host passes **false**, so the key is **case-sensitive**.
- `basename` (through `cRZPath`): resolve the path, split it, and keep the file-name component **without the
  directory and without the last extension**, case preserved. `"C:\x\advisors\Taxi.lua"` → `"Taxi"`;
  `"adv_const.lua"` → `"adv_const"`.

The shipped `SimCity_1.dat` holds 158 scripts of type `0xCA63E2A3`: 80 in group `0x4A5E8EF6` (advisors, tutorial,
city situations, MySim) and 78 in group `0x4A5E8F3F` (automata). All but two instance IDs match `CRC24` of a shipped
file name (§16, T1–T3). The two exceptions are `0xFFAEC42B` and `0xFF8085FD` (§15).

### 7.2 `LoadResourceScript`

- By key (`0x005FB7B0`, cdecl `(lua, key*, chunkName)`): get the resource. Missing → return **2**. Data null or size 0
  → return **0** without running anything. Size < 0 → **2**. Otherwise `DoBuffer(data, size, chunkName)` (§4.4) and
  return its status.
- By name (`0x005FBF50`, cdecl `(lua, group, name)`): `key(group, name)`, then the call above with
  `chunkName = basename(name)`.

### 7.3 `UpdateResourcesFromDirectory(group, directory)` (`0x005FC510`) *[compat]*

Lets loose files override packaged scripts:

1. List the files matching `*.lua` in `directory`.
2. For each file: `k = key(group, file)`. If the resource manager already has a resource for `k` (interface
   `0xCA63D8E9`): read the whole file. If its size is > 0, replace the resource's data with the file contents
   (resource slot 3, `+0x0C`, set data). Count the file whether or not it was replaced. Release.
3. Files whose key does not already exist are **ignored**. Overrides cannot add new scripts.
4. Return the count. Every caller ignores it.

The advisor system calls it for `<data dir>\advisors\` and then `<plugin dir>\advisors\` (advisors.md §4.2), so a
plugin file replaces a data-directory file of the same name.

### 7.4 `LoadPackageScripts(lua, group, firstPackage)` (`0x005FBA20`) *[compat]*

1. Set the Lua global `current_resource_group_id_ca636e36 = group` (pushed as an unsigned number).
2. List every resource key of type `0xCA63E2A3` (resource manager slot 18, `+0x48`).
3. For each key with `key.group == group`, in list order: get the resource. Consider it only if its size is
   > 23 bytes and its first three bytes are `--#`. Within the first `min(size, 512) − 1` bytes, take the text after
   `--#` up to the next `#`, parse it as a command line, and read the switch `package` (`-package:<hex>`) as a
   hexadecimal `uint32`. If the ID is non-zero and **not yet in the map**, map ID → key (the **first** key in list
   order wins for a duplicate ID).
4. If `firstPackage ≠ 0` and it is in the map: load it first (§7.2 by key, chunk name `""`). If that succeeded,
   remove it from the map.
5. Load the remaining packages in **ascending order of the ID as a signed 32-bit integer**, each with
   `LoadResourceScript(lua, key, "")`.
6. Return the number of packages whose status was not 0.

Shipped package headers (the header line is `--#-package:<8 hex digits>#`):

| Group | Script | Package ID |
|---|---|---|
| `0x4A5E8EF6` | `package_sys` | `0x00000001` |
| `0x4A5E8EF6` | `package_startup` | `0x00002000` |
| `0x4A5E8EF6` | `package_advisors` | `0x00003000` |
| `0x4A5E8EF6` | `package_tutorial` | `0x00004000` |
| `0x4A5E8EF6` | `package_ep1_advisors` | `0x01001000` |
| `0x4A5E8F3F` | `_package_startup` | `0x00000001` |

Scripts without a header run only when a package script loads them with `dofile` (§7.5). The `package_sys` script
replaces `dofile` with `_dofile_from_resource`. From then on every `dofile` in the scripts is a resource load.

Callers: `reload_data` (`firstPackage = 0`, result ignored); `cSC4AutomataScriptSystem::ExecuteScripts`
(`0x4A5E8F3F`, `firstPackage = 0`). The automata system imports its tables **only if the result is 0**. Otherwise it
formats the message "Error code %d executing automata _config.lua script!" and skips the import.

The chunk name of a package is empty. So `_TRACEBACK` and error texts name these chunks `""` *[compat]*.

### 7.5 `_dofile_from_resource(name)` (`0x005FC020`, Lua C function) *[compat]*

1. Find the host through the 4 bytes before the state and QI it for `cISCLua` (§1.3).
2. If the stack is not empty and the **top** value is a string: `group = _ftol(globals.current_resource_group_id_ca636e36)`
   if that global is a number, else 0. Run `LoadResourceScript(lua, group, name)` (§7.2 by name).
3. Push `0` if the status was 0, otherwise `nil`. Return 1 result. A non-string argument (or none) also returns
   `nil`.

The script's own results are left on the stack below the pushed value. Only the pushed value is returned. The
directory part of `name` is ignored and the match is case-sensitive: `dofile("Taxi.lua")` and `dofile("taxi.lua")`
are different resources.

---

## 8. Extension registration

### 8.1 The collectors

Two static collectors, constructed by the CRT in this order, append `{name, function, isGlobal}` entries to one
vector:

1. Simulation collector (`cLUAExtensionsCollector` `0x0040B8D0`): 38 entries. 35 are `game.*` functions (advisors.md
   §8.3), then three globals: `_combine_guids` (`0x0040AD10`), `_dofile_from_resource` (`0x005FC020`), `print`
   (`0x005FB850`).
2. UI collector (`cLUAExtensionsCollectorUI` `0x0040CAB0`): 9 `game.*` entries, `tool_plop_network`,
   `tool_plop_zone`, `tool_plop_building`, `tool_plop_flora`, `tool_button`, `window_budget`, `window_map`,
   `window_query` and `window_graph` (`0x0040C890`), in that order.

### 8.2 `register_api(bool global)` (`0x00418940`, pops 4, returns AL) *[compat]*

For each entry in vector order:

- an entry with `isGlobal == global` is registered; other entries are skipped;
- target table: for a global entry, `GetGlobals()`; for a `game` entry, `GetGlobal("game")`;
- if the stack top changed (the push happened): `Register(fn, "_tf_")`, push the name, `GetGlobal("_tf_")`,
  `SetTable(−3)` (target[name] = fn, **unprotected**), `Pop(1)`.

Finally restore the stack top. **Always returns true** (AL = 1).

Consequences:

- `_tf_` is a real global. After `register_api(true)` it is `print`. After `register_api(false)` it is
  `window_graph`'s function *[quirk]*.
- If the global `game` is `nil` when `register_api(false)` runs, `SetTable` on `nil` raises a Lua error outside any
  protected call. Lua's panic path then **ends the process** (§4.2). In the shipped scripts, a package script creates
  `game` before `reload_data` calls `register_api(false)`.
- `GetGlobal("game")` always pushes something, so the "top changed" test is always true.

### 8.3 Host globals

| Global | Set by | Behaviour |
|---|---|---|
| `print(...)` | `register_api(true)`; instance 2 `Register` | §8.4 |
| `_dofile_from_resource(name)` | `register_api(true)`; instance 2 `Register` | §7.5 |
| `_combine_guids(a, b)` | `register_api(true)` | `a XOR b` as a signed int32, −1 on bad arguments (advisors.md §8.3) |
| `_tf_` | `register_api` | last registered function (§8.2) |
| `current_resource_group_id_ca636e36` | §7.4 | number |
| `sc4game` | registry mirror (§9) | table tree |
| `_ALERT` | every thread at creation | §4.3 |
| `_TRACEBACK` | `luaopen_debug` | used by §4.1 |
| `tag`, `settag`, `settagmethod`, `gettagmethod`, `newtag`, `copytagmethods`, `globals` | compat library (§3.3) | §3.4 |
| `coroutine.*` | host thread library (§3.1 step 5) | |
| `_IN_GAME`, `_FINAL_RELEASE`, `_MASTER_RELEASE` = `1`; `_BETA_BUILD`, `_MASTER_DEBUG` = `nil` | instance 2 only (`InitScripts`) | build flags read by the automata scripts |

Script-side names that the exe does not set but that scripts read (all `nil` unless a script sets them):
`_on_reload` (called by `reload_data`, advisors.md §4.2) and `__dofile_directory` (its name exists in the exe's
strings but no code references it).

### 8.4 `print(...)` (`0x005FB850`) *[compat]*

For each argument in order: call the global `tostring` on it with `RawCall(1, 1)` (unprotected). If the result is
not a string, `Error(...)` raises the non-string value itself. **Nothing is output.** Returns 0 results.

FreeCity4 *[choice]*: may log the text. It must keep the error behaviour and return nothing.

### 8.5 Tags and the gc method

The two advisor tags and the shared gc method are in advisors.md §8.1. On the host side: `NewTag` returns the next
process-wide tag (§3.4). In instance 1 these are tags 1 and 2 when no earlier `newtag` ran (§16, T10). The method is
attached with `SetTagMethod(tag, "gc")` → `__gc` (§3.4).

---

## 9. The game-data registry mirror (`sc4game`) *[compat]*

### 9.1 Subscription

Each `cSCLua` instance, on `Open`, adds its notification target to the game-data registry (service `0xEBE0E860`, IID
`0xE898ED03`, slot 25). It removes it on `Close` (slot 27). The target's slot 3
`OnGameDataRegistryNotification(registry, type, entry)` (`0x00881650`, pops 12) keeps a Lua mirror of the registry
tree. Every entry also has a Lua registry slot keyed by the entry pointer as light userdata.

Whether the registry replays existing entries to a newly added target is not known (§15). Instance 1 is subscribed
before any city component publishes. Instance 2 is subscribed only while the automata scripts run.

### 9.2 `AddEntry` (type 3)

- **Root node:** create a table, store it as the global `sc4game` and at `registry[lightud(root)]`.
- **Other entries:** parent table = `registry[lightud(parent)]`. Key = the entry name, or `lightud(entry)` if the name
  is empty.
  - A **node** becomes a new table with field `_e = lightud(entry)`, stored at `parent[key]` and at
    `registry[lightud(entry)]`.
  - A **leaf** becomes a C closure with three upvalues (the registry, the entry, the `cSCLua`), stored at
    `parent[key]`. If the entry has flag 1 (`EntryFlagCallFunction`, `CheckEntryFlags` slot 19) the closure is the
    **call** function `0x00881260`; otherwise the **exchange** function `0x00880F50`.

### 9.3 Exchange closures (`sc4game.x.y(...)`)

1. If `IsValidEntry(entry)` is false, nothing is pushed and the closure returns 1 result. Lua then returns whatever
   is on top: the last argument, or (with no arguments) an unspecified value *[inferred]*.
2. `set = (argument count > 0)`. When set, argument 1 is converted to a variant (table below).
3. `DoValueExchange(entry, variant, set)` (registry slot 15).
4. On **get**, push the variant converted back (table below). On **set**, push nothing. The closure still returns
   1, so the call yields its last argument.

| Lua → variant (set) | Variant → Lua (get) |
|---|---|
| `nil` → `cIGZUnknown` null | type 1 (bool) → boolean |
| boolean → Bool | types 2–14 (all integer and float scalars) → number (`GetValFloat64`) |
| number → Float64 | type 16 (`cIGZUnknown`) → the mirror table of `GetEntryID(unknown)` (slot 9), or `nil` |
| string → String | anything else (void pointer, empty, arrays, strings) → string (`GetValString`) |
| table → the node entry of its `_e` field (`ToDataRegistryNode`, §5 slot 76) | |
| light userdata, function, userdata, thread → Float64 `0.0` | |

### 9.4 Call closures

Clear the instance's **two shared parameter sets** (input, output). Convert each Lua argument `i` (same table as
§9.3) into input parameter `i − 1`. Call `DoValueCall(entry, in, out)` (registry slot 16). Then iterate the output
parameters (first, then next until `−1`), skip the parameter ID `0x2B999D2B`, push each one converted, and return
their count. The parameter sets belong to the instance, not to the call. A call entry whose callback runs Lua that
makes another `sc4game` call would therefore overwrite them *[inferred]*.

### 9.5 `RemoveEntry` (type 2) and `Shutdown` (type 1)

- Remove: `parent[key] = nil`. For a node, visit its immediate children (`VisitImmediateChildren`, slot 23, callback
  `0x00880710`) and clear the Lua registry slot of each child node.
- Shutdown: `sc4game = nil`, clear the children's registry slots the same way, and reset one internal member
  *[inferred]*.

What the scripts use (from the shipped scripts; owners specified elsewhere): `sc4game.sitmgr.*`,
`sc4game.automata.*`, `sc4game.mysims.*` (mysims.md), `sc4game.mysims_ui.*`, `sc4game.effects.*`,
`sc4game.aura.*`, `sc4game.civic.*`, `sc4game.budget.*` (budget.md §4.4).

---

## 10. The string detokenizer (host side)

### 10.1 The service *[compat]*

`cISCStringDetokenizer` (`cSCStringDetokenizer`, vtable `0x00AA6DFC`, 12 interface slots) is reached through
`cISC4App` `+0xA8` and cached in `0x00B43CDC`.

| Slot | Off | Method | Impl. | Pops |
|---|---|---|---|---|
| 3 / 4 | `+0x0C` / `+0x10` | `Init` / `Shutdown` (shared stubs) | `0x00648F00` | 0 |
| 5 | `+0x14` | `SetTokenReplacementValue(token, value)` | `0x00601AA0` | |
| 6 | `+0x18` | `SetTokenReplacementMethod(token, fn, ctx)` | `0x00601B80` | |
| 7 | `+0x1C` | `AddUnknownTokenReplacementMethod(fn, ctx, bool add)` | `0x00600410` | 12 |
| 8 | `+0x20` | `bool Detokenize(cGZPersistResourceKey const&, cIGZString& out)` | `0x005FFEE0` | 8 |
| 9 | `+0x24` | `bool Detokenize(cIGZString const&, cIGZString& out)` | `0x005FFEB0` | 8 (jumps to the class's `PerformDetokenize` `0x00600810`) |
| 10 / 11 | `+0x28` / `+0x2C` | `SetHTMLCleanMode(bool)` / `GetHTMLCleanMode()` | `0x005FFEC0` / `0x005FFED0` | |

**SDK error:** the two `Detokenize` overloads are compiled in **reverse** declaration order (foundations §16): slot 8
is the resource-key overload (it reads the text resource through the resource manager and detokenizes it), and slot 9
is the string overload.

`PerformDetokenize` (string → string, returns bool "every token resolved"):

1. A doubled `##` is an escaped literal `#`.
2. Tokens are delimited by single `#` characters: `#name#`. The text between a pair is trimmed and resolved (below).
   The result replaces the whole `#…#`. Text outside tokens is copied unchanged. An unmatched final `#` and the rest
   of the text are copied as they are.
3. Resolution (`ConvertToken` `0x00600C20`), in this order:
   a. the fixed-value map (`SetTokenReplacementValue`). Lookup is case-insensitive (`strnicmp`);
   b. the method map (`SetTokenReplacementMethod`, same lookup): call `fn(token, out, ctx)`; use it if it returns true;
   c. every unknown-token method (`AddUnknownTokenReplacementMethod`), in the order of the set that holds them
      (ordered by method record, not by registration *[inferred]*), until one returns true;
   d. otherwise the replacement is the **bare token text**, and the overall result becomes false.
   If HTML-clean mode is on, the replacement is cleaned (`0x00600640`).
4. `AddUnknownTokenReplacementMethod(fn, ctx, add)`: `add = true` inserts `{fn, ctx}` (a duplicate is ignored);
   `add = false` erases it. The callbacks are cdecl `bool fn(cIGZString const& token, cIGZString& out, void* ctx)`.

### 10.2 The advisor hook (`gAdviceDetokenizerFunction`, `0x00417AF0`)

The advisor system registers this unknown-token method with `ctx` = the system (advisors.md §4.1 step 4). It is
advisor code. Its fixed names (`link_id`, `advisor`, building costs, mission tokens) are in advisors.md §8.5. The
part that is Lua host behaviour is the **expression token**:

1. Optional format prefix: if the token is longer than 1 character and its second character is `@`, the first
   character selects the format and both characters are removed. `S`/`s` → integer (`%i`), `M`/`m` → money
   (`SC4MakeMoneyString`), `D`/`d` → date (a `cRZDate` built from the number; month, day and year through the localisation
   formatter), `T`/`t` → time of day (a `cRZTime` built from the number; hour and minute through the formatter;
   the unit of the number is not established *[inferred]*), `P`/`p` → locale number followed by `%`, any other letter → locale number. Without `X@`, the format is the locale number.
2. Run `DoString("_dasd_=(" + expression + ")")` on instance 1 (§4.4: unprotected `_ALERT` reporting, no traceback).
   A non-zero status → unresolved.
3. Save the stack top, `GetGlobal("_dasd_")`:
   - **number** → truncate (`_ftol`) and format by the prefix;
   - **userdata** whose `Tag` equals the advisor system's string tag (advisors.md §8.1) → the boxed string;
   - **string** → the text itself;
   - anything else → unresolved.
   Restore the stack top.
4. Unresolved: the method writes `%UNKNOWN TOKEN --<expression>--%` and returns **false**. `ConvertToken` then tries
   the next method and finally replaces the output with the bare token text (§10.1 step 3d). So that text is not what
   the player sees *[inferred]*.

The global `_dasd_` keeps the last value. It is never cleared *[compat]*.

Shipped scripts use `#s@game.g_year#`, `#s@game.g_month#`, `#s@game.g_day#` and `#d@game.g_date#` (and plain names
such as `#city#`, `#mayor#`, `#link_id#`). Three texts use `%name%`, which the detokenizer does **not** expand.

---

## 11. Users of the host (call sites only)

| Caller | Address | Use |
|---|---|---|
| `cSC4App::Init` | `0x0044C320` | creates and opens instance 1 |
| `cISC4App` `+0xBC` (`GetLuaInterpreter`) | `0x0044B670` | returns instance 1 |
| `SL::OnAppCreate` / `SL::OnAppInit1` | `0x00601C00` / `0x00602290` | cache app, Lua (`0x00B43CE8`), detokenizer (`0x00B43CDC`), resource manager (`0x00B43CA8`) |
| `cSC4AdvisorSystem::Init` | `0x0041B570` | `Open`, `register_api(true)`, `NewTag`×2, `SetTagMethod`, detokenizer hook |
| `reload_data` | `0x0041A4F0` | `_on_reload`, §7.3, §7.4, `register_api(false)`, tutorial `SetLua` |
| `register_api` | `0x00418940` | §8.2 |
| `sNotify<cISC4Advisor>` | `0x00418A80` | QI `0xEA10C4A2`; calls script functions |
| `gAdviceDetokenizerFunction` | `0x00417AF0` | §10.2 |
| `cSC4CitySituationAdvice` | `0x00423090` Shutdown, `0x004230D0` ReadUint32FromTable, `0x004239D0` Init, `0x00423E20` UpdateSituationFields, `0x00424450` GetProgressText, `0x00424600` ReadStringFromTable | table reads and script calls on instance 1 |
| `cSC4CitySituationManager` | `0x0052DC40` Init, `0x00524F00` CallTargetScriptFunction | instance 1 |
| `cSC4TutorialRegistry` | `0x004B7720` Init, `0x004B7D70` SetLua (via `cISC4App` `+0xC4`) | instance 1 |
| `cSC4MySim` | `0x00429040` CreateLuaTable, `0x00429830` ActivateLuaContext | instance 1 |
| `cSC4AutomataScriptSystem` | `0x0051C4C0` InitScripts, `0x00523B10` ExecuteScripts | instance 2 (§1.4, §7.4) |
| `cGZLuaScriptServer` | `0x008B0860` Init | instance 3 |
| `cRZSysServPtr<cIGZLuaScriptServer>` | `0x004496F0`, used by `cGZMiscCommandSystemSupport::DoMessage` `0x005C8B30` | console scripts |
| every `LUAExtension_*` function, `print`, `_dofile_from_resource` | §8.1 | QI from the state (§1.3) |
| registry publishers (budget, MySim, automata, aura, civic, effects, situations) | their specs | indirect: through the registry mirror (§9) |

---

## 12. Dependencies

| Interface | Compiled slot (offset) | Method | Pops | Return |
|---|---|---|---|---|
| `cIGZPersistResourceManager` (foundations §17.6) | 3 (`+0x0C`) | `GetResource(key, iid, void**, …)` | | AL |
| | 18 (`+0x48`) | `GetAvailableResourceListForType(list, type)` | | |
| Lua script resource (`0xCA63D8E9`) | 3 (`+0x0C`) set data; 4 (`+0x10`)/5 (`+0x14`) data/size *[inferred]* | | | |
| `cISC4GameDataRegistry` (foundations §17.7) | 9 `GetEntryID(cIGZUnknown*)`, 10 `GetNode`, 11 `GetValue`, 12 `IsValidEntry`, 13 `IsNodeEntry`, 15 `DoValueExchange`, 16 `DoValueCall`, 19 `CheckEntryFlags`, 23 `VisitImmediateChildren`, 25 `AddNotificationTarget(target*)`, 27 `RemoveNotificationTarget(target*)` | | | |
| `cISC4App` | `+0xA8` GetStringDetokenizer, `+0xBC` GetLuaInterpreter, `+0xC4` GetTutorialRegistry, `+0xD4` GetDataDirectory, `+0xDC` GetPluginDirectory | | | |
| `cRZPath` | ResolvePath, Splitpath, GetFileName (non-virtual framework code) | | | |
| `cGZCommandParameterSet` | constructed by `Open` (§9.4) | | | |

The `cIGZLua5`/`cIGZLua5Thread`/`cIGZLuaScriptServer`/`cISCStringDetokenizer` vtables above are not yet in
`docs/abi`.

---

## 13. Replaceability

- **Class registry.** The three Lua class IDs (`0xEA10C4A2`, `0x4B2811AA`, `0x2B2810F7`) are registered through
  GZCOM by `cSCLuaCOMDirector`. A replacement could in principle register its own factory for them. Every user
  creates the VM through `CreateInstance`. No user constructs `cSCLua` directly.
- **Blockers for a different VM:**
  1. Every game C function finds its host through the **4 bytes in front of the `lua_State`** (§1.3). The game's
     extension functions QI that pointer for `cISCLua`. The registry closures call it as a `cIGZLua5Thread` (slot 82
     `GetUpValueIndex`, `ToUserData`, push and table methods). A replacement must keep this layout and give a pointer
     whose QI answers `0xEA10C4A2` and whose vtable is a `cIGZLua5Thread`.
  2. The extension functions are Lua 5.0 `lua_CFunction`s (`int __cdecl (lua_State*)`). They use the stack only
     through `cISCLua`, so a replacement may use another Lua core if it keeps the 5.0 C-function calling contract
     and stack semantics.
  3. Script compatibility needs Lua 5.0 semantics plus the Lua 4 compatibility layer (§3.3–§3.4): global `getn`,
     `dostring`, `tag`/`settag`/`newtag`, the degree-based math globals, and so on.
  4. The process-wide tag counter (§3.4) and the absolute-index read in `NewTag` (§5 slot 59) are observable.
- **Reach-ins.** No code outside the Lua classes reads `cSCLua` memory. The app keeps instance 1 in a member and the
  `SL` global. Users hold `cISCLua*` without AddRef through the global (a cached pointer: a replacement must keep
  instance 1 alive for the app's lifetime).
- **Grids.** The host owns no grids (simgrid §6.4 does not apply).
- **What a FreeCity4 replacement must keep:** §3.1 flags, §3.4 tags, §4.1 reporting, §5 slot semantics including
  the quirks, §7 keys/packages/order, §8 registration and `_tf_`, §9 mirror, §10.2 expression tokens.

---

## 14. Quirks (reproduced)

- Q-1. `NewTag` reads stack index 1, not the top (§5 slot 59).
- Q-2. `GetN` returns its index argument when `getn` is missing or fails (§5 slot 64).
- Q-3. `ToUserTag` dereferences the userdata pointer without a type check (§5 slot 28).
- Q-4. `Error(msg)` ignores `msg`. `Ref(x)` ignores `x`.
- Q-5. `register_api` leaves `_tf_` set. It never fails, but a missing `game` table ends the process (§8.2).
- Q-6. Game C functions called on a coroutine thread dereference null (§1.3).
- Q-7. `print` prints nothing but still raises on a non-string `tostring` result (§8.4).
- Q-8. Registry exchange setters return their last argument. An invalid entry returns an unspecified value (§9.3).
- Q-9. `Open` subscribes to the registry even when the base open failed (§3.1).
- Q-10. Package IDs are ordered as **signed** integers, so an ID ≥ `0x80000000` would load first (§7.4).
- Q-11. `UpdateResourcesFromDirectory` counts files that are present but empty as updated (§7.3).
- Q-12. Script keys are case-sensitive and ignore the directory (§7.1).
- Q-13. The `_dasd_` global keeps the last expression value (§10.2).
- Q-14. `Unref` runs without the precision guard (§4.5).

---

## 15. Open questions

- Q1. Whether the panic function is Lua 5.0's default (which returns, so the process exits) or one set by the host.
  Either way the process ends (§4.2).
- Q2. Lua 5.0 or 5.0.1 (identical version string). Behaviour that differs between them is not relied on.
- Q3. The names of the two script resources `0xFFAEC42B` and `0xFF8085FD` (group `0x4A5E8EF6`). `0xFF8085FD` is a
  loader script that `dofile`s the package scripts. Neither matches a shipped file name.
- Q4. The order of the resource manager's key list (§7.4 step 3), which decides duplicate package IDs. No shipped
  duplicates exist.
- Q5. Whether the default error reporter writes anywhere (§4.3).
- Q6. The meaning of the output parameter ID `0x2B999D2B` that call closures skip (§9.4).
- Q7. Whether the game-data registry replays existing entries to a newly added notification target (§9.1). This
  matters for instance 2 and 3, which subscribe late.
- Q8. The exact text and output of the default reporter (only its parts are known).

---

## 16. Conformance tests (must exist in `engine/conformance`)

1. **T1 CRC24.** `CRC24("123456789") = 0x21CF02` (the RFC 4880 check value). `CRC24("") = 0xB704CE`.
2. **T2 keys.** `ID("adv_const") = 0xFF048A3E`, `ID("package_sys") = 0xFF3500DB`, `ID("package_startup") =
   0xFF53C39E`, `ID("package_advisors") = 0xFF5EEA5A`, `ID("package_tutorial") = 0xFFB144C1`,
   `ID("package_ep1_advisors") = 0xFF2CE858`, `ID("_package_startup") = 0xFF823A6D`, `ID("_adv_sys") = 0xFF81C5A9`,
   `ID("adv_game_data") = 0xFFBF9CCF`, `ID("tutorial_data") = 0xFF263B45`, `ID("_config") = 0xFF3863CB`,
   `ID("_system") = 0xFF85FBC9`, `ID("museum") = 0xFF402430`, `ID("_constants") = 0xFFB0A8D0`.
3. **T3 case and basename.** `ID(basename("C:\\a\\b\\Taxi.lua")) = 0xFF4492E9`; `ID("taxi") = 0xFF9C8B45` (differs).
4. **T4 package order.** Group with headers `0x00004000`, `0x00000001`, `0x00003000`, `0x01001000`, `0x00002000`
   (in any key order) and `firstPackage = 0` → load order `1, 0x2000, 0x3000, 0x4000, 0x01001000`. Adding a package
   `0x80000000` → it loads **first**. Two keys with the same ID → only the first in list order loads.
5. **T5 header rules.** A 23-byte script starting `--#-package:00000001#` is not a package (size must be > 23). A
   24-byte one is. A script that does not start with `--#` is not, wherever its header text is.
6. **T6 firstPackage.** `firstPackage = 0x3000` → `0x3000` loads first, then the rest ascending without it. If
   `0x3000` fails, it loads again in its ascending position.
7. **T7 LoadResourceScript.** Missing → 2. Empty → 0 and nothing runs. Syntax error → 3.
8. **T8 `_dofile_from_resource`.** Existing script → returns `0`. Missing → `nil`. Number argument → `nil`.
   Group comes from `current_resource_group_id_ca636e36`.
9. **T9 register_api.** After `register_api(true)`: `_tf_ == print`. With `game = {}`, after
   `register_api(false)`: `_tf_ == game.window_graph` and `game` has the 44 non-global names. Return is true in both
   cases.
10. **T10 tags.** Fresh process, empty stack: `NewTag()` → 1, then 2. `Tag(untagged table)` → −1. With the number
    `7` at stack index 1 and a non-empty stack, `NewTag()` returns 7 and the internal counter still advances.
11. **T11 GetN.** `GetN` on a non-table → 0. On `{1,2,3}` → 3. With `getn = nil`, `GetN(−1)` on a table → −1 (its
    argument).
12. **T12 print.** `print("a", 1)` → no output, 0 results. With `tostring` replaced by a function returning a
    table, `print(1)` raises an error whose value is that table.
13. **T13 CallProtected.** A chunk that errors → status 1, the message is popped and the reporter receives
    `(thread, 1, message-with-traceback)`. The stack top is back where it was minus the function and arguments.
14. **T14 Update from directory.** A loose `adv_const.lua` with content X → the resource now runs X. A loose
    `brand_new.lua` → no new resource.
15. **T15 detokenizer.** `"A ## B"` → `"A # B"`. `"#city#"` with a fixed value `"Foo"` → `"Foo"`. `"#nope#"` with
    no method → `"nope"`, result false. `"#s@1+2#"` (advisor hook) → `"3"`. `"#p@50#"` → locale number `50`
    followed by `%`. `"#m@1000#"` → the money string of 1000.
16. **T16 registry mirror.** A node `"budget"` under the root and an exchange leaf `"total_funds"` with value 5 →
    `sc4game.budget.total_funds()` returns 5. `sc4game.budget._e` is a light userdata. Removing the node →
    `sc4game.budget == nil`.
17. **T17 Open.** `Open(0x7E)` on an open instance → true, no second registry subscription. `Close()` → true.
18. **T18 precision.** From a 24-bit context, `DoString("x = 1/3")` then `ToNumber` of `x` →
    `0.333333333333333314829616256247…` (a double, not `0.3333333432674408`). After the call the precision is
    24-bit again.

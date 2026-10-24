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
simulation-owned random draws. Lua library randomness remains available (§3.5). It has no save record (§2.2).

### 1.2 Classes and interfaces

| Class | CLSID(s) (registered by `cSCLuaCOMDirector`) | Interfaces answered by QI | Size |
|---|---|---|---|
| `cSCLua` (derives from `cGZLua5`) | `0xEA10C4A2` and `0x4B2811AA`, both with factory `0x008800A0` (new object per call) | `0xEA10C4A2` `cISCLua`; `0x6B2811A6` `cIGZLua5`; `0xEB2810F1` `cIGZLua5Thread` (its main thread); `cIGZUnknown` | `0x58` |
| `cGZLua5Thread` | `0x2B2810F7` (constructor `0x008B8DD0`) | `0xEB2810F1` `cIGZLua5Thread`; `cIGZUnknown` | `0x1C` |
| `cGZLua5CommandGenerator` | `0x2BD13FB4` (constructor `0x008B6480`) | `0x0B90251F`, `0x2BD144DF` | — |

`cSCLuaCOMDirector` (constructor `0x008800F0`, getter `0x00880180`) registers the four class IDs. The factory for
`0xEA10C4A2` and `0x4B2811AA` constructs a `cSCLua` (constructor `0x008809E0`, base constructor `0x008B9670`).

**ScriptServer is a separate registration (LH1).** The four entries above are the Lua director's
registrations, not an exhaustive list of owners using Lua. `cSC4COMDirector` registers CLSID
`0xCBD101A9` with no-argument object factory `0x008B10D0`: the factory pointer is supplied at
`0x0046021A`, the CLSID getter `0x008AE770` returns that value, and AddCls is called at
`0x00460227`. The public SDK independently names it `GZCLSIDDefs::kcGZLuaScriptServer`.
This is neither the service ID `0x4BD10133` nor its priority `0xFFF0BDC1`.

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

There is no single Lua VM. Each `CreateInstance` makes a new Lua state; some native process/thread
state remains shared (§3.5):

| # | Owner and creation | Open flags | Lifetime | Used for |
|---|---|---|---|---|
| 1 | `cSC4App::Init` (`0x0044C320`): `GZCOM CreateInstance(0xEA10C4A2, 0xEA10C4A2)` into an app member, then `Open(0x7E)` at `0x0044C925` | `0x7E` | the app's lifetime; reached through `cISC4App` slot `+0xBC` (`0x0044B670`) and cached by `SL::OnAppInit1` (`0x00602290`) in the global `0x00B43CE8` | advisors, city situations, tutorial, MySim (§11) |
| 2 | `cSC4AutomataScriptSystem::InitScripts` (`0x0051C4C0`): `CreateInstance(0xEA10C4A2, 0xEA10C4A2)`, `Open(0x7E)` | `0x7E` | created in `ExecuteScripts` (`0x00523B10`, from the automata system's `Init`), **closed and released** at the end of the same call | automata configuration tables (group `0x4A5E8F3F`) |
| 3 | `cGZLuaScriptServer::Init` (`0x008B0860`): `CreateInstance(0x4B2811AA, 0x6B2811A6)` (a `cSCLua` reached as `cIGZLua5`), `Open(0x7E)` | `0x7E` | the script server's lifetime (a framework system service added by `cSC4App::AddSystemServices` `0x00450350`) | console scripts run by `cGZMiscCommandSystemSupport::DoMessage` (`0x005C8B30`) through `cIGZLuaScriptServer` |

Instance 1 is opened by the app before anything else touches it. The advisor system's `Open(0x7E)` (advisors.md
§4.1 step 1) is therefore a no-op that returns true (§3.1).

#### 1.4.1 Native ScriptServer acquisition and insertion (LH1)

Application PreFrameWorkInit calls AddCOMDirectors then AddSystemServices (`0x00452831`,
`0x00452838`). The relevant creation/registry order is CommandServer, game-data registry,
ScriptServer, language manager. ScriptServer's sequence is:

1. Zero a temporary and request **CLSID `0xCBD101A9`, IID `0x4BD10129`** through GZCOM
   `+0x0C` (`0x00450813–0x0045082F`). The SDK names this operation GetClassObject; this spec
   also calls it CreateInstance. It creates an acquired interface, rather than finding a service.
2. Only on a false factory result, allocate `0x4C`, construct `0x008B07C0`, store its primary
   pointer and AddRef it (`0x00450831–0x00450858`). This concrete fallback bypasses the factory.
   Its subsequent dereference has no null-allocation guard. Likewise true factory status requires
   a usable output; true-with-null is not handled as a recoverable failure.
3. QI for **`cIGZSystemService`, IID `0x287FB697`**, then on success call framework
   **AddSystemService**, `+0x0C`, and Release that acquired QI view (`0x0045085B–0x00450887`).
   QI failure skips registration. AddSystemService's result is ignored. No Init occurs here.
4. Release the original temporary (`0x0045088B–0x00450891`). The application does not retain
   a ScriptServer member here; the framework holds a successfully registered service.

Framework AddSystemService records signed priority and service ID, retains service references in
its priority and ID collections, rejects duplicate IDs, and does not call Init (`0x0087AC7B–0x0087AD39`,
reference assignment `0x0094909A`). Additions also fail while its enumeration/lookup exclusion flags
are set. GetSystemService finds the **service ID**, then QIs for the requested **IID**
(`0x0087A0C4–0x0087A0ED`). Thus helper `0x004496F0` requests **service `0x4BD10133`, IID
`0x4BD10129`**. It initializes its pointer null; absent framework leaves it null. Successful
acquisition owns a reference released by the smart pointer's destructor. Borrowing its contained
pointer does not extend lifetime.

After application PreFrameWorkInit and framework PreFrameWorkInit hooks, the first service Init
pass precedes PreAppInit hooks and application Init (`0x0087AFDD–0x0087B046`). It visits signed
priorities **greater than `-2000000` in descending order**, calls service `+0x20` Init, and calls
SetServiceRunning(true) only for true status (`0x00879F76–0x00879F9F`). Signed priority insertion
is `0x00879EA2–0x00879EAE`; the snapshot walks predecessors from the maximum
(`0x00879B4C–0x00879B98`, predecessor `0x0040D500`). Therefore ScriptServer priority `-999999`
initializes **before CommandServer priority `-1000000`** (`0x0092AF81–0x0092AF96`), despite
CommandServer's earlier creation/registration. Service acquisition does not imply that service's
Init already ran. This is separate from initialization of the advisor/app Lua VM in §1.5.

**Factory availability and limits.** The registration in §1.2 is native and reachable; an empty
export xref result for the factory is insufficient to conclude otherwise. Director GetClassObject
searches child directors before its own class map, calls the no-argument factory, then QIs its
result (`0x0090DFB3–0x0090E00B`). Successful QI acquires the initially zero-refcount object;
failed QI uses AddRef/Release to dispose it (`0x0090E015–0x0090E023`). GZCOM selects a mapped
loaded library if present, otherwise delegates to the root director (`0x0087BE87–0x0087BED8`).

There is a real **factory-dispatch seam before application service registration**: a factory
already selected at `0x0045082A` can supply the owner. That does not prove that a duplicate CLSID
registered in an arbitrary plugin callback wins dispatch. This trace establishes neither a public
plugin callback guaranteed to precede that request nor a duplicate-class override policy; no safe
callback-based installation recipe is established. Public AddSystemService inserts an absent ID;
it cannot replace an existing ID. Creating another instance by the valid CLSID does not replace
the registered instance. RemoveSystemService followed by AddSystemService after acquisition is
not a supported live-substitution contract: it does not rewrite retained references, transfer scripts
or move the old owner's tick registration. No alternate public retargeting operation is established.

| Caller | Acquisition/caching behavior |
|---|---|
| Misc command support DoMessage | Fresh local service smart pointers at `0x005C9468` and `0x005C975C`; conditional nonnull use, local release obligations |
| Script console / script view | First window Init acquires and copies/AddRefs service into `+0x12C`, releases previous member and temporary; initialized-window Init skips acquisition (`0x008AEDE6–0x008AEE43`, `0x008AF126–0x008AF183`) |
| Script view General | Same retained-service pattern at `+0x128` (`0x008AFA96–0x008AFAF1`) |
| Tutorial registry | If cached Lua is null, temporarily acquires ScriptServer, obtains its Lua and QIs/caches `cISCLua` (`0x004B7731–0x004B77A4`); that cache is a Lua reference, not a service smart pointer |

These acquisitions depend on events; they do not prove every cache predates every plugin callback.
Once a caller retained the service or its Lua, later registry edits leave that old pointer unchanged.
Even before such caching, the owner can already have initialized in the earlier service pass.

#### 1.4.2 Observation boundary: no complete public call observer (LH6)

**No supported record-only installation point captures all three owners before their first
Open/load/registration and observes every later interface call and VM table access.** The public
interfaces in §§5–6 provide operations on the receiver, not an observer-registration operation.
Acquiring or QI'ing an existing receiver does not redirect other holders' calls to the acquirer.
The factory creates a fresh object (`0x008800A0–0x008800E1`); it does not enumerate or attach to
existing objects. The app opens its result immediately (§1.4); automata creation is immediately
followed by Open and extension registration (`0x0051C4E4–0x0051C51F`), and ScriptServer likewise
opens its factory result (`0x008B08A1–0x008B08C0`). App access therefore neither enumerates the
other VMs nor recovers the temporary automata VM after its release. The factory-dispatch seam in
§1.4.1 is a creation/substitution boundary, not an existing-instance observer or a guaranteed early
plugin callback. This answers LH6 negatively; it is not a missing observer slot to invent.

The distinct boundaries must not be conflated:

| Available mechanism | What it observes or changes | Limit |
|---|---|---|
| Retained app Lua or ScriptServer Lua interface | Current state and operations explicitly requested through that view | No historical chunks, other owners, or notification of calls through other retained views |
| Thread OnCreate/OnDestroy (§6.2.1), error reporter (§4.3), ScriptServer control notifications (§6.3) | Their specified lifecycle/error/control events | Not an all-call or table-access observer; changing a callback is itself a state change |
| Lua debug hook | Execution events on the configured Lua context | Not every host method or table access; no pre-Open all-owner installation guarantee. ScriptServer installs its own automatic-yield hook (§6.3.5) |
| Metatable methods | The Lua operations that invoke that metamethod | Ordinary reads of present table fields and raw operations bypass it; installing methods changes script behavior |
| Extension collector plus public raw Lua reads (§8.6) | Ordered collector fields and current callback bindings in the sampled VM | A registration-state sample, not registration history, closure internals or callback execution |

The bypass is concrete: `cISCLua::GetTable` jumps to the thread wrapper, which calls Lua directly
(`0x008803C0`, `0x008B7B68`). VM execution can instead call the table lookup directly
(`0x008C6C28`), using the index metamethod only on a missing value (`0x008C6C32–0x008C6C3B`).
Neither path reports the operation to a retained interface holder. Lua's hook setter replaces the
context's hook fields (`0x008B9770–0x008B97A1`); it is not a multiplexer for external observers.
Consequently the public observation contract here supports the bounded §8.6 comparison only.
Full chunk/call/state replay has no established installation contract under record-only public
access; it remains disabled. A later full-replay design requires a separately justified capture
mechanism and cannot claim completeness from these samples (§16.2).

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

### 3.5 External inputs and replay limits (LH7)

**The original host defines no replay exclusion, recorded-input injection or value-normalization
contract.** Library calls and foreign callbacks execute normally; their results and side effects
are visible to scripts. Time, random draws, pointer identities, registry notifications and table
iteration are not silently discarded or normalized by the host. Thus LH7 does not authorize a
full same-process replay to exclude those differences. Under §1.4.2 full replay remains disabled;
the bounded §8.6 comparison executes none of those libraries/callbacks and has no nondeterminism
exclusions. Deterministic foreign services may supply fixture inputs in standalone conformance
tests **[choice]**, but this is not a native recording/injection API or evidence of full replay.

| Source | Original observable behavior | Consequence for a comparison |
|---|---|---|
| Library time and environment | OpenFlagIO exposes `io`/`os`; the clock wrapper calls the CRT clock (`0x008C3471`), and the environment wrapper calls CRT getenv (`0x008C3450`) | No VM-start clock epoch, frozen environment or substitution of recorded values is established. Files, process environment, locale and date/time facilities are external inputs, not guaranteed repeats |
| `math.random` / `math.randomseed` and compatibility aliases | Random calls CRT rand (`0x008C09FC`); seed calls CRT srand (`0x008C0B32`). Other game functions call that same rand entry point | Not a per-Lua-state random stream: native callers on the same OS thread share CRT state. A second execution can consume or reset that live state; do not reseed/draw on the original as a replay setup or assume matching per-VM seeds isolate it |
| `game.*`, command and registry callbacks | Native functions perform their specified foreign operations; registry get/call returns are converted by §§9.3–9.4 | An output-only recording does not reproduce writes, messages, reentry, errors or scheduling. There is no blanket permission to rerun or replace a foreign callback merely because it has a return value |
| Userdata, light userdata, tables, threads, closures | Identity is observable; registry mirror keys and `_e` retain entry pointers (§9.2), and closures capture pointers | Do not collapse objects to type names, compare unrelated heap addresses by image relocation, or discard aliasing/lifetime. §8.6 compares C callback entry points, not closure/userdata identity |
| Registry notifications | Subscription synchronously replays existing entries, including root, in the specified traversal order (§9.1); subsequent notifications update the mirror (§9.5) | Preserve the ordered entry events and pointer associations. Do not sort, deduplicate, or replace late subscription with an empty starting tree; arbitrary mutation during replay has no snapshot guarantee |
| `next` / `pairs` and foreign output enumeration | Lua table traversal scans array slots then occupied hash nodes in their storage order (`0x008C8F90–0x008C906C`); registry call outputs retain foreign enumeration order (§9.4) | Not a lexical-key order contract. Do not sort return sequences to obtain agreement; equal table contents alone do not prove equal iteration behavior |
| Process-wide Lua 4 tags | Every VM's `newtag` consumes the shared counter (§3.4) | A second VM is independent in its tables, not isolated from all process state. Do not relabel returned tag numbers to hide a differing creation history |

The random-state warning follows actual shared call targets: e.g. the non-Lua shuffle call at
`0x0049CC00` and Lua at `0x008C09FC` both target `0x009EFB9F`. Rand and srand access the
same CRT per-OS-thread data field (`0x009EFB92–0x009EFBC0`), not Lua coroutine-local state.
This does not assert that all game randomness uses that generator. No numeric random-sequence
formula or tolerance is assumed.
Similarly, the iteration rule describes this executable's traversal, not a promise that another
allocator/table construction history yields the same hash-node order. A future full replay needs
its own capture, side-effect isolation and identity/order contract before it can claim agreement;
these are design obligations, not unanswered native observer methods.

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
| 6 | `int32 CreateThread(cIGZLua5Thread**, int32 flags, uint32 threadClassID)` | `0x008B8A00` | |
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

#### 6.2.1 Construction, ownership and opening (LH4)

All offsets in this paragraph are relative to the allocation, not an adjusted interface. A new
`cGZLua5Thread` (`0x008B8DD0`, size `0x1C`) has reference count zero at +04, thread view at +08,
ID `-1` at +0C, null Lua context at +10, null retained Lua5 owner at +14 and null user context
at +18. SetUserContext stores the supplied pointer without ownership. `cGZLua5` embeds that
main-thread state, adds its Lua5 view at +1C, registry at +20 and default error reporter at +3C
(`0x008B9670`). The registry begins empty with no retained owner and next ID zero
(`0x008BD600`); its bucket sizing argument is 1000. AddRef increments the shared allocation
count. Release decrements counts above one; a count of zero **or** one invokes the deleting
destructor and returns zero (`0x0090DA1E`, `0x0090D997`). These are not independent counts for
each view. The thread destructor releases its owner and storage; it does not unregister a live
thread or close a VM (`0x008B9300`).

OnCreate(owner, context, ID) writes the ID and context, retains a changed owner (AddRef new,
then Release old), installs global `_ALERT` as a C closure with no upvalues and returns true
(`0x008B8E20–0x008B8EBF`). It does not set the context's backpointer; the registry does that
after OnCreate succeeds. `_ALERT` reports only a string at the top of the invoking stack, with
error code 1 and that thread; absent owner/reporter or a non-string does nothing, and it returns
zero Lua results (`0x008B8D70`). OnDestroy clears context first, clears/releases owner, sets ID
to `-1`, and returns true; user context survives (`0x008B8ED0–0x008B8F22`). Neither hook itself
removes a registry entry. Hooks on a factory-supplied thread may override these results.

Base Open differs from the game-specific idempotent Open in §3.1: an already non-null main
context makes **base** Open return false. Otherwise it allocates a Lua state and initializes the
registry **before** opening the base library, optional libraries in §3.1's order, the compatibility
library if selected and finally the thread library; success clears the main stack and returns
true (`0x008B88B0–0x008B8998`). Registry initialization retains the Lua5 owner, obtains its
borrowed main-thread view and calls OnCreate with the current next ID. On success it writes
`registry[lightuserdata(mainContext)] = lightuserdata(registryObject)`, writes the thread-view
backpointer at `mainContext - 4`, AddRefs that view and inserts a running main record with no
parent or children, then increments next ID (`0x008BCC50–0x008BCD33`). The first main ID is
**0**. Closing and reopening does not reset next ID, so a later main need not have ID zero.

There is no separate Boolean allocation-failure recovery in registry Init: its false branch is
the OnCreate false branch. Owner retention precedes that hook; false does not release it, undo
hook mutations, close the newly allocated VM, insert/root a record, or advance next ID. Base
Open returns false without cleanup. The concrete hook always returns true; storage exhaustion
is not a documented recoverable false result. A custom false hook that has already assigned
the main context leaves it assigned and a later Open therefore fails the non-null check. A hook
that leaves context null does not cause Open to save the allocated context elsewhere. Do not
infer an atomic rollback or an unconditional retained main context on every failure.

Base Close saves the main context; if non-null, it shuts down the registry and closes that saved
VM, then returns true (`0x008B89B0`). Registry Shutdown repeatedly destroys a present entry
with null requesting context until empty, then clears/releases its retained owner; it does not
reset next ID (`0x008BD630`). Thread OnDestroy invalidates the main view during this process.
The Lua5 destructor closes a still-open VM before destroying the registry (`0x008B9240`). An
open instance holds references to itself through the registry and main-thread owner/root; clients
must Close to release these, rather than expect releasing the external reference to close it.

#### 6.2.2 Child creation, IDs and destruction (LH4)

CreateThread requires a writable output and a function/value at the top of the main stack. Its
second argument is **flags**, not an argument count; the third selects the factory class. It
clears the output before requesting a child, returns signed ID or `-1`, and AddRefs the successful
output for the caller (`0x008B8A00`). The registry creates a Lua coroutine, then moves exactly
two values from the source: the former top value and the newly pushed coroutine object. Success
roots the coroutine in the shared Lua registry under its own context pointer, removes the
coroutine object from the child stack, and leaves the former source top as the child's sole
initial stack value. Lower source values stay in place; initial function arguments are supplied
later by ResumeThread. The initial value is not type-checked here
(`0x008BCD66–0x008BCD75`, `0x008BCE88–0x008BCEB6`).

The factory request is `(threadClassID, IID 0xEB2810F1)`; OnCreate receives the retained Lua5
owner, new context and proposed next ID. A successful factory must return a usable interface.
Only after factory/hook success and any parent validation does creation consume the next ID,
insert the record, write the context backpointer, AddRef the registry-held thread view and root
the coroutine. The factory temporary is released on every normal exit. Factory failure, false
OnCreate or missing required parent returns `-1` with output null; the consumed source top is
**not restored**, no root or record is installed and no compensating OnDestroy is called. The
unrooted child is left for Lua collection. IDs increase in 32-bit storage, with no free-ID reuse
or collision search. Proposed ID `-1` is rejected after increment, not skipped by a search
(`0x008BCD50–0x008BCED2`). Exhaustion/wrap is not a safe reusable-ID policy.

| Creation flags | Registry behavior |
|---|---|
| `0x00000001` | Keep a completed thread registered; absent this bit, a resume ending at Status 0 attempts automatic destruction |
| `0x00000010` | Record the creating context as parent and append the child context to its child list; missing creator backpointer/record fails creation |
| `0x00000100` | Only recorded parent may resume; independent of bit `0x10`, so setting this alone leaves no parent that can satisfy an ordinary resume |
| `0x10000000` | Running marker; initially set for main, set around resume and cleared afterward if the entry still exists |

Other supplied bits are retained without an additional decision in these operations. GetThread(ID)
returns the context backpointer or null, **without AddRef** (`0x008B8A80–0x008B8ADC`). Keeping
that borrowed view across destruction requires an explicit AddRef. This differs from the owned
CreateThread output and from QI.

DestroyThread on the Lua5 view uses the main context as requester. Thread.Destroy uses that
thread's own context; null context returns false. The registry rejects missing ID; for a child
with bit `0x10`, it permits destruction only by itself, its parent, or a null requester used by
Shutdown. Other records have no requester restriction. It destroys children from the end of the
child list first, then requires a backpointer and true OnDestroy on the target. A false hook
stops target cleanup, but does not undo children already removed. Parent removal uses swap with
the last child, not stable ordering. Success removes the Lua registry root, Releases the registry
reference, clears the context backpointer and erases the record, in that order
(`0x008BCEE0–0x008BD02E`). It does not close the Lua coroutine or pop its existing values.
There is no running/main protection check in this destroy path; arbitrary self-invalidation during
execution has no additional safe-return guarantee. Shutdown likewise has no escape for hooks
that repeatedly refuse destruction.

#### 6.2.3 Pause, validity, status and resume (LH4)

Thread.IsValid requires non-null context, that context's backpointer equal to this thread view,
and ID not `-1` (`0x008B6C80`). It is not a registry lookup. Status returns `-1` for null context,
ID `-1` or missing record; a registered running marker gives 2; otherwise an activation frame
or nonempty stack gives 1, and no frame plus empty stack gives 0
(`0x008B6D70`, `0x008BC540–0x008BC5CD`). Thus a new child has Status 1, the ordinary main
has Status 2, and a retained completed child has Status 0. Pause(nresults) forwards to Lua yield
with exactly that result count; it does not maintain the ScriptServer pause counter
(`0x008B6C30`). Operations requiring a Lua context are not made safe by a prior OnDestroy.

ResumeThread(ID, nargs, resultCount*) uses main as the source/destination. A missing ID, running
thread, or parent-only restriction violation pushes one error string and returns internal code 1,
without moving the supplied arguments. The respective texts are `Resuming Lua thread with invalid
ID: %d.`, `Lua thread %d is already running.`, and `Lua thread %d is only resumable by its parent.`
Otherwise it moves nargs values to the child, marks it running, invokes Lua resume, clears the
marker if still registered, moves **all** child stack values back to the source, then attempts
automatic destruction when mask `0x1` is absent and Status is 0. Both yield and normal completion
with Lua resume code zero count as success; runtime errors return false. The host does not
translate error codes into thread-status values (`0x008BD040–0x008BD16E`).

The public wrapper returns true exactly for internal code zero. On nonzero code with a reporter,
it reports `(current borrowed thread or null, code, main top converted to string)`, substituting
`<unknown description>` for a null conversion. Automatic destruction can therefore make the
reported thread null. If initial main top is T and final main top is U, the reported count is
**U − (T − nargs)**, including on failure. A non-null count output leaves those stack values;
null count output pops that many. The count is not clamped: rejected resumes do not consume
arguments, so the count includes those retained arguments as well as the error. For example,
T=3 and nargs=2 with invalid ID gives U=4 and count 3; null output removes all three and leaves
the original lower value. Reporter invocation precedes count measurement and popping
(`0x008B8B59–0x008B8C16`). Counts and IDs use 32-bit integer arithmetic, not floating arithmetic.

### 6.3 `cIGZLuaScriptServer` (vtable `0x00AC8710`, 25 slots)

`cGZLuaScriptServer` owns the framework scripting scheduler, script records, breakpoint lists and control
notifications. Lua compilation/execution and command execution are foreign services; the decisions below
belong to the server. This is instance 3 in §1.4. Evidence is the Windows 1.1.641 export, with addresses below;
decompiler types are not interface declarations. REA was unavailable during the 2026-10-22 audit (no open
target), so instruction queries and the existing class decompilation supply the evidence.

#### 6.3.1 Owner inventory and interface views

The export has 40 names containing `cGZLuaScriptServer`, totalling 6,724 body bytes, including the factory,
class-ID getter, adjustment thunks and the misnamed 71-byte map-find helper `0x006C0030` (already corrected
in `re/symbols/lua-host.tsv`). The existing attribution pass instead assigns **51 functions / 7,700 bytes**
to this owner. All 51 were audited, including the attribution artifact described below. Shared/folded methods
and foreign helpers were inspected separately; the two totals measure different sets. No coverage promotion
or attribution-table change follows from this audit.

Named-body inventory, descending size (bytes):

| Entry | Bytes | Responsibility |
|---|---:|---|
| `008B2540` | 862 | Build a thread's `arg` table |
| `008B32B0` | 665 | RunScriptString |
| `008B2380` | 441 | OnTick, entered through the service view at object +4 |
| `008B13A0` | 408 | EnumerateScriptIDs |
| `008B0860` | 372 | Init |
| `008B42C0` | 277 | Find/create breakpoint entry |
| `008B1180`, `008B1290` | 270, 265 | PauseScript, ResumeScript |
| `008B1A80`, `008B1E00`, `008B22B0` | 243, 206, 194 | RunScriptFile, ShouldBreak, KillScript |
| `008AED10`, `008B51A0`, `008B4210` | 171, 166, 164 | Global argument setup, set condition, breakpoint enumeration |
| `008AEBC0`, `008B4EC0`, `008B4F60`, `008B5070` | 152, 150, 147, 146 | Script notification, set/toggle existence, set enabled |
| `008B01C0`, `008B0410`, `008AEC60` | 145, 143, 140 | Shutdown, GetScriptInfo, breakpoint notification |
| `008B5110`, `008B07C0`, `008B5000`, `008B1B80` | 132, 125, 111, 107 | Get condition, construction, get enabled, remove entry |
| `008B4E60`, `008AE850`, `006C0030` | 93, 86, 71 | Existence, yield hook, signed-ID map lookup |
| `008B04E0`, `008B04A0`, `008AE7D0` | 55, 49, 44 | Set/get auto-yield, QI |
| `008AECF0`, `008B10D0`, `008AE810`, `008AE830` | 24, 24, 21, 21 | Current ID, factory, Release, destruction dispatch |
| `008AE800`, `008B10C0`, `008B0400`, `008AE770`, `00A6CAB0` | 8, 8, 8, 6, 4 | AddRef, Init/Shutdown adjustments, class ID, GetLua |

Additional attributed bodies, also audited:

| Entry | Bytes | Responsibility |
|---|---:|---|
| `008B3200`, `008B1540`, `008B0320` | 170, 149, 126 | Record construction; unlink/destroy/free one breakpoint node; plain server destruction |
| `008B0520`, `008AE8B0`, `008B2250` | 111, 108, 93 | Default breakpoint payload; global `arg` lookup callback; record destruction |
| `008B1D00`, `008AFCF0`, `008AFD40` | 90, 79, 51 | Destroy breakpoint-list sentinel; map iterator post-increment; recursive map-node freeing |
| `00677261` | 42 | False reach attribution through the bytes of the string `arg`, **not a server callback** |
| `008B0840` | 30 | Server deleting destructor |
| `008B03A0`, `008B03B0`, `008B03C0`, `008B03E0`, `008B03F0` | 8 each | +4 QI/Release; +8 QI/Release/deleting-destructor adjustments |

The attribution total excludes the named map-find, factory, class-ID getter and folded AddRef/GetLua above;
it also excludes shared `0x006AAC50` and +8 AddRef adjustment `0x008B03D0`. It includes the string artifact:
the sole exported reference to `0x00677261` is data at `0x00AC77E8`, whose actual bytes are `arg` plus NUL
(little-endian value `0x00677261`). `0x008AED2F` passes that address to PushString. There are zero direct
calls to the alleged function; it begins in a misdecoded instruction fragment. Thus 50 attributed bodies /
7,658 bytes carry actual owner behavior, with 42 bytes reconciled as an export/attribution artifact. Do not
implement an extra callback from that address. Shared list helpers `0x008B02B0`, `0x008B15E0`,
`0x008B17C0`, `0x008B1F30` construct/clear/copy/allocate breakpoint storage; the latter's exported
`cRZString::IsEqual` name is incorrect for this allocator, as its instructions show.

The primary interface has slots 0 QI, 1 AddRef, 2 Release, 3 Init, 4 Shutdown, 5 GetLua,
6 RunScriptString, 7 RunScriptFile, 8 KillScript, 9 PauseScript, 10 ResumeScript, 11 GetBreakpointExists,
12 SetBreakpointExists, 13 ToggleBreakpointExists, 14 GetBreakpointEnabled, 15 SetBreakpointEnabled,
16 GetBreakpointCondition, 17 SetBreakpointCondition, 18 GetBreakpointEntryList, 19 EnumerateScriptIDs,
20 GetScriptInfo, 21 GetCurrentRunningScriptID, 22 GetScriptAutoYieldEnabled, 23 SetScriptAutoYieldEnabled,
24 GetLastCreatedScriptID. Slot 25 is destruction dispatch, outside the 25-slot public interface.
The service view is object +4, vtable `0x00AC86DC`, with Init/Shutdown adjustment entries
`0x008B10C0`/`0x008B0400` and OnTick at slot 10. The further view at +8 has vtable `0x00AC86C4`.
Offsets in subsequent tables are relative to the primary object, unless explicitly called a record offset.

#### 6.3.2 Owned state, lifetime and initialization

| Object offset | Meaning and initial value |
|---|---|
| `+20` | Reference count, zero |
| `+24` byte | Init latch, false |
| `+28` | Tick recursion depth, zero (OnTick receives object +4) |
| `+2C` | Attempt serial for generated display names, zero |
| `+30` | Last successfully created thread ID, `-1` |
| `+34`, `+38` | Owned Lua5 and command-generator interface references, null |
| `+3C`, `+40` | Signed 32-bit script-ID ordered map and its count, empty |
| `+48` | Borrowed current-running record, null |

Construction `0x008B07C0` initializes the base system service with **service ID `0x4BD10133`** and
**signed priority `0xFFF0BDC1` (`-999999`)**, installs the three interface views and
allocates the empty map sentinel. These are the first and second constructor arguments, respectively
(`0x008B07C5–0x008B07DA`). The base constructor stores the first as service ID and the second as both
service priority and initial tick priority (`0x0091DE23–0x0091DE37`); the public SDK constructor
`cRZBaseSystemService(uint32_t serviceID, int32_t servicePriority)` agrees. Neither argument is a
CLSID; the actual CLSID is `0xCBD101A9` (§1.2).
QI `0x008AE7D0` recognizes `1` and `0x4BD10129`, writes the primary pointer and calls AddRef;
other IDs delegate to the base service. AddRef increments the unsigned 32-bit count at +20.
Release `0x008AE810` decrements only when the unsigned count exceeds one; count zero or one both
dispatch slot 25 and return zero. Destruction dispatch `0x008AE830` calls virtual Shutdown before the
deleting-destructor path through the +8 view. There is no extra Release-to-zero Shutdown after that dispatch.
The +8 slot-3 adjustment subtracts eight and enters `0x008B0840`: run plain destruction `0x008B0320`,
free the primary allocation only when the caller's flag has bit zero set, then return its address. Plain
destruction reinstalls the three server vtables, frees map nodes and sentinel, releases remaining generator
then Lua references, and invokes base-service destruction on +4. It does not itself KillScript or Close;
the normal slot-25 path already performed Shutdown. Factory `0x008B10D0` allocates `0x4C` bytes and
constructs the object, returning null on allocation failure; its reference count is still zero.

Init `0x008B0860` tests and sets the latch **before** doing external work. With a clear latch and null Lua:

1. Create (`0x4B2811AA`, `0x6B2811A6`) into the owned Lua reference; on a successful factory result call
   Open(`0x7E`). An Open failure clears the reference before releasing it. A null Lua means false return.
2. Create (`0x2BD13FB4`, `0x2BD144DF`) into the owned generator reference, first clearing/releasing any old
   generator. On factory success call generator +10 with the temporary reference obtained from the global
   command-system accessor, the Lua pointer, and string `sc4app`, in that argument order. If that call fails,
   clear/release the generator. A null generator closes the Lua, clears/releases it and returns false.
3. If initialization reaches this point, register object +4 with framework +24, if a framework exists.
   A preexisting nonnull Lua skips both creation steps but still reaches registration.
4. Every Init reaching the common tail, including repeated Init with a set latch, calls global argument setup
   `0x008AED10`; its result is the Init result. That setup returns false on null Lua.

The latch stays set after failure (`0x008B087F`, `0x008B09A9`). Repeating Init does **not** retry factories;
Shutdown then Init permits retry. Missing framework alone does not fail Init. Boolean results use AL;
unspecified upper register bits are not meaningful status fields.

Shutdown `0x008B01C0` always returns true. With a clear latch it does nothing. Otherwise it clears the latch
first, unregisters object +4 through framework +28 when available, then repeatedly calls virtual KillScript
on the smallest current signed ID until the map is empty. It obtains generator +0C's returned interface and
calls that interface +10, clears/releases the generator, closes the Lua, and clears/releases the Lua, in this
order. It does not reset last-created ID or the attempt serial. Clearing pointers before Release is observable
to callbacks. Shutdown has no independent map snapshot or recursion guard.

The server's QI returns primary `P` for IID `1` and `0x4BD10129`; base-service QI returns `P+4`
for IID `0x287FB697`, acquired in either case (`0x008AE7D0`, `0x0091DD67`). The `P+8` base
unknown/destruction subobject is not a second public ScriptServer identity: server QI for IID 1
returns `P`. Native adjustment offsets describe this allocation, not a required replacement layout.
Registry insertion is §1.4.1. Init's framework `+0x24` is **AddToTick**, and Shutdown's `+0x28`
is **RemoveFromTick**, not registry insertion/removal; their results are ignored
(`0x008B09BB–0x008B09C3`, `0x008B01D2–0x008B01E8`).

##### Foreign command endpoint identity (LH2)

The temporary constructed at `0x008B091D` is **a CommandServer service reference**, despite the
export's misleading `cRZAutoRefCount<cIGZLua5>::operator=` name for `0x00562DA0`. Its instructions
zero the output, get the framework and call `GetSystemService(0xEB903A32, 0xCB90253A, out)`
(`0x00562DA3–0x00562DC1`). No framework leaves it null. The public SDK's `GZServPtrs.h` declares
`cIGZCommandServerPtr` with those IID/service-ID template arguments; `cRZSysServPtr.h` acquires
through `RZGetFrameWork()->GetSystemService` and releases a nonnull reference on destruction.
Thus acquisition needs no fixed-address accessor and no invented CommandSystem API.

All offsets below are byte offsets on 32-bit Windows interface vtables, with `this` in ECX.
Object/interface pointers and string-reference arguments are 32-bit; Boolean status is in AL.

| Receiver and slot | Typed native boundary | Ownership |
|---|---|---|
| Lua-generator IID `0x2BD144DF`, `+0x10`, `0x008B6750` | Init taking `cIGZCommandServer*`, `cIGZLua5*`, `const cIGZString&`, returning bool; callee pops 12 argument bytes | Inputs are borrowed for the call; the caller owns the temporary CommandServer acquisition, Lua and the temporary string |
| Same receiver, `+0x0C`, `0x008B59B0` | AsIGZCommandGenerator taking no arguments, returning `cIGZCommandGenerator*` | Borrowed sibling view; no AddRef and no matching Release |
| Returned CommandGenerator IID `0x0B90251F`, `+0x10`, `0x008B68D0` | Shutdown taking no arguments, returning bool | Does not consume the returned view; ScriptServer ignores this status and releases its owned Lua-generator reference next |

The two generator views are distinct: QI returns allocation +`0x0C` for `0x2BD144DF`, allocation
+`0x08` for `0x0B90251F`, and AddRefs either successful query (`0x008B59C0–0x008B5A01`).
The Lua-generator vtable is `0x00AC8BB0`; its sibling vtable is `0x00AC8BC4`. AsIGZCommandGenerator
subtracts four from its receiver and returns immediately; it does not query or acquire anything
(`0x008B59B0–0x008B59BD`). It must not be confused with the generic generator's `+0x0C` Init(),
which has no three-argument signature. The public SDK supplies `cIGZCommandGenerator` with
Init/Shutdown at `+0x0C`/`+0x10`; a declaration of the Lua-specific generator remains separate
from that public interface.

The three Init arguments are, in order, the temporary's contained CommandServer pointer, the
owned Lua5 pointer, and a reference to the freshly constructed `cRZString("sc4app")`
(`0x008B0911–0x008B0945`). After the call the CommandServer temporary is destroyed/released
(`0x008B0954`, destructor `0x00562DD0`) and the string is destroyed (`0x008B096A`), before a false
result clears/releases the generator. Neither temporary is transferred to ScriptServer's state.
The actual generator checks string length first, then null Lua and null CommandServer, and
returns false for those invalid inputs (`0x008B6751–0x008B677A`); a null string-reference address is
not a supported input. It registers the borrowed generic view through public
`cIGZCommandServer::RegisterCommandGenerator`, `+0x3C` (`0x008B678E–0x008B6796`).

After registration, the generator obtains the Lua's **borrowed main-thread view** using
`cIGZLua5::AsIGZLua5Thread`, `+0x0C`. On success it retains that thread and the CommandServer
with AddRef assignments and copies the string into its own string member
(`0x008B679C–0x008B67AD`, `0x008B6810–0x008B682D`, assignment `0x005447B0`, string assignment
slot resolving to `0x0090E230`). It does not take ownership of the supplied Lua pointer or string
storage. CommandServer RegisterCommandGenerator retains the generic view in its own collection
(`0x0092AB0B–0x0092AB68`); that is independent of both the borrowed sibling conversion and the
factory/QI reference owned by ScriptServer. Owner service lifecycle and these foreign-host
references are distinct.

Generator Init accepts the named Lua global if a table, creates it if nil, otherwise unregisters
its generic view and returns false (`0x008B67AF–0x008B6810`, `0x008B6847–0x008B6876`). Failed
command registration also returns false. True status follows the setup in §6.3.2.1 (`0x008B68BE–0x008B68CE`)
and does not promise later command success. The owner handles false by the clear/release and
Lua Close sequence above. A false factory result is followed by a null-pointer test; this does
not validate a factory reporting true with null output.

Generic generator Shutdown invokes CommandServer **UnregisterCommandGenerator**, `+0x40`,
records that Boolean result, clears/releases its retained CommandServer, main thread and parameter
sets, then clears working collections (`0x008B68D5–0x008B6931`, `0x008B698C–0x008B6992`).
Without a retained CommandServer, status starts true. ScriptServer ignores the status, clears/releases
its owned generator, calls Lua Close and clears/releases Lua (`0x008B0208–0x008B024A`). No separate
Release applies to the borrowed sibling. There is no null check between AsIGZCommandGenerator
and its returned view's Shutdown dispatch: the view must be usable while the generator is alive.
Neither method consumes its receiver. Temporary CommandServer lifetime ends after Init;
generator-retained endpoints survive it until Shutdown/destruction. Shutdown removes registration
before the owner releases its generator reference.

**Public binding result.** CommandServer acquisition/register/unregister, Lua5's main-thread
accessor and generic CommandGenerator Shutdown have public SDK declarations. No CommandSystem
accessor is needed. The inspected SDK include tree lacks the Lua-specific IID `0x2BD144DF`
declaration; the two extra slots above are a native contract recovered from vtable words and method
bytes. A typed declaration must preserve the three Unknown slots, AsIGZCommandGenerator and
three-argument Init. Generic CommandGenerator Init cannot stand in for it. This foreign boundary
does not solve the owner insertion-timing limit in §1.4.1, establish native activation, justify
fixed-address calls or permit using the original ScriptServer as a scheduler/adapter.

##### Independent boundary discriminators (LH1/LH2; specified, not executed)

| Discriminator | Required observation |
|---|---|
| Class/service/priority roles | Factory uses `CBD101A9/4BD10129`; lookup uses `4BD10133/4BD10129`; signed priority is `-999999`. Looking up `FFF0BDC1` must not find this service. |
| Creation versus initialization | Selected factory request, service QI, AddSystemService and temporary Releases precede owner Init. Duplicate service-ID insertion fails; application ignores that result. |
| Foreign acquisition timing | ScriptServer Init acquires the registered CommandServer before that service's own Init; acquisition must not manufacture Init. |
| Cache timing | Command operations acquire anew; initialized windows retain old service, tutorial retains old Lua. Registry changes must not retarget those references. |
| Missing generator and retry latch | First failure closes/releases Lua, returns false, leaves latch set. Restoring the factory then repeating Init runs argument setup only. Shutdown then Init permits factory retry. |
| Missing CommandServer | With valid Lua/string and present generator, pass null CommandServer to generator Init; it returns false. Temporary cleanup precedes generator release and owner Lua Close/release. |
| Borrowed versus acquired | AsIGZCommandGenerator adds no reference; explicit QI for `0B90251F` does. Do not Release the conversion result separately. |
| Temporary inputs | Release temporary CommandServer and destroy `sc4app` string immediately after generator Init. Successful retained thread/server references and string copy survive that cleanup. |
| Shutdown and failure status | RemoveFromTick, KillScript loop, borrowed-view Shutdown, clear/release generator, Lua Close, clear/release Lua. False foreign Shutdown does not stop cleanup or change owner's true result. Second owner Shutdown with clear latch has no foreign calls. |

##### 6.3.2.1 Command-generator construction and initialization (LH5)

The generator owns the Lua binding of CommandServer commands, rather than implementing the commands.
Construction (`0x008B6480–0x008B6520`) starts reference count zero, null main-thread/server/input-set/
output-set references and an empty global name. It constructs 500 default command records, an empty
free-index list and an empty ID-to-index map (bucket sizing argument 100). These records are not 500
registered commands. Successful specialized Init clears the logical record array and both working
collections. Reference counting is the shared Unknown rule in §6.2.1.

The generic view has exactly these operations after Unknown. Vtable words at `0x00AC8BC4` establish
their order; the exported name of the generic Init body belongs to a folded unrelated class.

| Slot | Contract | Body |
|---|---|---|
| 3 | Generic Init(), no arguments: false, no setup | `0x00648EF0` |
| 4 | Shutdown(), bool | `0x008B68D0` |
| 5 | CommandRegistered(commandID, name string reference, description string reference), bool | `0x008B6A00` |
| 6 | CommandUnregistered(commandID), bool | `0x008B5A40` |

The three-argument Lua-specific Init remains the entry specified above. It registers the generic view
before obtaining/retaining the main thread. The global lookup and one-value pop leave the previous
stack depth unchanged; a nil global is replaced by a new table, and an existing table is preserved.
Once accepted, retain the main-thread view and the actual **CommandServer argument**, copy the name,
and allocate/construct two independent `0x28` CommandParameterSets, retained as input and output.
Then clear the record array, free-index list and ID map and return true
(`0x008B6810–0x008B68CE`). There is no subsequent command-enumeration call, callback installation,
or initialized latch in this tail. Commands arrive through registration notifications. Registration's
foreign callbacks occur at the earlier call boundary; no promise of deferred notification or readiness
during a reentrant callback is added by this Init.

There is no null check of the borrowed main view or of the parameter sets after assignment. A failed
parameter-set allocation stores null and Init still returns true; later command callbacks require usable
sets. Neither a successful Init result nor generic Init supplies a failure-recovery allocation policy.
Invalid inputs or failed registration return false before endpoint/name/set mutation. Invalid global
type additionally invokes UnregisterCommandGenerator and returns false, still without replacing
previously retained state. That unregistration result is ignored.

Repeated specialized Init has no already-initialized shortcut: it re-registers, revalidates the new
global, replaces retained references/name/sets on success and clears bookkeeping. It does not first
unregister a previous server or remove bindings from an old global table. Even using the same server
does not repopulate records by enumerating its commands in the success tail. Callers that need a
clean registration lifecycle use Shutdown; do not infer transactional replacement on repeated Init.

##### 6.3.2.2 Command notifications and generated callbacks (LH5)

Each record contains a command ID, name and description. The map selects its array index. A free
index is reused from the front of the free-index list. If none exists, append 20 default records and
make those indices available in ascending order, then take the first. A new CommandRegistered copies
both strings and installs a C closure under `globalTable[name]`, capturing **index as a number**,
**generator allocation pointer as light userdata**, and **retained main-thread view pointer as light
userdata**, in that order (`0x008B6A00–0x008B6C28`). It saves and restores the main stack depth.
If the global has ceased to be a table, return false after record/map insertion; there is no rollback
of the consumed index or strings. For an existing ID with a valid matching record, return the stored
name's equality result with the supplied name (comparison flag 1); do not update the description,
replace a closure or inspect the global. Name collisions between different IDs overwrite the same
Lua table field; the map remains keyed by ID.

CommandUnregistered requires a map entry, in-range index and matching record ID. Missing/stale entries
return false. Otherwise push the index onto the **front** of the free list, erase the map entry,
look up the current global, and if it is a table assign nil at the stored name. Pop the lookup value
and return true, even if the global was not a table (`0x008B5A40–0x008B5B62`). The array record
and its strings remain until reuse/clear. Removal does not test that the table field still contains
this generator's closure: it can erase a caller replacement or a colliding command's binding.

The closure (`0x008B5B70–0x008B5E6A`) obtains the invoking context's thread backpointer. It accepts
its captured generator only if non-null with a retained server, the captured main-thread pointer equals
the generator's current retained view, and the captured index converted to integer is nonnegative and
within the record array. It does **not** check the map, record ID against the originally registered
ID, or whether this invocation is on the captured main thread. A saved Lua closure can therefore
execute a removed record or, after index reuse, the new command occupying that index. The light
userdata captures acquire no reference; calling a closure after generator destruction has no valid
lifetime contract.

On an accepted call, clear both parameter sets with Clear(0), then add each Lua argument under keys
0, 1, …, preserving argument order. Conversion is by exact Lua type:

| Lua argument | Input variant |
|---|---|
| Boolean | Bool |
| Number | Float64; no intermediate Float32 |
| String | Copied framework string |
| Nil, table, function, userdata, light userdata, thread, any other type | Float64 zero |

Dispatch CommandServer `+0x50` with `(record.commandID, inputSet, outputSet)` and push its signed
32-bit status converted to a Lua number as the **first** result. Do not use status as a Boolean
or suppress output on failure. Enumerate output keys using First/Next (`+0x34/+0x38`) to sentinel
`0xFFFFFFFF`; skip key `0x2B999D2B`. For each other key push nil if lookup yields null; otherwise
variant type 1 becomes Boolean, types 2 through 14 become a Lua number via Float64 conversion,
and all other types become a string through the variant's string conversion. Enumeration order
is the parameter set's order; no numeric-key sorting occurs here. Return one plus the number of
non-skipped entries. No output keys means the sole result is command status. A failed closure
validation instead returns exactly one Lua number **2**, without executing or clearing sets.
Numeric widths are verified by double stores at `0x008B5CC5`, `0x008B5D31`, `0x008B5DAB`;
the fallback double 2 is at `0x00A80AC8`, loaded at `0x008B5E40`.

##### 6.3.2.3 Cleanup and failure boundaries (LH5)

Shutdown unregisters the generic view from the retained server and preserves that bool as its
result (true with no retained server). It then clears/releases server, main thread, input set,
output set, and clears records, free list and ID map, even after false unregistration. Global
name survives. It does **not** enumerate the records to remove Lua bindings or delete the global
table. Any binding removals caused by foreign unregistration notifications belong to those calls;
Shutdown itself does not synthesize them (`0x008B68D0–0x008B6992`). Repeated Shutdown with no
retained server returns true and repeats empty collection clearing.

The destructor (`0x008B61C0`, deleting dispatch `0x008B6530`) destroys name/storage and releases
remaining server, output set, input set and main-thread references. It does **not** call Shutdown
or UnregisterCommandGenerator. Releasing an external reference does not force destruction while
the CommandServer still retains registration. Allocation failures, destroyed captured pointers and
arbitrary recursive calls sharing the two working sets have no extra recovery/reentrancy guarantee.
These are separate failure boundaries from the ordinary false returns above.

#### 6.3.3 Control and inspection

Each record (`0x008B3200`, size `0x54`) owns a thread reference at +00, source/name/argument strings at
+04/+18/+2C, a signed pause count at +40 initially zero, resume-transition counter +44 initially zero,
schedulable byte +48 false, auto-yield byte +49 false, last yielded source line +4C initially `-1`, and a
breakpoint list at +50. These are per-script values, not shared Lua state. Record destruction `0x008B2250`
destroys the breakpoint list and argument/name/source strings, then releases a nonnull thread reference.

PauseScript(id) returns zero for an absent ID. Otherwise increment the signed 32-bit pause count and, only
when the new value is one, post script message `0xEBAAF9BE` with payload one; return the current count
after notification (`0x008B1180`). ResumeScript(id) decrements with no zero clamp; only a new value zero
posts that message with payload zero, then increments the resume-transition counter; return the current
pause count (`0x008B1290`). The counter increment is after notification. Both methods accept `-1` as an
all-scripts selector: enumerate a temporary ID snapshot, invoke their own virtual single-ID slot for each
snapshot element, discard individual results, free the snapshot and return one even when empty.
KillScript has an ordinary map lookup, with no such all-scripts special case.

EnumerateScriptIDs (`0x008B13A0`) **appends** every map key in ascending signed-ID order to the supplied
32-bit element vector. It does not clear existing elements and has no success/failure return contract.
GetScriptInfo (`0x008B0410`) takes an ID and six optional outputs, in order: thread pointer, source string,
name string, pause count, resume-transition counter, schedulable byte. Missing ID returns false without
touching any output. Present ID returns true and writes only nonnull outputs in that order; returning the
thread calls AddRef. String outputs receive copies. The last byte output is **not auto-yield**.

GetCurrentRunningScriptID (`0x008AECF0`) returns the current record's thread +28 result, or `-1` if either
record or thread is null. GetLua (`0x00A6CAB0`) simply returns +34, without AddRef. GetLastCreatedScriptID
(`0x006AAC50`) returns +30 even after the corresponding script has died. GetScriptAutoYieldEnabled returns
the found record's +49 byte, or false on a miss. SetScriptAutoYieldEnabled writes that byte and returns true
on a hit, false on a miss (`0x008B04A0`, `0x008B04E0`); it does not change scheduling eligibility or notify.

Independent state/result examples (foreign calls are recorded, not executed):

| Input state and operation | Required result/state |
|---|---|
| Fresh server; Lua factory fails; Init twice | Both false; one Lua factory attempt; latch remains set |
| Then Shutdown; successful Init | Retry permitted; Lua Open then generator initialization then registration then argument setup |
| Record 7 count 0; Resume(7) | Return `-1`; no notification; transition counter unchanged; count `-1` |
| Then Pause(7), Pause(7), Pause(7) | Returns `0,1,2`; only middle call notifies payload 1 |
| Count 2, transitions 4; Resume twice | Returns `1,0`; second notifies payload 0 before transitions becomes 5 |
| Existing output vector `[99]`, IDs `{8,2}`; enumerate | `[99,2,8]` |
| Missing ID; info outputs prefilled; auto-yield get/set | Info false with all outputs untouched; get false; set false |
| GetLua followed by GetScriptInfo with thread output | Lua is borrowed; thread output owns one added reference |

#### 6.3.4 Creation, IDs, foreign execution and tick decisions

RunScriptString consumes three string-interface inputs: source, chunk name, command-line arguments
(`0x008B32B0`, stack cleanup 12 bytes). There is no auto-Init, null-input check, or execution in this method.

1. Allocate and construct a `0x54` record; increment the 32-bit attempt serial at object +2C **even if
   compilation will fail** (`0x008B32DB`). Copy source and name, set eligibility false and last line `-1`.
   Allocation failure is unchecked before record dereference, not a defined `-1` recovery path.
2. Obtain the main thread through Lua +0C. Call its LoadBuffer (+134) with source bytes, source byte length,
   and chunk-name C string. Only integer result zero is success. On nonzero, Pop(1) from the main Lua stack,
   release any record thread after clearing its pointer, destroy/free the record and return `-1`.
3. On success, obtain a display-name component from the chunk name. When FileExists(name) is false, use
   the name unchanged; when true, use GetPathName(name, output, 0). Format `Main - "<component>" - <serial>`
   with signed decimal `%d`. On the main stack, push this name, duplicate the compiled value at -2 and
   RawSet(globals, `-10001`): the named global receives the compiled function while the original function
   remains on the stack (`0x008B33E8–0x008B3403`). No rollback of this global is attempted on later failure.
4. Clear/release any old record thread, then call Lua CreateThread (+18) with the **address of the record's
   thread field**, integer `1` and `0x2B2810F7`. This foreign call supplies the script ID; the server's display
   serial is **not** its ID allocator. Only ID `-1` is rejected. On rejection, clear/release any returned thread,
   destroy/free the record and return `-1`. Source/name temporary strings are freed on both paths.
5. For an accepted ID, call thread SetUserContext (+18) with the record address; write last-created ID;
   copy arguments; build the thread argument table (§6.3.5), ignoring that helper's Boolean result; set
   eligibility true; look up the ID in the signed map. Insert `(ID, record)` only when absent. Then obtain
   the thread's raw context (+10), install `LuaYieldHook` with Lua hook mask `4` and count `0`, and finally
   post creation message `0x6BA990D8`, payload zero. Return the supplied ID.

The native duplicate-ID branch clears/releases the **new** record's thread and then unconditionally
dereferences it to install the hook (`0x008B3496–0x008B34AB`). It is an invalid foreign-service outcome,
not a supported replace/retry case. The map's old entry survives that branch up to the fault. A valid VM
service must return distinct live IDs and a nonnull owned thread on successful creation. No server retry
or transactional rollback is present. Main-stack effects beyond the explicit operations above are those
of the foreign LoadBuffer/CreateThread calls; this method does not save/restore the original stack height.

RunScriptFile (`0x008B1A80`) takes filename and arguments. It copies the filename bytes to a temporary
path string and asks ReadFileIntoString to fill a temporary source string with limit argument **1,000,000**.
If that service fails it returns **zero**, leaves the attempt serial/last-created ID unchanged, and does not
call RunScriptString. Otherwise it calls its own virtual slot 6 with `(source, original filename, arguments)`
and returns that result unchanged, freeing temporaries afterward. The one-million argument is the server's
request, not a separately established truncation/oversize policy for the foreign file reader.

OnTick (`0x008B2380`) receives the service pointer at primary +4. It increments the 32-bit recursion depth
at primary +28; only new depth one performs a pass. Nested calls decrement the depth and return true with
no scheduling. An empty pass also decrements and returns true. The pass traverses the **live** map in
ascending signed-ID order:

| Record/state | Owner decision |
|---|---|
| Signed pause count >0, or eligibility byte zero | Skip entirely, including status checks and cleanup |
| Eligible, count ≤0 | Read thread Status (+34) |
| First status =1 and ShouldBreak false | Set current record, call Lua ResumeThread (+20) with `(thread.GetID(), 0, null)`, clear current record |
| First status =1 and ShouldBreak true | Do not resume; no pause count, transition count or notification changes |
| Any eligible path | Read Status **again**, independently of first result and ignored ResumeThread result |
| Second status =1 | Retain record and advance |
| Second status ≠1 | Post termination `0xCBA990D3`, payload zero; if second status =0 also call Lua DestroyThread (+1C) with thread.GetID(); clear/release record thread; destroy/free record; erase map node and decrement count |

ResumeThread is always on the server's Lua, not the generator. Its Boolean result does not decide retention.
For the foreign execution service the input is `(ID,0,null)`; a test supplies its result and subsequent Status.
The server itself decides whether to call it, the map order, current-ID scope, and termination cleanup.
Negative pause counts are runnable. A paused dead script remains enumerable until unpaused or explicitly
killed. Successful returns from the pass always restore recursion depth to its incoming value.

KillScript (`0x008B22B0`) returns false on a map miss. On a hit it calls Lua DestroyThread(ID) **first**,
posts termination using the record thread's GetID (not necessarily the lookup key under a faulty foreign
service), clears/releases the thread, destroys/frees the record, erases its map node, decrements count and
returns true. It does not inspect Status or pause count. Unlike tick completion it destroys the foreign thread
regardless of Status. Neither kill nor completion resets last-created ID or removes the display-name global.

Only OnTick's ResumeThread interval sets current record. In creation/control calls and the tick's terminal
notification/cleanup it is normally null. The server does not hold an extra thread/record reference around
foreign execution or notification calls. Nested OnTick is specifically suppressed, but other callbacks are
not serialized, deferred, or protected by the depth counter. Map traversal is not a pre-pass snapshot;
ordinary insertion of later IDs can extend a pass. Self-kill, Shutdown or removal of an iterator still in use
can invalidate native borrowed pointers. Do not infer a safe deferred-kill protocol or a stable post-callback
result for such invalidating reentry. Pause/Resume's all-scripts selectors are the explicit snapshot exception.

#### 6.3.5 Global `arg`, per-thread arguments and automatic yielding

Global setup `0x008AED10` runs at each Init tail with a nonnull Lua. Save the main thread's GetTop;
push string `arg`, allocate **one byte** of Lua userdata, duplicate it, move the duplicate before the key,
RawSet(globals, `-10001`), create a metatable with `__index` equal to C callback `0x008AE8B0` (zero
upvalues), attach that metatable to the remaining userdata at -2, then SetTop to the saved height. Return
true. The one byte is not initialized/read by the server. Thus global `arg` is a userdata proxy, replaced
on repeated Init; it is not the per-thread table itself (`0x008AED2F–0x008AEDB0`).

Thread setup `0x008B2540` returns false without stack changes for a null record or null record thread.
Otherwise, on that thread, PushLightUserData(thread interface pointer), create table T and set numeric
key **0.0** to the copied chunk-name string. If the argument string is nonempty, construct cRZCmdLine
from its C string and traverse its parsed tokens in increasing argv index, starting with index zero.
Tokenization/quoting belong to that parser; the following rules apply to the tokens it supplies:

- Skip empty tokens and tokens consisting entirely of `-`/`/` prefixes.
- Strip all leading `-` and `/` for the named-key calculation. Split at the first subsequent `:`.
  With a colon, T[name] is the suffix string (possibly empty); without a colon it is Boolean true.
  The name can be empty when the first unstripped character is a colon. No numeric conversion is performed.
- Also append the **original full parsed token**, with prefixes and colon, to numeric T[1], T[2], etc.
  Skipped tokens consume no numeric index. Repeated names overwrite their named fields but still append.
- Finally assign T["count"] to the **next numeric index**, starting at one, so an empty token list gives
  count 1, not zero. This final write overrides any named token `count`.
- RawSet(registry, `-10000`) with the lightuserdata key and T; return true. The temporary key/table pair is
  consumed; there is no general GetTop/SetTop restoration around this helper.

The initial zero is a 64-bit double from `0x00A80990`; numeric indices/count are signed 32-bit integers
converted with x87 FILD to double (`0x008B257A`, `0x008B279E`, `0x008B2855`). Integer increments are
32-bit, not floating-point accumulation. These operations and table write order are established by bytes.

`arg.__index` (`0x008AE8B0`) obtains the thread wrapper from raw Lua-state address -4. If null, or GetTop ≤0,
return zero Lua results. Otherwise push lightuserdata(wrapper), RawGet(registry). If that result is a table
(type 5), duplicate the original lookup key at -2, RawGet on that table at -2, return one result. Otherwise
push nil and return one result. It does not fall back to another script's arguments. It leaves temporary
values below the returned top value; Lua's C-return protocol discards them. Null-key/type errors on unusual
direct calls are delegated to Lua. The server has no explicit registry-entry deletion during KillScript.

The installed line hook (`0x008AE850`) obtains that same wrapper and its user context (+14). With either
missing, a nonzero **16-bit** field at raw-state +2E, or record auto-yield byte false, it does nothing. Otherwise
call lua_getinfo with option `l` (its result is not tested), read the 32-bit line at debug-record +14, compare with the saved record line,
and only on a difference store the line and lua_yield(state, **0**). It does not inspect breakpoint lists or
pause counts. Repeated hooks for the same source line do not yield; toggling auto-yield does not reset the
saved line. No event-kind test or wall-clock budget is added. The VM owns the meaning of the raw-state
field and the yield transfer; the server owns the gating, comparison and update order.

Independent creation/scheduling/stack vectors:

| Input/service responses | Required owner outputs |
|---|---|
| Attempt serial 8, last ID 40; LoadBuffer returns 3 | Serial 9, last ID 40; Pop(1); no CreateThread or notification; result `-1` |
| Next attempt: compile zero, CreateThread returns ID 71 and owned thread | Serial 10, last ID 71; user-context/arg setup, map insert, line-hook install then creation notification; result 71 |
| File reader false before either attempt | Result zero; no compile, serial increment or last-ID change |
| Eligible record, pause count -2; Status responses `[1,1]`, no matching breakpoint | One ResumeThread(ID,0,null), current ID exposed inside it, record retained; ResumeThread result ignored |
| Eligible records 2 then 8; record 2 Status `[0,0]`, record 8 pause count 1 | Terminate 2 then DestroyThread(2), release/erase; no Status call for 8 |
| Eligible record Status `[2,2]` | Termination/release/erase without DestroyThread |
| ResumeThread calls nested OnTick | Nested call true with no Status/Resume work; outer pass continues |
| Chunk `job`; parsed tokens `--fast`, `/n:12`, `:x`, `---`, `count:9` | T[0]="job"; T.fast=true; T.n="12"; T[""]="x"; T[1..4] are the four original accepted tokens; T.count=5 |
| Auto-yield true, saved line 4; hook lines `4,5,5,4`, raw +2E zero | Yields only on second and fourth hooks, each with zero results; saved lines `4,5,5,4` |

#### 6.3.6 Breakpoint storage and slots 11–18

All breakpoint methods first look up a **single** script ID. There is no all-scripts `-1` selector here.
Each record owns an insertion-ordered doubly linked list. A breakpoint payload is `0x34` bytes, with
script ID at +00, copied source-name string at +04, 32-bit line at +18, enabled byte at +1C, copied condition
string at +20; list nodes add eight linkage bytes. Default construction `0x008B0520` gives ID/line `-1`,
enabled true, empty name/condition. Creating an entry fills ID from record.thread.GetID() (or `-1` with
null thread), copies the supplied name and line, and appends to the tail. There is no line-range validation.

Find/create `0x008B42C0` and remove `0x008B1B80` match **name and line**, not the payload's stored ID.
Name equality uses string +24 with flag **zero**: case-insensitive equality, implemented by lowercasing
temporary copies (`0x0090E242` forwards to `0x0090EC57`; `0x0090ECCE–0x0090ECD6`). No path
normalization, basename conversion or separator folding occurs here. The first match wins. Find/create
clears its created-output byte on entry, returns the existing entry if found, returns null for a miss when
creation is disabled, or appends the new entry and sets created true. Remove clears its changed-output
byte, unlinks/destroys/frees the first matching node and sets changed true on a hit, otherwise returns false.
Removal helper `0x008B1540` unlinks before destroying the condition then name string and freeing the node.
List destruction `0x008B1D00` clears all payload nodes, then frees the sentinel. Neither operation notifies
on its own. The public operations are:

| Slot / entry | Inputs after ID | Result and mutation |
|---|---|---|
| 11 `008B4E60` GetBreakpointExists | name, line | True only when record and entry exist; never creates |
| 12 `008B4EC0` SetBreakpointExists | name, line, final flag | **Zero flag creates/finds and forces enabled=true. Nonzero flag removes.** Existing record returns true even if no entry changed; missing record returns false. Notify only on actual creation/removal |
| 13 `008B4F60` ToggleBreakpointExists | name, line | Remove if present; otherwise create with defaults. Notify once on actual change; return true for an existing record, false for a missing record |
| 14 `008B5000` GetBreakpointEnabled | name, line | True only if entry exists and enabled byte is nonzero; false on any miss |
| 15 `008B5070` SetBreakpointEnabled | name, line, enabled byte | Create if absent. Store supplied byte when different. Notify once if created or byte changed. True for a found record, false for a missing record |
| 16 `008B5110` GetBreakpointCondition | name, line, output string | Clear output to length zero **before** testing for a record/entry. Copy stored condition bytes and return true on hit; false on miss with output still empty. Never creates |
| 17 `008B51A0` SetBreakpointCondition | name, line, condition string | Create if absent. Compare stored condition to supplied text with flag **one** (case-sensitive equality). **Only if equal**, copy supplied text and notify; if unequal, leave condition unchanged and notify only if entry was newly created. True for an existing record, false on record miss |
| 18 `008B4210` GetBreakpointEntryList | source-name filter, output list | Clear/destroy old output entries first, even on record miss. Append deep copies of matching-name entries, in insertion order, including stored ID, line, enabled and condition. True for a found record (even zero matches), false otherwise |

The inverted-looking flag in slot 12 is established by `0x008B4EF1–0x008B4F30`; it is not interpreted
from the method's SDK name. Likewise slot 17's equality branch is literal (`0x008B5202–0x008B520E`):
it does **not** assign unequal text. In normal public creation all conditions start empty, so setting a
nonempty condition does not make it persist. Reapplying SetBreakpointExists with flag zero can enable an
existing disabled entry **without a notification**, because that method observes only the created flag.
Output strings/lists in slots 16/18 must be valid mutable objects; they are not optional/null-checked.

ShouldBreak `0x008B1E00` is an independent scheduling test. Empty list immediately returns false.
Otherwise ask the debug-info service for current source name and line at stack depth zero; scan entries
in insertion order for case-insensitive name equality and exact 32-bit line equality. The first match means
true. There is **no enabled-byte test, condition evaluation, pause-counter update, resume-counter test,
or notification** in this function (`0x008B1E78–0x008B1E98`). Disabled entries therefore still prevent a
status-1 script's next resume. Removing the entry allows the next tick to resume; incrementing the
resume-transition counter alone does not bypass it. The debug service supplies name/line; VM/debug
behavior is not an owner-produced oracle result.

#### 6.3.7 Notification protocol, failure boundaries and conformance inputs

The script notification helper `0x008AEBC0` asks the framework for service pair
`(0x04FA845B,0x652294C7)` into a temporary owned reference. If no service is returned, it performs no
message allocation/send and simply releases the temporary reference. Otherwise allocate/construct a
`0x2C` standard message, AddRef it, invoke message +14 with type, +2C with ID, +40 with the 32-bit payload,
then message-server +10 with `(message,0)`, Release the message, and release the temporary service.
The return from the send is ignored. The helper does not pass the server pointer in a message field.

| Event | Message type | ID | Payload | State at service invocation |
|---|---|---|---:|---|
| Creation | `0x6BA990D8` | CreateThread result | 0 | Last ID written, record inserted, hook installed |
| Pause transition to 1 | `0xEBAAF9BE` | Lookup ID | 1 | Pause count already 1 |
| Resume transition to 0 | `0xEBAAF9BE` | Lookup ID | 0 | Pause count already 0; transition counter not yet incremented |
| Explicit kill | `0xCBA990D3` | Thread.GetID result | 0 | Foreign DestroyThread already called; record still in map |
| Tick terminal status | `0xCBA990D3` | Thread.GetID result | 0 | Record still in map; optional status-zero DestroyThread has **not** yet been called |

Breakpoint helper `0x008AEC60` uses the same service lookup/allocation/reference/send protocol but type
`0x8BD13EE2`, writes only the script ID through message +2C, and makes no +40 payload setter call.
Message delivery policy is foreign to the server: these are ordered service invocations, not a claim that
observers execute synchronously. No retry or propagation of send failure occurs. A missing message service
does not undo any script/breakpoint mutation. Message allocation failure is unchecked before virtual calls;
no false-return or rollback contract exists for exhaustion. The same absence of defensive checks applies
to failed script-record/list/string allocation. Valid foreign interfaces and available storage are required
for successful paths; do not invent SDK recovery behavior.

The following independent vectors complete the owner boundary cases; they supplement §16 without
requiring a real VM. Supply thread Status/debug name/line, returned IDs, factory results and callback
events as inputs; compare the server's state changes and foreign call sequence as outputs.

| Initial state / input | Expected result and ordered effects |
|---|---|
| Live record; no breakpoint; slot 12 `(name,10,0)` twice | True twice; one enabled entry; exactly one breakpoint notification |
| Same entry disabled; slot 12 `(name,10,0)` | Entry enabled; no notification |
| Same entry; slot 12 `(name,10,1)` twice | True twice; first removes/notifies, second does neither |
| No entry; SetBreakpointEnabled(name,10,false) | Creates disabled entry; exactly one notification; GetEnabled false, GetExists true |
| That disabled entry; eligible status 1; debug name equal ignoring case and line 10 | ShouldBreak true, no ResumeThread; no control notification |
| Existing empty condition; SetCondition("x > 0") | True; condition remains empty; no notification |
| Missing entry; SetCondition("x > 0") | True; entry created enabled with empty condition; one notification |
| Existing empty condition; SetCondition("") | True; copy equal empty text and notify once |
| Caller output condition "old"; missing record or breakpoint | False, output now empty |
| Caller list nonempty; missing record | Old copied payloads destroyed, list empty, false |
| Insert `(A,30)`, `(a,10)`, `(B,20)`; list filter `a` | Copies first two entries in insertion order `[30,10]`, not line order; result true |
| Missing message service; any create/remove/pause operation above | Same mutation/result; no message allocation or send |
| Record count 1; Shutdown with no framework | Latch clears; Kill smallest ID (DestroyThread before termination), record freed/erased, generator shutdown-view call/release, Lua Close/release; true |
| Last-created ID 71; kill ID 71; query last/current | Last remains 71; current is `-1` outside tick ResumeThread |

No unresolved owner decisions remain for normal, non-invalidating calls in this audit. The external parser,
file reader, base system service, Lua execution/debug service and message delivery retain their own
contracts. In particular no interpretation of Lua instructions, invented safe self-destruction, or additional
breakpoint-condition language is part of this owner contract.

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

The advisor system calls it for `<plugin dir>\advisors\` (`cISC4App` `+0xD4` `GetPluginDirectory(out, -1)`) and then
`<SKU-specific dir>\advisors\` (`+0xDC` `GetSkuSpecificDirectory(out)`) (advisors.md §4.2, AD10), so a SKU-specific
file replaces a plugin file of the same name. Calling convention: `cdecl`, caller pops 8.

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

### 8.6 Bounded collector and registration-state comparison (LH6/LH7)

This is a verification scope **[choice]**, not a native replay feature. The original supplies the
collector and registration behavior in §§8.1–8.2. Observe the collector using the documented
[collector layout](advisors.md#82-registration-register_apibool-global): begin/end/capacity at
preferred-image addresses `0x00B207CC/0x00B207D0/0x00B207D4`, stride 12, with name pointer,
callback pointer and isGlobal byte. Validate readable bounds and strings before using them.
Compare the 47 names in collector order, their flag bytes and non-null callback entry points.
Padding is not a field. Treat the original callback entry points as opaque foreign identity
inputs to the local registration model, not functions for the model to execute.

For a sampled live app VM, save its stack height and use public **raw** table operations to read
the three global bindings, the 44 `game` bindings and `_tf_`. Read a C function's entry point
through the public conversion, not its closure allocation address. Do not execute a chunk,
callback or metamethod to obtain the sample. Restore the stack on completion and report a
capture failure if a required table, C function, readable collector field or balanced stack is
absent. After both normal registration passes, `_tf_` is `window_graph` (`0x0040C890`); after
only the global pass it is `print` (§8.2). Compare to the corresponding registration stage,
not to an assumed historical stage. A snapshot at another stage is not evidence that an earlier
pass failed. The local registration model must also preserve its initial stack height.

Names, ordering, flags, binding presence, callback identities and stack restoration have **no
nondeterminism exclusions**. In one process compare callback addresses exactly. When expressing
an executable address as a portable evidence label, use preferred address = actual address −
loaded image base + `0x00400000`, only for a validated address inside that image. This relocation
label is not permission to erase identity differences or to apply that arithmetic to heap or
foreign-module pointers. Closure addresses/upvalues, callback bodies, scripts and unsampled VMs
are outside this comparison's scope, not fields counted as matched. Publish the bounded result
as `lua-registry`; it supplies no full-VM `lua` verification credit (§1.4.2).

---

## 9. The game-data registry mirror (`sc4game`) *[compat]*

### 9.1 Subscription

Each `cSCLua` instance, on `Open`, adds its notification target to the game-data registry (service `0xEBE0E860`, IID
`0xE898ED03`, slot 25). It removes it on `Close` (slot 27). The target's slot 3
`OnGameDataRegistryNotification(registry, type, entry)` (`0x00881650`, pops 12) keeps a Lua mirror of the registry
tree. Every entry also has a Lua registry slot keyed by the entry pointer as light userdata.

The native registry synchronously replays existing entries to a new target, including the root,
in depth-first reverse-sibling insertion order; duplicate registration returns false without replay
([registry §5](game-data-registry.md#5-notification-targets-and-replay), `0x00474F70`,
`0x00474DE0`, `0x00473BD0`). This resolves legacy Q7 for nonmutating registration.
Instance 1 is subscribed before any city component publishes. Instance 2 is subscribed only while
the automata scripts run. Mutation during replay has no established snapshot guarantee.

This subscription replay is **registry entry delivery**, not Lua execution replay (LH7, §3.5).
It invokes the new target synchronously and may build a mirror during Open itself. A later sample
cannot reconstruct notification timing from the final `sc4game` table. Entry pointers are used as
identity keys, not normalized script values; the bounded §8.6 comparison does not inspect or
claim equivalence of this mirror.

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
| `cISC4App` | `+0xA8` GetStringDetokenizer, `+0xBC` GetLuaInterpreter, `+0xC4` GetTutorialRegistry, `+0xCC` GetDataDirectory, `+0xD4` GetPluginDirectory(out, int32 language), `+0xDC` GetSkuSpecificDirectory (AD10: the earlier names for `+0xD4`/`+0xDC` were wrong) | | | |
| `cRZPath` | ResolvePath, Splitpath, GetFileName (non-virtual framework code) | | | |
| `cGZCommandParameterSet` | constructed by `Open` (§9.4) | | | |

The `cIGZLua5`/`cIGZLua5Thread`/`cIGZLuaScriptServer`/`cISCStringDetokenizer` vtables above are not yet in
`docs/abi`.

---

## 13. Replaceability

- **Class registry.** The three Lua class IDs (`0xEA10C4A2`, `0x4B2811AA`, `0x2B2810F7`) are registered through
  GZCOM by `cSCLuaCOMDirector`. A replacement could in principle register its own factory for them. Every user
  creates the VM through `CreateInstance`. No user constructs `cSCLua` directly.
- **Observation is separate from replacement (LH6).** The factory possibility above does not
  establish an early public observer or redirect cached interfaces. Under record-only access,
  use §8.6's bounded comparison; do not infer complete call/state replay (§1.4.2).
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
- Q7 answered: existing entries are replayed synchronously (§9.1; archived as LH3).
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

### 16.1 Thread and command-generator vectors (LH4/LH5; specified, not executed)

These are independent contract vectors for implementers. Observe stack values, references, registry
membership and endpoint call order, using controllable factory/hooks/command services. They require
no original-host forwarding. They do not claim that executable conformance coverage already exists.

| Vector | Input / initial state | Expected observation |
|---|---|---|
| LH4-A | Fresh concrete thread | ID -1, context/owner/user null, count 0; IsValid false, Status -1, Destroy false |
| LH4-B | Open fresh base host; repeat base Open | First true, main ID 0 and Status 2; second false; AsIGZLua5Thread and GetThread(0) borrow |
| LH4-C | Main stack `[lower,F]`; CreateThread flags 1, valid factory | Stack `[lower]`; owned child output ID 1; child stack `[F]`, Status 1; root and backpointer installed |
| LH4-D | Same stack; factory false, or OnCreate false | ID -1, output null, stack `[lower]`; no record/root; release factory temporary, no OnDestroy; next ID unchanged |
| LH4-E | Destroy ID 1; create again; then Close/Open | Next child ID 2; after reopen main ID 3; removed IDs are not recycled |
| LH4-F | Main OnCreate override false after writing context/retaining owner | Open false; context/owner mutations survive, registry owner retained, no record/root or counter increment; no Lua close |
| LH4-G | Successful resume, T=3, nargs=2, child returns `[A,B]` | True; main `[lower,A,B]`, count 2; child empty. With null count output main `[lower]` |
| LH4-H | Invalid ID, T=3, nargs=2 | False; reporter gets null/code 1/error text; main retains arguments plus error, count 3; null count output leaves one lower value |
| LH4-I | Child with flags 0 completes; then same with flags 1 | First record removed and retained wrapper invalidated; second remains with Status 0 and empty child stack |
| LH4-J | Child yielding one value, then completion | Yield reports true and returns that value on main; child Status 1 while frame survives; completion follows flag-based removal rule |
| LH4-K | Thread OnDestroy, user context P | Context null, owner released, ID -1, user still P; hook alone does not erase registry entry |
| LH4-L | Attached child; requester neither child nor parent | Destroy false without mutation; null requester succeeds; parent destruction removes descendants first |
| LH5-A | Generic Init on fresh generator | False; no registration, endpoint retention or parameter allocation |
| LH5-B | Specialized Init with nil global and successful registration | Create table, retain thread/server, copy name, create two sets, clear bookkeeping, true; no success-tail enumeration |
| LH5-C | Specialized Init with existing table / number global | Table identity preserved and true / register then unregister, false, original stack depth preserved |
| LH5-D | Parameter-set allocation returns null | Specialized Init still true with null stored set; command execution is not a supported null-set path |
| LH5-E | Register ID A/name `x`; register A/name `y` | First installs closure; second returns stored-name comparison, changes neither name nor binding/description |
| LH5-F | Register A/`x`, B/`x`; unregister A | B overwrites `x`; removing A clears `x` despite B's live map entry |
| LH5-G | Saved A closure; unregister A; register B reusing index | Old closure uses B's record; no original-ID validation. Saved A before reuse can still dispatch A |
| LH5-H | Callback args `(true,1.25,"x",nil,{})`; command status -7 | Input keys 0–4 = Bool true, Float64 1.25, String x, Float64 0, Float64 0; first Lua result -7 |
| LH5-I | Output enumeration keys `[9,0x2B999D2B,2]`, values Bool false/reserved/null | Results `[status,false,nil]`, count 3; reserved skipped, order unchanged |
| LH5-J | No output parameters / captured main pointer mismatch | `[status]` / `[2]`; mismatch does not clear sets or call server |
| LH5-K | False UnregisterCommandGenerator during Shutdown | False result, but all four owned endpoint/set references released and bookkeeping empty; global name survives |
| LH5-L | Shutdown twice; then destroy | Second true with no unregister; destructor does not synthesize unregister or remove Lua globals |

### 16.2 Observation and external-input vectors (LH6/LH7; specified, not executed)

These prose vectors specify scope and observable behavior, not new executable tests or full-VM
coverage. Native-side bounded sampling uses public raw operations only (§8.6). Lifecycle,
library and foreign-service fixtures below run in an isolated implementation test, not as extra
calls against the live game's random generator or callbacks.

| Vector | Input / initial state | Expected observation |
|---|---|---|
| LH6-A | Retain app Lua after its Open; request another Lua through its class factory | New receiver is a distinct instance; app acquisition provides no historical Open/chunks or automatic call observer. It does not acquire the temporary automata or ScriptServer Lua |
| LH6-B | Table has an existing key and an index metamethod that records invocations; evaluate a Lua read of that key | Existing value returned, index method not invoked. A metatable trace cannot certify complete table-access capture |
| LH6-C | Read all 47 collector records and an app sample after both registration passes, with stack height S | Ordered names/flags and matching non-null callback entry points; three global and 44 game bindings; `_tf_` equals window_graph; native and model stack heights both S. Result label is lua-registry only |
| LH6-D | Same sample, but exchange two collector records / alter a flag / bind one name to another callback / change `_tf_` | Each is a difference; do not sort records, ignore a flag or normalize callback identities to obtain a match |
| LH6-E | A required collector record is unreadable, a binding is not a C function, or final stack height differs | Capture/binding/stack failure as applicable; never publish an exact sample or silently omit the field |
| LH6-F | Identical bounded samples before and after an unsampled temporary VM ran scripts and closed | Samples alone establish no equality of that VM's chunks, calls, notifications or final state; no full lua verification credit |
| LH7-A | Two isolated VMs on the same OS thread use a controlled shared CRT random source: seed in A, then draw in B and A | B consumes the first draw, A the next. A's seed is not private to A; no automatic reseed or replay exclusion. Assert draw consumption, not a guessed random numeric sequence |
| LH7-B | Controlled clock/environment or foreign registry getter supplies different results on successive calls | Results follow each supplied input and §§9.3–9.4 conversion. Host does not freeze the first result or normalize the difference away |
| LH7-C | Two node entries have identical names but distinct pointers; the same entry pointer is referenced twice | Pointer-key associations remain distinct for the two entries and shared for repeated references to one entry; type-only or name-only identity normalization fails |
| LH7-D | Nonmutating registry: root with children added A then B; register a new target, then register it again | First registration synchronously delivers root, B, A (type 3); duplicate returns false, no replay. No sorting or deduplication of the first delivery |
| LH7-E | Table fixture with populated array keys 1 and 2 plus hash keys; traverse with next | Array entries 1 then 2 precede hash entries in native storage traversal order. Hash entries are not required to be lexically ordered; sorting an observed sequence is not an exact comparison |
| LH7-F | Fresh process, empty stacks, NewTag on VM A then VM B | Returns 1 then 2 (§3.4/T10), not 1 per VM; no tag-number relabeling |
| LH7-G | Bound collector sample with callback actual address 0x0060C890 and executable loaded at 0x00600000 | Portable preferred-address label 0x0040C890. Same-process equality still compares actual pointers; do not apply image rebasing to an entry/userdata heap pointer |

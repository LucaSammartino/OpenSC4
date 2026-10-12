<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# FreeCity4 engine

Clean-room engine code for FreeCity4. It is written only from [`docs/specs/`](../docs/specs/) and the LGPL
gzcom-dll SDK. Read [CONTRIBUTING.md](../CONTRIBUTING.md) before you change anything here.

| Path | What | Language |
|---|---|---|
| [`core/`](core/) | `fc4-core`: the portable simulation core. No GZCOM or OS types, no `unsafe`. | Rust |
| [`capi/`](capi/) | `fc4-capi`: a stable C ABI over the core (`include/fc4.h`), built as a static library | Rust + C header |
| [`conformance/`](conformance/) | Language-neutral JSON test vectors (the migration contract) and their Rust runner | JSON + Rust |
| [`plugin/`](plugin/) | `FreeCity4.dll`: GZCOM director with a subsystem table, the `cISC4Simulator` adapter, the budget classes (`cISC4BudgetSimulator`, `cISC4DepartmentBudget`, `cISC4LineItem`), stream and property helpers, and our own standard message class (foundations §5.1; the SDK's `cRZMessage2Standard` is not used). Windows, x86 only. | C++17 |
| [`tests/`](tests/) | `capi_smoke` (drives the C ABI from C++ with the plugin's compiler and CRT), `message_test` (foundations §7 item 12) and `plugin_load_test` (loads the DLL like a host and drives the simulator and the budget) | C++17 |
| `third_party/gzcom-dll/` | gzcom-dll SDK (LGPL-2.1+), git submodule | C++ |

```
SimCity 4.exe ── GZCOM ──► FreeCity4.dll (plugin/, C++)
                              cFC4Director  ─ registers the classes of FC4Subsystems.cpp
                              cFC4Simulator ─ cISC4Simulator, cIGZSerializable,
                                              cIGZSystemService, cIGZMessageTarget2
                              cFC4BudgetSimulator, cFC4DepartmentBudget, cFC4LineItem
                                     │  fc4.h (C ABI)
                                     ▼
                           fc4_capi.lib (capi/, Rust staticlib)
                                     │
                                     ▼
                           fc4-core (core/, Rust): simulator, budget, streams, class-object
                           store, variants and property holders, curves, RNGs
```

The C ABI is the seam between the two languages. A future all-Rust GZCOM layer can replace `plugin/` and keep
`fc4.h` unchanged.

## Prerequisites

- Rust (stable, 1.75 or newer) with the `i686-pc-windows-msvc` target:
  `rustup target add i686-pc-windows-msvc`.
- For the DLL: Visual Studio 2022 Build Tools (MSVC x86, Windows SDK). CMake 3.20+ and Ninja ship with them.
- The SDK submodule: `git submodule update --init`.

## Build and test

Rust only (any OS):

```sh
cd engine
cargo test                                   # host target
cargo test --target i686-pc-windows-msvc     # 32-bit Windows, the plugin's target
```

Run cargo from `engine/`, so `engine/.cargo/config.toml` applies. It links the C runtime statically
(`+crt-static`) for the Windows targets.

The plugin, from a shell with the x86 MSVC environment
(`"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars32.bat"`):

```bat
cmake -S engine -B build-x86 -G Ninja -DCMAKE_BUILD_TYPE=RelWithDebInfo
cmake --build build-x86
ctest --test-dir build-x86 --output-on-failure
```

CMake runs `cargo build --release -p fc4-capi --target i686-pc-windows-msvc`, with its output in
`build-x86/cargo/`, then links `fc4_capi.lib` into `build-x86/FreeCity4.dll`. Everything uses the static release
CRT (`/MT`), so the DLL needs no runtime DLLs. CTest runs `cargo test` for the same target, plus the C++
`capi_smoke` test. On other platforms or architectures CMake builds and tests only the Rust parts and skips the
plugin.

## Installing and checking in game

1. Copy `build-x86/FreeCity4.dll` to the game's plugin folder, for example
   `Documents\SimCity 4\Plugins\` or `<game dir>\Plugins\`.
2. Start the game. `FreeCity4.log` appears next to the DLL, or in `%TEMP%` if that folder is not writable.
   - On start you should see `FreeCity4 … loaded (… replacement mode)` and `PostAppInit (simulator class 0x2990C1E5
     provided by FreeCity4)`.
   - When a city opens, you should see `cFC4Simulator created`, `Read: …`, `Init: …` and then `PostCityInit: hidden pause released`. While the city runs you
     should see one `SimNewMonth …` line per game month.
   - Diagnostics: `FC4_LOG_LEVEL` = `0` (off), `1` (lifecycle only) or `2` (default, verbose). Verbose mode traces
     `DllMain`, every director call (`InitializeCOM`, `OnStart`, `EnumClassObjects` with each class ID reported,
     every `GetClassObject` with the clsid, the iid, the result and whether our factory ran), plus simulator
     `QueryInterface` (the first 200 calls), `Init`, `Read`/`Write`, `OnTick` (the first tick and every 1000th),
     `DoMessage` (the first 50), agent registration and pause calls. Lines marked `[v]` are verbose. Every line is
     written straight to the OS. The log is opened with full sharing, so a viewer that holds it open cannot block
     writes. The path never changes once chosen. Failed writes are retried once and then reported on the next line.
   - Crash tracing: the class-object path (`GetClassObject`, the factory, `QueryInterface`) runs under a Win32
     structured-exception guard. A first-chance vectored handler also logs every exception raised inside
     `FreeCity4.dll`'s image, with code, address and offset from the DLL base, and passes it on unhandled.
     Exceptions outside our image are reported at most twice per faulting address (16 addresses), as
     `module+offset`, with the return addresses on the stack and the last interface calls. The handler never
     blocks: it takes the log lock only with a time-limited try-lock and otherwise writes through a spare
     handle. Stack overflows are only counted. `FC4_DIAG_SELFTEST=1` raises one deliberate access violation at
     load to prove that both work.
   - Heartbeat (always on, also at `FC4_LOG_LEVEL=1`): one line per speed or pause change and per month start,
     and a `heartbeat (periodic)` line every `FC4_HEARTBEAT_MS` (default 10000) with the julian date, speed,
     the pause/hidden/emergency flags, and since the last heartbeat the `OnTick` count, agent deliveries,
     interface calls and verbose lines suppressed by caps.
   - Hang watchdog: a background thread watches markers the simulation thread publishes without locks (inside
     `OnTick`, the agent delivery in progress with level, message and agent name, core `fc4_*` calls, our
     interface methods). If the simulation thread stays inside our code for more than `FC4_WATCHDOG_STALL_MS`
     (default 5000), or no `OnTick` arrives for `FC4_WATCHDOG_NOTICK_MS` (default 15000) while a city runs and
     is not hidden-paused, it logs `WATCHDOG: … stalled` with that activity, the last 64 interface calls, and a
     sample of the thread: suspended only to copy EIP/ESP and its stack, then resumed before anything is
     formatted. The sample lists the return addresses into `SimCity 4.exe` and `FreeCity4.dll` as
     `module+offset`, plus process and thread CPU time (a thread that waits gains no CPU, a spinning one does).
     It repeats every `FC4_WATCHDOG_SAMPLE_MS` (default 5000), at most 10 samples per stall, and ends with
     `WATCHDOG: stall over after …`. `FC4_NO_WATCHDOG=1` disables the thread.
   - Scheduler tracing: the first 300 agent deliveries are logged with level, message, agent and result. Per
     agent, the log counts deliveries, "not finished" results and the current streak of consecutive ones, and
     notes when a streak passes 100, 1,000, 10,000, and so on. Every `FC4_STATUS_EVERY` ticks (default 500) it
     writes a status block: the day-barrier conditions (`DAY_GATE_OPEN`, interval timer, blocking levels,
     counters), and for each level its cursor (agent name or END), its size and its top five false streaks. The
     first 200 broadcast posts are logged, and every SimNewDay, SimNewWeek, SimNewMonth and SimNewYear post. Date
     reads are logged too: the first 20 calls to each `GetSimDate` variant and `GetSimDateNumber`, and the first
     200 method calls on our `cIGZDate` objects.
   - At `PostAppInit`, verbose mode also asks `GZCOM()->GetClassObject(0x2990C1E5)` once. The answer shows whether
     the game's registry resolves the class to our factory. The object it creates is released at once. Set
     `FC4_NO_PROBE=1` to skip this check.
   - Limit-timer calibration (spec §5.5): the simulator keeps the game's two CPU-cycle coefficients
     (`CyclesToMillisecs`, `MillisecsToCycles`) up to date. Subsystems such as the lot developer use them to budget
     their work. Writing them needs the version-specific addresses in `plugin/compat/`, and only happens after the
     host has been identified as the exact 1.1.641 executable. The log shows each check (`compat: …`) and the
     published values (`calibration …`), both at `Init` and in every status block. `FC4_DISABLE_CALIBRATION=1`
     turns off the writes (for A/B tests).
   - Budget: `budget Init: …` when a city opens (funds, departments, bonds, tax rates), `budget Read/Write`,
     cheats and bonds as they happen, and three `budget settlement: …` lines each month: funds before and after
     with income and expense, the tax of each of the 12 groups with its rate, and the bonds (count, borrowed,
     monthly payments). `FC4_BUDGET_FIX_CS3=1` enables the intended CS$$$ tax weight (budget spec §13.1; off by
     default, as in the original).
3. **A/B against the original:** `FC4_DISABLE` takes a comma- or space-separated list of subsystems to leave to
   the game (`simulator`, `budget`, or `all`), e.g. `FC4_DISABLE=budget`. `FC4_DISABLE_SIMULATOR=1` still works.
   The DLL still loads, but it does not register those classes, so the game's own ones run. The simulator and the
   budget only talk through the SDK interfaces, so either can be switched off alone.
4. **Bisection (`FreeCity4.ini`):** a `key = value` file next to `FreeCity4.dll` (`#` or `;` after whitespace
   starts a comment, `[sections]` are ignored). A key in the ini wins over the environment variable.
   - `disable = budget` means the same as `FC4_DISABLE`. It is read once, when the game enumerates the classes,
     so a change needs a game restart.
   - Each budget behaviour that touches game objects has a switch. The switches are re-read at every city load
     (budget `Read`/`Init`). Each defaults to `1`; `0` makes the behaviour a no-op with a neutral result. The
     matching environment variable is `FC4_BUDGET_<KEY>`. `budget_default = 0` turns every unset switch off.

     | Key | When off |
     |---|---|
     | `lot_scan` | `ForEachLot` sees no lots, so property tax is 0 |
     | `property_writes` | `AddPropertyVariant` and property copies are skipped but reported as written |
     | `property_reads` | properties read as absent |
     | `message_posting` | no budget messages are posted |
     | `history` | no history records or queue lengths |
     | `building_name` | empty line-item names |
     | `exemplar_loads` | the tuning exemplar is not loaded (code defaults) |
     | `disaster` | RiskyMoney never starts the earthquake |
     | `cheats` | the budget's cheats are not registered |
     | `holder_refs` | the budget core holds no references on game objects (references taken while the switch was on are still given back) |
     | `ltext` | LTEXT lookups return "not found" |

     The log shows `config (…): budget switches OFF (n of 11): …` at each load. At budget `Shutdown` it shows how
     many calls each switch skipped.
   - `refaudit = 0` turns the reference audit off. `refaudit_lines = N` sets its verbose line cap (default 2000).
   - **Reference audit:** every `AddRef`/`Release` we do on a game object (vtable outside our DLL), and every
     reference the game hands us (QueryInterface, out-parameters, lot lists, save references), is logged at
     verbose level under its own cap. Each line gives the object, its class (`GetGZCLSID`), its vtable as
     module+offset, the count returned and our call site.
     - Always logged: `WARNING: released last reference to game object …` and `WARNING: over-release …`.
     - At each simulator or budget `Shutdown`: the net count per class and the objects we still hold.
5. **Checked build:** `cmake -S engine -B build-x86-checked -G Ninja -DCMAKE_BUILD_TYPE=RelWithDebInfo
   -DFC4_CHECKED=ON`.
   - The plugin is built with `/Od /RTC1`, so an argument-count or callee-pop mismatch on any vtable call is
     caught at the call.
   - RTC failures go to the log, not a dialog: the kind, the call site as `FreeCity4.dll+offset` with our
     function and line, the frames, the activity and the last interface calls. Copy `FreeCity4.pdb` next to the
     DLL for the names.
   - Small buffers we pass to game methods (`GetSize`, `GetLocation`, `GetSimDateParts`, QueryInterface and
     `GetResource` out-pointers, the lot list, `ReadGZSerializable`) are surrounded by canaries. A write past
     them logs `OUT-PARAM OVERRUN`.
   - `FC4_CHECKED_SELFTEST=1` provokes one of each at start-up.

## Conventions

- Every source file starts with an `SPDX-License-Identifier: GPL-3.0-or-later` header.
- Spec gaps are decided in code and tagged `[choice]`. Behaviour the SDK cannot express yet is tagged
  `TODO(spec §n)`.
- Panics never cross the C ABI. Release builds use `panic = "abort"`, and each `fc4_*` entry point also catches
  unwinding.
- Every spec behaviour needs a conformance vector. See [`conformance/README.md`](conformance/README.md).
- Game interfaces are called through vtable slots, so slot order is part of the ABI (foundations §16). An
  interface with overloaded methods is declared in [`plugin/FC4Interfaces.h`](plugin/FC4Interfaces.h) in the
  spec's verified slot order with a unique name per method, never taken from the SDK header: MSVC regroups and
  reverses overloads, so with overloads the layout follows the compiler, not the declaration.
  `tests/vtable_layout_test.cpp` checks every slot of every interface we implement against the spec tables, and
  `plugin_load_test` calls each overload of the real objects through its slot with the game's argument count and
  checks that the stack balances. New code must not implement or call overloads of an interface until its spec
  has a verified slot table. Declared from the foundations §17 tables: `cFC4IGZDate` (implemented by
  `cFC4Date`), `cFC4IGZVariant` (implemented by `cFC4Variant`, the only variant we hand to the game; never pass
  the SDK's `cRZBaseVariant`, whose slots 8 and 60 differ from the game's), `cFC4ISCPropertyHolder`,
  `cFC4ISC4LotManager`, `cFC4ISC4Lot`, and `cFC4ISC4NeighborsSimulator` (from the ABI table: slots 32/33
  return 32 bits). Interfaces still used through SDK declarations: `cIGZString`, `cISC4LineItem`,
  `cISC4BuildingDevelopmentSimulator`, `cISC4DisasterManager`, `cIGZPersistResourceManager`,
  `cISC4HistoryWarehouse`, `cISC4OrdinanceSimulator`, `cISCResExemplar`.
- **ABI conformance** (`tests/abi_conformance_test.cpp`, CTest `abi_conformance_test`): every declaration above
  is checked against [`docs/abi/interfaces.tsv`](../docs/abi/interfaces.tsv), read at run time. Per slot:
  argument bytes (each argument rounded up to 4, int64/double 8, plus 4 for a hidden result pointer) ==
  `callee_pop`; float/double return ⇔ `ret = x87`; int64 return ⇒ `edx:eax`, and `edx:eax` (medium
  confidence) ⇒ int64. Per interface: slots contiguous and as many as the table's. The slots are decoded from
  the compiler's vcall thunks, so the test checks what MSVC actually laid out. After adding a method or an
  interface, run `python tests/gen_abi_entries.py` (from `engine/`) and rebuild; `--dump` prints every slot
  next to its table row. SDK methods that disagree with the game but are never called by the plugin are
  waived (`kWaivers`); the waiver lapses, and the test fails, as soon as a file in `plugin/` calls one.
  Reviewed slot-count differences (class-private trailing virtuals) are in `kReviewedSlotCounts`.
- Return types are part of the ABI too: SDK declarations can be wrong (budget erratum:
  `cISC4DepartmentBudget` slot 26 returns a float POINTER in EAX, not a float in ST0; `cIGZDate` slots 33–36
  return a `uint32` day number and leave the date alone; `cISC4Lot` slot 61 returns `SC4Percentage` by value
  through a hidden result pointer; from the ABI table: `cIGZVariant` slot 41 and `cISC4Lot` slot 15 return in
  ST0, `cISC4BudgetSimulator` slots 20–23 in EAX only). Such interfaces are declared in `FC4Interfaces.h`, and the slot tests check
  the x87 stack and the stack balance: only slots the spec says return a float may leave a value there.

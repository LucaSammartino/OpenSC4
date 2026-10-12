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
     `FC4_DIAG_SELFTEST=1` raises one deliberate access violation at load to prove that both work.
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
  has a verified slot table. Interfaces still used through SDK declarations that contain overloads, pending
  verification: `cIGZDate` (implemented by `cFC4Date`), `cISCPropertyHolder`, `cIGZString`, `cISC4LotManager`,
  `cISC4BuildingDevelopmentSimulator`, `cISC4DisasterManager`, `cIGZPersistResourceManager`, `cISC4Lot`,
  `cISC4HistoryWarehouse`, `cISC4NeighborsSimulator`, `cISCResExemplar`.
- Return types are part of the ABI too: SDK declarations can be wrong (budget erratum:
  `cISC4DepartmentBudget` slot 26 returns a float POINTER in EAX, not a float in ST0). Such interfaces are also
  declared in `FC4Interfaces.h` (`cFC4ISC4DepartmentBudget`), and the slot tests check the x87 stack: only
  slots the spec says return a float may leave a value there.

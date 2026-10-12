# FreeCity4 architecture and roadmap

## Goal

Make the SimCity 4 engine **open source source code that compiles**, with no copyrighted code from the original.
Players supply their own copy of the game's data.

## Why reimplement instead of recompiling the decompilation

The decompiled output in `decomp/` is derived from EA's copyrighted binary, so it can't be published as open source.
It also isn't a buildable reconstruction (it's Ghidra pseudocode). Successful projects in this space (OpenRCT2, OpenTTD,
OpenMW) instead **reimplement** behaviour and verify it against the original. We use our reverse-engineering only to
write behaviour **specs** (`docs/specs/`). Engine code is written from those specs by a different role (see
[CONTRIBUTING.md](../CONTRIBUTING.md)).

## How the replacement plugs in

SimCity 4 is built on GZCOM, a COM-like object system. Two facts make piece-by-piece replacement possible:

1. **The game builds its subsystems through the class registry.** `cSC4City` asks
   `GZCOM()->GetClassObject(CLSID, IID, &p)` for the simulator, the demand, traffic, budget and other simulators,
   and so on.
2. **Classes registered by plugin DLLs take precedence over the exe's built-in classes.** The registry checks
   DLL-registered CLSIDs before it falls back to the executable's own factory.

SimCity 4 loads GZCOM DLLs from its `Plugins` folder. So `FreeCity4.dll` registers its own implementation of a class
ID, such as `cSC4Simulator` `0x2990C1E5`, and the game uses ours. Everything else keeps running the original code, so
every step can be tested in the real game. A component is replaceable this way if the game reaches it only through
interfaces. We checked this for the simulator: its only external callers are COM plumbing.

```
SimCity 4.exe (original, unmodified)
  └─ GZCOM registry ──► FreeCity4.dll (GPL)        engine/plugin  thin C++ shim over the gzcom-dll SDK (Win32/x86)
                          └─ C ABI (fc4_sim_*)     engine/capi    stable boundary, fc4.h
                              └─ fc4-core          engine/core    portable Rust: all engine logic
```

### Language: Rust core, C++ shim

- **Rust** holds all engine logic. It is memory-safe, has modern tooling, performs on par with C++ in games, and is a good
  base for the eventual standalone engine.
- **C++** is used only for the shim that must match the game's MSVC `__thiscall` vtable ABI. The LGPL gzcom-dll SDK
  already declares those interfaces in C++, so the shim stays small.

Migration path (to all-Rust, or any other rewrite) built in from day one:
1. **Specs** in `docs/specs/` are language-neutral and normative.
2. **Conformance vectors** in `engine/conformance/` are data files (scenario script + expected events and state) that
   any implementation must pass.
3. **The C ABI** in `engine/capi/include/fc4.h` is the only contract between core and shim. Replacing the C++ shim with
   Rust GZCOM bindings means reimplementing just that layer against the same API and vectors.

## Layout

| Path | Contents |
|---|---|
| `engine/core/` | Rust crate `fc4-core`: portable engine logic, no GZCOM types. It builds and tests on any OS. |
| `engine/capi/` | Rust crate `fc4-capi` (staticlib) and the C header `fc4.h`: the stable boundary |
| `engine/conformance/` | language-neutral conformance vectors derived from the spec sections |
| `engine/plugin/` | C++ GZCOM DLL director and adapters exposing the core as `cISC4*` interfaces (MSVC, Win32/x86) |
| `engine/third_party/gzcom-dll/` | LGPL-2.1+ SDK headers (submodule) |
| `docs/specs/` | clean-room behaviour specs: the only input for engine authors |
| `docs/subsystems/`, `re/`, `tools/` | research material for spec writers |

## Roadmap

| Milestone | Scope | Exit criteria |
|---|---|---|
| **M0: toolchain** | CMake project, core library, tests, a plugin DLL that loads in the game | the DLL loads in SC4 and its director logs `PostAppInit`; tests pass |
| **M1: the clock** | `cSC4Simulator` replacement ([spec](specs/simulator.md)) | a city runs at all speeds; pause/emergency/save/load work; a differential log of agent messages matches the original on the same save |
| M2: scalar sims | Budget, Demand, Ordinance, Weather | the budget and demand panels match the original after N months on reference saves |
| M3: grid sims | SimGrid, LandValue, Pollution, Crime, Aura, Power, Plumbing, Protection, Flora, Flammability | grid snapshots match within tolerance |
| M4: growth | BuildingDevelopment and the lot/tract/growth/zone developers | the growth statistics distribution matches across seeded runs |
| M5: traffic | traffic simulator and pathfinding | commute and volume maps match within tolerance |
| M6+: beyond simulation | data formats (DBPF, exemplars, S3D, FSH), rendering, UI, audio, region; then a standalone executable | FreeCity4 starts without `SimCity 4.exe` |

Verification uses **differential testing**: run the same save in the original and with our replacement, log messages
and grid states from a small observer plugin, and diff the results.

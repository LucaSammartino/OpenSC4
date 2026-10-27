# FreeCity4 architecture and roadmap

## Goal

Make the SimCity 4 engine **open source code that compiles**, with no copyrighted code from the original.
Players supply their own copy of the game's data. The result is a standalone game that behaves exactly like
SimCity 4: the same inputs under the same fixed work schedule give identical results.

## Why reimplement instead of recompiling the decompilation

The decompiled output in `decomp/` is derived from EA's copyrighted binary, so it can't be published as open source.
It also isn't a buildable reconstruction (it's Ghidra pseudocode). Successful projects in this space (OpenRCT2, OpenTTD,
OpenMW) instead **reimplement** behaviour and verify it against the original. We use our reverse-engineering only to
write behaviour **specs** (`docs/specs/`) and to record interface facts (`docs/abi/`). Engine code is written from
those specs by a different role (see [CONTRIBUTING.md](../CONTRIBUTING.md)).

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
Several other replacements also need direct calls into the exe and compat hooks. These are recorded in
[docs/abi/direct-calls.md](abi/direct-calls.md) and sit behind the 1.1.641 version check in `engine/plugin/compat`.

```
SimCity 4.exe (original, unmodified)
  └─ GZCOM registry ──► FreeCity4.dll (GPL)                               engine/plugin  thin C++ shim over the gzcom-dll SDK (Win32/x86)
                          └─ C ABI (fc4.h, versioned by FC4_ABI_VERSION)  engine/capi    stable boundary
                              └─ fc4-core                                 engine/core    portable Rust: all engine logic
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
| `engine/formats/` | Rust crate `fc4-formats`: readers for the game's data files (DBPF, QFS, exemplars, LTEXT, FSH, S3D); not used by the plugin |
| `engine/conformance/` | language-neutral conformance vectors derived from the spec sections |
| `engine/plugin/` | C++ GZCOM DLL director and adapters exposing the core as `cISC4*` interfaces (MSVC, Win32/x86) |
| `engine/tests/` | C++ tests: the C ABI, the plugin, and `abi_conformance_test` against `docs/abi/` |
| `engine/third_party/gzcom-dll/` | LGPL-2.1+ SDK headers (submodule) |
| `docs/specs/` + `docs/abi/` | clean-room behaviour specs and interface facts: the only inputs for engine authors, besides the SDK headers ([CONTRIBUTING.md](../CONTRIBUTING.md)) |
| `docs/NEXT_STEPS.md` | what we are doing next: the one pick-up document, rewritten in full on every change, never a log |
| `docs/ops/` | operating knowledge: rules, running the game, lessons (§5.x), coordinating the merge train, conventions |
| `docs/lanes/` | one file per lane, and the generated lane table `docs/lanes/INDEX.md` |
| `docs/subsystems/`, `re/`, `tools/` | research material for spec writers |

## Roadmap

Current status: [coverage-report.md](coverage-report.md) (measured by `python tools/coverage.py`).

| Milestone | Scope | Exit criteria |
|---|---|---|
| M0: toolchain | CMake project, core library, tests, a plugin DLL that loads in the game | the DLL loads in SC4 and its director logs `PostAppInit`; tests pass |
| M1: the clock | `cSC4Simulator` replacement ([spec](specs/simulator.md)) | a city runs at all speeds; pause/emergency/save/load work; a differential log of agent messages matches the original on the same save |
| M2: scalar sims | Budget, Demand, Ordinance, Weather | the budget and demand panels match the original after N months on reference saves |
| M3: grid sims | SimGrid, LandValue, Pollution, Crime, Aura, Power, Plumbing, Protection, Flora, Flammability | grid snapshots match within tolerance |
| M4: growth | BuildingDevelopment and the lot/tract/growth/zone developers ([specs](specs/building-development.md)). **Replacement units:** the growth developer together with the building development simulator's developer creation (one unit); the zone developer is headless-limited (its `Init` needs the 3D view) | the growth statistics distribution matches across seeded runs; the daily growth budget is calibrated in the original (building-development.md §12) |
| M5: traffic | traffic simulator and pathfinding ([specs](specs/traffic-simulator.md)). **Replacement unit:** the traffic simulator and the pathfinder together (they read each other's internals) | commute and volume maps match within tolerance; the traffic work budget is calibrated in the original (traffic-simulator.md §12) |
| M6+: standalone | our own host: save round trip, headless simulation, then rendering, UI, audio and region; runs in parallel with M4-M5 ([NEXT_STEPS](NEXT_STEPS.md) step 6) | FreeCity4 starts without `SimCity 4.exe` and matches the references |

Verification holds both games to **one fixed schedule** ([NEXT_STEPS](NEXT_STEPS.md) section 2; simulator.md §5.7):
every wall-time source reads one virtual clock, idle work runs at each day's settle point in a fixed order, no call that
does work is cut by a time limit, posted messages are delivered at the day's end, every seed derives from one session
seed and saved objects are numbered in registration order. `FreeCity4.dll` holds the original to that schedule at run time (never an exe file edit), the
original is recorded once per corpus scenario as the reference, and our game, with owners switched on (replace mode),
must produce identical checkpoints: save files first, then draw calls, sound events and UI state. The metric is
Exact-replaced (NEXT_STEPS section 3). Same-process shadows (`<sub>_shadow = 1`: the original keeps running and our
core computes the same step beside it) remain a debugging aid for finding the first difference; they are not proof.

### After the port: parity and beyond

Collected from [issue #1](https://github.com/tloriato/OpenSC4/issues/1) and design discussions. Every core keeps the
original's behaviour under the fixed schedule; nothing below changes results under it.

**Parity (a standalone game that replaces SC4):**

- **Finish the port** (M4-M6+ above).
- **Fixed schedule.** Some of the original's simulation work is paced by frames or real time, not by game days
  (simulator.md §5.7, the answer to DG31), so its cities grow differently at different frame rates, machine speeds and
  game speeds. FreeCity4 runs one fixed schedule, chosen in the spec as close to an infinitely fast machine at normal
  speed as the original allows, and comparisons hold the original to the same schedule ([NEXT_STEPS](NEXT_STEPS.md) section 2). It gives
  reproducible separate runs, a turbo clock with the same results as normal speed, replays, a headless/server mode, a
  path to lockstep multiplayer, and lets the long passes run on worker threads. The original's clock-bound pacing
  stays available as a switch (`pacing = original`) for shadow debugging. Saves are unaffected (only scheduling
  changes).
- **Visual assets for the standalone executable.** The game's own assets cannot be shipped; the standalone build
  either reads them from the player's own SC4 install or uses assets we can distribute.
- **Drop the C++ shim while keeping mod support.** Keep what current mods rely on (DBPF plugins and exemplars, Lua,
  and the most used DLL mods, supported or ported).
- **Profiling and benchmarks:** CPU per subsystem, multithreading actually used, RAM, and how the clocks and the
  pacing affect the simulation.
- **Keep a path to WASM:** the core stays free of platform calls (threads, files and time behind host interfaces).
- **Review the graphics path** (DirectX use and what a renderer of our own needs).

**The future: swappable packs.**

This repository stays a canonical reimplementation of the original game: its cores reproduce the original's
behaviour, and different algorithms do not go into them. The long-term direction is a modding architecture in which
each major part (each simulator, the developers, traffic, the pacing) sits behind a stable, documented interface
and can be swapped as a pack. The original's behaviour is then the pack the game ships with, and other games built
on the engine (a city builder with different algorithms, community overhauls) are packs of their own, maintained in
their own repositories and chosen by the player, never imposed on everyone. The replacement seams that the plugin
uses today (one class per subsystem, `enable = <key>`, the C ABI) are the starting point for those interfaces.

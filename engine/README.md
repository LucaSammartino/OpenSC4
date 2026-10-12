<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# FreeCity4 engine

Clean-room engine code for FreeCity4. It is written only from [`docs/specs/`](../docs/specs/) and the LGPL
gzcom-dll SDK. Read [CONTRIBUTING.md](../CONTRIBUTING.md) before you change anything here.

| Path | What | Language |
|---|---|---|
| [`core/`](core/) | `fc4-core`: the portable simulation core. No GZCOM or OS types, no `unsafe`. | Rust |
| [`capi/`](capi/) | `fc4-capi`: a stable C ABI over the core (`include/fc4.h`), built as a static library | Rust + C header |
| [`conformance/`](conformance/) | Language-neutral JSON test vectors (the migration contract) and their Rust runner | JSON + Rust |
| [`plugin/`](plugin/) | `FreeCity4.dll`: GZCOM director, the `cISC4Simulator` adapter over the C ABI, and our own standard message class (foundations §5.1; the SDK's `cRZMessage2Standard` is not used). Windows, x86 only. | C++17 |
| [`tests/`](tests/) | `capi_smoke` (drives the C ABI from C++ with the plugin's compiler and CRT) and `message_test` (foundations §7 item 12) | C++17 |
| `third_party/gzcom-dll/` | gzcom-dll SDK (LGPL-2.1+), git submodule | C++ |

```
SimCity 4.exe ── GZCOM ──► FreeCity4.dll (plugin/, C++)
                              cFC4Director  ─ registers CLSID 0x2990C1E5
                              cFC4Simulator ─ cISC4Simulator, cIGZSerializable,
                                              cIGZSystemService, cIGZMessageTarget2
                                     │  fc4.h (C ABI)
                                     ▼
                           fc4_capi.lib (capi/, Rust staticlib)
                                     │
                                     ▼
                           fc4-core (core/, Rust): Simulator, julian, save
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
3. **A/B against the original:** set `FC4_DISABLE_SIMULATOR=1` in the environment that starts the game. The DLL
   still loads and logs `PASSTHROUGH`, but it does not register the class, so the game's own simulator runs.

## Conventions

- Every source file starts with an `SPDX-License-Identifier: GPL-3.0-or-later` header.
- Spec gaps are decided in code and tagged `[choice]`. Behaviour the SDK cannot express yet is tagged
  `TODO(spec §n)`.
- Panics never cross the C ABI. Release builds use `panic = "abort"`, and each `fc4_*` entry point also catches
  unwinding.
- Every spec behaviour needs a conformance vector. See [`conformance/README.md`](conformance/README.md).

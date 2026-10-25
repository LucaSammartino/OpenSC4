# Contributing to FreeCity4

FreeCity4 is an **original, open-source reimplementation** of the SimCity 4 engine. It is licensed under
GPL-3.0-or-later. Players must own the game: FreeCity4 uses their installed copy's data files and, until the
replacement is complete, its executable.

Keep analysis of the original program separate from engine implementation.

## Two roles

The [lane prompt](.claude/workflows/fc4-lane.js) is authoritative for lane instructions and reading scope.
This table defines the project roles; a task can narrow access further.

| | Analysis (spec writers) | Implementation (engine authors) |
|---|---|---|
| May read | original executable, disassembly, `decomp/`, `decomp-mac/`, `ghidra/`, `re/`, specs and interface tables | `docs/specs/`, `docs/abi/`, `engine/` and its LGPL SDK headers in `engine/third_party/gzcom-dll`; operating docs within the prompt's reading scope |
| Produces | behaviour specs in `docs/specs/`; proposed ABI rows as the RE prompt directs | code in `engine/` and spec conformance tests |
| Must not | read or edit `engine/` | open, list, grep or read `decomp/`, `decomp-mac/`, `external/`, `ghidra/` (any part), `re/`, `tools/`, `docs/subsystems/`, `docs/sim-engine.md`, the game executable or disassembly/decompiled output, in either checkout; run scripts from `tools/` (`.claude/lane-bin/` may be run) |

Never hold both roles on one subsystem or put RE material in an engine task.

## The wall covers logic, not interfaces

`docs/abi/` records vtable slots, argument sizes, return registers, struct sizes and message IDs.
Engine interfaces must agree with it (`abi_conformance_test`). Algorithms, formulas, state changes and
ordering reach implementers only through behaviour specs.

## Specs

- Describe observable state, inputs, outputs, ordering, timing, formats and compatibility constants.
- No original code, decompiler output or line-by-line translations. Use prose, tables, worked examples and pseudo-math.
- Tag compatibility requirements *[compat]* and free design choices *[choice]*.
- Mark unknown behaviour explicitly; engine lanes follow the prompt's `SPEC-GAP` rule.

## Engine code

- Engine logic is **Rust** (`engine/core`, with the data-format readers in `engine/formats`) and must build and test
  on any OS. The C++ in `engine/plugin` is GZCOM glue (adapters, shadows, harness hooks); logic stays in Rust. It
  talks to the core through the C ABI in `engine/capi`. The C++ tests are in `engine/tests`.
- No panics or exceptions cross the FFI or GZCOM boundaries. Replacements plug in through GZCOM interfaces, such as
  class factories registered by our DLL director. Any dependency on a game layout or address must be an interface
  fact recorded in `docs/abi/`, sit behind the 1.1.641 version check (`engine/plugin/compat`), and be covered by a
  test.
- Every behaviour in a spec section should have a conformance vector in `engine/conformance`.

## How work lands

Follow the prompt's finish procedure and the [record formats](docs/lanes/README.md).
The [merge train](docs/merge-train.md) owns landing on `main`.

## Never commit

- Game files (executables, DLLs, `.dat` archives, saves) or anything extracted from them.
- Decompiled or disassembled code (`decomp/` is git-ignored on purpose).
- Anything from KillaVolt's repository, which grants no license and also ships EA's assets.


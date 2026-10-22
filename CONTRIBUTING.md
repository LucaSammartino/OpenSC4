# Contributing to FreeCity4

FreeCity4 is an **original, open-source reimplementation** of the SimCity 4 engine. It is licensed under
GPL-3.0-or-later. Players must own the game: FreeCity4 uses their installed copy's data files and, until the
replacement is complete, its executable.

The project only stays open source if every line in `engine/` is our own work. We separate the people
who study the original program from the people who write engine code.

## Two roles

This file is the single source for the clean-room roles. Other docs link here.

| | Analysis (spec writers) | Implementation (engine authors) |
|---|---|---|
| May read | the original executable, disassembly, Ghidra projects, `decomp/`, `decomp-mac/`, `ghidra/export/`, everything in `re/` | `docs/specs/` (questions for the analysis side go in `docs/specs/questions/<spec>.md`; `docs/specs/QUESTIONS.md` is the generated index), `docs/abi/` (interface facts), `engine/` with the public SDK headers in `engine/third_party/gzcom-dll` (LGPL), the game's data files, general literature; and the operating docs `README.md`, `CONTRIBUTING.md`, `docs/HANDOFF.md`, `docs/ops/`, `docs/ARCHITECTURE.md`, `docs/harness.md`, `docs/manual-checks.md`, `docs/lanes/INDEX.md`, `docs/lanes/README.md`, `docs/merge-train.md` (keep decompiled and RE-only material out of all of these) |
| Produces | **behaviour specs** in `docs/specs/` (what happens, in prose, tables and pseudo-math) and interface facts in `docs/abi/` | code in `engine/`, plus tests that check it against the specs |
| Must not | write or edit `engine/` code | open, list or grep `decomp/`, `decomp-mac/`, `external/`, `ghidra/` (any part), `re/`, `tools/`, `docs/subsystems/`, `docs/sim-engine.md`, the game executable, or any disassembly or decompiled output, in any checkout; paste anything from them; or run scripts from `tools/` (the lane tools in `.claude/lane-bin/` hold no RE content and may be run) |

One person may hold different roles on different subsystems, but **never both roles on the same subsystem**.
An agent doing engine work never reads RE material in the same task.

## The wall covers logic, not interfaces

How the game *calls* things is an interoperability fact, and engine authors may use it directly. That covers
vtable slots, argument sizes, return registers, struct sizes and message IDs. `docs/abi/` holds these facts,
recorded by the analysis side (tables from `tools/abi_extract.py` plus hand corrections). Every interface the engine
implements or calls must agree with it, and `abi_conformance_test` checks this. What a method *does* (algorithms,
formulas, state changes, ordering) still reaches engine authors only as a behaviour spec.

## Specs

- Specs describe observable behaviour: state, inputs, outputs, ordering, timing, file formats, message IDs and
  constants that are needed for compatibility.
- Specs contain **no code from the original program**: no decompiler output, no translated control flow line by line,
  no original local variable structure. Short pseudo-code that states an algorithm is fine. Transliterating a
  decompiled function is not.
- Tag compatibility requirements *[compat]* and free design choices *[choice]*.
- When behaviour is unknown, say so. Do not guess silently.

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

Work happens in lanes: one agent, one worktree, one branch, one task. A lane records its state and report in
`docs/lanes/<lane>.md` and each in-game run in `docs/runs/<tag>.md`, writes a merge-queue entry and **stops**; a
single merge-train owner lands batches on `main`. A lane never merges `main`, never bumps `FC4_ABI_VERSION` and
never edits a `<!-- BEGIN GENERATED: ... -->` region. The formats and the queue contract are in
[docs/lanes/README.md](docs/lanes/README.md); what the train does is in [docs/merge-train.md](docs/merge-train.md).

## Never commit

- Game files (executables, DLLs, `.dat` archives, saves) or anything extracted from them.
- Decompiled or disassembled code (`decomp/` is git-ignored on purpose).
- Anything from KillaVolt's repository, which grants no license and also ships EA's assets.


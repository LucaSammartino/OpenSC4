# Contributing to FreeCity4

FreeCity4 is an **original, open-source reimplementation** of the SimCity 4 engine. It is licensed under
GPL-3.0-or-later. Players must own the game: FreeCity4 uses their installed copy's data files and, until the
replacement is complete, its executable.

The project only stays open source if every line in `engine/` is our own work. We separate the people
who study the original program from the people who write engine code.

## Two roles

| | Analysis (spec writers) | Implementation (engine authors) |
|---|---|---|
| May read | the original executable, disassembly, Ghidra projects, `decomp/`, `external/extracted/`, everything in `re/` | `docs/specs/`, `docs/subsystems/` (behaviour descriptions), public SDK headers (gzcom-dll, LGPL), the game's data files, general literature |
| Produces | **behaviour specs** in `docs/specs/` (what happens, in prose, tables and pseudo-math) | code in `engine/`, plus tests that check it against the specs |
| Must not | write or edit `engine/` code | open decompiled output, disassembly, `decomp/`, `external/extracted/`, `ghidra/project/`, or paste anything from them |

One person may hold different roles on different subsystems, but **never both roles on the same subsystem**.

## Specs

- Specs describe observable behaviour: state, inputs, outputs, ordering, timing, file formats, message IDs and
  constants that are needed for compatibility.
- Specs contain **no code from the original program**: no decompiler output, no translated control flow line by line,
  no original local variable structure. Short pseudo-code that states an algorithm is fine. Transliterating a
  decompiled function is not.
- Tag compatibility requirements *[compat]* and free design choices *[choice]*.
- When behaviour is unknown, say so. Do not guess silently.

## Engine code

- C++17, no exceptions across GZCOM boundaries, and no dependency on the original program's private memory layout.
  Replacements plug in through public GZCOM interfaces, such as class factories registered by our DLL director.
- Platform-independent logic lives in `engine/core` and must build and test on any OS. GZCOM/Windows glue lives in
  `engine/plugin`.
- Every behaviour in a spec section should have a test in `engine/tests`.

## Never commit

- Game files (executables, DLLs, `.dat` archives, saves) or anything extracted from them.
- Decompiled or disassembled code (`decomp/` is git-ignored on purpose).

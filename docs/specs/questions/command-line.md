# command-line: open questions
<!-- Questions about [command-line.md](../command-line.md). Append new entries at the END; when one is answered, move it to answered/command-line.md. Rules: README.md. -->

<!-- from: re-cmdline (2026-10-19) -->
- **CL1 - §3:** slot 16 of `cRZCmdLine` (`0x00913E3B`, argument `i` as a `cRZString const&`, or the empty string)
  has no direct caller in the executable. Is it part of the published `cIGZCmdLine` interface, so plugins can call
  it, or a virtual of `cRZCmdLine` only? (Slot 15 is the deleting destructor, so slot 16 follows the interface.)

<!-- from: core-cmdline4 (2026-10-21) -->
- **CL2 - §4.3, §3 slot 9:** slot 11 (`GetIndexOfSwitch(name, start)`) with an **empty** `name`: §4.1 says slot 8
  returns false for an empty name, §4.3 has no such rule. Read literally, an empty name matches a lone prefix
  argument (`-`, `/` or `\`, length 1), so `IsSwitchPresent("")` (slot 9) is true on `[-]`. Is that the original
  behaviour, or does slot 11 also return −1 for an empty name? (FreeCity4 core implements the literal reading.)
- **CL3 - §3 "any case", §4:** for bytes 0x80-0xFF, does the "any case" comparison (`MakeLower` / `tolower`, and the
  single-letter compare of slots 10 and 12) leave them unchanged, or does the game's locale lower some of them (for
  example Latin-1 `É` 0xC9 to `é` 0xE9)? (FreeCity4 core lowers `A`-`Z` only.)

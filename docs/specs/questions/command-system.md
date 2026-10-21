# command-system: open questions
<!-- Questions about [command-system.md](../command-system.md). Append new entries at the END; when one is answered, move it to answered/command-system.md. Rules: README.md. -->

<!-- from: re-commands (2026-10-19) -->
- **GC1 - §1.3, §3.3, §4.2, §4.5:** the names of the interfaces behind the second IIDs: `0x4B8C39E4` / `0x4B8C39E5`
  (parameter set), `0x0B90251E` (string generator), `0x0B903CC3` / `0x0B903CC4` (dispatcher), `0x4B99446A` /
  `0x0B903CC5` (command message). Only `0x4B99446A` is asked for in the executable (two view handlers). Do plugins
  ask for any of the others?

<!-- from: re-commands (2026-10-19) -->
- **GC2 - §1.3:** what is `ValidateParameter`'s second argument? Only `0xFF` gives true, whatever the value's type,
  and the function has no direct caller in the executable.

<!-- from: re-commands (2026-10-19) -->
- **GC3 - §4.4:** does any handler execute a command while it handles one (for example `ExecuteScript` or
  `ExecuteCheat` running text that issues commands), so that the shared message and default sets are reused during
  delivery? A probe that logs nested `ExecuteCommand` calls would settle it.

<!-- from: re-commands (2026-10-19) -->
- **GC4 - §1.5:** which status values do the handlers use besides 2? A survey of the `SetStatusParameterValue` calls
  in the handlers would give the vocabulary.

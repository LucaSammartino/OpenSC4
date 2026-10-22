# tutorial: open questions
<!-- Questions about [tutorial.md](../tutorial.md). Append new entries at the END; when one is answered, move it to answered/tutorial.md. Rules: README.md. -->
<!-- from: glue-tutorial4 (2026-10-21) -->
- **TU7 - section 4.3 (the debug cheat's subcommands):** the cheat `0x6A382BAF` "Tutorial" handler (`0x0043F870`,
  message `0x230E27AC`) has subcommands reset, save, step, next, quit. Where in the message is the subcommand
  (the cheat string, data field 1/2, a `cIGZString`?), how is it parsed, and what does each one do (which slots /
  fields / registry and window calls, in order)? Only reachable when `cISC4App` slot 21 is true. [the handler is
  registered as specified, logs the cheat and ignores it]

# sc-resources: open questions

<!-- Questions about [sc-resources.md](../sc-resources.md). Answered entries move to answered/sc-resources.md. -->

No open questions. SCR1–SCR5 and SCR7 were answered by re-scres-scr5;
SCR8/SCR9 were answered on 2026-10-24 by re-scres-scr8 in spec section 10.
See [answered entries](answered/sc-resources.md). Runtime coverage remains
separate from these answered native contracts.

## SCR10 — Physical file-line observation ABI

Section 10.2 identifies `cRZTextFile::ReadLine` at `0x00930833` as the natural
startup file-input boundary, but section 10.1 does not declare its ECX view,
stack argument types/order, return channel, stack cleanup or output lifetime.
Please publish those and the open/path association needed to correlate reads
with nested `DoParseFile` calls, including EOF versus read failure and newline
handling. An engine observer cannot infer these from the entry address.

glue-scres-native6 implements the already specified shared ParseCommand and
ParseCommandArguments boundaries, registered paths and nested DoParseFile
calls. It compares physical command-line tokenization before substitution,
separately counting expanded/standalone dispatch. Complete file-parser replay
(including include directives, blank lines, EOF/failure and physical line
positions) remains default-off pending this input/output contract. The command
token comparison does not claim macro/variable/custom-command state parity.

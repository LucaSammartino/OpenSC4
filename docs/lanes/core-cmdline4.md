| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-cmdline4 (pure Rust core for cRZCmdLine, command-line.md §1-§8) | `core-cmdline4` / (engine lane) | ready: engine/core/src/cmdline.rs, all 17 slots, 18 tests; no ABI, no game run; gate cargo 3821 passed / clippy 0 / ctest 63/63; questions CL2, CL3 filed (CL1 open) | 1. Coordinator: tools/questions.py index (CL2, CL3). 2. Glue lane: cIGZCmdLine wrapper over CmdLine if the plugin ever needs to own one (slot 16 waits on CL1) |

## Report

What changed (pure Rust; no C ABI, plugin, CMake or tools changes):
- `engine/core/src/cmdline.rs`: `split` (§2.1) and `join` (§2.2) as free functions; `CmdLine` with one method per
  vtable slot: `query_interface`/`add_ref`/`release` (0-2, count starts at 0, release at 1 returns 0 = destroy),
  `argc`, `argv`, `get_command_line`, `find`, `find_sub_string`, `is_switch_present_value` (8),
  `is_switch_present` (9), `is_switch_present_char` (10), `get_index_of_switch` (11), `get_index_of_switch_char`
  (12), `insert_argument` (13), `erase_argument` (14), Drop (15), `get_argument` (16). Constructors `new` (char
  const*) and `from_parts`/`copy_of` (the cIGZCmdLine copy: text not re-split). Byte oriented (`&[u8]`).
- Native-undefined paths are explicit errors, object unchanged: `argv`/`get_argument` with a negative index ->
  `CmdLineError::NegativeIndex`; `erase_argument` on an empty list -> `EraseFromEmpty`, negative index ->
  `NegativeIndex`. Slot 8's one-quote value gives an empty value (the spec's [choice]).
- Slot 8/10 keep the out-parameter shape so "value not written" (no match; bare `-c`) is exact.
- `engine/core/src/cmdline/tests.rs`: 18 tests, every §8 conformance item (1-8) plus edges (quote counting,
  `\r`, join of empty/tab/newline args, insert clamping, erase errors leave the text unrejoined, slot 10 single
  quotes and 3-char args, empty-substring rule, i32::MIN index).

Measured: baseline cargo 3803 passed -> 3821 passed / 0 failed; clippy -D warnings 0; CMake build PASS; ctest
63/63; rustfmt clean on the new files. No game run: core only, nothing the plugin calls yet (no shadow key).

Questions (docs/specs/questions/command-line.md): CL2 (slot 11 with an empty name; literal reading: a lone prefix
matches), CL3 (bytes 0x80-0xFF under "any case"; implemented as A-Z only). CL1 (slot 16 publication) stays open; no
ABI was guessed.

Coverage honesty: every documented behaviour of the 17 slots is implemented in core; the owner is not glued, so
shadow/verified/replaced stay "-".

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-audio-events5: audio event handler owner closure | Codex; `core-audio-events5` | wip: specified paths implemented; AU13–AU16 block closure; Rust checks pass, native configure stalled | Resolve spec gaps, regenerate question index, rerun native/full gate in a working build environment. |

## Result

WIP; this is not an owner-closure claim. Implementation commit: `8565c6a4`.
The audio module had no README, so the new `engine/core/src/audio/README.md`
documents the boundary and maps each relevant spec section and conformance
requirement to code, tests, or the precise question blocking closure.

Implemented successful service acquisition/registration, separate modal/main
subscription sets, city-only subscriptions/delivery, and shutdown preserving the
city set until PreCityShutdown. Added the complete specified message-route table,
null/non-null return contract, foreign radio slot forwarding, and the queued
station-one message. Added full ordinary automata payload handling, capability/QI
gates, balanced successful payload queries, typed position forwarding, specified
play/stop arguments, and a shared limiter that draws randomness only on restart.
`Default` now agrees with `new`. Existing word-only helpers remain compatible
fixture APIs; the README distinguishes them from the foreign-payload entry.

Foreign service/payload traits keep the sound system, radio and simulators outside
the owner. Missing owner algorithms report explicit gap diagnostics rather than
inventing sound selection. No plugin, capi, C ABI, ABI version or coverage-map
changes. No game files read or committed.

## Verification

- Focused handler run: 19 passed, zero failed, before the last identity test was
  added. Final workspace run includes all 20 handler tests (12 added here).
  Tests cover lifecycle calls, routing, reference balancing, entry failures,
  positions, radio slots, exact limiter boundary, and actual automaton posts
  through the handler and sound-system events into the manager model.
- Explicit-file formatting: PASS, all three owned Rust files, zero diff lines.
- Required `qbuild all`: run once and collected to terminal result, overall FAIL.
  Cargo: **6121 passed, 0 failed, 0 ignored**, summed over 30 result lines.
  Clippy: **PASS, zero diagnostics**.
- Native configure stalled at MSVC C compiler ABI detection. After 707 seconds,
  process inspection found the lane-owned CMake/Ninja subtree with no compiler
  child and almost no CPU. Only that stalled subtree was terminated; the same
  qbuild operation then reported configure FAIL and CTest skipped. No native
  build/CTest pass is claimed. The verifier additionally flagged the docs-only
  handoff commit made during the run; Rust source stayed unchanged throughout.
- Evidence: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-configure.log`, `native-interruption.log`, `gate-terminal.txt`, and
  `audio-handler-focused.log`. Initial q-wrapper failure (Bash
  CreateFileMapping access denied) is retained in
  `build-runs/q/20261023-180623-10600.log`; focused cargo subsequently ran natively.
- No game run requested: core-only foreign-trait work, no plugin integration or
  shadow key. Game verdict/ref-warning/shadow measurements are not applicable.
  No operation or request remains outstanding.

## Questions and remaining owner coverage

Appended `AU13`–`AU16` to `docs/specs/questions/audio.md`:

- **§7.1 and §7.2/§7.3 lifetime/failures — AU14:** omitted constructor defaults,
  capability acquisition, missing services, failed registration/subscription,
  repeated lifecycle calls, and exact precondition failure returns.
- **§7.2 and §13 dispatch algorithms — AU13:** construction, radio selection,
  EC4 message construction, LAB branch, modal begin/end and mouse behavior;
  slot10/slot2 ownership and unlisted subscribed-type behavior.
- **§7.3 and §13 alternate automata/vector details — AU15:** A9E3B6E3 branch
  selection/arguments/derived ID; EA3ADA33 calls, arguments and state; vector wire
  packing, instance-key source and failed vector lookup behavior.
- **§7 callback/destruction and §7.4/§9.5 torch attribution — AU16:** DoEvent's
  complete event table, effects/failure returns, destructor cleanup, and conflicting
  attribution plus unspecified torch algorithms.

These are omissions in the supplied behavior source, not behavior already marked
open by its author; the lane therefore remains WIP. Spec answers are needed
before claiming complete owner coverage. Existing AU8 also keeps radio internals
foreign. The coordinator must regenerate/check the question index: this lane
cannot run `tools/questions.py` or edit generated `QUESTIONS.md`.

## Next steps

1. Answer AU13–AU16, then implement the missing owner algorithms and replace gap
   diagnostics with the specified behavior and tests.
2. Regenerate the question index through the coordinator's permitted workflow.
3. Resolve the native compiler-probe stall and run the full gate on an unchanged
   tree before promoting this lane to ready. No duplicate gate was launched.

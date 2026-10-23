| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-auto-game-pause4 (portable cSC4AutoGamePause scoped class and SC4GamePause four-stage helper, auto-game-pause.md §§2–4) | `core-auto-game-pause4` / (engine lane) | ready: engine/core/src/auto_game_pause; 11 literal fixture tests; gate cargo 5737 passed / 0 failed, clippy 0, ctest 68/68; ABI unchanged; no game runs; no questions filed | 1. Merge train lands it. 2. A later lane may bind Host/Storage to the plugin; AGP1 (F1 interface identity) stays open. |

## Report

- `engine/core/src/auto_game_pause/mod.rs`: `AutoGamePause<S: Storage>` stores low8(F1..F4) at offsets 0..3 before any callback, enters with the original full words `(1,F1..F4)` and returns itself; `destruct` reads offsets 3,2,1,0 zero-extended and calls the helper with `(0,...)`, without touching storage or returning a status.
- `sc4_game_pause` runs the four stages in fixed order, each gated by its own low byte: zeroed temporary, framework acquisition, slot `+0x14` lookup with the literal §3.1 keys, null-output guard, direction-selected operation (F1 `+0x48`/`+0x4C`, F2 `+0x30(1/0)`, F3 `+0x28`/`+0x34`, F4 app `+0x94` city, city `+0x158` clock, clock `+0x68(1/0)`), then one Release `+0x08` of the lookup output. Lookup, operation and Release replies are ignored; there is no rollback and no saved bindings.
- Host is a trait of individual raw primitives (framework, lookup, call, current_city, clock, release). Storage is a four-byte trait so tests can see the order of stores and reads.
- `tests.rs`: 11 tests on the §4 literal traces. They cover the 16-row selection table in both directions, the raw-byte fields (`2/0x80/0xFFFFFFFF/0x100`), repeated entry, interrupted A/B pairing, every null guard, ignored false replies, and reacquisition with changed identities. They are traces the spec proposes, not native runs.
- This round: added a type alias for the test observation tuple to fix clippy `type_complexity`, which was the only gate failure.
- Gate (final source): cargo 5737 passed, 0 failed (summed over 27 `test result:` lines); clippy -D warnings 0 diagnostics; CMake build 217/217; ctest 68/68 passed. rustfmt is clean on both module files. `core/src/lib.rs` only gains one appended `pub mod` line. Its existing unsorted order on main was left alone.
- Not done on purpose: no plugin/CAPI wiring, no game binding, no Drop-based exit; AGP1's typed interface is left unresolved (raw slots only); allocation, copy/move, duplicate destruction, reentry and unwind are outside the contract per §2.

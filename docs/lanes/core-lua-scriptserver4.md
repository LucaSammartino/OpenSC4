| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-lua-scriptserver4: pure Rust ScriptServer | Codex / `core-lua-scriptserver4` (engine, detached) | ready: complete 50 real bodies / 7658B; 31 public vectors + 34 edge tests; cargo 4354 PASS, clippy 0 diagnostics, CMake 4/4, ctest 67/67; game n/a; questions none; ABI unchanged | 1. Coordinator publish detached HEAD and integrate queued core owner. |

## Report

Implemented the complete pure Rust cGZLuaScriptServer owner from lua-host.md section 6.3,
using only the public contract and re-lua-scriptserver4 report. The code lives in
`engine/core/src/lua_script_server`; `OWNER.md` maps all 50 real attributed bodies / 7658B.
The false 42B string-argument reach has no invented callback. No attribution/coverage maps changed.

The owner implements shared QI/reference lifetime, explicit destruction, the Init failure latch and
Shutdown retry, foreign VM IDs and signed display serials, source/file failure distinctions,
signed pause/resume counters, live ordered tick traversal and nested-tick suppression, all eight
breakpoint methods, enumeration and optional info outputs, current/last IDs and auto-yield,
global userdata and per-thread registry argument tables, lookup callbacks and line hooks.
Notification setup, mutation, send and reference-release order belongs to Rust.

Foreign primitives cover VM/thread execution and stack operations, command-generator services,
the parser, string folding, file/debug/base services and message transport. No original ScriptServer
outputs are delegated. Invalidating callback paths, duplicate live IDs, malformed replies, null
successful threads and unavailable required message storage have explicit typed failures or storage
preconditions. There is no deferred-kill, rollback or invalidating-reentry recovery guarantee.
The embedding layer must call explicit Release/destruction while a foreign provider is available.

## Measured validation

- 31 independent public vectors in `engine/conformance/tests/lua_script_server.rs`, in the three
  specification tables' order: PASS.
- 27 additional integration edge tests and 7 core edge tests: PASS. They cover ownership, factory/
  initialization failure and retry, notification wire/order, snapshot versus live traversal, valid
  and invalidating callback reentry, optional output ownership, signed/wrapping counters, ID protocol,
  all breakpoint methods, registry isolation and hook gates.
- Last focused quiet gate: 3677 tests passed, 0 failed; dependent crates compiled; CMake 4/4 (132.9s).
- One final `qbuild.py all` on code commit `9cf51d14`: cargo workspace **4354 passed / 0 failed**;
  clippy `-D warnings` **0 diagnostics**; CMake **4/4**; full ctest **67/67**. fc4verify PASS (92.1s).
  Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
- Formatting wrapper: all 8 new Rust files PASS. Checking shared `engine/core/src/lib.rs` also exposes
  existing module-order differences (e.g. commands/cmdline); its single appended export is retained
  without mass reformatting. `git diff --check` PASS.
- Final scope: 11 files, confined to this lane report, core owner/export and conformance tests/support.
  No C ABI, CAPI, plugin, native registration, game, spec or coverage-map changes.

The coordinator's stable baseline was accepted as instructed; no pre-edit full gate was repeated.
A test-only quick build once produced a Ninja-restat 1/4 incomplete verdict despite exit zero;
the final gate rebuilt only this worktree's DLL and capi smoke output and completed all four edges.
q.py could not start Bash, so bounded PowerShell commands were used. Detached commits succeeded
despite Git's packed-refs.lock permission warning; refs/config were not modified.

## Questions, runtime status and next step

Questions: none; no missing normal owner contracts. Game runs: n/a, no requests outstanding.
This is complete pure-core owner work with typed foreign dependencies; native binding, runtime shadow
and replacement are separate work and receive no claim here. ABI exports unchanged.

Milestones: `d217437c` owner and 31 vectors; `ae7636ea` ownership/reentry audit and full inventory;
`9cf51d14` final counter/ID edges. This report commit follows the successful full gate.
Coordinator: publish the actual detached HEAD to the reserved lane branch and integrate the ready queue.

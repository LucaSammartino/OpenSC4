| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-lua-scriptserver4: pure Rust ScriptServer | Codex / `core-lua-scriptserver4` (engine, detached) | in progress; no ABI changes | 1. Implement lua-host section 6.3 and 31 independent vectors. 2. Run focused checks and final gate. |

## Handoff
- Goal: complete pure Rust cGZLuaScriptServer owner, with typed foreign execution/services.
- Base: a4fe4219; detached HEAD intentionally retained; clean starting tree.
- Read: short handoff index, lane/queue formats, re-lua-scriptserver4 public report.
- Done: d217437c owner/31 vectors; ae7636ea ownership/50-body inventory; final tests cover 27 integration and 7 core edges.
- Done: all 50 real bodies / 7658B mapped in engine/core/src/lua_script_server/OWNER.md; false 42B excluded.
- Quick gate PASS: 3677 tests, 0 failures; dependents compile; CMake 4/4 edges (132.9s).
- Formatting: 8 new Rust files check PASS. Shared lib.rs has preexisting module-order diffs; only additive mod export retained.
- Next: one stable final full cargo/clippy/ctest gate; then final report and ready queue.
- Build note: test-only changes can cause Ninja restat to show 1/4 edges despite exit zero; wrapper marks incomplete.
- Baseline: coordinator stable native train gate accepted; task forbids repeating pre-edit full gate.
- q.py cannot start Bash (E_ACCESSDENIED); use capped direct PowerShell commands.
- No game requests, no questions, no native/ABI/plugin changes.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-lua-scriptserver4: pure Rust ScriptServer | Codex / `core-lua-scriptserver4` (engine, detached) | in progress; no ABI changes | 1. Implement lua-host section 6.3 and 31 independent vectors. 2. Run focused checks and final gate. |

## Handoff
- Goal: complete pure Rust cGZLuaScriptServer owner, with typed foreign execution/services.
- Base: a4fe4219; detached HEAD intentionally retained; clean starting tree.
- Read: short handoff index, lane/queue formats, re-lua-scriptserver4 public report.
- Done: complete owner modules plus 31 independent public conformance cases and 4 core edge tests.
- Quick gate PASS: 3647 tests, 0 failures; dependents compile; CMake 206/206 edges.
- Next: ownership/failure/reentry edge audit, full owner inventory, formatting check, one stable final full gate.
- Baseline: coordinator stable native train gate accepted; task forbids repeating pre-edit full gate.
- q.py cannot start Bash (E_ACCESSDENIED); use capped direct PowerShell commands.
- No game requests, no questions, no native/ABI/plugin changes.

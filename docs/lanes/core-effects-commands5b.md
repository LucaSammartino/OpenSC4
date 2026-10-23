| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-effects-commands5b: settled effects script commands | Codex / `core-effects-commands5b` | in progress | Implement settled parser owners and vectors; full gate |

## Handoff
- Goal: close settled effects-parser sections 9–13 and effects section 19 in existing core modules, no ABI/plugin changes.
- Done: concrete token readers, dispatch/registration dictionaries, description directory identity, explicit final lifetime in script.rs; SW1–8/DI1–5/LT1–3/DR1–6 tested (5 tests passed).
- Next: implement root command execution and rotation composition, then definition/child families through current EC276.
- Open residual questions: EF15 and EFP7; verify precise scope from specs.
- No game requests outstanding; core-only task.
- Focused cargo test passed; fmt two new files clean. q.py cannot launch Bash (Win32 error 5); use native PowerShell; evidence build-runs/q/20261023-183457-12916.log.
- Formatting helper build-runs/format-patch.py emits apply_patch input from rustfmt stdin (no filename headers); uses engine/rustfmt.toml.

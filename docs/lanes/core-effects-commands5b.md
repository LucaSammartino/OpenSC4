| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-effects-commands5b: settled effects script commands | Codex / `core-effects-commands5b` | in progress | Implement settled parser owners and vectors; full gate |

## Handoff
- Goal: close settled effects-parser sections 9–13 and effects section 19 in existing core modules, no ABI/plugin changes.
- Done: concrete token readers, dispatch/registration dictionaries, description directory identity, explicit final lifetime in script.rs; SW1–8/DI1–5/LT1–3/DR1–6 tested (5 tests passed).
- Next: implement particle handlers §§19.18–26 and 19.39–51 (EC72–119/EC183–276), decal §§19.35–38 (EC161 onward), and §19.52 dictionary integration; read each spec section first.
- Open residual questions: EF15 and EFP7; verify precise scope from specs.
- No game requests outstanding; core-only task.
- Focused cargo test passed; fmt two new files clean. q.py cannot launch Bash (Win32 error 5); use native PowerShell; evidence build-runs/q/20261023-183457-12916.log.
- Formatting helper build-runs/format-patch.py emits apply_patch input from rustfmt stdin (no filename headers); uses engine/rustfmt.toml.
- Milestone be2e2dbb: readers/dispatch/directory/lifetime.
- Root owners through effects §19.11 now implemented in script/commands.rs with EC1–50 fixtures; focused script tests 12/12 passed, fmt four files clean.
- Owner progress table added; final closure still requires definitions/children and integration. No new questions or ABI changes.
- Milestone 46e94dd6: root owners EC1–50; be2e2dbb: dependencies.
- Added definitions/defaults/inheritance/publication and effectBase (EC51–68), particle registration (EC69–71), dynamic/sequence/shake/light children and registrars (EC120–160).
- Added rotation Arithmetic instruction backend and RO1–6 fixtures; legacy parse_options uses it, default absent backend returns Numeric (EFP7).
- Latest focused parser suite 60/60 passed; fmt all 14 module .rs files clean; diff --check clean.
- Full gate started once on current source: session 83508; cargo workspace 6182 passed/0 failed, clippy clean; CMake configure still pending at compiler ABI detection. Collect SAME session to terminal; do not start another gate.
- Gate logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-configure.log. No game run is needed/claimed for core-only owners.
- Dynamic model keeps full signed EAX and selected indices as separate primitive inputs (EC126/130); no guessed native full-register return.
- Definitions use sparse native-coordinate maps with absent indeterminate words; reuse existing effects::EffectDescription and ComponentRecord for visual/runtime payloads.
- Commands methods return before generic push/pop; no turnkey script interpreter/lexer wired yet. Complete settled handler routing before claiming final closure.

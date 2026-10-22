| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-paths-contractfix4: PA4-PA8 portable path corrections | `Codex` / `core-paths-contractfix4` (engine lane) | in progress | Audit published contracts, correct portable paths, run full gate |

## Handoff
- Goal: apply PA4-PA8 to production paths and directory-entry lifecycle; reuse cRZDirectory.
- Scope: portable Rust only; no ABI/shared files, native glue, game runs, or whole-owner completion claim.
- Done: first substantive PA4 correction preserves the full counted ResolvePath input on the full-prefix branch; independent NUL/copy vector added.
- Baseline qbuild all launched; logs in build-runs/qbuild-*.log (result pending).
- Done: added signed-byte CRT primitive boundary/errors; counted ResolvePath, entry default/copy/name-only drop, signed SplitDirectory with partial writes, retained find cache/retry/finish.
- Done: removed duplicate paths directory owner/comparator; paths reexports authoritative rz_files owner. Primitive-only test host bridges it. Added PA4-PA8 independent vectors.
- Tests: baseline all finished compilation in 22m35s and is running workspace tests; quick still compiling. fmt check passed for all 7 touched Rust files (absolute paths required).
- Next: resolve compile/fmt findings, final stable full gate, report exact scope and enqueue if green.
- New questions PA9/PA10: negative signed capacity follows literal signed-loop no-output rule; zero-part SplitDirectoryPath fills both safe empty outputs. PA1/PA2/PA3 retained.
- Done: nullable next/finish boundaries added; conformance runner propagates unsupported CRT domain errors and accepts signed split count; literal JSON zero/one-byte/root vectors added.
- Entry list remains the authoritative owner's passive payload; standalone lifecycle adds no native getters/COM or owner lifetime extension.
- PA1/PA2/PA3 remain open; preserve selected normalization rule.
- Correction reservation logical round 2/3; this task permits one round only.
- q.py cannot launch Bash (E_ACCESSDENIED); use bounded PowerShell output and required qbuild wrappers.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-paths-contractfix4-r1-manifest.json.

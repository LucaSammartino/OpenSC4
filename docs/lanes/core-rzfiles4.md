| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-rzfiles4 (pure Rust core for cRZFile + cRZIniFile, framework-rz-files.md §1-5) | `core-rzfiles4` / (engine lane) | in progress | see Handoff |

## Handoff

- Goal: engine/core/src/rz_files/ (host trait, file.rs, ini.rs, tests) exact to spec §1-5; no C ABI/plugin/CMake; game false.
- Done: worktree created from main.
- Next: write module, qbuild quick, then all; questions in docs/specs/questions/framework-rz-files.md (prefix RZ, new).
- Facts: question prefix RZ unused in QUESTIONS.md; start at RZ1.

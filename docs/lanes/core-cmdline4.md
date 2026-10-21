| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-cmdline4 (pure Rust core for cRZCmdLine, command-line.md §1-§8) | `core-cmdline4` / (engine lane) | in progress | see Handoff |

## Handoff

- Goal: engine/core/src/cmdline.rs: all 17 cRZCmdLine slot behaviours (split/join/insert/erase/find/5 switch
  lookups), safe errors for the native-undefined negative index paths; unit tests from spec §8 + edge cases. No
  plugin/CAPI/CMake/ABI changes. CL1 stays open.
- Done: worktree created (base main b5253abf).
- Next: baseline `qbuild.py all`, then write cmdline.rs.

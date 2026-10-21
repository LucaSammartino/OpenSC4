| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-fonts4: clean-room cGZFont / cGZFontSys core from docs/specs/drawing-fonts.md | `core-fonts4` / (engine lane) | in progress | see Handoff |

## Handoff

- Goal: Rust core module engine/core/src/fonts (cGZFont first, then cGZFontSys) from drawing-fonts.md; no plugin/CAPI/CMake edits; no cGZDrawFontRenderer.
- Done: worktree at main 2b91de8c.
- Next: read spec §1-§5, record baseline `qbuild all`, start engine/core/src/fonts/mod.rs.
- Facts: round 1 of at most 3.

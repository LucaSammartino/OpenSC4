| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-fonts4: clean-room cGZFont / cGZFontSys core from docs/specs/drawing-fonts.md | `core-fonts4` / (engine lane) | in progress | see Handoff |

## Handoff

- Goal: Rust core module engine/core/src/fonts (cGZFont first, then cGZFontSys) from drawing-fonts.md; no plugin/CAPI/CMake edits; no cGZDrawFontRenderer.
- Done: worktree at main 2b91de8c; baseline all PASS (cargo 3980, clippy 0, ctest 65/65). engine/core/src/fonts/ (mod, aa, cache, director, encoding, font, fontsys, layout, thai, tests: 14 tests) committed; quick PASS (fc4-core 2160).
- Next: file questions FN7-FN14 (docs/specs/questions/drawing-fonts.md; cannot run tools/questions.py index), clippy via qbuild all, then a game run is not useful (no plugin glue allowed): report.
- Facts: round 1 of at most 3.

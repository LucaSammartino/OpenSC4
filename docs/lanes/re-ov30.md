| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-ov30 / overlay-manager OV17; kind docs | Codex / .claude/worktrees/re-ov30 / reserved branch re-ov30; detached HEAD | ready; spec commit fcbf452c3; 8/8 consumer call boundaries and worked calls documented; OV17 answered; docs checks pass; cargo/clippy/ctest n/a; game/shadow n/a; C ABI unchanged, no seam reservation | 1. Land docs queue. 2. Resume replace-overlays28 terrain-view routing using §4.6.1. |

- Added overlay-manager.md §4.6.1: full terrain-view-base ECX, three argument layouts, entry-stack offsets, all eight callee cleanup addresses and return channels.
- Ordinary land/water: rectangle value then context, 20-byte callee cleanup. Extension land: context, extension, rectangle value, 24-byte cleanup. Shadows: rectangle pointer then context, 8-byte cleanup.
- Eight worked calls specify expanded bounds, pointer values, entry ESP and balanced return ESP. The shadow caller's 16-byte rectangle local is separate from its arguments.
- Evidence: native dispatch, view/manager Init, QueryInterface and all eight consumer entries/returns queried with re_query.py asm --bytes; existing decomp used only as a locator/cross-check. REA not used.
- OV17 moved to questions/answered/overlay-manager.md with MATCHES disposition; question index regenerated (overlay-manager 5 open / 12 answered).
- Final validation: questions.py check passed; git diff --check and staged diff check passed after removing a trailing blank line found by the first whitespace check. No build or game run required for docs-only changes.
- replace-overlays28 at 1b2642844c83e4751bb4e436236cb4d70e5c48bc was already an ancestor; merge reported Already up to date. Git emitted a shared packed-refs lock permission warning; detached commit succeeded.
- No engine, docs/abi or C ABI changes; no ABI-row proposal. OV8 geometry algorithms remain outside this answer.

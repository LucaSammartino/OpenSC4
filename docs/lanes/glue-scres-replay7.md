| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-scres-replay7: physical file replay | Codex, `glue-scres-replay7` (detached HEAD) | in progress | Apply SCR10, gate, and measure a 2032 turbo run. |

## Handoff
- Goal: natural physical file-parser replay exact with zero resyncs; preserve scres and house shadows.
- Done: initial handoff b101ce315; implementation 60758578d (native constructor/open/ReadLine association and production file-loop replay).
- Focused capi scres tests: 7/7 passed (including 2 replay tests); initial Bash wrapper failed due sandbox CreateFileMapping (log preserved), direct cargo succeeded.
- Formatting PASS for both owned Rust files, no diffs.
- Stable-source qbuild all: Rust 6904 passed, 0 failed/ignored; clippy 0 diagnostics.
- Native request qbuild-20261024-041758-build-ctest completed PASS: build 240/240 (116.2s), CTest 74/74 (143.6s), total 263.3s; qbuild all PASS.
- Native gate's tree-changed note is the docs-only c05a06dd4 handoff commit; engine source stayed at 60758578d throughout.
- Next: collect unique scres7-physical-2032-ash verdict using build-runs/wait-verdict.py, then inspect counted replay lines.
- Keep new observation default-off behind ini; no public C ABI changes planned.
- Game request scres7-physical-2032-ash being enqueued with this milestone. No native operation outstanding.
- Current house-profile string is in prohibited tools/regress.ps1; do not read/edit it or merge main. Coordinator must append exact new key after review.

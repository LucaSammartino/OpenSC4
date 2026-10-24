| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-scres-replay7: physical file replay | Codex, `glue-scres-replay7` (detached HEAD) | in progress | Apply SCR10, gate, and measure a 2032 turbo run. |

## Handoff
- Goal: natural physical file-parser replay exact with zero resyncs; preserve scres and house shadows.
- Done: initial handoff b101ce315; implementation 60758578d (native constructor/open/ReadLine association and production file-loop replay).
- Focused capi scres tests: 7/7 passed (including 2 replay tests); initial Bash wrapper failed due sandbox CreateFileMapping (log preserved), direct cargo succeeded.
- Formatting PASS for both owned Rust files, no diffs.
- Stable-source qbuild all: Rust 6904 passed, 0 failed/ignored; clippy 0 diagnostics.
- Native request qbuild-20261024-041758-build-ctest outstanding via qbuild all; collect original exec session 95113 to terminal, never duplicate.
- Next: finish native gate; then unique scres7-physical-2032-ash game request (not yet written).
- Keep new observation default-off behind ini; no public C ABI changes planned.
- No game operation outstanding. Native operation above is still outstanding.
- Current house-profile string is in prohibited tools/regress.ps1; do not read/edit it or merge main. Coordinator must append exact new key after review.

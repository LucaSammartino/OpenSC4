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
- Game ash (run 403) clean to 2032-01-04, ref0; exact existing 76/76, overall scres 76/149 because 73 file replay lines each mismatches=72, resyncs=0, refused=0.
- Captured 1107 physical reads, 122 empty, two false reads, 3 opens (one failed), 1061 commands, one include. Inclusive totals 1530.
- Diagnostic gate PASS: Rust6904, clippy0, native47/47 (2.5s), CTest74/74 (75.2s), queue qbuild-20261024-042645-build-ctest (79.3s), terminal collected.
- Birch run404 clean/ref0 to2032-01-15; scres76/149, same raw72 differences; 45 reads skipped starting at physical216 (#<). SCR11 filed for unspecified block grammar.
- Added separate partial file-control replay with observed preprocessing as explicit foreign input; strict raw replay still reports its differences (no suppression/resync).
- Focused tests now8/8, fmt PASS. Next: stable-source final full gate, then unique scres7-control-2032-cedar to2032.
- Keep new observation default-off behind ini; no public C ABI changes planned.
- No game/native operations outstanding. Prior ash/birch evidence retained in build-runs/game-results/; records403/404.
- Whole cFileParser verification remains blocked on SCR11; do not credit that owner row or append the new key to house profile unless raw replay becomes exact.
- Current house-profile string is in prohibited tools/regress.ps1; do not read/edit it or merge main. Coordinator must append exact new key after review.

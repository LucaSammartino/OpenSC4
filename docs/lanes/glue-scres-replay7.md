| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-scres-replay7: physical file replay | Codex, `glue-scres-replay7` (detached HEAD) | in progress | Apply SCR10, gate, and measure a 2032 turbo run. |

## Handoff
- Goal: natural physical file-parser replay exact with zero resyncs; preserve scres and house shadows.
- Done: initial handoff b101ce315; implemented native constructor/open/ReadLine association and production file-loop replay.
- Focused capi scres tests: 7/7 passed (including 2 replay tests); initial Bash wrapper failed due sandbox CreateFileMapping (log preserved), direct cargo succeeded.
- Formatting checked and corrected with apply_patch; final formatting check next.
- Next: final focused conformance tests and stable-source full gate; then 2032 turbo game request.
- Keep new observation default-off behind ini; no public C ABI changes planned.
- No build or game operation outstanding.

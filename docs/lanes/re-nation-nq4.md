| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-nation-nq4: tutorial directory case mapping | `Codex` / `re-nation-nq4` (RE, detached HEAD) | in progress | Resolve NQ4 from export, add vectors, regenerate index and run doccheck. |

## Handoff

- Goal: close core-nation10 NQ4 with behavior and conformance vectors, no code edits.
- Initial handoff committed as 0e581e39d; read core-nation10 lane report and question via git show.
- Owner closure table is under engine/ and cannot be read under this task's explicit prohibition; use the published lane report and NQ4 contract instead.
- Done: nation §6.4 defines UTF-8 / active CRT LC_CTYPE folding and converter quirks; §5.2 adds ten vectors.
- Done: NQ4 archived as DIFFERS, question index regenerated; questions check, doccheck check and diff whitespace check pass (2026-10-24).
- Next: commit this spec milestone, then replace report and enqueue actual full HEAD as kind re / ready.
- Evidence: MakeLower 0090eac5; UTF-8 converter 00910e18; CRT wide lowercase 009f4c35 / 009f4bd4; language locale 00928021.
- C locale retains É; en-US folds É to é. Malformed bytes 41 E9 42 become 61; F0 9F 98 80 becomes EF 98 80 in C locale.
- Init/reset use verbatim registry results; registry folds at 004b7998 and 004b7bb2, not at the nation boundary.
- Main RE source root: C:/Users/tlori/FreeCity4; no REA binary open, use local export.
- No game requests, ABI proposals or engine changes planned.

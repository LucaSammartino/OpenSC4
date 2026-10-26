| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-ss29c sound-system SS9–SS12; kind docs | Codex; .claude/worktrees/re-ss29c; reserved branch re-ss29c (detached HEAD) | ready; content commits 47f6a9032, 2b16b9f0e; doccheck passed, diff check passed; cargo/clippy/ctest n/a; game n/a, shadow n/a; ABI/seam reservations none; SS9–SS12 moved to answered ledger | 1. Implement the recovered contracts. 2. Observe SS11 locale and unsafe IO boundaries in §9.1. |

Updated docs/specs/sound-system.md §§7.1–9 with instruction citations and worked cases.
SS9: construction defaults, resource/creator/cache lookup, output failures, autoplay retention and shutdown ordering.
Shutdown preserves registered data sources and service references; destruction releases them later.
SS10: all 17 primary and 11 secondary director slots; factory identities, failure and aggregation boundaries.
PostAppInit creates the global property set and ignores its result; a null cache is unchecked.
SS11: append-only Open, sequential M3U/PLS grammar, titles/durations and literal path processing.
An unterminated final logical line is not consumed; plain M3U repeats it.
SS11 runtime observations remain limited to locale-dependent conversion bytes and zero-size/allocation-failure IO outcomes.
SS12: first-match erasure, replacement registration, target remove/fast/slow batches and handler LIFO-add/FIFO-remove replay.
SS9, SS10, SS11, SS12 are recorded in docs/specs/questions/answered/sound-system.md; question index regenerated.
Verification: doccheck check passed; whitespace check passed. No engine files read or changed; no ABI rows proposed.
REA was not needed; export queries, existing decompilation and executable instructions supplied the evidence.

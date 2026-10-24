| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-www-q17 (web and network contracts) | Codex / `re-www-q17` (RE lane, detached HEAD) | in progress; resumed remaining HTTP/WWW contracts | Qualify request normalization, then residual owner contracts within the round budget; enqueue ready. |

## Report

The engine lane core-www-close13 Owner closure table was read first. This round resolves
the cookie/cache literal and persistence contracts and WWW matched null-target completion.
It does **not** finish the full requested owner closure: WWW Q7/Q9 and HTTP Q9/Q13 remain open.
Investigation stopped at the brief's two-thirds budget bound: the last research call reported
97715/150000 context units; subsequent work wrote answers, regenerated the index and reviewed docs.

Changes in docs/specs/web-network.md:

- §3.2: splitter returns actual count, so Port drops its last item; only space/tab trim;
  delimiter-completed empties versus omitted empty terminal item; consumed double quotes in
  cookie grouping, plain quotes in ports; no separate name/value trim; wrapping 32-bit atol.
- §3.4: copying lookup miss preserves output; header separators use original indices,
  including leading separators after skipped attributes.
- §3.6: CookieData.ini literal; identity-copy byte strings then uppercase hex INI encoding,
  Admin/Binary=1 marker, read limits and invalid-input boundary; binary setter assignment
  versus path-change reload/clear. The method's encryption name does not describe a cipher.
- §4.7: null target after an active match removes the active job and returns true without
  conversion, cache/cookies, delivery, waiting selection or foreign-resource clearing.
- §§6.1/6.4/6.4.1: FileCache.ini literal; destructive source-file admission and forced copy
  across directories; reservation and descending-time filename candidates; rightmost-seven-comma
  loader, limits/clamps and file checks; exact-filename orphan cleanup; native buffer deallocation;
  direct writer materialization without getter side effects and hash bucket/chain ordering.

Added 15 independent **proposed/unexecuted** vectors: CK07–CK13, NFC13–NFC19 and WWW-21.
These use constructed deterministic fixtures, not observed game runs or implementation outputs.
The WWW checklist now credits cookie constructor 008f9bad to §3.6; all other open rows remain open.

Moved Q1/Q3/Q10/Q11/Q15 from docs/specs/questions/web-network.md to
docs/specs/questions/answered/web-network.md with dated MATCHES/DIFFERS answers. Also answered
Q16 supplied only on core-www-close13, preserving its origin in the answered entry. Q15 covers
both CookieManager and NetworkFileCache. Regenerated docs/specs/QUESTIONS.md. No new questions filed.

Evidence: main checkout tools/re_query.py function/search/xref/byte-instruction queries,
bounded reads of decomp classes and unnamed regions, and read-only PE pointer/string decoding.
Numeric claims cite instruction addresses; the lane never used REA or opened another MCP session.
No engine/, plugin, capi, ABI table, coverage map, game file or shared Git ref edits.
No proposed ABI rows. Helpers remain ignored under this worktree's build-runs/.

Milestones: `448d45595` cookie/cache rules and vectors; `21d033ce5` null-target completion,
answered question archive and regenerated index. The final report commit includes review corrections
to summary rows and address citations. Git printed a packed-refs.lock permission error during each
commit, but both commits completed successfully on detached HEAD; no ref/config repair was attempted.

Validation: doccheck check PASS; doccheck links PASS. Final diff whitespace check and doccheck
are recorded in the closing tool result. Cargo/clippy/CTest n/a for this spec-only lane.
Game runs/shadow n/a; no request outstanding and no dynamic equivalence claim.

## Handoff

- Resumed final round: first re-read core-www-close13 Owner closure table; previous six answers remain intact.
- Remaining goal: WWW Q7/Q9 and HTTP Q9/Q13 in questions/web-network.md; Q12 is outside this brief.
- Read core-www-close13 Owner closure table for exact remaining exclusions; do not read engine files.
- Next RE step: Q9 HTTP request normalization at 0081ca90, 0081acf0 (273 decomp lines) and 0081ac20.
- 0081ca90 converts raw headers only when parsed headers are empty, then clears raw text; it can erase the first empty-valued parsed entry. Do not publish complete normalization semantics from this partial inspection.
- WWW cookie-header construction is in 008f8ffe before its 0081cda0 call; still unqualified.
- Q7/Q9 residual body addresses remain O in web-network-www-bodies.md; cookie constructor 008f9bad alone changed P→C this round.
- Q13: request wire/retry/redirect, 00819800 framing/spill, 0081a470 finalization, 00818490/00818680 date grammar, 00b18ba8 adjustment, native slots 0/3 remain open.
- Query main tools/re_query.py first; use build-runs/evidence.py body ADDRESS --skip N --limit N for bounded original decomp reads.
- Q16 exists only in core-www-close13 before this round; its answer is already archived here and must not be re-added as open during merge.
- No ABI changes, no game labels, no external operations outstanding. Queue state wip; no whole-owner completion claim.

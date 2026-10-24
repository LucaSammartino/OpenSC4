| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-www-q18: WWW residual specifications | Codex / re-www-q18 (RE) | ready: Q7/Q9/Q13 answered; doccheck check/links PASS; game n/a; ABI unchanged | Land documentation; Q12 remains outside scope |

## Report

Closes WWW Q7/Q9 and HTTP Q13, carrying forward re-www-q17 prerequisites.
The HTTP contract is now sufficient to replace the Q13 exclusions in the engine
lane's owner-closure table. This is documentation readiness, not a claim that the
engine has implemented these new contracts or that live transport is equivalent.

### Changes

- Prerequisites `a688b387c`: relevant web documentation from re-www-q17, including
  cookie/cache literal answers, null-target completion, native slots, FTP commands
  and admission, WWW Cookie header construction and HTTP request normalization.
  Six prior archived answers preserved; local duplicate open Q16 removed after
  confirming its existing archive entry. No unrelated branch changes imported.
- `238109854`: web-network.md §4.15 describes listener/reference lifetime,
  socket setup/close/drain, progress forwarding, container ownership, formatter
  overflow, lookup/address support and interface adapters. Q7/Q9 moved to the
  answered archive and questions index regenerated. HTTP normalization (Q9 aspect)
  is settled in §4.14. No new question IDs filed.
- Remaining 46 WWW bodies / 2517 bytes qualified: unchanged inventory is now
  148 C / 34049 bytes, zero P/O. C means documented, not implemented or verified.
  Native unsafe-input domains remain explicit. No whole web-family credit.
- SUP01–SUP09 are constructed, unexecuted conformance expectations for identical
  listener assignment, the 1/50 progress quirk, receive counts, shallow pointer
  copying, raw erasure, formatter limits, byte order, counters and lookup order.
- `5bc2608bc`: HTTP Q13 moved to the answered archive. New
  `docs/specs/web-network-http-wire.md` specifies request bytes and worker retries,
  framing/chunk/trailer quirks, memory/file failures, redirect finalization,
  fixed-position dates, CRT timezone adjustment and destructor dependencies.
  HTTP10–HTTP35 add 26 independent proposed/unexecuted fixtures. Parent section 5
  links the supplement and removes stale inventory exclusions. Q12 is untouched.

### Evidence and validation

Main-checkout re_query.py export queries and bounded decomp reads; no REA query
or binary-open attempt. No pseudocode/assembly pasted into specs. Numeric
instruction assertions in ignored build-runs/verify-www-q18.py pass for 27 bodies;
inventory validation confirms 148 bodies / 34049 bytes. Earlier assertions failed
on three harness register-name/width tokens (EAX/EDX and AL/EAX), corrected against
instructions; no behavior changed. Initial diff check found a trailing blank line,
subsequently removed.

Q13 evidence is paged `re_query.py asm --bytes` plus bounded decomp reads of
cRZHTTPClient and region_00810000, and CRT tzset instructions. Read-only PE literal
checks establish HTTP wire delimiters, weekday/month/day tables and initial CRT
timezone/daylight words. Ignored evidence: `build-runs/http-*.asm.txt`,
`http-q13-evidence.py`, `http-q13-data.py`. Numeric checks cover unsigned deadline,
chunk size+2, strict spill threshold, redirect bound, year pivot, erroneous yearday
bias, asctime colon+2 substitution, mktime subtraction and timezone writes.

Doccheck check PASS; doccheck links PASS; git diff --check PASS.
Cargo/clippy/CTest n/a (specification-only). Game/shadow n/a; no pending request.
No ABI exports/version/table changes or proposed ABI rows. No engine/, plugin,
capi, coverage-map, game-file or external/ edits. Git emits a packed-refs.lock
permission error but reports successful detached-HEAD commits; shared refs/config
were not repaired. Supervisor publishes the reserved branch.

Validation corrections: the first large spec patch failed its context match and
made no edits; it was reapplied as a linked supplement plus parent closure text.
The first staging command named docs/QUESTIONS.md incorrectly, so no commit was
created; corrected staging committed successfully. `tools/questions.py index`
wrote its actual generated path, docs/specs/QUESTIONS.md (ignored, no tracked
delta). These tooling failures are distinct from the passing final doccheck.
The prior round's 100060/150000 research limit described that round only; this
resumed round qualified the HTTP evidence rather than relying on its preliminary
pseudocode notes. No external REA or binary-open dependency was needed.

## Handoff

- Goal complete: Q7/Q9/Q13 archived; Q12 remains open and unmodified.
- Done: prerequisites a688b387c; WWW 238109854; HTTP Q13 5bc2608bc; final parent/report commit follows.
- Next: supervisor publishes detached HEAD and lands ready queue entry with full tip.
- Engine core-www-close13 should implement the linked HTTP supplement and its vectors before owner credit.
- No game/shadow requests, running operations, ABI proposals or export changes.

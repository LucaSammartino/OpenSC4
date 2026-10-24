| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-www-q18: WWW residual specifications | Codex / re-www-q18 | in progress: Q7/Q9 closed; finishing HTTP Q13 | Qualify HTTP wire, framing, finalization, dates and destructor; doccheck; enqueue |

## Report

Closes WWW Q7/Q9 and carries forward re-www-q17 prerequisites. This deliverable
does **not** complete the brief: HTTP Q13 wire/date/finalization/destructor
qualification remains open. Ready means these documentation commits can land,
not that the HTTP owner is complete.

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
  is settled in §4.14; Q12 is outside scope, Q13 remains open. No new IDs filed.
- Remaining 46 WWW bodies / 2517 bytes qualified: unchanged inventory is now
  148 C / 34049 bytes, zero P/O. C means documented, not implemented or verified.
  Native unsafe-input domains remain explicit. No whole web-family credit.
- SUP01–SUP09 are constructed, unexecuted conformance expectations for identical
  listener assignment, the 1/50 progress quirk, receive counts, shallow pointer
  copying, raw erasure, formatter limits, byte order, counters and lookup order.

### Evidence and validation

Main-checkout re_query.py export queries and bounded decomp reads; no REA query
or binary-open attempt. No pseudocode/assembly pasted into specs. Numeric
instruction assertions in ignored build-runs/verify-www-q18.py pass for 27 bodies;
inventory validation confirms 148 bodies / 34049 bytes. Earlier assertions failed
on three harness register-name/width tokens (EAX/EDX and AL/EAX), corrected against
instructions; no behavior changed. Initial diff check found a trailing blank line,
subsequently removed.

Final doccheck check PASS; doccheck links PASS; git diff --check PASS.
Cargo/clippy/CTest n/a (specification-only). Game/shadow n/a; no pending request.
No ABI exports/version/table changes or proposed ABI rows. No engine/, plugin,
capi, coverage-map, game-file or external/ edits. Git emits a packed-refs.lock
permission error but reports successful detached-HEAD commits; shared refs/config
were not repaired. Supervisor publishes the reserved branch.

Research stopped at measured context 100060/150000 at the two-thirds boundary.
Subsequent work was documentation, archive/index maintenance and validation.
Preliminary HTTP reads are not published as settled contracts: malformed framing,
numeric operation order and runtime date behavior still need instruction checks.

## Handoff

- WWW Q7/Q9 complete at documentation level; no O/P inventory rows remain.
- Q13 contracts drafted in docs/specs/web-network-http-wire.md and linked from section 5.
- Q13 moved to answered archive; index and doccheck pending. Q12 untouched.
- Next: regenerate questions index, validate doccheck/check+links and diff; review facts; commit final report and queue.
- Main source: decomp/cRZHTTPClient.c and decomp/_unnamed/region_00810000.c.
- Wire builder 0081b160 starts at region line 7341; worker 0081ba80 at HTTP line 1154.
- ReadMessage 00819800 at HTTP line 542; finalization 0081a470 at line 1026.
- Date parsers 00818490/00818680 at HTTP lines 164/248; adjustment 00b18ba8.
- Destructor 0081cee0 at region line 8022: tail instructions read, cleanup not settled.
- Paged instruction evidence: ignored build-runs/http-*.asm.txt; literal reader http-q13-data.py.
- Numeric/date checks include unsigned year pivot, table bias, asctime colon+2, mktime subtraction, timezone writes.
- HTTP10–HTTP35 are proposed/unexecuted; no live IO or runtime timezone measurement.
- Native slots 0/3 and HTTP normalization are already specified; do not reopen.
- No ABI changes, pending processes, game requests or outstanding run labels.

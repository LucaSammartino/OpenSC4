# Open questions: web-network.md

## Q12 (re-www-ww5, 2026-10-23) — command-generator wire grammar and execution

Section 7 specifies message/poll/accept boundaries. Settle address-helper `0081e7d0` and exact
bind address/port; ReadCommandInput `0081ef00` and line reader `0081ed50` framing/limits;
malformed percent escapes in `0081ebd0`; ExecuteCommand `0081eb30` service/result mapping;
and exact response concatenation plus zero-progress/error behavior in `0081f180`. Pseudocode
for CheckForCommands incorrectly removes the accepted-command branch; instructions retain it.

## Q13 (re-www-ww5, 2026-10-23) — remaining HTTP wire and date behavior

Section 5 qualifies progress and synchronous notification, and links existing lifetime/admission
contracts. Still required for whole-client implementation: request wire generation, worker retry
and redirect policy, ReadMessage `00819800` body framing/chunk handling/spill-to-file failures,
DoJobFinalization `0081a470`, exact date-parser acceptance `00818490`/`00818680`, runtime timezone
adjustment at `00b18ba8`, and residual native slots 0/3. This is HTTP-owner work, not a reopening
of parked WWW-owner Q9. Proposed vectors do not certify complete HTTP transaction equivalence.

2026-10-24 (re-www-q17), resumed round: §5.1 now settles slots 0/3 (ID-1-only
QueryInterface preserving miss output; deleting-destructor wrapper/free flag). The
destructor dependency's full cleanup and the other wire/date items remain open.

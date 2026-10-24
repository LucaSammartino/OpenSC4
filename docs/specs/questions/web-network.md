# Open questions: web-network.md

## Q12 (re-www-ww5, 2026-10-23) — command-generator wire grammar and execution

Section 7 specifies message/poll/accept boundaries. Settle address-helper `0081e7d0` and exact
bind address/port; ReadCommandInput `0081ef00` and line reader `0081ed50` framing/limits;
malformed percent escapes in `0081ebd0`; ExecuteCommand `0081eb30` service/result mapping;
and exact response concatenation plus zero-progress/error behavior in `0081f180`. Pseudocode
for CheckForCommands incorrectly removes the accepted-command branch; instructions retain it.

## Q13 (re-www-ww5, 2026-10-23) — remaining HTTP wire and date behavior
<!-- from: core-www-close15 r2 (2026-10-24) -->
- **Q20 - HTTP response parsing and worker record/resource transition:** wire 1 specifies header delimiter/status-zero handling but does not qualify status/version token parsing, proxy connect destination selection, or how the admission record `[ID,1,0,1,0,request,0,custom]` transitions into the completion record: lifetime 2 calls word +0x14 the internal resource, whereas admission puts the request there. Please specify the dequeue/response-resource construction and field transition before completion, and malformed status-line behavior. [Portable transaction accepts a dequeued timestamp, parses literal `HTTP/major.minor decimal-status`, returns body/header outcomes, and reuses the specified retry/finalization helpers. Native resource publication and proxy endpoint selection remain unconnected; no invented request-to-resource cast.]

2026-10-24 (re-www-q17), resumed round: §5.1 now settles slots 0/3 (ID-1-only
QueryInterface preserving miss output; deleting-destructor wrapper/free flag). The
destructor dependency's full cleanup and the other wire/date items remain open.

## Q7 (re-web-largest4, 2026-10-22) — owned FTP scheduler, wire state and listing parser
For the reached FTP bodies, what exact queue/start/cancel/shutdown order, retry limits, response-code transitions,
and listing-format selection/entry acceptance rules must be reproduced? Round 2 specifies the listing
entry parser 00812de0 (3607 B), RETR 00814560 (1878 B), STOR 00813e90 (1741 B) and transport helpers in
web-network.md §4.13 / web-network-ftp.md, including the async state-4 reentry rejection and unsafe
fixed-column input domains. Final round 3 additionally resolves outer scheduler 00816ee0,
listing acquisition 008163d0, login/format selection 00815d30/008126d0, constructor 00812390,
listing replacement/ownership, disconnect and session shutdown/destruction in
web-network-ftp-control.md. The scheduler has no retry loop; cancellation still sends a failure
completion while stop suppresses the final callback at its check. Login can return true after
a rejected reply and disconnect. Async reentry can reject state 4.
Still unresolved: commands 00815a60 (717 B), 00814cc0 (657 B), 00814f60/008153a0/008155f0
(579 B each), 00815840 (538 B), 008151b0 (489 B); initialization entry 00813d70;
scheduler admission 00817600, lifecycle/container/control helpers and exact per-address O: Q7
rows in web-network-www-bodies.md. No full Q7 answer is claimed. Final round 3/3: **PARK**,
no fourth round; the combined Q7/Q9 residual is exactly 57 bodies / 7292 B.

2026-10-24 (re-www-q17), resumed round: web-network-ftp-control.md §9 now qualifies
all seven command entries listed above, scheduler constructor 00813d70 and admission
00817600, with FTPC01–FTPC08 proposed/unexecuted fixtures. These nine rows are now C;
remaining O scheduler lifecycle/control/container rows still keep Q7 open. The preceding
PARK and 57-body count describe the earlier round, not the current checklist totals.

## Q9 (re-web-largest4, 2026-10-22) — other reached scheduler/resource and support bodies
Complete the still-open bodies in web-network-www-bodies.md outside Q7/Q8, especially HTTP job submission and
thread/resource lifetime (0081cda0, 0081cc10, 00818210 and 0081ca90), shutdown/drain/callback ordering,
and Job/list support helpers. Which callbacks remain possible after disconnect or cancellation, and which
resource/target references do those reached bodies retain/release? Section 4 specifies direct WWW behavior,
including its missing target Release on rejection/cancel, but does not assign the foreign scheduler's
effects to the WWW call. Shared/folded symbols must be checked by their bytes rather than their names.
Round 2 §4.14 now specifies 0081cda0/0081ca90 submission and 0081cc10 cancellation, queue copying,
method conversion and request Release. Cancellation alone does not join, close or guarantee callback
suppression; raw queued-node cancellation does not release its request. Still open: constructor
remaining support/adapters and cookie constructor 008f9bad. Final round 3 resolves constructor
00818210 and the bounded worker completion/resource/stop/drain behavior in
web-network-http-lifetime.md: normal completion notifies before releasing internal then external
resources; stop skips that completion and both releases; cancellation alone permits a failed
callback; shutdown/drain does not join and may start a replacement worker before clearing raw
queued nodes. An old worker exit can clear the new worker pointer. Foreign worker 0081ba80 and
its companion completion/stop/drain bodies are evidence only, not WWW inventory credit.
No full Q9 answer is claimed: all residual support/adapters retain their exact O/P checklist rows.
Final round 3/3: **PARK**, no fourth round, no whole-owner completion claim.

2026-10-24 (re-www-q17): cookie constructor `008f9bad`, including its CookieData.ini
path literal, is now qualified in §3.6 and marked C in the owner checklist. This does not
close Q9's other support/adapters, WWW cookie-header construction or HTTP request normalization.

<!-- from: core-www5 (2026-10-23) -->
- **Q15 - sections 3.2, 3.4, 3.6 and 6.4 literal edge cases:** what exactly are
  the cookie splitter's blank characters and empty-item convention, and does it trim
  name/value separately after trimming each item? What is atol overflow behavior?
  Do skipped first cookie attributes still cause the header's leading separator?
  Are the cookie path/binary setters plain assignments, and what is the cookie
  constructor path literal at `00b16134`? Does the cache writer
  materialize records directly, or call the public file getter with its expiration,
  counter and access effects, and in which map enumeration order? [Current core uses
  ASCII whitespace, preserves empty semicolon items in nonempty input, trims only
  items, uses wrapping 32-bit decimal conversion, follows literal pair indices for
  separators, assigns cookie settings directly, and directly materializes cache
  records in byte-key order without read-side effects. These are literal choices,
  not measured native behavior; no new round is requested for parked Q7/Q9.]

<!-- from: core-www-close13 (2026-10-24) -->
- **Q16 - section 3.4 copying lookup miss:** does GetCookieDataForURL's copying
  overload leave the output record untouched when no k-th match exists? Section
  3.4 qualifies assignment on a match but does not state a miss-side output effect.
  [Literal success-only assignment: core `lookup_into` returns false and leaves
  the supplied record unchanged on a miss. This does not reopen parked Q7/Q9.]
2026-10-24 (re-www-q17), resumed round: §4.4 now qualifies the one-record Cookie header
construction and temporary ownership; §4.14 qualifies raw-text parsing, preservation with
preexisting pairs, and single first-empty-value erasure. WWW-22a–c and NORM01–NORM08 are
proposed/unexecuted distinguishing fixtures. Those two blockers are resolved; Q9 remains
open only for the residual O support/adapters in the owner checklist.

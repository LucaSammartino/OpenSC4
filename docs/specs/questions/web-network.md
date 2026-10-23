# Open questions: web-network.md

## Q1 (re-webnet4, 2026-10-21) — cookie Port list length
ParseCookieText (008f94f9) stores n-1 of the n comma-separated `Port` values. Does HTTPMessage::GetSeparatedStringList
return the item count plus one (so all values are kept), or is the last port really dropped? Needs a read of
HTTPMessage::GetSeparatedStringList; spec section web-network.md §3.2.

## Q3 (re-webnet4, 2026-10-21) — cookie file format
Write/ReadCookieDataFile (008f7784, 008f9e2b): byte layout of each `CookieData%05u` entry and the encryption chosen
by SetShouldEncryptCookieDataFile. Spec section web-network.md §3.6.

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

<!-- from: core-ftp4 (2026-10-22) -->
## Q10 — matched HTTP completion with no WWW target
Section 4.7 says a matched job without a target skips delivery, and distinguishes
resource cleanup on absent active matches. For a matched job whose target is null,
does it still convert/cache/process cookies, select waiting jobs and clear the foreign
resource, or skip some/all of that suffix? The portable core removes the matched live
entry then returns Unsupported for this exceptional snapshot; it makes no suffix
equivalence claim. This does not request a new RE round or reopen the parked owner.

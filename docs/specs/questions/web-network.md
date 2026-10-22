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
fixed-column input domains. Still unresolved: outer scheduler 00816ee0 (1628 B), listing acquisition
008163d0 (1466 B), login/format selection 00815d30 (1245 B), initialization 00812390 (825 B), other
commands and list ownership. Q7 stays open; see web-network-www-bodies.md for each remaining address.

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
00818210, worker final resource ownership/drain/shutdown and remaining support/adapters. Worker
0081ba80 belongs to another owner but is needed evidence for the callback-suppression question.

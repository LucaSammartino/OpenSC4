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
and listing-format selection/entry acceptance rules must be reproduced? Largest unresolved bodies: 00812de0
(3607 B, listing parser), 00814560 (1878 B), 00813e90 (1741 B), 00816ee0 (1628 B), 008163d0 (1466 B),
00815d30 (1245 B). These were inventoried and initially inspected, not fully specified. Section 4.8 covers
the WWW admission/notification boundary only. See web-network-www-bodies.md for every pending address/byte count.

## Q8 (re-web-largest4, 2026-10-22) — WWW response cache-insertion policy
Section 4.7 calls 008f7a50 (1062 B) after successful HTTP conversion and before response-cookie handling.
What precise Pragma/Cache-Control token matching, Date/Expires/max-age precedence, integer overflow behavior
and default expiration arithmetic does it use, and which insertion flags differ for RAM and filename data?
Do not substitute RFC semantics: pseudocode shows suspect substring-length and token-removal conditions.
The call boundary and ignored insertion failure are specified; this body's policy is still partial.

## Q9 (re-web-largest4, 2026-10-22) — other reached scheduler/resource and support bodies
Complete the still-open bodies in web-network-www-bodies.md outside Q7/Q8, especially HTTP job submission and
thread/resource lifetime (0081cda0, 0081cc10, 00818210 and 0081ca90), shutdown/drain/callback ordering,
and Job/list support helpers. Which callbacks remain possible after disconnect or cancellation, and which
resource/target references do those reached bodies retain/release? Section 4 specifies direct WWW behavior,
including its missing target Release on rejection/cancel, but does not assign the foreign scheduler's
effects to the WWW call. Shared/folded symbols must be checked by their bytes rather than their names.

# Answered questions: web-network.md

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

**ANSWERED 2026-10-24 (re-www-q18):** DIFFERS — §4.15 completes the remaining
scheduler lifecycle/control/container contracts. Forwarded progress is 1 for any
nonzero total, 50 for zero; pointer-valued queue fields are not retained by copying
or released by raw range erasure. Together with §4.13 and FTP-control §9, no Q7
inventory rows remain open. SUP01–SUP09 are constructed, unexecuted fixtures.

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

2026-10-24 (re-www-q17), resumed round: §4.4 now qualifies the one-record Cookie header
construction and temporary ownership; §4.14 qualifies raw-text parsing, preservation with
preexisting pairs, and single first-empty-value erasure. WWW-22a–c and NORM01–NORM08 are
proposed/unexecuted distinguishing fixtures. Those two blockers are resolved; Q9 remains
open only for the residual O support/adapters in the owner checklist.

**ANSWERED 2026-10-24 (re-www-q18):** MATCHES — §§4.4/4.14 settle Cookie header
construction and HTTP request normalization; §4.15 finishes support/adapters,
shallow pointer copying and exact listener/collection ownership. The 148-body WWW
inventory is now documented; no whole HTTP transaction or runtime verification
is claimed. HTTP wire/date work remains separately open as Q13.


## Q1 (re-webnet4, 2026-10-21) — cookie Port list length

ParseCookieText (008f94f9) stores n-1 of the n comma-separated `Port` values. Does HTTPMessage::GetSeparatedStringList
return the item count plus one (so all values are kept), or is the last port really dropped? Needs a read of
HTTPMessage::GetSeparatedStringList; spec section web-network.md §3.2.

**ANSWERED 2026-10-24 (re-www-q17):** MATCHES — §3.2 confirms a genuine final-item
drop. The splitter returns the actual item count; a single explicit port leaves a zero first
entry (unrestricted), while an empty Port attribute selects the URL port/80. CK07–CK08 distinguish these.

## Q3 (re-webnet4, 2026-10-21) — cookie file format

Write/ReadCookieDataFile (008f7784, 008f9e2b): byte layout of each `CookieData%05u` entry and the encryption chosen
by SetShouldEncryptCookieDataFile. Spec section web-network.md §3.6.

Partial evidence 2026-10-23 (re-www-ww5): section 3.6 specifies clear-before-read/write,
text re-admission, unsigned expiration filtering, contiguous naming and 10000-entry read cap.
The 50-byte helper `008f7752` calls string-copy/assignment helpers; exact binary transform,
character conversion and the writer marker literal still need qualification. No cipher is claimed.

**ANSWERED 2026-10-24 (re-www-q17):** DIFFERS — §3.6 establishes identity-copy text
followed by uppercase hexadecimal INI encoding, not encryption or raw CookieData serialization.
The marker is Admin/Binary=`1`; byte-string conversions preserve length-bearing bytes. Read accepts
either hex case and stops on a failed/zero binary read. Odd-length hex is outside the safe paired
input domain. CK11 supplies an exact encoded value.

## Q11 (re-www-ww5, 2026-10-23) — cache persistence and source-file admission

Section 6 qualifies byte admission, reads and expiration. What are exact source-file overload
`008fd76c` ownership/copy behavior, filename generation/reservation (`008fcd49`, `008fceeb`),
INI-loader token acceptance/escaping (`008fdb0d`), orphan cleanup (`008fc6e0`) and holder buffer
deallocation semantics? Verify the default INI-name pointer `00b16144`. Do file/RAM capacity
settings have enforcement outside the specified periodic expiration sweep? Failed allocation
and reference-count overflow remain outside the published domain.

**ANSWERED 2026-10-24 (re-www-q17):** DIFFERS — §6.4.1 specifies destructive source-file
admission, forced copying across directories, CREATE_NEW reservations with 10000 descending-time
candidates, rightmost-seven-comma parsing without escaping, unsigned acceptance limits and time
clamps, exact-filename orphan matching, and 16-bit holder release through GlobalHeapFree.
The default is FileCache.ini; this owner has no size-based eviction. NFC13–NFC19 cover observable
distinctions; allocation failure and reference overflow remain outside the input domain.

## Q10 (core-ftp4, 2026-10-22) — matched HTTP completion with no WWW target

Section 4.7 says a matched job without a target skips delivery, and distinguishes
resource cleanup on absent active matches. For a matched job whose target is null,
does it still convert/cache/process cookies, select waiting jobs and clear the foreign
resource, or skip some/all of that suffix? The portable core removes the matched live
entry then returns Unsupported for this exceptional snapshot; it makes no suffix
equivalence claim. This does not request a new RE round or reopen the parked owner.

**ANSWERED 2026-10-24 (re-www-q17):** DIFFERS — §4.7 specifies true return after removing
the active entry, skipping conversion, caching, cookies, notifications, waiting-job selection and
resource clearing. WWW-21 preserves both the waiting entry and foreign resource on this path.

## Q15 (core-www5, 2026-10-23) — cookie and cache literal edge cases

What exactly are the cookie splitter's blank characters and empty-item convention, and does
it trim name/value separately after trimming each item? What is atol overflow behavior?
Do skipped first cookie attributes still cause the header's leading separator? Are the cookie
path/binary setters plain assignments, and what is the constructor path at `00b16134`?
Does the cache writer materialize directly or call the public getter, and in what order?
The core's literal choices were ASCII whitespace, empty items in nonempty input, whole-item
trimming, wrapping decimal conversion, index-based separators, plain setters and direct cache
materialization in byte-key order without read-side effects.

**ANSWERED 2026-10-24 (re-www-q17):** DIFFERS — §§3.2/3.4/3.6 and §6.4 qualify space/tab
only, delimiter-completed empties with no extra terminal empty, no name/value retrim, wrapping
32-bit atol, index-based leading separators, plain binary assignment but path-change reload,
CookieData.ini, direct materialization without getter effects, and native hash bucket/chain order.
CK09/CK12/CK13 and NFC15/NFC19 distinguish the literal choices.

## Q16 (core-www-close13, 2026-10-24) — copying cookie lookup miss

Does GetCookieDataForURL's copying overload leave the output record untouched when no k-th
match exists? Section 3.4 qualified assignment on a match without stating a miss-side effect.
The engine lane's literal success-only assignment leaves output unchanged and returns false.
This question was supplied on branch core-www-close13 rather than present in this base.

**ANSWERED 2026-10-24 (re-www-q17):** MATCHES — §3.4 confirms success-only CookieData
assignment; the miss branch never writes the supplied output. CK10 uses a prefilled sentinel record.

## Q8 (re-web-largest4, 2026-10-22) — WWW response cache-insertion policy
Section 4.7 calls 008f7a50 (1062 B) after successful HTTP conversion and before response-cookie handling.
What precise Pragma/Cache-Control token matching, Date/Expires/max-age precedence, integer overflow behavior
and default expiration arithmetic does it use, and which insertion flags differ for RAM and filename data?
Do not substitute RFC semantics: pseudocode shows suspect substring-length and token-removal conditions.
The call boundary and ignored insertion failure are specified; this body's policy is still partial.

**ANSWERED 2026-10-22 (re-web-largest4):** DIFFERS — §4.12 records case-sensitive Pragma substring
rejection, six whole-token case-insensitive Cache-Control comparisons (so ordinary max-age arguments
do not match), ignored must-revalidate/Last-Modified, Date-or-now plus three days with 32-bit wrap
unless Expires is present, RAM flags (1,1) versus file flags (0,1), and the borrowed insertion/return
boundary. Fourteen independent constructed vectors distinguish these behaviors. Instruction addresses
are cited in that section; no network request or implementation output was used.

## Q2 (re-webnet4, 2026-10-21) — cookie store capacity
AddCookie evicts when the store holds at least the capacity (manager +0x10). What value does Init or the constructor
set? Spec section web-network.md §3.3.

**ANSWERED 2026-10-22 (re-web-largest4):** MATCHES — constructor 008f9bad sets +0x10 to 0x4000 (16,384 records),
verified by the immediate store at 008f9bc5. Init 008f6a7c does not reset capacity; it conditionally reads persisted
data when initially empty, then marks initialized and returns true. Rule written in web-network.md §3.6.

## Q4 (core-url4, 2026-10-21) — ChangeURL empty-path chain
§2.7, N path empty: "only if N's params are empty, B's params; only if those are empty too, B's query; only if that is
empty too, B's fragment". Is B's query copied over N's own non-empty query (so `?new` against `http://a/b?old` keeps
`?old`), or only when N's query is empty too? Same for the fragment. [literal: copied unconditionally once the previous
field is empty, so `?new` is replaced by B's query when B has one]

**ANSWERED 2026-10-21 (re-url-contracts4):** DIFFERS — each step of the chain tests N's own field: N params empty -> take B params; then N query empty -> take B query; then N fragment empty -> take B fragment; a non-empty N field is never overwritten (`?new` against `http://a/b?old` keeps `?new`). 00823d6c..00823deb; spec §2.7.

## Q5 (core-url4, 2026-10-21) — port digit run and atol overflow
§2.4 server step: with a run of more than 15 port characters, how many characters are erased from rest (the whole run
or the kept 15)? And for a kept run whose value exceeds 32 bits (or `LONG_MAX`), what port does `atol` give: wrapped
modulo 2^32, or clamped to `LONG_MAX`? [whole run erased; first 15 digits wrap modulo 2^32]

**ANSWERED 2026-10-21 (re-url-contracts4):** MATCHES — the whole digit run is erased (00822914 erases c+1+full run length), only the first 15 digits are copied; atol 009ef7c2 wraps modulo 2^32 with no overflow check; classifier is isdigit (mask 4). Runs of 16+ digits stay undefined in the original; use the first 15. Spec §2.4.

## Q6 (core-url4, 2026-10-21) — unrecognised `xyz://` scheme
§2.3 step 4/5: for a scheme found by `://` that matches no prefix (e.g. `gopher://h/x`), is the scheme field cleared
before steps 6/7, or kept? Kept means an allowRelative URL ends as type 9 with scheme `gopher`, server `h`, path `/x`,
and ChangeURL treats it as "N has a scheme". [kept]

**ANSWERED 2026-10-21 (re-url-contracts4):** MATCHES — kept: scheme is cleared only at the start of ParseScheme (00822187..0082219b); `gopher://h/x` with allowRelative ends type 9, scheme gopher, server h, path /x. Spec §2.3 step 5.

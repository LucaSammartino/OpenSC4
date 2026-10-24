# HTTP wire, framing, finalization and dates

Parent: [web-network section 5](web-network.md#5-crzhttpclient--bounded-client-contract).
Completes HTTP Q13, 2026-10-24, re-www-q18. *[compat]* throughout. This is a
behavior contract for the original Windows client, including its quirks. Export
instructions were checked for constants, widths and numeric branches; the local
executable supplied literal/table bytes. No live network transaction was run.
Admission and method IDs remain in parent section 4.14; lifetime/completion/stop
remain in [HTTP lifetime](web-network-http-lifetime.md), sections 1–3.

## 1. Request wire and worker policy

Evidence: builder `0081b160–0081b795`, worker `0081ba80–0081c9a4`, header reader
`0081b7a0–0081ba79`. The transport is a plain stream socket; URL spelling does
not introduce TLS or CONNECT tunnelling here.

The worker builds the request header only when request+0x4c is empty. Otherwise
it uses those bytes as supplied. Builder failure sets job result 2. Unknown method
ID zero fails the builder; recognized methods start with the stored method string,
preserving spelling. Assemble the target from absolute path followed by each
nonempty params, query and fragment field, **verbatim**, in that order. The
fragment is included; there is no additional escaping or normalization pass.
The component strings are the getters specified in parent section 2.

| Configuration | Bytes after method, before version |
|---|---|
| Direct OPTIONS | ` *` |
| Direct other method | one space, then assembled target |
| Proxy OPTIONS | ` http://`, origin server, optional `:` and unsigned decimal origin port; no target suffix |
| Proxy other method | ` http://`, origin server, optional `:` and unsigned decimal origin port, then assembled target |

Append the port whenever GetPort is nonzero, including a default port returned by
the URL parser. Proxy mode removes header-name ID 14 with the existing record
eraser. Direct mode appends a `Host` record containing the origin server only when
a case-insensitive Host lookup fails. An existing empty Host counts as present.
There is no automatic Content-Length or body encoding in this builder
(`0081b3aa–0081b509`, `0081b609–0081b752`).

Append ` HTTP/1.1\r\n`, then each record in list order as name bytes, `: `, value
bytes, `\r\n`, then a final `\r\n`. Duplicate records and embedded bytes are not
sanitized. Literal addresses: `00abfd18`, `00a92700`, `00aa1634`; 24-byte record
stride and append order: `0081b51f–0081b5c1`. The worker sends header concatenated
with the body at request+0x68, using byte lengths. Short successful sends advance
by the actual count. Successful zero-byte sends can repeat until cancellation or
deadline; a failed send enters retry policy (`0081bf20–0081c0b6`).

Socket reuse requires initialization, connected status, equality of the low 16
bits of the origin ports, and a case-insensitive origin-server match. Otherwise
close before resolving/connecting. Numeric addresses bypass name lookup. Name
resolution failure is result 3, connect failure 4, timeout 5, exhausted transport
failure 6, cancellation 7. Deadline is dequeue timestamp plus client timeout,
with wrapping 32-bit addition: expire when deadline is **unsigned less than**
current runtime seconds. Equality does not expire; progress does not refresh the
timestamp (`0081bcaa–0081bec0`, `008198d0–008198f3`, `0081a0af–0081a0cb`).

The header reader accumulates 1024-byte reads until the first CRLFCRLF. Bytes
after it become initial body bytes; the header goes through the record parser in
parent 4.14. This reader has no header-size ceiling. Its skip-informational flag
repeats for status 100–199 when enabled; **the worker passes false**. Thus an
informational header need not be skipped to obtain a later final response.
Status zero is failure. A successful zero-byte read makes no progress; no-readiness
polls repeat while connected. Cancellation returns zero, clears the cancel byte
and records 7; timeout records 5 (`0081b7a0–0081ba79`).

Header/body failures retry while count is below 3, the worker's reusable-connection
flag is true, and result is neither 5 nor 7. The flag is false when parsed version
major <2 and either major or minor is zero; otherwise true (HTTP/1.0 disables
retries, HTTP/1.1 enables them). Send failure tests the existing flag. Retry
increments count, sets result 1, clears the wait word, appends a job copy to the
queue tail under lock, clears remembered peer and closes the socket. Do not
complete that requeued job in the same iteration. Header/body failure with result
still 1 becomes 6 when retry is unavailable. See `0081bfa0–0081bfff`,
`0081c2a0–0081c310`, `0081c4d0–0081c75d` and the lifetime contract.

### 1.1 Status and version tokens (Q20, 2026-10-24)

Status and version are **independent interpretations of the first parsed record's
name bytes**. Neither searches for a later well-formed status line. Header splitting
and record construction still follow parent §4.14. The status helper 00817f60 uses
the name's C-string representation, while the version helper 00818140 also uses its
stored byte length. These helpers do not require a reason phrase.

**Status.** An empty record list, first name length at least 384, or fewer than two
whitespace-separated nonempty tokens returns zero. Otherwise scan two tokens with
CRT `%s%s` (literal at 00abfc98); ignore the first token and everything after the
second. Whitespace is the runtime CRT scanner's whitespace (in the C locale:
space, tab, CR, LF, vertical tab and form feed). Starting at the **second byte of
the second token**, retain consecutive ASCII digits and truncate at the first
nondigit. Its first byte is left untouched. Apply native signed 32-bit decimal
conversion to that truncated token: an initial sign is allowed, no leading digit
after that sign yields zero, and accumulation/negation wraps modulo 2^32 rather
than clamping. Thus `garbage 200suffix` yields 200, `HTTP/1.1 +200` yields 200,
`HTTP/1.1 -200` yields -200, and `HTTP/1.1 2x00` yields 2. There is no check for
`HTTP`, a slash/dot, exactly three status digits, or the HTTP status range.
Evidence: 00817f6c–00817f86 (length), 00817fc9–00818002 (scan, truncation, convert),
009ef7c2–009ef849 (CRT decimal conversion, especially 009ef834–009ef847).

A zero result is a failed header read. A nonzero signed result, including a
negative or wrapped value, passes the reader's status test unless cancel/stop is
observed. The worker applies the existing framing, retry and finalization rules;
nonzero does not itself mean success. With worker skip-informational=false, a
status 100 is returned as that response, not automatically followed by another
header (0081b9cb–0081ba65). A zero status without timeout/cancel leaves the pending
job result for the existing transport-failure/retry policy to resolve.

**Version.** A resource starts with both cached version words +0x44/+0x48 equal
to -1. Only when major is -1 does the helper derive a version, first setting both
words to zero. A first-record name of at least eight bytes must begin with the
exact uppercase four bytes `HTTP`. If so, byte index 5 supplies major and byte
index 7 supplies minor, independently, when each is an ASCII digit; nondigits
leave the corresponding zero. Byte indices 4 and 6 are not validated. Remaining
bytes, including multi-digit extensions, are ignored. `HTTPx1y1` therefore gives
1.1, `HTTP/12.34` gives 1.0, and lowercase `http/1.1` gives 0.0. A cached major
other than -1 bypasses parsing and returns both stored words unchanged. The helper returns
true even when no version is recognized (00818143–008181da; `HTTP` at 00abfca0).
The worker disables reusable-connection retry for major 0, or major 1 with minor 0;
recognized 1.1 and major greater than 1 enable it (0081c2ed–0081c315).

The bounded-status rejection at 384 is fully decided by the code. Allocation
failure is a separate unsafe native domain, described in the lifetime supplement;
it is not a malformed-status outcome. Locale-sensitive whitespace follows the
injected CRT locale; no guessed Unicode whitespace or stricter protocol grammar
is part of this contract.

### 1.2 Proxy mode and actual connect destination (Q20)

Proxy mode in the request builder changes the request-line form and header
handling in section 1. **The worker still selects the origin endpoint.** It parses
the retained request URL at +0x1c, obtains that URL's server and port before
building the header, and uses those values for reuse, numeric-address conversion,
name lookup and connection. It does not substitute the client's proxy address
or proxy port, negotiate CONNECT, or parse a separate proxy status grammar.
Evidence: 0081bba1–0081bc81, 0081bca7–0081bd9d, 0081be4d–0081be75. The proxy
configuration therefore can produce an absolute-form request sent to the origin.

Use the low 16 bits of the URL port. Zero selects port 80 for connection; nonzero
is converted to network byte order. The remembered reuse key remains the URL
port, so URL port zero and explicit 80 are different keys even if both connect
to port 80. Reuse also requires an open connected socket and case-insensitive
origin-server equality. A different origin closes/drains before fresh resolution;
names require lookup return exactly 1, while numeric strings bypass lookup.
No port is inferred from proxy settings (0081bccb–0081bd9d). Any response received
on this connection uses section 1.1, regardless of the proxy flag.

**Constructed conformance vectors (PROPOSED / UNEXECUTED).** Status/version rows
inject the specified first-record name into a fresh resource, without transport
failure. S9/S10 vary only the stored name length. P rows observe a fake resolver
and socket, avoiding live external endpoints.

| ID | Input / boundary | Required result |
|---|---|---|
| WWW20-S1 | `HTTP/1.1 200 OK` | status 200; version 1.1; reusable retry enabled |
| WWW20-S2 | `garbage 200suffix` | status 200; version 0.0; reusable retry disabled |
| WWW20-S3 | `HTTPx1y1 200` | status 200; version 1.1 despite absent slash/dot |
| WWW20-S4 | `http/1.1 200` | status 200; version 0.0 |
| WWW20-S5 | `HTTP/12.34 200` | status 200; version 1.0 |
| WWW20-S6 | second tokens `+200`, `-200`, `2x00`, `x200` | statuses 200, -200, 2, 0 respectively |
| WWW20-S7 | `HTTP/1.1`, or empty record list | status zero; header-read failure |
| WWW20-S8 | `HTTP/1.1\t200 ignored 500` in C locale | status 200; later tokens ignored |
| WWW20-S9 | `HTTP/1.1 200 ` padded with spaces to 383 name bytes | status 200 |
| WWW20-S10 | same name padded to 384 bytes | status zero before token scan |
| WWW20-S11 | second tokens `4294967496`, `4294967296` | status 200, zero respectively (32-bit wrap) |
| WWW20-S12 | `HTTP/1.1 100 Continue`, worker skip flag false | reader returns 100; no implicit second header read |
| WWW20-S13 | cached version 3.7, then replace first name with lowercase HTTP | helper returns cached 3.7 unchanged |
| WWW20-P1 | proxy configured as `proxy.test:3128`; request URL `http://origin.test:8080/a` | resolve/connect origin.test:8080; request line uses absolute origin URL |
| WWW20-P2 | numeric origin, proxy enabled | no name lookup; connect numeric origin; same response grammar |
| WWW20-P3 | inject origin port zero followed by explicit 80, otherwise same server | both destination ports 80; reuse key mismatch closes/reconnects |
| WWW20-P4 | only proxy endpoint changes, same origin and live connected socket | reuse test remains satisfied; no connect to the new proxy |

## 2. Response body framing

The worker initializes length to zero, uses decimal runtime `atol` on
Content-Length (header ID 38), and selects chunk mode only for Transfer-Encoding
(ID 5) exactly equal to the case-sensitive C string `chunked`. No token-list
parsing or case folding is applied to that value. Zero/missing length without
chunks normally skips ReadMessage. The exception is GET or POST, status 200–299,
and Connection (ID 2) missing or starting with exact lowercase `close`: use
unknown-length sentinel `0x7fffffff`. A longer Connection value is truncated to
five bytes first, so `closed` matches too. HEAD is not specially excluded when a
nonzero length exists (`0081c310–0081c426`).

ReadMessage returns a 32-bit count, not a boolean. Zero is a worker-level failure,
even for a valid empty chunked body. Its overflow-output string is cleared on
entry. Receive counters count all new wire bytes, including chunk syntax and
excess; progress counts body bytes and uses parent 5.3's unknown-length rule.
Reads request 1024 bytes; readiness polls use argument 1000.

### 2.1 Fixed/unknown length

Count the initial body buffer before reading. If it already exceeds the requested
length, copy excess to overflow but retain the **entire initial body and its
count**. Later reads append only the remaining declared bytes and copy excess to
overflow. Successful zero-byte reads do not finish a positive remaining length.
Read failure for the unknown-length sentinel ends with accumulated count; read
failure for an ordinary declared length takes IO-error cleanup. A disconnected
socket detected by the poll fallback returns zero. Length zero reads nothing and
returns zero (`0081a037–0081a090`, `0081a161–0081a1df`, `0081a421–0081a45f`).

### 2.2 Chunks

Move initial body bytes into a private wire buffer and clear decoded response
body. Size lines end at CRLF. First byte must be a runtime hex digit; convert the
consecutive leading hex run with base-16 unsigned `strtoul`. Ignore the rest of
the line: `1;x=y` and `1garbage` both declare one byte. Leading whitespace and `+`
fail. There is no explicit overflow rejection. For a nonzero size erase the size
line including CRLF, consume size+2 bytes (wrapping 32-bit addition), append them
to decoded storage, then remove the last two bytes **without validating CRLF**.
Retain excess privately for the next size line (`0081994f–008199e6`,
`00819a31–00819bd0`, `00819dd5–00819e91`).

For zero size erase the size-line bytes **excluding** CRLF, then wait for
CRLFCRLF in the remainder. `0\r\n\r\n` terminates. Trailer bytes are not parsed
into response headers; the private buffer, including trailers and subsequent
pipelined bytes, is discarded rather than returned as overflow. Missing syntax
waits until IO failure, disconnect, timeout or cancellation. The connected check
occurs even with buffered input. An invalid first size byte returns zero without
erasing an already decoded partial body/file flag (`00819e9c–0081a030`); this is
not a successful partial response.

### 2.3 Memory/file boundary and failure states

An append crossing **strictly above 16384 bytes** triggers spill; initial buffered
bytes alone do not. Derive the URL path extension, append the client suffix,
allocate a path through GetNewTempFilePath, set the response file flag, open with
arguments (2,3,1), write accumulated body and replace body storage with the path.
Later data goes to file. Path allocation, open or checked write failure returns
zero, with no successful in-memory fallback. The memory threshold is unsigned
(`00819ce6`, `0081a1f0–0081a32b`).

Chunk-mode file writes while draining the private wire buffer ignore the write
return; seek/truncate calls stripping its final two bytes are also unchecked.
Direct socket-to-file writes are checked. Do not replace these distinct paths
with uniform checked writes (`00819a46–00819a8a`, `00819c63–00819de0`).

Checked IO-error cleanup closes/removes an open temp file, clears the file flag
and response body and returns zero. Normal nonzero return closes and retains the
file. Cancellation clears cancel byte, file flag and body, sets result 7 and
returns zero. Timeout sets 5 and returns zero, but need not clear the stored
body/path or file flag. Common exit closes an open file and removes it if returned
count is zero. Invalid-size parsing, IO failure and timeout therefore need not
produce identical partial storage states (`00819fd2–0081a030`,
`0081a360–0081a463`).

## 3. Finalization and redirect policy

DoJobFinalization returns 1 only for a requeued redirect, otherwise 0. All methods
accept status 200–299 as result 0. OPTIONS, DELETE, TRACE and unknown IDs map
every other status to 8, including 401. GET, HEAD, POST and PUT map 401 to 9;
other terminal HTTP failures map to 8. PUT never redirects (`0081a470–0081a847`).

GET/HEAD/POST consider statuses **300–303 inclusive**, provided request exists and
its unsigned redirect count is below 5. Location (header ID 26) must be present;
there is no additional nonempty check. GET/HEAD obtain it from the **response**
and resolve relative bytes against current request URL via parent section 2.
POST obtains it from the **request** and uses it without relative resolution.
This asymmetry is literal (`0081a50e–0081a53f`, `0081a667–0081a6d8`). Missing
Location, count >=5 or status 304–399 produces 8.

On the first redirect copy current URL to original-URL field +0x30. Increment
redirect count and replace current URL. GET/HEAD erase cached header +0x4c;
POST preserves it. Both set job result 1, increment attempt word +0xc, clear wait
word +0x10 and append the job under lock. Method and body are preserved; count
is not reset. Count 4 follows once and becomes 5; count 5 cannot follow again
(`0081a519–0081a5bd`, `0081a620–0081a7a9`).

## 4. Date grammar and runtime adjustment

GetDateType rejects byte lengths >=48, finds the first comma, and uses runtime
whitespace `%s` scans. Required token counts below mean successful assignments;
extra trailing tokens are not rejected. Parsing modifies a private copy.

| Type | Classification | Substitutions before numeric scan |
|---|---|---|
| 1 | first comma index 3, length 29, six tokens; retry six-token scan after replacing 7 and 11 by spaces if needed | replace 3,7,11,19,22 by spaces; require eight tokens |
| 2 | comma at index other than 3, length 30–35, four tokens, second token ends at offset 9 | for comma c, replace c,c+4,c+8,c+14,c+17 by spaces; require eight tokens |
| 3 | as type 2, second token ends at offset 11 | replace c,c+4,c+8,c+16,c+19 by spaces; require eight tokens |
| 4 | no comma, length 24, five tokens | first colon p: replace p and **p+2** by spaces; require seven tokens |
| 0 | otherwise | failure |

Types 1–3 read weekday/day/month/year/hour/minute/second/ignored-zone; type 4 reads
weekday/month/day/hour/minute/second/year. Types 1/4 use exact case-insensitive
English three-letter weekday names; 2/3 use full names. Monday=1 through Sunday=7,
miss=0. Month accepts exact case-insensitive Jan–Dec, 1–12, miss=0. Neither miss
causes parser failure. Numeric fields use 32-bit runtime `atol`: decimal prefix,
zero for no digits, no calendar/time range validation. Zone text does not affect
time conversion. Year is interpreted unsigned: 0–49 gain 2000, 50–99 gain 1900,
all other bit patterns remain unchanged.

Reported yearday equals preceding-month cumulative total + day + **1**, plus one
for a Gregorian leap year **regardless of month**. One-based month indexes the
word table at `00abf8a0 + 4*month`; valid preceding totals are
0,31,59,90,120,151,181,212,243,273,304,334. Month zero instead selects bytes
`Dec\0`, interpreted as `0x00636544`. Do not correct that value or reject the miss.
Evidence: `00818490–00818677`, `008186e7–008186f5`, `00818885–008188a1`,
`00818a02–00818a48`; unsigned year/leap/table arithmetic at
`0081878b–008187fd`, `00818987–008189e4`, `00818af0–00818b79`.

In particular `Sun Nov  6 08:49:37 1994` parses hour=8, minute=4, second=0:
p+2 replaces the second minute digit and leaves the second colon leading the
seconds token. Some punctuation substitutions pass while variable-width dates
fail. Types 2/3 check c and c+17 against length before writing; type 4 writes
before its bounds checks. Inputs whose substitutions leave allocated storage
are outside the safe native domain, not ordinary false-return cases. Ordinary
parser failure returns false; TimeStringToSeconds returns zero without mktime.

TimeToSeconds constructs CRT `tm` with supplied day, month-1, year-1900,
weekday-1, yearday-1, supplied hour/minute/second, **tm_isdst=1**. It returns the
32-bit mktime result minus current word `00b18ba8`, with wrapping subtraction.
It does not preserve mktime's -1 failure sentinel (`00817d80–00817de5`). Runtime
calendar normalization/epoch limits apply after parsing; weekday and yearday are
advisory inputs which mktime recomputes.

`00b18ba8` is CRT timezone, seconds west of UTC, not a fixed game constant. Initial
executable values are timezone 28800, daylight-enabled 1, daylight bias -3600.
Runtime tzset uses TZ when present, otherwise Windows GetTimeZoneInformation:
Bias*60 plus StandardBias*60 when a standard transition exists. TZ parsing skips
three zone-name bytes, reads hours and optional minutes/seconds, and applies a
leading minus to the combined offset. Daylight configuration is separate. Source:
`009f8b09–009f8b15`, `009f8b50–009f8bb2`, `009f8c47–009f8ca1`. Timezone lookup
failure/unchanged TZ can retain previous runtime state. Conformance fixtures must
provide runtime timezone/daylight/mktime behavior; parsed `GMT` does not select
UTC. Even mktime failure returns `0xffffffff - timezone` modulo 2^32.

## 5. Destructor dependency

The deleting wrapper/flag contract in parent 5.1 is unchanged. Body
`0081cee0–0081d02f` restores the HTTP vtable and calls Shutdown (`0081cd40`).
Inactive Shutdown immediately succeeds; active Shutdown locks, stops/closes an
active worker via the lifetime helpers, clears active byte, unlocks and succeeds.
Next release/clear the notification target. For nonnull receive then send log
handles, call IsOpen (+0x10), Close (+0x14) only if open, and destroy via slot
+0x80 with flag 1 if still nonnull. These handles are destroyed rather than merely
Released (`0081cf01–0081cf65`).

Scalar-delete the nonnull lookup helper +0xb4 (slot 0, flag 1), then destroy proxy
string +0xc8, queue-list storage +0xb0, critical section +0xa4, thread member +0x9c,
and strings +0x80,+0x6c,+0x54,+0x40,+0x28,+0x14, in that order. The raw queue
helper's export name is misleading; it does not enqueue a job here. No additional
completion callback appears in this body. Only the deleting wrapper's flag
controls freeing the enclosing object.

## 6. Independent vectors (PROPOSED / UNEXECUTED)

Inject socket, clock, file failures and CRT time service as specified. These are
behavior fixtures, not captured game runs; no whole-client equivalence is claimed.

| ID | Input / observation point | Outcome |
|---|---|---|
| HTTP10 | direct GET `http://h/a`, no records; builder | `GET /a HTTP/1.1\r\nHost: h\r\n\r\n` |
| HTTP11 | direct OPTIONS same URL | `OPTIONS * HTTP/1.1\r\nHost: h\r\n\r\n` |
| HTTP12 | direct GET; empty host and duplicate X records | no added Host; empty host preserved; X order retained |
| HTTP13 | header +0x4c=`custom`, body=`xyz` | builder bypassed; transmit `customxyz` |
| HTTP14 | TE=`Chunked`, `chunked, gzip`, `chunked` | only last selects chunk mode |
| HTTP15 | fixed length 3; initial body `abcXY` | count 5; body `abcXY`; overflow `XY` |
| HTTP16 | fixed length 3; initial `a`; read `bcXY` | count 3; body `abc`; overflow `XY` |
| HTTP17 | chunks `1garbage\r\naXY0\r\n\r\n` | count 1; body `a`; XY discarded without validation |
| HTTP18 | chunks `0\r\n\r\n` | count 0; worker failure/retry policy applies |
| HTTP19 | chunks `1\r\na\r\n0\r\nX: y\r\n\r\nNEXT` | body `a`; no X header; no NEXT overflow |
| HTTP20 | append reaches memory length 16384 then 16385 | memory then spill attempt |
| HTTP21 | spill path allocation fails | zero return; no successful memory fallback |
| HTTP22 | current time equals wrapped dequeue+timeout | no timeout at equality |
| HTTP23 | GET 302, relative response Location, count 4 | resolve; preserve original URL; count 5; erase header; requeue; subsequent redirect refused |
| HTTP24 | POST 302 with response Location only | result 8; request-side Location required |
| HTTP25 | GET/PUT/OPTIONS, status 401 | results 9/9/8 |
| HTTP26 | `Sun, 06 Nov 1994 08:49:37 GMT` | type 1; weekday/day/month/year/yearday/hour/minute/second = 7/6/11/1994/311/8/49/37 |
| HTTP27 | `Sunday, 06-Nov-94 08:49:37 GMT` | type 2; same fields as HTTP26 |
| HTTP28 | `Sunday, 06-Nov-1994 08:49:37 GMT` | type 3; same fields as HTTP26 |
| HTTP29 | `Sun Nov  6 08:49:37 1994` | type 4; 7/6/11/1994/311/8/4/0 |
| HTTP30 | type-1 shape, years 0049 and 0050 | years 2049 and 1950 |
| HTTP31 | type-1 shape, January 1, 2000 | reported yearday 3, including leap increment |
| HTTP32 | type-1 shape, weekday Bad, month Xxx, zone PST | parse succeeds; weekday/month 0; zone ignored; yearday uses index-zero word |
| HTTP33 | mktime stub returns 10000; timezone 28800 | `0xffffb690`; tm_isdst input 1 |
| HTTP34 | mktime stub returns -1; timezone 28800 | `0xffff8f7f`; failure sentinel not preserved |
| HTTP35 | destructor: open receive log, closed send log | shutdown, target release; receive test/close/destroy(1); send test/destroy(1); remaining members in section 5 order |

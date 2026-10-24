# WWW-reached HTTP lifetime boundaries

Companion to web-network.md §4.14. Native GOG 1.1.641 evidence only. HTTP worker and
completion bodies below belong to cRZHTTPClient, **not** the 148-body WWW inventory;
they explain the reached call boundaries, without claiming that foreign owner complete.

## 1. Reached client constructor (00818210, 481 B)

This WWW-owned attribution row constructs the HTTP client, despite the folded export's
particle-simulator table name. It installs table 00abfca8 (raw executable pointers,
not the export's class label); clears initialized byte +4, words +8/+0xc, target +0x10;
constructs empty strings at +0x14, +0x28, +0x40, +0x6c, +0x80, +0xc8 with eight-byte
capacity and zero auxiliary words; clears +0x3c; constructs suffix +0x54 from the pointer
at 00b0ee4c (initial bytes `.tmp` at 00abf9a0), clears byte +0x68; clears +0x94/+0x98;
constructs thread +0x9c and critical section +0xa4. Set worker +0xac=null; allocate a
40-byte queue sentinel +0xb0 linked to itself; set lookup +0xb4=null; construct address
+0xb8 with (0,0); set word +0xdc=0, timeout +0xe0=15, +0xe4/+0xe8=0. Finally allocate
36 bytes and construct lookup 0081d8d0, assigning +0xb4; allocation failure leaves it
null and still returns this. Earlier string/sentinel allocations have no recoverable
failure path. No request or target hold, socket connect, callback or thread start occurs.
Instruction evidence: 00818216–008183f0; suffix-pointer bytes independently read from EXE.

## 2. Final completion: callback before releasing resources

Terminology correction (Q20, 2026-10-24): job +0x14 is the retained **request**,
and +0x18 is the separately allocated **response**. The older internal/external
resource labels below describe these two holds, not a request-to-response cast.
Section 2.1 specifies how they reach completion.

At worker final boundary 0081c821–0081c8bd, cancel (+6) or stop (+5) clears cancel and
sets job status=7; stop is then checked separately. If stop is false, call completion
008196e0. Cancellation alone therefore still permits a failed completion callback.
Stop true skips that call. Neither observation promises atomic suppression of callbacks
already entered, or callbacks in a different phase.

008196e0 first copies internal resource strings (+8, +0x1c, +0x30) and redirect count
+0x74 into the external resource when both resource pointers exist. These are content
assignments, not pointer swaps. Set job progress=100. If target exists, send 0x789af322
with client pointer, success=(status==0), and borrowed job pointer through 00817ed0.
**After the callback**, release internal job resource +0x14, then external resource +0x18:
unsigned refcount >1 decrements, otherwise invoke deleting destructor slot 0 with flag 1.
Both holds still exist during callback. No extra AddRef of the target or resource is
performed here. Return true (00819780–008197f2; copy sequence 008196e0–00819780).

After that call or its stop bypass, the worker clears record current-job +0x6c, destroys
local URL/strings and loops. The stop bypass does **not** execute either resource release
in 008196e0. It must not be implemented as a common completion-and-release operation.
This proves skipped releases in that path, not a universal claim about later owners.

At worker exit, lock client +0xa4, unconditionally clear client worker pointer +0xac,
close the record socket through 00817af0, destroy that socket, free the record, unlock,
destroy local buffers, return zero (0081c92a–0081c9a4). The static wrapper 0081c9b0 saves
owner, runs worker and Releases owner. No thread join occurs in these bodies.

### 2.1 Dequeue, response creation and publication (Q20)

The eight-word admission value is not a different completion-record type. Its
offsets remain stable for the entire job:

| Offset | Meaning / initial admission value |
|---|---|
| +0x00 | Job ID |
| +0x04 | Result, initially 1 (pending) |
| +0x08 | Dequeue time, initially zero |
| +0x0c | Attempt word, initially 1; incremented on retry/redirect |
| +0x10 | Progress, initially zero; reset on retry/redirect; 100 at completion |
| +0x14 | Retained request pointer supplied to admission |
| +0x18 | Response pointer, initially null |
| +0x1c | Caller custom word |

Under client lock +0xa4, dequeue copies all eight words from the first queued
node into a worker-local record, unlinks/frees only the node, publishes the local
record's address in worker +0x6c, overwrites job +8 with the current 32-bit time,
then unlocks. No request/response reference changes accompany that shallow copy
(0081bb33–0081bb9e). URL parsing, header construction and connect/send use the
request still at +0x14. Early builder, resolution or connection failure therefore
can reach completion with response null; the request is still released there.

After send has finished, and provided cancellation is not already set, allocate
a **new** 0x78-byte resource and construct it with 0081a910. This is the same
resource layout as requests, but a distinct object. Constructor reference count
is zero; worker increments it to one. Initial strings +8 (method), +0x1c (URL),
+0x30 (original URL), +0x4c (raw header), +0x68 (body) are empty; header-record
range +0x58..+0x60 is empty, versions +0x44/+0x48 are -1, file-data byte +0x64
is false and redirect count +0x74 is zero. The initialized response's raw-header
buffer first receives the worker's carried receive bytes, then header acquisition
and parsing operate on that object (0081c0c3–0081c0f0, 0081c292–0081c2ca;
constructor 0081a910–0081a9f2). Allocation returning null is subsequently
dereferenced; there is no recoverable out-of-memory result to infer.

Header acquisition separates initial body bytes into response +0x68, keeps the
header in +0x4c and populates its record list. Status/version use this response,
as specified in [HTTP wire §1.1](web-network-http-wire.md#11-status-and-version-tokens-q20-2026-10-24).
Body acquisition and representation/file handling also use this response. Only
when the worker reaches finalization does it **store the response pointer in job
+0x18**, immediately before calling 0081a470 (0081c597–0081c5d7; accounting for
six pushed arguments, store at stack +0x60 is local job base +0x30 plus +0x18).
The original request pointer in +0x14 is never replaced by this store.

Terminal completion copies the request's method, current URL, original URL and
redirect count to the response, when both pointers exist. Response headers,
version, body and file flag remain the response's own values. Both objects stay
held during notification; afterwards Release request, then response, according
to section 2. The callback's job pointer is borrowed and is cleared from worker
+0x6c after completion/stop bypass. A caller needing either resource afterwards
must obtain its own reference while valid.

The publication boundary also matters on failure and requeue. Header failure or
body-read failure before finalization leaves +0x18 unchanged, even though a local
response may have been allocated and partially filled. The worker does not
publish or Release that local response on those paths. This is a native missing
release, not a usable partial-response result. Transport retries shallow-copy
the existing eight words, without completion or new resource holds. Redirect
finalization requeues with the published response pointer; it likewise neither
releases it nor resets it to null (0081a470–0081a847, append 008180c0). On a later
attempt an early failure can therefore complete with that earlier response;
successful new publication overwrites it without releasing the old pointer.
Completion releases only the pointers then present in the record. Stop bypass
still skips completion and its two releases, as section 2 states. None of these
leaks or stale values authorizes converting the request into a response.

**Constructed conformance vectors (PROPOSED / UNEXECUTED).** Observe pointer
identity, fake reference counts and call order, with a non-stopped worker unless
specified. The values here concern native ownership; no live network was used.

| ID | Setup / boundary | Required observation |
|---|---|---|
| WWW20-R1 | Admission then first dequeue at time 1000 | Same request pointer and custom word; +8=1000, +0x18=null; no AddRef on dequeue |
| WWW20-R2 | First-attempt lookup failure | result 3; callback response null; request Release after callback |
| WWW20-R3 | Valid 200 response with one body byte, just before finalization | distinct request and response, +0x14=request, +0x18=response, response hold=1 |
| WWW20-R4 | R3 with request method/URL/original URL/redirect count set | response metadata copied before callback; response headers/body retained; request Release precedes response Release afterwards |
| WWW20-R5 | First-attempt header status zero, retry unavailable | failed completion still has +0x18=null; allocated local response is not released/published by this path |
| WWW20-R6 | Body-read failure before finalization, retry allowed | requeued record keeps request and old/null response; new local response not published or released; no completion |
| WWW20-R7 | Redirect publishes response A, next attempt fails lookup | queued/current +0x18 remains A; completion metadata comes from current request; Release request then A |
| WWW20-R8 | Redirect response A, next attempt reaches finalization with B | +0x18=B; overwrite does not Release A; terminal completion releases current request and B |
| WWW20-R9 | Stop observed at final boundary with both pointers populated | status 7; neither completion callback nor either completion Release |

## 3. Reached shutdown and queue drain boundaries (foreign evidence)

Native table +0x14 points to 0081cd40, not to a semantic interpretation of its folded
name. If initialized byte +4 is false, return true untouched. Otherwise lock +0xa4.
Only if a worker exists and active byte +4 is true, save that worker's socket address,
call 0081ccd0 (drain below), then close the saved socket via 00817af0. Clear initialized,
unlock, return true. No wait/join or target Release occurs (0081cd40–0081cd90).

0081ccd0 locks recursively, calls stop helper 0081cba0, frees every queued raw 40-byte
node and resets sentinel links, unlocks, returns true. Raw nodes receive no request or
resource Release (0081cced–0081cd25). The stop helper also takes the lock: if a record
exists, set its cancel and stop bytes, clear client worker pointer +0xac, count queued
nodes, and start a new worker with 0081c9d0 if nonempty. Thus shutdown/drain may briefly
start another consumer before freeing queue nodes, while retaining the outer lock.
0081cd40 closes the **saved old** socket, not a new record's socket. There is no explicit
join in that sequence. A stopped old worker later clears client +0xac unconditionally,
even if a replacement has populated it; callers cannot treat this field alone as proof
that no worker survives (0081cba0, 0081c9d0, 0081c948).

These are bounded ownership/order contracts. Other worker transport, retry/redirect,
progress, and destructor behavior remain outside completion claims; no whole HTTP owner
credit is assigned. WWW target behavior in the already documented §4.7/§4.14 is preserved.

## 4. Independent discriminating vectors

| Constructed schedule | Required distinction |
|---|---|
| Final boundary: cancel=1, stop=0 | Status 7, cancel cleared, failed callback then both resource releases |
| Final boundary: stop=1 | No 008196e0; its callback and both releases skipped |
| Both resources refcount=2 at completion callback | Callback sees both holds; each decremented afterward |
| Internal resource refcount=1, external=2 | Internal deleted first, then external decremented |
| Target null on normal completion | No notification, but progress and both releases still occur |
| Uninitialized shutdown with queued nodes | Returns true without draining them |
| Active shutdown with queued nodes | Stop/detach old worker; replacement start may occur before raw-node drain; no join |
| Old stopped worker exits after replacement published | Old exit still writes client worker pointer null |
| Lookup allocation fails in constructor | Constructor returns this with lookup=null; no callback |

## 5. Final residual

Q9 remains partial for the exact O/P rows in web-network-www-bodies.md. Constructor
00818210 is now complete, and worker completion/resource/stop/drain boundaries above
are resolved; this does not close uninspected support/adapters or cookie constructor.
Final round 3/3 disposition: **PARK**; no fourth round or whole-owner credit.

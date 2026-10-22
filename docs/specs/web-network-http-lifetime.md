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

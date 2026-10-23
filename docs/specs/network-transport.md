# Native transport owners (behaviour spec)

Status: bounded partial publication, 2026-10-23, re-network-transport4. No native execution,
socket creation, resolver request or game run was performed. Literal traces below are
**PROPOSED / UNEXECUTED**, derived independently from instructions, not implementation output.
Related scope and exclusions: [web-network](web-network.md). Implementers must not read RE artifacts.

## 1. Evidence, domain and raw boundaries

Windows GOG 1.1.641, image base `00400000`; main checkout `bin/SimCity4.exe` SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b` matches
`ghidra/export/windows/export-summary.txt`. Primary export `instructions.asm` SHA-256
`31b87fdd94db47d22d1b5af1863dae33706b93d0867b766e5a82117e14659416`;
`functions.tsv` SHA-256 `ffada4c2a05f6e99dc446d9d830da084eb94c4562fd7f10cebe9f7eb27400c6d`.
Queries use `tools/re_query.py asm --bytes` against that export. Local pseudocode is navigation only.
Names are exported associations, not independent proof of ownership. Offsets below identify
observed state, not a new public ABI. Provisional owner totals 4049/1700/2799 bytes are not coverage credit.

Supported inputs use valid objects, accessible nonaliasing input/output storage and well-formed
raw replies. Allocation, foreign runtime initialization, callback reentry and OS failures outside
the explicit return paths are unsupported. Each foreign socket operation supplies only its raw
return, writes to its passed buffers, and (when requested) a raw 32-bit last error. It never supplies
the final owner boolean, state transition or a completed transfer loop. No textbook retries are implied.
Timers and generic runtime facilities remain separate dependencies.

All selected instance methods use 32-bit x86 `this` in ECX. Boolean results are AL only;
upper EAX bits are unspecified. Stack arguments are 32-bit slots, including pointers.
`T(e)` denotes the pure error translation helper at `0081fed0` (mapping in section 5).
Owner error fields are translated error E at +44 and raw error R at +48,
both 32 bits. Recording an error means obtaining raw last error, then translating, then storing E/R.
Normal successes below do **not** clear stale E/R unless explicitly stated.

## 2. State and lifecycle

| State | Width / initial value | Evidence |
|---|---|---|
| ID +04 | 32 bits, 0 | `008201ef` |
| Handle H +08 | 32 bits, `ffffffff` (invalid) | `008201f2` |
| Bound flag B +10 | byte, 0 | `008201f9` |
| Local +12, peer +22 address | 16-byte records; only first 16-bit word initialized to 0 | `008201fc–00820204`, `0081e6c0` |
| Family/style/protocol +34/+38/+3c | 32 bits each, 2/1/0 | `00820209–00820217` |
| E/R +44/+48 | 1/0 | `0082021a–0082022d`, section 5 |
| Save-on-delete +4c | byte, 0 | `00820230` |
| Notification target/ID/window-record +50/+54/+58 | 32 bits each, 0 | `00820233–00820239` |
| Stream state S +5c | 32 bits, 0 | `00821638–0082164d` |

The constructor returns `this` in EAX. It does not initialize all address bytes or open a
socket. The stream constructor first runs the base constructor, then sets S=0 and the
family/style/protocol to 2/1/0 and installs its dispatch table (`00821630–00821657`).
Unwritten padding/address bytes are unsupported as expected literals.

| Operation and stack arguments | Exact owner policy | Evidence |
|---|---|---|
| CreateSocket(), AL | Call raw socket(family,style,protocol) once; store returned H unconditionally. H=`ffffffff`: record error and return false; otherwise true. No old-handle close, no preservation on failure, no B/S mutation. | `00820410–00820449` |
| Base CloseSocket(), AL | If H invalid, skip raw close. Otherwise raw closesocket(H), then set H invalid regardless of raw result. Nonzero raw result records error and returns false; zero or already invalid returns true. B=0 on every return. | `00820450–00820490` (unnamed body, base destructor call establishes association) |
| Stream CloseSocket(), AL | Set S=0 before checking H; otherwise base-close policy above, including B=0. | `00821320–00821367`, stream vtable +0c |
| Stream ShutDownSocket(how:32), AL | Set S=4 before checking H. Invalid H returns true without OS call. Otherwise raw shutdown(H,how), zero returns true; nonzero records error and returns false. H and B stay unchanged. Pops 4 argument bytes. | `00821370–008213b2` |
| BindSocket(address:ptr), AL | Copy all 16 bytes to local address **before** raw bind(H,local,16). Zero sets B=1, returns true; nonzero sets B=0, records error, returns false. Failure retains copied address and H. Pops 4 bytes. | `008204e0–0082053d` |
| StartListen(backlog:32), AL | raw listen(H,backlog), zero sets S=3 and returns true; nonzero records error, returns false without S change. No handle validity guard. Pops 4 bytes. | `008213c0–008213fd` |
| StopListen(), AL | Invalid H: true with S/B unchanged. Valid H: raw close, then invalidate H. On raw zero set S=0 and return true; nonzero record error and return false with S unchanged. B is unchanged even on success. | `00821400–00821440` |
| Connect(completed:byte*, timeout:32), AL | Construct local foreign callback timer. If timeout nonzero, set callback `008201c0` with current H and start once(timeout,0). Call raw connect(H,peer,16). Zero writes completed=1, S=2; nonzero writes completed=0 and records error. Error 10035 returns true, all other errors false; errors leave S unchanged. Destroy timer before return on all normal paths. H and B unchanged by this body. Pops 8 bytes. | `00821450–008214f1` |

Timer callback `008201c0–008201d3` calls raw closesocket with its captured 32-bit handle;
on nonzero result it calls raw last-error and returns that EAX, otherwise raw zero. It
does not access or invalidate owner H or update E/R/S. Timer scheduling, cancellation,
units and races remain foreign/unknown; use timeout=0 for the deterministic contract.
In particular, true with completed=0 is not connected state.
The address-taking overload first calls virtual peer-setter +3c, then virtual Connect +f4,
passing completed/timeout, and returns that AL (`0081fb50–0081fb76`, pops 12 bytes).
Foreign callbacks may not be folded into a raw connect reply.

The base destructor closes a valid H unless save-on-delete is nonzero; then, if a window
record exists, destroys its first-word window handle, frees the record and clears +58;
then releases a nonnull target via virtual +08 and clears +50. This order is primary-proved
at `008215d0–00821622`; general destruction/reentry semantics remain outside the domain.
The stream deleting destructor `00821660–0082167d` calls the base destructor first, then
frees the object if bit 0 of its 32-bit argument is set; returns its original pointer in
EAX and pops 4 argument bytes. It does not dispatch through stream CloseSocket first.

Accept(handleOut:u32*, peerOut:16-byte*, completed:byte*) at `00821500–00821562` pops
12 bytes. It calls raw accept(H,peerOut,&localLength), localLength initially 16; always
writes the raw returned handle to handleOut. On -1, write completed=0, record E/R, return
true only for R=10035. Otherwise write completed=1 and return true. No owner H/B/S mutation;
peerOut retains exactly the raw primitive's writes, including on error. Returned length
is discarded. The object overload `0081fb80–0081fbcf` (8 bytes: destination stream*,
completed*) passes destination.H and destination.peer to virtual Accept +fc. Only when
AL and completed are both nonzero does it set destination.S=2 and call destination virtual
SetSocketStyle +20 with listener family/style/protocol; it ignores the style-setter result
and returns the original Accept AL. There is no destination old-handle close or rollback.

SetSocketStyle(family,style,protocol), `0081f980–0081f999`, copies all three 32-bit
arguments and returns AL=1 (12 bytes popped). IsSocketCreated, `0081f9a0–0081f9ab`, is
exactly H != ffffffff, AL result, independent of S/B. SetID(u32) stores +04 and leaves that
argument in EAX (`0081e7a0–0081e7a9`); SetSaveSocketOnDelete reads the low byte of its
one argument, stores +4c and leaves that byte in AL (`0081f900–0081f909`). Neither return
is an independently established success boolean. Both pop 4 bytes.

## 3. Stream transfers

Read(buffer:ptr, count:i32*) and Peek/ReadOOB use 8 argument bytes; Write(buffer:ptr,
count:i32*, flags:32) uses 12. Each count is both request and output, stored as 32 bits.
The owner never fills the rest of a short transfer or loops until complete.

| Operation | Initial count bypass | One raw call when not bypassed | Evidence |
|---|---|---|---|
| Read | signed count <=0: false, count/buffer/errors/state untouched | recv(H,buffer,count,0) | `00821010–0082109d` |
| Peek | count=0: true, untouched | recv(H,buffer,count,2) | `008210a0–00821129` |
| ReadOOB | count=0: true, untouched | recv(H,buffer,count,1) | `008211a0–00821229` |
| Write | count=0: true, untouched | send(H,buffer,count,flags) | `00821130–0082118f` |
| WriteOOB | count=0: true, untouched | send(H,buffer,count,flags OR 1) | `00821230–00821294` |

For a raw -1 transfer, obtain last error, write count=0, translate and store E/R, then
return true only for R=10035 or R=10036. Other errors return false. H/B/S are unchanged.
There are no owner buffer writes on this branch; any raw primitive buffer writes remain
visible. The contract does not assume an OS failure leaves memory untouched.

For nonnegative raw receive n>0, store count=n and return true, leaving errors/state
unchanged. Raw receive 0 writes count=0, stores E=T(10057), R=10057, then calls virtual
CloseSocket +0c and returns false. Thus close failure may replace the EOF errors, and
the stream close sets S=0/B=0/H=invalid. Peek and ReadOOB have this EOF-close behavior too.
No EOF close occurs merely because the requested count was zero. Raw successful send
stores count=n (including 0), returns true, and leaves E/R/H/B/S unchanged.
Read's residual negative-result clamp is not a supported raw-reply domain; ordinary
raw replies are -1 or 0..requested. Negative Peek/OOB/Write requests reach the raw call,
but their foreign meaning and buffer extent are unsupported, not an expected false.

## 4. Independent literal traces (PROPOSED / UNEXECUTED)

These are owner decision traces with injected typed raw primitive replies. Buffer bytes
change only where the raw reply explicitly writes them. No analytical trace is native evidence.

| ID | Input and injected reply | Expected literal observation |
|---|---|---|
| L1 | H=41, B=1, S=3; Create raw=73 | H=73, B=1, S=3, AL=1; one socket call, no close |
| L2 | Same start, Create raw=-1, last error=10061 | H=ffffffff, B=1, S=3, R=10061, E=T(10061), AL=0; old 41 never closed |
| L3 | H=ffffffff, B=1, S=2; stream Close twice | each AL=1, H invalid/B=0/S=0, zero close/error calls |
| L4 | H=41; stream Close raw=-1, error=10038; then Close again | first AL=0 and R=10038; H invalidated after raw close; second AL=1 and errors unchanged, no second raw close |
| L5 | H=41, S=3, completed=7; Connect timeout=0, raw=0 | completed=1, S=2, AL=1; errors untouched |
| L6 | Same start; raw=-1,error=10035 (then separately 10061) | completed=0, S=3, R=10035, AL=1 (respectively R=10061, AL=0); H=41 |
| I1 | Read count=8, buffer all aa; raw writes 31 42 53 and returns 3 | count=3, AL=1; buffer=31 42 53 aa aa aa aa aa; one recv, no second call |
| I2 | Read count=0 (then -2), sentinel buffer | AL=0; count remains 0 (then -2), buffer/errors unchanged; no recv |
| I3 | Peek count=0, sentinel buffer | AL=1, no recv or close, all outputs untouched |
| I4 | Read count=8; raw=-1, no buffer writes, error=10036 (then 10061) | count=0, buffer unchanged; R=10036, AL=1 (then R=10061, AL=0), H unchanged |
| I5 | Read count=8; raw=0, close raw=0 | count=0, R=10057, E=T(10057), AL=0; order recv, translation, virtual close; H invalid/B=0/S=0 |
| I6 | Write count=8, flags=4; raw=3 (then 0) | count=3 (then 0), AL=1; one send(H,buffer,8,4); no retry or close |
| I7 | Write count=8; raw=-1,error=10035 | count=0, R=10035, AL=1; one send, no retry; source buffer has no owner writes |

Counts at this milestone: lifecycle L1–L6 = 6 vector groups; transfers I1–I7 = 7 groups.

## 5. Error translation and raw address representation

`T` accepts one 32-bit stack value, returns a full 32-bit EAX, caller cleans the argument.
It performs no foreign call or store. Primary comparisons, branch targets and immediate
returns: `0081fed0–00820076`; primary lookup bytes at `00820078–0082019f`.

| Raw input e | T(e) |
|---|---|
| 10004, 10009, 10013, 10014, 10022, 10024 | respectively 2, 3, 4, 5, 6, 7 |
| 10035 through 10071 inclusive | e - 10027 (8 through 44) |
| 10101 | 45 |
| 10091 through 10093 | e - 10045 (46 through 48) |
| 11001 through 11004 | e - 10952 (49 through 52) |
| Every other 32-bit pattern, including 0 | 1 |

Transport address arguments are raw 16-byte records. Bind, Connect and Accept use raw
length 16. The socket owner performs no host/network byte-order conversion on these records.
SetMyAddress and SetPeerSocketAddress copy four 32-bit words in increasing offset order,
return AL=1, pop 4 bytes (`0081fa00–0081fa21`, `0081fa30–0081fa51`). Their copies include
all padding. BindSocket() passes the embedded local record to virtual +1c and returns
its AL (`0081f9b0–0081f9b9`). Arbitrary overlapping records are outside the domain.

GetSocketAddress(addressOut*, handleIn:u32*) and GetPeerAddress(addressOut*,handleIn:u32*)
at `00820540–00820589` / `00820590–008205d9` pass *handleIn, addressOut and a local
length initialized to 16 to raw getsockname/getpeername. Zero returns AL=1 without owner
address mutation; nonzero records error and returns AL=0. No output clearing or rollback:
only foreign writes affect addressOut. Returned length is discarded. Both pop 8 bytes.
GetMyAddress/GetMyPeerAddress pass the supplied addressOut and &H to virtual +28/+34,
returning its AL (`0081f9c0–0081f9d0`, `0081f9e0–0081f9f0`, 4 bytes popped).

Adjacent address adapters are dependencies, not credit to these three owners:
`0081e740–0081e751` returns AL byte index i of the raw +04 address word copied onto the
stack (supported i=0..3). `0081e770–0081e782` returns AL byte index i of zero-extended
16-bit +02 port (supported i=0..1). Both pop 4 bytes. `0081e790–0081e79b` stores the low
16 bits of its 32-bit argument at +02 **without swapping**, pops 4, leaves that word in AX;
no boolean contract. These unnamed adapters' association is structural, not named-owner credit.
For network bytes `12 34`, callers must supply raw word `3412`; the socket boundary does
not convert a host integer `1234` into those bytes. A complete public host-port constructor
has not been qualified here.

## 6. Readiness and available byte count

Timeout is a 32-bit **unsigned** millisecond value. `ffffffff` passes a null timeout
pointer; otherwise seconds=floor(timeout/1000), microseconds=(timeout mod 1000)*1000,
two 32-bit values. There is no owner clock read, deadline subtraction, wrap comparison
or readiness retry in these bodies. Raw select return -1 denotes error; 0 timeout; a
positive scalar denotes reported readiness. Raw fd-set mutations and membership tests
remain primitive replies, not a precomputed owner result.

| Body, arguments, result | Owner dispatch and mutations |
|---|---|
| IsWaitingDataAvailable(timeout), AL; `008206d0–0082075b`, pop 4 | One read fd-set containing H; select(1,read,null,null,time). Error records E/R then **sets H invalid**, returns false, leaves B/S unchanged; other results return AL=(raw !=0). No raw close. |
| IsWritable(timeout), AL; `00820760–008207e4`, pop 4 | One write fd-set containing H; select(1,null,write,null,time). Error records E/R and returns false, **preserving H**, B/S. Other results return AL=(raw !=0). |
| IsReadableAndOrWritable(mask,timeout), EAX; `0081fa60–0081fb45`, pop 8 | Initially empty read/write sets; bit 0 inserts H into read, bit 1 into write; other mask bits ignored. Pass null for an unrequested set, except set null, nfds=0. Raw select -1 returns full EAX=-4, no last-error call or owner mutation. Otherwise invoke raw membership(H,read) then membership(H,write), including locally empty unrequested sets; return full EAX of ORed bits 1/2 for nonzero replies. Even raw select=0 reaches both membership calls. |
| GetWaitingDataSize(sizeOut:u32*,timeout), AL; `008205e0–008206c7`, pop 8 | timeout=0 skips select and directly calls raw ioctlsocket(H,4004667f,sizeOut). Nonzero timeout first selects read readiness, using above timeout encoding. Select -1 records E/R, returns false **without writing sizeOut or invalidating H**. Select 0 writes sizeOut=0 and returns true. Select positive calls ioctl. Ioctl zero returns true, retaining raw size writes; nonzero first writes sizeOut=0, then records E/R and returns false. |
| IsSocketConnected(), AL; `008212a0–0082131e` | Invalid H immediately false, no S change. Otherwise select(1,read{H},null,null,{0,0}). Any raw return except -1 returns true, including 0; does not inspect fd membership or S. Error records E/R, invalidates H, sets S=0, returns false; B unchanged; no close. |

## 7. Name lookup, cache and request ownership

This is a bounded contract for normal **already initialized** global state: critical-section
handle at `00b4fe14`, request list at `00b4fe2c`, cache map at `00b4fe4c`, counter at
`00b4fdb4`, three worker pointers at `00b4fda0`, sleep bytes at `00b4fdac`, stop bytes at
`00b4fdb0`. Global startup/teardown and the lookup object's construction are unproved;
do not infer a per-instance cache or constructor from adjacent unnamed bodies. A matched
name is insufficient attribution. See [open questions](questions/network-transport.md).

Lock/unlock are foreign critical-section primitives at `00930b35`/`00930b6d`. The former
has a misleading exported list::erase name; its actual interface is a lock call on the
critical-section's handle and increment of nesting state, not list deletion. General
lock failure/reentry is unsupported. The shared scalar clock at `009efb59` has a misleading
NetworkFileCache name; it is retained as **foreign raw clock(argument=0)->u32**. Its
implementation, units and epoch are not republished through this owner.

### 7.1 Immediate lookup

DoLookup(name:string*, idOut:u32* nullable, target:ptr nullable, addressOut:u32* nullable)
uses ECX as request-owner pointer, pops 16 argument bytes and returns **full EAX**:
0 failure, 1 immediate success, 2 queued. `0081e400–0081e636`.

1. Pass name's character pointer to raw inet_addr through IsAddressNumeric
   (`0081e720–0081e734`, caller cleanup, AL = raw result != ffffffff).
2. If numeric, independently convert the original text through `0081e6d0–0081e71a`:
   scanning digits, a period transfers immediately to another raw inet_addr call; all
   digits to terminator calls foreign decimal-to-i32 then byte-reverses its 32 bits;
   an intervening nondigit/nonperiod returns ffffffff. Supported decimal conversion
   is restricted to representable nonnegative values. No internal dotted-address parser
   is claimed. Write conversion result to addressOut if nonnull. Lock; increment shared
   counter modulo 2^32; copy new counter to idOut if nonnull; unlock. Return EAX=1 if
   converted address !=0, otherwise 0. A zero numeric address still writes output and ID.
3. If not numeric, lock; call GetNameFromCache(name,&temporaryTriple). On hit copy only
   its address word to addressOut if nonnull, increment/copy counter as above, unlock,
   return 1 even if cached address=0. Target is ignored. On miss unlock before proceeding.
4. Cache miss and target=null: call raw gethostbyname(nameChars) once. Null reply writes
   addressOut=0 if nonnull, calls WSAGetLastError but discards its value, returns full EAX=0;
   idOut and counter stay untouched, with no cache insertion. Nonnull reply: load pointer
   at reply+0c, its first pointer, then exactly its first 32-bit word; copy to addressOut
   if nonnull. Lock, increment/copy counter, unlock; copy the name and AddNameToCache
   with address and duration=0; destroy temporary name; return EAX=1. No byte swap,
   length/family check, second address, name/alias copy, resolver-memory release or
   reference retention occurs. Zero first address is still success. A nonnull reply
   with null/empty/short first-address storage is **unsupported**, not expected false.

### 7.2 Cache policy

GetNameFromCache(name*, tripleOut*) at `0081dcb0–0081dd6b` is caller-cleaned with two
32-bit stack arguments, AL result. Copy name to a temporary string, lowercase it using
foreign string semantics, find that key. On absent key return false with output untouched.
On present entry (address:u32, duration:u32, inserted:u32), call raw clock(0) and compare
**unsigned** `(inserted+duration) mod 2^32 > now`. Strictly greater copies all three words
to tripleOut, returns true. Equality or less erases that entry and returns false, output
untouched. Destroy temporary string before return. This body does not acquire the lock
itself; DoLookup calls it while locked. Lowercasing/ordering beyond ordinary ASCII is foreign.

AddNameToCache(name*, address:u32, duration:u32) at `0081e020–0081e0d1` is caller-cleaned,
three 32-bit arguments, AL=1 on its normal path. A zero duration becomes 1800. Lowercase
the **supplied name object in place**, then lock and find key. Insert if absent, replace
if present; store address, effective duration and fresh raw clock(0). Unlock and return
true. No capacity eviction or duplicate-address suppression is present in this body.
Allocation failures are unsupported, with no invented transactional rollback. Add's
name mutation is observable for direct callers; DoLookup supplies a temporary copy.

### 7.3 Queued requests and bounded worker sequence

On nonnumeric cache miss with nonnull target, DoLookup locks, increments/copies counter,
constructs an entry, sets owner=this, ID=counter, name=copy, reserved/address word=0,
target=target, following word=0, trailing word=ffffffff. It invokes owner's virtual +0c
then target's +04 (retention calls), appends the copied entry at the request list's back,
calls WakeWorkerThread, unlocks, destroys local name and returns EAX=2. addressOut is
**untouched** on this path. The append's generic copying/allocation internals are foreign;
the explicit owner retention calls and enqueue-before-wake order are part of this contract.

WakeWorkerThread, `0081e350–0081e3f1`, caller-cleaned/no arguments, no established scalar
return: scan slots 0..2 for an existing worker with sleep byte nonzero. First such slot
gets sleep=0, then worker virtual +14, returning immediately. Otherwise scan 0..2 for
first null worker. If all occupied, return without starting another. For an empty slot,
allocate/construct a foreign thread, store pointer, allocate four-byte index argument,
write slot index and call worker virtual +08 with WorkerThreadFunction and that argument.
The explicit null-argument allocation branch still calls +08 with null. Allocation/start
failure behavior and concurrent scheduling are unsupported; normal domain assumes both
allocations succeed. No claim that wake waits for completion.

WorkerThreadFunction(indexPtr), `0081e0e0–0081e34f`, is a one-stack-argument callback,
callee pops 4, full EAX=0 on exit. It reads index then frees indexPtr; supported index=0..2.
While own stop byte is zero: lock, inspect front of queue. Empty queue counts nonnull
workers. If more than one, leave the loop; otherwise set own sleep=1, unlock, call foreign
suspend-current primitive `009205c9`, set sleep=0, and recheck stop. Nonempty queue:
copy front entry, unlink/free front node and call RemoveLookupEntry with its ID while
still locked, then unlock. The latter can remove a second queued entry with the same ID;
ordinary unique IDs make it a no-op. Do not invent cancellation of an in-flight local copy.

If copied target is null, destroy the local name and continue; no resolver/message or
explicit owner-release call occurs on this branch. Otherwise allocate/construct a foreign
message, invoke message +04, set type via +14(43544312), ID via +30(requestID), then call
raw gethostbyname(nameChars). Success selects exactly the first four bytes as above,
calls message +40(address), +58(&name), then AddNameToCache(name,address,0). Null reply
calls message +40(0) then +58(&name), with no last-error fetch or cache insertion.
Then call target +0c(message), owner +10(), target +08(), message +08(), in this order;
destroy local name and recheck stop. Message field/method names beyond these raw slots
are not required as a foreign whole-lookup oracle: only raw message setters/delivery and
reference operations are primitive. Failed message allocation leads to unchecked null
use, outside the supported domain.

On stop or surplus-worker exit, lock, if worker slot nonnull call virtual +34(1), then
clear slot, unlock, return 0. No join or queue drain is implied. Message allocation,
synchronization, actual threads and callbacks were not executed.

RemoveLookupEntry(id:u32), `0081da00–0081da87`, caller-cleaned, full EAX=0/1: lock,
search queue from front for first matching ID, unlink/destroy that node if found, save
its target/owner, unlock; call nonnull target +08 then nonnull owner +10. Return 1 iff
saved target nonnull (so removal with null target returns 0); absent ID returns 0 without
reference calls. This is queued removal only, with no raw resolver cancellation.

ExitAllWorkerThreads(timeout:u32), `0081d7f0–0081d8c8`, caller-cleaned, full EAX=count
of slots cleared by this routine. If timeout=ffffffff use that as deadline, otherwise
deadline=(raw clock(0)+timeout) mod 2^32. Set three stop bytes=1. Each pass locks; for
slots 0..2, each nonnull worker increments pass count; if asleep clear sleep and invoke
+14; query +2c, and if zero invoke +34(1), clear pointer and increment result count.
Call foreign SleepCurrentThread(250000) **while locked**, then unlock. If pass count=0
exit without reading clock; otherwise repeat while unsigned raw clock(0) <= deadline.
Finally clear all three stop bytes and return result count. No infinite-timeout sentinel
branch inside the loop, no elapsed-time/wrap correction, no mandatory draining of queued
requests, and no join beyond the stated virtual calls. Clock units and foreign sleep
units are not inferred to match. Result counts cleared slots, not completed requests.

## 8. Further independent literals (PROPOSED / UNEXECUTED)

| ID | Input / raw replies | Expected literal observation |
|---|---|---|
| L7 | S=3,B=1,H=41; StopListen raw=-1,error=10061; repeat with invalid H | first AL=0,H=ffffffff,S=3,B=1,E=34,R=10061; repeat AL=1,S=3,B=1,no second close |
| L8 | H=ffffffff,B=1,S=2; ShutDownSocket(2) | AL=1,S=4,H/B unchanged,no shutdown |
| L9 | Accept handleOut=77,completed=7; raw=-1,error=10035,no peer writes | handleOut=ffffffff,completed=0,AL=1,E=8,R=10035; peer bytes untouched |
| L10 | Accept into destination H=77,S=3; raw=89, peer writes 16 bytes | destination.H=89,S=2; completed=1; style copied after S; no close(77) |
| L11 | stream destructor H=41,save=0,target nonnull,window record nonnull; raw close=0 | raw close(41), H invalid/B=0, destroy window, free record, clear +58, target +08, clear +50; optional object free last |
| A1 | peer record `02 00 12 34 01 23 45 67 80 81 82 83 84 85 86 87`; SetPeer then Connect timeout=0,raw=0 | same 16 bytes passed to connect; port raw word=3412,address raw word=67452301; completed=1,S=2 |
| A2 | local record `02 00 ab cd 89 ab cd ef 90 91 92 93 94 95 96 97`; Bind raw=-1,error=10049 | local retains all 16 bytes, B=0,E=22,R=10049,H unchanged; address word=efcdab89,port=cdab |
| A3 | port adapter input=1234 versus 3412 | raw +02 bytes=34 12 versus 12 34; no htons inferred |
| R1 | IsWaitingDataAvailable timeout=999,1000,1001 in three separate calls | time pairs {0,999000}, {1,0}, {1,1000}; each exactly one select |
| R2 | same timeout=fffffffe versus ffffffff | pair {4294967,294000} versus null pointer; no clock call |
| R3 | H=41,S=2,B=1; read-ready raw select=-1,error=10038; separately write-ready same | both AL=0,E=11,R=10038; read H invalid versus write H=41; S/B unchanged; neither closes |
| R4 | combined mask=3,timeout=0; select=0, memberships 0 then 0; separately select=-1 | EAX=0 after two membership calls versus EAX=fffffffc without membership/error-fetch calls; H/errors untouched |
| R5 | sizeOut=deadbeef; timeout=1000; select=-1,error=10038 | AL=0,sizeOut still deadbeef,H unchanged; no ioctl |
| R6 | sizeOut=deadbeef; timeout=0; ioctl writes 7 then fails,error=10038 | no select; sizeOut=0,AL=0,E=11,R=10038 |
| R7 | H=41,S=0; IsSocketConnected select=0 | AL=1,H=41,S=0; this does not establish connected S=2 |
| N1 | numeric `0.0.0.0`, both inet_addr replies=0; counter=9,idOut=77,addrOut=deadbeef | EAX=0,counter=10,idOut=10,addrOut=0; no cache/resolver |
| N2 | numeric `1.35.69.103`, both inet_addr replies=67452301; counter=ffffffff | addrOut=67452301,idOut=0,counter=0,EAX=1; two raw parse calls |
| N3 | numeric decimal `16909060`, numeric-test reply non-ffffffff, decimal primitive=01020304 | output=04030201,EAX=1; second inet_addr not called; ID increments |
| N4 | nonnumeric miss,target=null; resolver returns two pointers to bytes `01 23 45 67` and `89 ab cd ef` | output=67452301,EAX=1,ID increments; only first word copied; cache add duration 1800 |
| N5 | same, resolver null,error=11001,idOut=77,addrOut=deadbeef,counter=9 | addrOut=0,idOut=77,counter=9,EAX=0; error fetched/discarded; no cache add |
| N6 | same, nonnull first entry=00000000 | EAX=1,addrOut=0,ID increments; contrasts N1 |
| N7 | cache triple {efcdab89,1800,100}, raw clock=1899 then separately 1900, output sentinel | first hit copies all 12 bytes; equality miss erases and leaves output untouched |
| N8 | triple {67452301,32,fffffff0}, raw clock=fffffff1 | wrapped expiry=00000010, unsigned miss; no elapsed-time repair |
| N9 | nonnumeric miss,target=T,owner=O,counter=9,addrOut=deadbeef,all workers occupied awake | owner +0c, T +04, enqueue ID10, wake with no thread-start, unlock; EAX=2,idOut=10,addrOut untouched |
| N10 | one queued ID10,T/O; worker resolver null | message type43544312,ID10,address0,name setter; T +0c(message), O +10,T +08,message +08; no last-error fetch/cache add |
| N11 | queue ID10,target=T,owner=O; RemoveLookupEntry(10) then repeat | first unlink,unlock,T +08,O +10,EAX=1; second no reference calls,EAX=0 |
| N12 | Exit timeout=32, initial raw clock=fffffff0, one awake worker +2c nonzero, next clock=fffffff1 | deadline10; one pass,sleep(250000) before unlock, exits on unsigned comparison, stop bytes cleared,EAX=0; worker pointer retained |

Additional counts: L7–L11=5, A1–A3=3, R1–R7=7, N1–N12=12 vector groups.
Together with section 4: **40 independently derived vector groups**, all proposed/unexecuted.
Empty resolver-list and short address storage are unsupported domains, not golden vectors.

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
`T(e)` denotes the pure error translation helper at `0081fed0`; its complete mapping is
not yet contracted. Owner error fields are translated error E at +44 and raw error R at +48,
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
| E/R +44/+48 | T(0)/0 | `0082021a–0082022d` |
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
| Connect(completed:byte*, timeout:32), AL | Construct local foreign callback timer. If timeout nonzero, set callback `008201c0` with current H and start once(timeout,0). Call raw connect(H,peer,16). Zero writes completed=1, S=2; nonzero writes completed=0 and records error. Error 10035 returns true, all other errors false; errors leave S unchanged. Destroy timer before return on all normal paths. H and B unchanged by this body. Pops 8 bytes. | `00821450–008214f1` |

Connect's timed callback and timer scheduling are not yet supported; use timeout=0 for
the deterministic contract. In particular, true with completed=0 is not connected state.
The address-taking overload first calls virtual peer-setter +3c, then virtual Connect +f4,
passing completed/timeout, and returns that AL (`0081fb50–0081fb76`, pops 12 bytes).
Foreign callbacks may not be folded into a raw connect reply.

The base destructor closes a valid H unless save-on-delete is nonzero; then, if a window
record exists, destroys its first-word window handle, frees the record and clears +58;
then releases a nonnull target via virtual +08 and clears +50. This order is primary-proved
at `008215d0–00821622`; general destruction/reentry semantics remain outside the domain.

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
Lookup, readiness, address conversion and the final body ledger follow in this bounded publication.

# WWW-owned FTP control and scheduling

Companion to web-network.md §4.13 and web-network-ftp.md. This adds native contracts to
the unchanged 148-body / 34049 B checklist. Addresses refer to GOG 1.1.641. Constructed
vectors below are discriminating inputs, not observations from a replacement or live server.
Socket/helper meanings and message payload positions follow the existing FTP companion.

## 1. Login (00815d30, 1245 B)

The entry requires state +0x1b8=0 and a nonempty user string +0x30. A rejected entry
still sends the login-result notification below, if a target exists, and clears +0xa15.
It returns false. With async +0xa14 true, +0xa15 false and target +0x28 nonnull,
set +0xa15, state=4, operation=101, start dispatcher 00816990 and return true without
notification. The dispatcher does not restore state: its login call rejects state 4,
reports false, clears +0xa15, and leaves state 4 (00815d61–00815df0).

Synchronous login sets state=1 and stage +0x1c0=401. Ensure the control socket exists
(00811e10 if necessary); creation failure resets state=0, stage=400. If the pending
host string +0x70 is nonempty, numeric text is converted directly; otherwise construct
a lookup and call 0081e640(host,0,0,out). Any nonzero result is accepted. Lookup
failure resets state/stage and destroys the lookup. Success sets the address and clears
the host string. With no pending host, a zero address also resets state/stage and fails.
An address with a zero 16-bit port receives the stored default word +0x68. Connect uses
timeout +0x20 and requires both the call result and its out-boolean to be true.
Connect failure goes directly to the result notification: state stays 1, stage stays 403,
and the control socket is not closed here (00815e07–00815f28, 0081611e–00816162).

Commands and transitions, in order:

| Action | Stage | Acceptance / next action |
|---|---:|---|
| Read greeting with 00813d30 | 404 | Only the helper boolean is checked; no greeting-class test |
| `USER <user>\r\n` then read | 405 | Signed class <4 continues; class 3 sends PASS, other classes skip PASS |
| `PASS <password>\r\n` then read | 406 | Signed class <4 continues; class 3 sends ACCT, other classes skip ACCT |
| `ACCT <account>\r\n` then read | 407 | Must be class 2 |
| `SYST\r\n` then read | 408 | Must be class 2 |

Each send/read failure calls forced disconnect 00816af0(1). A rejected reply class
also calls forced disconnect, **but does not clear the saved boolean**: the result is
the last send/read boolean. Thus a successfully read USER 530, PASS 530, ACCT 530,
or SYST 500 can disconnect and still report/return true. This follows BL preservation
at 00815f94–00815fb3, 00816000–0081601a, 00816063–00816077,
008160bb–008160cf, 00816164–0081616e, 00816202. Do not substitute authentication
success for that return channel. A false send/read returns false after disconnect.

Accepted SYST invokes the format detector on saved reply +0x1f8, ignores its return,
sets state=2, stage=400, clears +0xa08, sets transfer representation +0x14=201, and
returns true (008160cf–0081611c). Every non-deferred exit with a target creates and
AddRefs a standard message, sets type 0x57386938, uint1=0, void1=0, uint3=boolean
result, uint2=this, directly calls target slot +0xc, then Releases the message.
Only after that callback does it clear +0xa15. It does not AddRef/Release the target.
Allocation failure has no safe fallback (0081616e–0081620c).

## 2. Listing-format selection (008126d0, 360 B)

Search the supplied SYST reply in this order; first match writes selector +0xc and
returns true. The first four searches are byte-exact, case-sensitive substring searches
(004426c0 compares bytes). Their minimum lengths are native guards, not token boundaries.

| Match | Minimum input length | Selector |
|---|---:|---:|
| `Windows_NT` | 10 | 314 |
| `WINDOWS NT Type: L8` | 19 | 315 |
| `UNIX` | 4 | 310 |
| `MultiNet` | 8 | 311 |
| `VMS` | none beyond search | 312 |
| `PC Type: L8` | none beyond search | 317 |

No match returns false without changing the selector; login still succeeds. The VMS
search accepts an unsigned found offset less than input length; the PC search accepts
anything except -1 (008127e5–00812835). Constant bytes verified at 00abf5c8/00abf5d8;
all selectors/guards checked at 008126e4–00812835. The formats themselves remain the
already documented parser contract, not a guessed interpretation of server names.

## 3. Listing acquisition (008163d0, 1466 B)

Require state=2. Otherwise clear +0xa15 and return false with no notification. Deferred
entry uses the same async predicate as login, writes state=4, operation=111, copies the
path to +0xa18, stores the **borrowed** output-list pointer +0xa44, starts dispatcher,
and returns true. Its worker reentry rejects state 4 without notification. No list or
target ownership is acquired by this handoff (00816404–008164fa).

Synchronous entry sets state=4, operation=111, stage +0x1c4=501, byte count +0x24=0,
and start time +0x1c to the current 32-bit time. A null output-list argument selects a
temporary empty list; a nonnull argument is used directly. Copy the first/path argument
into +0x1e0, remove **one** final slash or backslash, and prepare the active listener via 00811e70
then 00811f30. Reading the last path byte is unconditional: an initially empty path
reads before its start, so no deterministic safe empty-path contract is asserted
(0081656f–00816593). A null path also dereferences a null descriptor. `/` becomes empty
after the safe one-byte check. Stack-accounting evidence: 00816534 pushes time's null
argument; 0081653d reads path at ESP+0xd0 **before** 00816544 pops that argument. This
is the same first argument read at ESP+0xcc on async entry, not the output-list argument.

If representation is not 201, send `TYPE A\r\n` at stage 502, require a successful
class-2 reply, and store 201. At stage 503, obtain listener address and send the same
active-mode `PORT` bytes as the transfer companion; require class 2. At stage 504,
send `LIST <trimmed path>\r\n`. Read reply lines until a final line; require class 1,
then accept the data connection with both boolean results true. Control waits set
stage=507. There is no PASV or retry loop for failed commands.

After accept, 00811f00 sets data-receive options. Initial receive capacity is the current
length of buffer +0x1ec. Wait on the data socket with timeout +0x20; receive into
buffer+byte-count, adding each nonzero successful count. Exactly when the total equals
the current buffer length, grow by 256 zero bytes and reset the next requested count
to 256. A partial receive leaves its returned count as the next capacity; it is not
reset to remaining space (008167f0–00816873). When wait/receive fails or count is zero,
resize to the accumulated byte count, call 00811f90, and read a final control reply.
A successful class-2 final reply parses the collected bytes and determines success;
data-loop failure is not separately latched. No progress event is sent by this loop.

All synchronous paths clear +0xa15, set state=2, operation=100, stage=500, close listener
then data socket if open, and invoke 008120e0 with type 0x57386930, uint1=0, uint2=this,
void1=0, void2=the selected list, uint3=result, uint4=0. The control socket stays open.
The callback sees the reset state and live list. The cleanup sequence destroys the temporary
list's nodes and sentinel **after** the callback; a supplied list survives. The callback
pointer does not confer retention rights (008168dc–00816989).

### 3.1 List replacement and ownership

00816230 (415 B) first clears every old output node through 00812a50, then iterates
lines with existing helper 00812d60. Apply the existing entry parser using selector +0xc;
append only accepted entries, in wire order, copying the parser's name and metadata
word into a newly allocated 32-byte doubly linked node. A malformed line is skipped,
not a whole-list failure. Return true at end even if no lines/entries were accepted.
Thus an empty successful listing replaces old contents with an empty list. Allocation
failure is not turned into a recoverable parse error (00816230–008163ce).

00812970 (71 B) destroys each node's string and node, then resets sentinel next/prev
to itself; it does not free the sentinel. 00812840 (94 B) constructs the scheduler's
job-value strings at +8 and +0x1c as empty eight-byte-capacity strings, with auxiliary
words +0x18/+0x2c=0. It does not initialize the job's leading ID/progress fields.

## 4. Disconnect (00816af0, 451 B)

State zero returns true without doing anything, including without clearing +0xa15.
Otherwise, with async enabled, no work already marked, a target, and force=false,
set +0xa15, state=4, operation=101 (literally 101, not dispatcher disconnect 102),
invoke thread starter 00816ac0, and return true. Consequently this deferred route
dispatches login, which rejects state 4. The native operation constant is checked at
00816b15–00816b43; it must not be repaired to 102.

The direct/forced route sets state=3, sends `QUIT\r\n`, and reads one completed reply
only if send succeeded; reply/result are ignored. Close data, listener, control in that
order when open; set state=0; if a target exists send 0x57386939 with uint1=0, void1=0,
uint3=1, uint2=this, then Release the message. Return true regardless of transport result.
Clear +0xa15 only if force=false, after notification. Forced disconnect preserves it.
No target hold or release is performed here (00816b48–00816cb2).

## 5. Independent discriminating vectors

| Constructed input / schedule | Required result |
|---|---|
| State=0, empty user, target present | False login-result event; no socket/USER; work flag cleared |
| Deferred login, then dispatcher runs with no external mutation | Initial true; worker false event; state remains 4 |
| Successful greeting read returning class 5, USER=230, SYST=215 | Greeting class alone does not reject; successful login |
| USER=530 read successfully | QUIT/close; login result true, state=0 |
| USER=331, PASS send fails | QUIT/close; login result false |
| USER=230, SYST=500 read successfully | Disconnect-result event before login-result true event |
| Connect call false, socket open | Login false, state=1/stage=403, socket retained |
| SYST contains both UNIX and Windows_NT | Selector 314, irrespective of which text occurs first |
| SYST `unix`, existing selector 312 | Detector false; selector remains 312; login may succeed |
| Deferred listing with borrowed list, no external mutation | Initial true; worker false without list-result event; old list retained |
| Listing path `abc//` | LIST argument `abc/`, removing only one slash |
| Data socket fails after one complete accepted line; final reply=226 | Parse collected bytes; success true despite data failure |
| Buffer length=256; first count=100, second=100 | Second requested capacity is 100; no growth at total 200 |
| Existing output has nodes; listing has only rejected lines | Old nodes removed; empty output; parser returns true |
| Null list argument and successful acquisition | Callback may inspect temporary nodes; they are freed on return |
| Forced disconnect while work flag=true | Callback sees state=0 and closed sockets; work flag stays true |
| Deferred nonforced disconnect | Operation=101; worker takes login rejection, not QUIT |

## 6. Outer FTP worker (00816ee0, 1628 B)

The scheduler is a serial consumer, with queue +0x30, lock +0x40 and worker record
+0x48. A record has active byte +4, stop +5, cancel +6, signed suspend count +8 and
current stack-job pointer +0xc. Construct one local job value and address. On each loop,
check stop first. A positive suspend count sleeps for the literal 1000000 argument to
SleepCurrentThread, then rechecks stop. Otherwise lock, inspect the queue, and if empty
clear active, unlock and exit. Exit locks again, clears scheduler +0x48, frees the record,
unlocks, destroys the local strings and returns zero (00816ee0–00816f50,
008174bc–0081753b). There is no retry count or retry queue in this body.

For a nonempty queue, copy the front value via 00813cb0, unlink and destroy the queue
node and its two strings, publish the local value at record+0xc, record current 32-bit
time in job+0x30, and unlock. Parse job URL (+8). Only URL type 5 enters FTP handling;
other URL types retain the copied status and still take the final completion path.
For FTP, construct a fresh session (section 7), resolve the server: numeric text sets
the address directly, names require lookup result **exactly 1**. Lookup failure writes
job status=3 and destroys the fresh session. Use the low 16 bits of URL port, byte-swapped
into the address port (00816f93–008170c1). URL user/password are not extracted here:
the new session's anonymous/user credentials apply.

Initialize the session if needed; retain the scheduler as its target (AddRef new before
Release old); set timeout to the low 32 bits of scheduler+0x18 multiplied by 1000;
disable async; set representation +0x10=203; clear pending host; copy resolved address,
using stored default port only for zero address-port. Before login and again after a
true login return, require both record cancel and stop false. Login follows section 1,
including its true return after some rejected replies. This worker does not independently
check the resulting state (008170c6–008171ab).

Take URL absolute path, compute its leaf via 00812b10, substitute scheduler default
directory +0x1c only when the job destination string is empty, append the leaf, and call
RETR with the full absolute path and composed destination. Preserve RETR's returned
boolean separately from the session's state. If state is nonzero, disconnect: the
normally selected direct branch sends QUIT, optionally reads its reply, closes data,
listener, control, resets state, emits the disconnect message and clears work flag.
The alternate inline branch (async enabled, work flag false, target nonnull) sets
state=4/operation=101 and starts 00816ac0, matching section 4. Only a true RETR return
writes job status=0. Disconnect results do not replace that boolean
(008171ab–0081741f). Then shutdown/destroy the session (section 7).

Immediately before final notification, if cancel or stop is set, clear cancel and write
job status=7. If stop is set, skip notification; otherwise set job progress +0x34=100
and call 00812190 with the local job. This means cancellation still delivers a failed
final event, while a stop observed at this boundary suppresses it. Clear record+0xc,
destroy the parsed URL, and loop (0081746e–008174bc). Stop is checked at specific points,
not an atomic guarantee that no callback can race a later stop write.

### 6.1 Job and worker support

| Body | Contract |
|---|---|
| 00813cb0 (119 B) | Assignment copies ID/status, assigns the two strings unless self-assignment, and copies +0x30/+0x34/+0x38 (time/progress/custom). No target ownership. |
| 00812190 (122 B) | If scheduler target +0x10 exists, construct/AddRef standard message 0xc4eccd8c, uint2=scheduler, void1=(job status==0), void3=borrowed job; call target +0xc; Release message. No target retention; no safe allocation-failure fallback. The apparent missing argument in pseudocode is a real stack job argument at 008121d2. |
| 00817580 (123 B) | Lock +0x40; if record +0x48 absent allocate 16 bytes, set owner, active/stop/cancel=0, suspend=0, current=null. Always clear stop. If inactive set active, AddRef scheduler, start thread-wrapper +0x38 with 00817560 and record. Unlock. Does not inspect a thread-start return or undo active/hold on failure. |
| 00817560 (32 B) | Save record owner, call 00816ee0(owner,record), Release saved owner after worker returns; return worker result. Record may already have been freed. |
| 00816ac0 (44 B) | Construct local thread wrapper; start 00816990(this); destroy wrapper. It does not alter session state/operation or acquire a session hold. Wrapper destruction is not evidence of joining. |

## 7. Session construction and destruction

00812390 (825 B) establishes these defaults; it does not open sockets, start threads,
or retain a target. Numeric stores checked at 008123a3–008124dd, 00812577–008125c4,
0081262c–008126ba; credential bytes at 00abf59c/00abf594.

| Field(s), byte offsets | Initial value |
|---|---|
| +4/+8 | zero |
| listing selector +0xc | 310 (UNIX parser) |
| representation +0x10/+0x14 | 203 / 201 |
| byte +0x18, time +0x1c, count +0x24, target +0x28 | zero |
| timeout +0x20 | 15000 |
| strings +0x2c/+0x40 | `anonymous` / `user` |
| account +0x54, pending host +0x6c | empty, eight-byte capacity, auxiliary word zero |
| stored port word +0x68 | 0x1500 (network-order port 21 on this executable) |
| address +0x80 | constructor 0081e7d0(0,0) |
| control/listener/data +0x90/+0xf0/+0x150 | constructed stream sockets |
| initialized byte +0x1b0, +0x1b4, state +0x1b8 | zero |
| operation/login-stage/transfer-stage +0x1bc/+0x1c0/+0x1c4 | 100 / 400 / 500 |
| command +0x1c8, extra +0x1d4, path +0x1e0, reply +0x1f8 | empty eight-byte buffers |
| receive buffer +0x1ec | 2048 zero bytes, capacity 2049 including trailing NUL |
| +0xa04 | zero |
| +0xa08 | empty eight-byte buffer |
| async/work bytes +0xa14/+0xa15 | false / false |
| strings +0xa18/+0xa2c, their auxiliary words +0xa28/+0xa3c | empty / zero |
| pointer arguments +0xa40/+0xa44 | null |

00816cc0 (197 B), shutdown, acts only if initialized +0x1b0 is true. Clear initialized,
Release target then null it **before** forced disconnect if state nonzero. This suppresses
disconnect notification from shutdown. Then close control/listener/data if still open;
reset state=0, operation=100, login-stage=400, transfer-stage=500, +0x1b4=0, +0xa04=0.
It does not clear async/work flags or join dispatcher threads. The low-byte return is
true even if already uninitialized (00816cc4–00816d81). If already uninitialized, even
a nonnull target is not released by this entry.

00816db0 (296 B), destructor, installs the native table, calls shutdown, destroys buffers
in order +0xa2c, +0xa18, +0xa08, +0x1f8, +0x1ec, +0x1e0, +0x1d4, +0x1c8, then
socket objects data/listener/control, then host/account/password/user strings. This is
not a wait for outstanding async entry calls. 00817540 (30 B) calls this destructor and
frees storage only when deleting flag bit 0 is set, returning this. 00816d90 (23 B)
calls shutdown and then virtual deleting destructor +0xc with flag 1 if this is nonnull.
00816210 (30 B) is a different deleting wrapper: call scheduler destructor 00813e10,
conditionally free on bit 0, return this. The latter callee remains an independently
open checklist entry; the wrapper does not establish scheduler draining.

## 8. Further discriminating vectors

| Constructed input / schedule | Required result |
|---|---|
| Worker suspend=1, stop=false | Sleep argument 1000000; no dequeue in that iteration |
| Name lookup returns 2 | Outer FTP treats it as failure/status 3; login's own lookup accepts nonzero |
| FTP URL carries credentials different from defaults | Worker does not copy those credentials into the session |
| RETR false, then QUIT/read succeed | No success status overwrite from disconnect |
| Cancel true at final boundary, stop false | Clear cancel, status 7, progress 100, final failed callback |
| Stop true at final boundary | Status 7, clear cancel, no final callback |
| Callback retains borrowed job pointer | No validity after local job reuse/worker return is promised |
| Initialized session with target and open sockets enters shutdown | Target Release precedes QUIT; no target disconnect callback |
| Uninitialized session with target enters shutdown | No Release/reset work is performed |
| Dispatcher startup fails after 00817580 set active/AddRef | This body performs no rollback of active or acquired hold |

## 9. Remaining session commands (Q7 qualification, 2026-10-24)

All seven entries below require session state 2. Any other state returns false,
clears work byte +0xa15, and sends no completion message. With async +0xa14 set,
work clear and a nonnull target, they set work, state 4 and their operation ID,
save arguments, start dispatcher 00816990 using a local thread wrapper and return
true. Startup failure is not checked. The saved argument is a copied string at
+0xa18 except PWD, which stores its borrowed output pointer at +0xa40. Rename
copies both arguments successively to **the same** +0xa18 string, leaving the
second argument there; +0xa2c is untouched (`00815a9b–00815acf`). There is no
session retention or join in these entries. Dispatcher reentry still sees state 4
and is rejected by the state-2 guard; do not repair this by implicitly resetting
state before reentry.

The synchronous path sets state 4, operation as below, transfer-stage 504, and
records current time at +0x1c. It sends the command through 00812290, then, only
after a successful send, sets stage 507 and reads a reply through 00813d30.
Ordinary commands require reply **class 2**, not a particular three-digit code.
Send failure, reply-read failure or another reply class returns false. There is
no retry or disconnect here. Before notifying, both success and failure restore
state 2, operation 100, transfer-stage 500 and clear work. Thus the callback sees
the restored state. Entry guards and operation/stage stores were checked in the
instructions at each table entry through its synchronous prefix.

| Entry | Operation | Wire command(s) | Completion message |
|---|---:|---|---|
| `00814cc0` (657 B) | 108 | `PWD\r\n` | 0x57386931 |
| `00814f60` (579 B) | 109 | `CWD <path>\r\n` | 0x57386932 |
| `008151b0` (489 B) | 110 | `CDUP\r\n` | 0x57386933 |
| `008153a0` (579 B) | 112 | `MKD <path>\r\n` | 0x57386934 |
| `008155f0` (579 B) | 113 | `RMD <path>\r\n` | 0x57386935 |
| `00815840` (538 B) | 105 | `DELE <path>\r\n` | 0x57386936 |
| `00815a60` (717 B) | 106 | `RNFR <source>\r\n`, then `RNTO <destination>\r\n` | 0x57386937 |

CWD/MKD/RMD copy the argument into the session path and remove exactly one final
`/` or `\` before formatting. In particular `/` becomes empty and `a//` becomes
`a/`. They do not normalize internal separators or quote the path. The final-byte
read precedes any emptiness test: an initially empty argument is outside the
qualified input domain (`0081504c–0081506b`, `00815490–008154ab`, corresponding
RMD body). DELE copies and formats the argument without that suffix removal.
Null string arguments are unsafe in the synchronous path. Strings with embedded
NUL retain their copied lengths but formatted wire paths end at the first NUL.

Rename copies source into +0x1e0 and destination into +0x1d4. Only a successful
RNFR send/read with class 3 permits RNTO. Before RNTO it sets stage 505; before
the next read it sets 507. Success requires the RNTO send/read and class 2.
There is no rollback command when either phase fails (`00815b15–00815c88`).

PWD additionally extracts the substring between the first two double quotes
from the reply into +0x1f8 and copies that byte string into stored-directory
+0xa08. If the caller output pointer is nonnull, assign the converted string to
it; null output still permits success. The first quote index must be less than
unsigned 32-bit `(reply length - 2)`. There is no independent check that a second
quote was found before using its index for substring length. Qualify successful
extraction only for replies with a valid quoted substring; malformed missing-close
quotes are not a deterministic false-return contract. Any ordinary PWD failure
clears reply +0x1f8, leaves the caller output and stored-directory unchanged,
and notifies with empty reply text (`00814e15–00814e87`, `00814e8b–00814f50`).

With a target, completion uses a standard message, AddRef'd before synchronous
delivery and Released afterward. For all rows, uint1=0, void1=null,
uint3=success encoded as 0/1, uint2=session. PWD additionally provides the
converted extracted reply as void2, with uint4=0, through 008120e0.
Other rows do not supply a path/text payload. Target is borrowed for the call;
the entry does not AddRef/Release it. Allocation failure is unsafe.

### 9.1 Scheduler construction and admission

`00813d70` (154 B) is scheduler **construction**, despite its earlier description
as an initialization entry. It installs table 00abf64c; clears reference word +4,
initialized byte +8, ID +0xc, target +0x10, +0x14, worker +0x48; sets timeout +0x18
to 15; constructs an empty default-directory string +0x1c with eight-byte storage,
a circular empty 0x44-byte queue sentinel +0x30, thread wrapper +0x38, lock +0x40,
and allocated-or-null lookup helper +0x34. It does not connect, start a worker or
retain a target (`00813d76–00813e09`).

`00817600` (399 B) requires a valid nonnull URL string object. Empty URL returns
zero with no queue insertion. A nonempty URL constructs a default job, copies
the URL, sets status 1, zeroes time/progress and stores the custom word. A null
or empty destination uses the scheduler default directory unchanged; a supplied
nonempty destination is copied and receives a trailing backslash only when it
ends in neither slash kind. Admission does not append the URL leaf: the worker
does that later. Under lock +0x40, increment the 32-bit ID, copy the job into a
new tail node, unlock, activate through 00817580, destroy local strings and
return the ID. Wrap to zero is allowed and still queues the job. No URL parsing,
scheme check, target retention or completion callback occurs in admission.
Allocation failure is unsafe (`00817600–0081778e`, increment `0081770b–00817719`).

### 9.2 Proposed/unexecuted distinguishing fixtures

| ID | Setup | Required behavior |
|---|---|---|
| FTPC01 | Any of the seven commands, state 4 and work set | false, work cleared, state stays 4, no wire send or callback |
| FTPC02 | Async rename in state 2, distinct `old`/`new`, target present | true, state 4, operation 106; saved +0xa18=`new`, +0xa2c unchanged; eventual guarded reentry sends nothing |
| FTPC03 | Synchronous CWD/MKD/RMD with `a//` and reply class 2 | wire path `a/`; success; callback observes state 2/operation 100/stage 500 |
| FTPC04 | RNFR class 2 versus class 3, controlled sends/reads | first fails without RNTO; second proceeds to RNTO, whose class must be 2 |
| FTPC05 | PWD reply `257 "/a b" ready`, output initially `old` | success, reply/stored-directory/output become `/a b` |
| FTPC06 | PWD send false, output initially `old` | failure, reply empty, output remains `old`; no reply read |
| FTPC07 | Admission ID 0xffffffff, nonempty URL, destination `x` | tail job ID 0/status 1/destination `x\`; activation after unlock despite zero return |
| FTPC08 | Empty URL versus null/empty destination with valid URL | first no queue/activation; second uses default directory without admission-side suffix change |

## 10. Exact residual boundary / final-round disposition

Q7 stays partial for the scheduler controls, lifecycle and containers still listed O
in web-network-www-bodies.md. Section 9 qualifies nine former O bodies in this resumed
round; the earlier three-round PARK is historical. The checklist remains the exact address/byte
inventory; no whole-owner credit. HTTP completion/stop/resource findings are in
[web-network-http-lifetime.md](web-network-http-lifetime.md).

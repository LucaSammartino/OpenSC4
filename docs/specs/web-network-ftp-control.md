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
temporary empty list; a nonnull argument is used directly. **The synchronous path-copy
instructions fetch the second argument again, treating output-list+4 as a string buffer
descriptor, instead of fetching the first/path argument** (0081653d–0081656a; compare
0081646e for the async path copy and 0081649c for the borrowed list). A null list selects
the temporary list but subsequently dereferences null in this path-copy sequence. Ordinary
list storage is not guaranteed to provide readable string-descriptor fields there. Do not
invent a successful null-list path or silently replace this argument with the path. The
remaining acquisition contract describes executions with readable, compatible descriptor
storage. Copy that source into +0x1e0, remove **one** final slash or backslash, and prepare the active listener via 00811e70
then 00811f30. Reading the last path byte is unconditional: an initially empty path
reads before its start, so no deterministic safe empty-path contract is asserted
(0081656f–00816593). `/` becomes empty after the safe one-byte check.

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
list's nodes and sentinel **after** the callback; a supplied list survives. That sequence
does not make the earlier null-list dereference safe. The callback pointer does not confer
retention rights (008168dc–00816989).

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
| Readable synchronous source descriptor contains `abc//` | LIST argument `abc/`, removing only one slash |
| Data socket fails after one complete accepted line; final reply=226 | Parse collected bytes; success true despite data failure |
| Buffer length=256; first count=100, second=100 | Second requested capacity is 100; no growth at total 200 |
| Existing output has nodes; listing has only rejected lines | Old nodes removed; empty output; parser returns true |
| Null list argument on synchronous entry | Temporary list selected, then null descriptor dereference; no defined successful callback |
| Forced disconnect while work flag=true | Callback sees state=0 and closed sockets; work flag stays true |
| Deferred nonforced disconnect | Operation=101; worker takes login rejection, not QUIT |

## 6. Remaining boundary

Outer scheduler 00816ee0, initialization 00812390, other commands and their lifecycle
helpers remain separate checklist entries until proved. This companion does not infer
shutdown joining, cancellation suppression or retries from the direct operations above.

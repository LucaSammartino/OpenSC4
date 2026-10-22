# WWW-owned FTP bodies: native contracts

Companion to web-network.md §4.13, original Windows 1.1.641, 2026-10-22.
Addresses identify bodies, not invented public method names. This document specifies the
listing parser, RETR/STOR operations and reached transport helpers. Login, directory acquisition,
the outer FTP job scheduler and shutdown are still Q7; this is not whole-owner credit.
All examples below are constructed independent vectors, without contacting a server.

## 1. Listing entry parser (00812de0; 3607 B)

Arguments are a writable NUL-terminated line, output record, and numeric format selector.
Success returns low byte 1 and copies a name into the output RZString, writing kind=4
(directory) or 8 (file) at output+0x14. Failure returns low byte 0 without clearing the
previous output. Input mutation can occur even on failure. Nothing allocates an output list
node or calls a target here. Native string allocation failure is not converted into a false
return. The six-byte 00813bf7 body is an exception landing pad returning the cleanup address
00813b9e; it is not a separately callable listing operation.

Use these two precisely defined destructive helpers in the table:

- **First word:** replace the first ASCII space with NUL, if present; tabs are not delimiters.
- **Last field (00811fc0):** repeatedly remove trailing LF, CR, space or tab only while length
  exceeds two. Find the last ASCII space; only if there is no space anywhere find the last tab.
  From that delimiter skip consecutive spaces/tabs, or use line start when neither occurs.
  Thus a space earlier than a later tab still wins. This helper returns a borrowed pointer
  into the mutated input and never allocates or changes an output record.

The selector jump-table is at 00813c00; selector−301 is range-checked unsigned against 16
(00812e09–00812e1b). All marker and suffix matches below are case-sensitive except the
explicit `<dir>` prefix comparison for selector 302. Names are not URL-decoded or sanitized.

| Selector | Exact selection and name rule |
|---|---|
| 301, 306, 308 | Detect `<DIR>` or `<dir>` anywhere before mutation. Take first word. Without a marker accept a nonempty word as file. With a marker accept as directory unless name is exactly `.` or `..`; an empty directory name is not separately rejected. |
| 302 | Write NUL at byte 30, then take last field. Case-insensitive first-five-byte comparison of the mutated line with `<dir>` decides directory; all others are files. Reject `.`/`..` only for directories. |
| 303 | First take last field, then search the resulting input for ` DIR ` or `<dir>`. Marker means directory, otherwise file. Reject `.`/`..` only for directories. |
| 304 | Write NUL at byte 18; remove trailing spaces while length ≥2. Replace the first space with `.`; compact the following non-space suffix left over any intervening spaces. Accept the resulting full line as file only when that suffix begins with a byte other than NUL/space/tab. A line without a space is dereferenced through a null pointer; there is no rejection guard. |
| 305 | Reject any input containing `Disk size`. Otherwise write NUL at byte 13 and trim trailing spaces. Nonempty name ending in neither slash nor backslash is a file. With a trailing separator remove that one byte and accept as directory unless name is exactly `.` (not `..`). If no first-column name was accepted, repeat using bytes 41–53 inclusive, writing NUL at 54, but reject both `.` and `..` for this second directory name. |
| 311, 312 | Require first byte different from ASCII space and a semicolon somewhere. Temporarily NUL the first semicolon. If the prefix length exceeds four and ends in exact `.DIR`, remove that suffix and accept the prefix as directory. Otherwise restore the semicolon, cut at the first space and then first tab (if any), and accept the resulting name as file, retaining its version suffix. |
| 313 | Take first word. No trailing slash/backslash means file; otherwise remove exactly one trailing separator, reject exact `.` and `..`, and accept as directory. No empty-name guard precedes the last-byte read. |
| 314 | If byte 8 is `-`, recursively use selector 310. Otherwise only uppercase `<DIR>` marks a directory. Name is everything after the final space; when a marker exists the last-space search begins at that marker. No space means false. Reject `.`/`..` only for a directory. Filenames containing spaces lose their earlier words. |
| 315 | Prefix `d` followed by four spaces means directory; find that prefix and take the name 51 bytes after its start. Otherwise find five spaces and take the name 51 bytes after that start as file. These searches have no null check and names have no dot-entry filter. |
| 317 | Same marker/first-word/directory filtering as 301. The file branch instead reconstructs fixed 8.3 columns as described below. |
| Every other selector, including 307, 309, 310, 316 | Take last field first. A leading `d` or `l` enters the directory branch. Otherwise accept as file only for leading `-` or `f`, except selector 307 accepts any leading byte. In the directory branch remove one trailing slash/backslash from the field. For leading `l`, require substring ` -> ` in the whole line, truncate at its beginning, take last field again, and accept as directory without a dot-entry filter. Leading `d` accepts that field except exact `.`/`..`. Leading `l` without the arrow returns false. |

Selector 317 is not a safe DOS-name normalizer. At 00813056–00813075, bytes 0–7 that
are not spaces are copied to the **same indices** of an uninitialized stack array; spaces do
not compact the name or initialize holes. NUL is written after the last copied index (or at
zero if none). Bytes 9–11 are copied to a separate extension array; spaces write NUL at
their own positions and nonspaces set an extension-present flag. If the flag is set, a dot
and that array as a C string are appended (0081307a–008130fc). A full three-byte extension
has no explicit trailing-NUL store. The output can therefore depend on uninitialized stack
bytes or read beyond that array. Only input with fully initialized prefixes/terminators has
a reproducible name from this body. No deterministic garbage bytes, corrected padding
algorithm, or safe rejection of these unsafe cases is specified. Fixed-column branches
also assume sufficient writable backing storage beyond early NULs; they do not validate
the length before their indexed reads/writes. This is a native input-domain limitation,
not license to invent a missing parser branch.

**Distinguishing vectors.** Treat each line as a fresh mutable buffer and retain an old
output name/kind before testing a rejected line:

| Selector/input | Native result |
|---|---|
| 301: `alpha beta` | File `alpha`; space overwritten with NUL |
| 301: `. <DIR>` | False; old output preserved, input still truncated |
| 301: `.. no-marker` | File `..` |
| 303: `meta DIR alpha beta` | Directory `beta` |
| 310: `-rw metadata long name.txt` | File `name.txt` |
| 310: `? metadata name` versus 307 with the same input | False versus file `name` |
| 310: `lrwx metadata link -> dest` | Directory `link`, not `dest` |
| 310: `lrwx metadata link` | False |
| 311: `ABC.DIR;1 other` | Directory `ABC` |
| 311: `ABC.dir;1 other` | File `ABC.dir;1` |
| 313: `../ metadata` | False; 305 first-column `../` instead accepts directory `..` |
| 314: `size last two words.txt` | File `words.txt` |
| Last-field helper on `a b\tc` (actual tab) | Borrowed suffix `b\tc`, because a space exists |
| Last-field helper on `a ` | Suffix is empty; trailing space survives at length two |

The caller 00816230 iterates lines using 00812d60, clearing the old destination list first,
and appends accepted names/kinds in input order. Its complete container/ownership contract
remains Q7; it is not credited as complete solely by tracing this parser.

## 2. RETR/STOR entry, paths and command ordering

The 1878-byte body 00814560 downloads (RETR); 1741-byte 00813e90 uploads (STOR).
They operate on one mutable FTP client, not on WWW's active-job map. Shared state offsets:

| Offset | Use in these bodies |
|---|---|
| +0x10 / +0x14 | Requested/current transfer type |
| +0x1c / +0x20 / +0x24 | Start time / socket-wait argument / transferred-byte counter |
| +0x28 | Borrowed message target; zero disables notifications |
| +0xf0 / +0x150 | Listening and connected data socket objects |
| +0x1b8 / +0x1bc / +0x1c4 | Client state / operation / detailed stage |
| +0x1d4 / +0x1e0 | Local path / remote path, narrow length-bearing strings |
| +0x1ec..+0x1f0 | Data buffer range |
| +0xa14 / +0xa15 | Asynchronous preference / asynchronous operation already selected |

Both reject unless state=2: clear +0xa15 and return false, without terminal notification
or socket cleanup. If asynchronous preference is nonzero, +0xa15 is zero and target is
nonnull, set +0xa15=1, state=4, operation=104 for RETR or 103 for STOR. Copy the first
argument into +0xa18 but store the second argument's pointer, unretained, at +0xa40.
Create a temporary cRZThread, request entry 00816990, destroy the local thread wrapper,
and return true. The start call's result is ignored: true does not prove a thread started,
and this branch does not roll back state if thread creation fails (008145ee–0081469b,
00813f64–00813fca, start helper 009208cb). The caller must keep the second argument alive.
There is also no implicit reset to state 2 in dispatcher 00816990 or thread trampoline
0092083d. With no intervening state mutation, the worker's RETR/STOR call therefore takes
the state≠2 rejection above: clears +0xa15, leaves state 4, and emits no terminal message.
Do not silently repair this state mismatch. An outer scheduler's other mutations are Q7.

In the synchronous path, reached only after passing the state=2 gate, set state=4, operation as
above, stage=501, transferred=0 and start time=current Unix seconds. Path rules:

- RETR's first argument is remote path; copy it to +0x1e0. Without a local argument, local
  path is the remote basename plus extension. With a local argument, copy it verbatim;
  if its basename+extension is empty, append the remote basename+extension.
- STOR's first argument is local path; convert it to +0x1d4. Without a remote argument,
  remote path is its basename+extension; with one, copy that argument verbatim.
- Basename helper 00812b10 calls native Splitpath and concatenates filename and extension,
  discarding drive and directory. It is not slash/URL normalization (00812b10–00812b99).
  Null mandatory arguments are not validated and can fault; there is no false-return path
  for that misuse. RETR path destinations/temporary addresses were checked at 008146cd–008147ae.

Next ensure the listener (00811e70) then data socket (00811f30) exist. Failure follows the
normal terminal cleanup below. If requested and current type are unequal, issue TYPE first:
requested=200 means I for RETR, but STOR classifies the local file with 00811cd0. Types 201
and 202 select A and E; every other value selects I. Require a successfully read final
class-2 reply, then store the selected type as current. If requested=current, no TYPE or
auto-classification happens, even for value 200.

Obtain the listening socket address, request the port's byte order conversion, then send
`PORT h0,h1,h2,h3,p0,p1` with CRLF (stage 503). Require a final class-2 reply. Send RETR or
STOR plus a space and the stored remote path plus CRLF (stage 504). Read replies until a
line whose fourth byte is not `-`; require that reply's class=1. This accepts any 1xx code,
not just 125/150. Accept a data connection through listener vslot +0x100; both its return
and output boolean must be nonzero. Close the listener, then open the local file. There is
no PASV command, reconnect, retry count or backoff loop inside either transfer body.

## 3. Transfer loops, failure and callback/reentry

RETR opens with native file arguments (2,3,1), seeks to zero and truncates length to zero.
It waits using +0x20, reads up to buffer capacity, and writes each nonempty received block
to file. Stop on failed wait/read, zero received length, or failed Write. Only successful
writes advance +0x24 and the local total. A failed Write returns failure after closing the
file. A failed wait/read or zero receive **does not itself force failure**; the last successful
write/accept result remains live. Close file; unless Write failed, close data socket and
read a final reply, succeeding only for a successful final read with class=2. On Write
failure the final-reply read is skipped. The zero-block path can therefore still succeed
on a final class-2 reply (00814a17–00814bad; BL retention checked against instructions).

STOR opens with (1,2,1), seeks to zero, obtains file length, then ReadWithCount into the
same buffer. Each successful nonempty read calls 00811da0 and advances +0x24 and total by
the **read** count. The chunk sender's boolean is ignored. Failed ReadWithCount and EOF
both exit to data-socket close and final-reply read; a successful final class-2 reply makes
the transfer successful regardless of the ignored send result or local read failure
(00814385–00814454). The local file is closed on destruction before the function returns;
unlike RETR it is not explicitly closed before the terminal callback.

For either direction, when a target exists and signed total ≥ previous-report+16384,
send one progress message, then update previous-report to 16384 times total/16384 with
signed division truncated toward zero. All counters/additions are 32-bit wrapping;
the comparison is signed. A large block crossing several boundaries produces one message,
not one per boundary. RETR uses message 0x5738692f, cumulative rounded bytes and zero total;
STOR uses 0x5738692d and file length as total (00814b1e–00814b59; 008143d6–00814403).
No mandatory final progress event is emitted for a small file or a leftover partial block.

Normal terminal cleanup, including failures after entry, clears +0xa15, sets state=2,
operation=100 and stage=500, then closes listener and data socket if each reports open.
Only then, if target is nonnull, construct/ref a standard message and call target vslot
+0xc directly: terminal type 0x5738692e for RETR or 0x5738692c for STOR, numeric fields
+0x2c and +0x40 set to zero, +0x54 set to success boolean, pointer setter +0x30 given this
FTP client. Release the message after the call. No target AddRef/Release surrounds these
notifications. Reentrant callbacks see the reset state and closed sockets and may start
another operation; STOR's local file wrapper still exists during that callback. Each
ordinary path emits at most one terminal message. Allocation failure is not recovered
as a clean failure notification. Return the same low-byte success used in that message.

## 4. Individual reached helper contracts

| Body | Contract |
|---|---|
| 00811cd0 (199 B) | Set output type=203. Open local file (1,2,1); inspect min(file length,512) bytes. If open/read fails return false; otherwise output=201 initially, reverting to 203 if any byte is outside 0x20–0x7e after treating CR and LF as spaces. Tab is binary. Empty input classifies 201. Close/destroy file and return true. No notification. |
| 00811da0 (98 B) | Send positive input count in pieces of at most 512 through supplied socket vslot +0x10c, flags zero. Each call must return true and exactly the requested piece count; a short successful send returns false immediately, without retrying its remainder. Nonpositive count returns true without a call. Borrowed buffer/socket. |
| 00812290 (110 B) | State zero returns false. Otherwise send the command string through control socket +0x90, vslot +0x10c. Each iteration submits the same original pointer and full original length, adding the returned count to an unsigned accumulated count until accumulated ≥ original length. A short send therefore resends the prefix; true/zero can loop forever. Empty command returns true. Socket false returns false. |
| 00811e70 (138 B) | Existing-open listener succeeds immediately. Otherwise create it, get control socket's local address, replace its port with zero, bind, listen with backlog 5. Bind/listen failure closes the created listener and returns false; create failure returns false. This supplies active-mode PORT, not a passive connection. |
| 00811f30 (84 B) | Existing-open data socket succeeds immediately. Otherwise create, bind to address/port zero; bind failure closes it. Return success/failure without notifications or target references. |
| 00812ba0 (437 B) | Set output final=false. Consume buffered bytes through the next LF in the 2048-byte control buffer, or wait/read more with +0x20. Trim trailing CR/LF/space; temporarily NUL reply byte 3 and apply CRT atol to the first three bytes. Restore byte 3; return class=signed code/100 and final=(byte 3 != '-'). Shift unconsumed bytes left. A socket wait/read failure returns false. No matching of a multiline opener's numeric code; any line with byte 3 != '-' is final. Short/malformed lines lack a safe length guard. |
| 00813d30 (53 B) | Call 00812ba0 repeatedly until it fails or returns a final reply whose class is not 1. Return that call's AL boolean unchanged, despite the decompiler's void signature (00813d49–00813d62). Thus a final 1xx is skipped, and any final non-1xx (including error classes) ends the helper successfully; callers must inspect class. |
| 008120e0 (174 B) | No target: no message. Otherwise create/ref standard message; arguments 1,2,4,6 feed setters +0x14,+0x2c,+0x40,+0x54. Nonzero arguments 3,5,7 additionally feed +0x30,+0x44,+0x58 respectively. Directly call target +0xc, then Release message; no target retain/release. RETR/STOR progress passes this-client as argument 3. Pseudocode's inferred register parameter is wrong; stack offsets at 00812115–00812165 establish the seven arguments. |
| 00812d60 (123 B) | Unsigned cursor ≥ source byte length returns false without changing output/cursor. Otherwise copy the next LF-terminated slice, including LF when present (or remaining tail), append an explicit NUL byte to the length-bearing string, update cursor past copied bytes and return true. There is no trimming. The fold named cGZDate::Year in pseudocode is byte append, as 00812dc9–00812dcd confirms. |
| 00816990 (242 B) | Worker dispatcher reads operation once: 101→00815d30, 102→00816af0(0), 103→STOR, 104→RETR, 105→00815840, 106→00815a60, 108→00814cc0, 109→00814f60, 110→008151b0, 111→008163d0, 112→008153a0, 113→008155f0. String arguments come from +0xa18/+0xa2c, pointer argument from +0xa40 (111 uses +0xa44). Other values do nothing. Ignore callee return; always return zero; no own notification/refcount operations. |

## 5. Independent transfer and helper vectors

Use fake socket/file/message interfaces with deterministic responses. These vectors do not require
running the game or generating network traffic.

| Setup | Distinguishing result |
|---|---|
| State≠2 on RETR/STOR entry | False, async-selected cleared; no terminal message |
| Async preference, state=2, target present, thread start fails | Entry still true and state=4; no rollback notification |
| Async branch starts a worker; no intervening state changes | Dispatcher calls transfer in state 4; worker rejects, clears async-selected, no terminal notification |
| RETR remote `dir/a.bin`, no local path | Local `a.bin`; remote command retains `dir/a.bin` |
| RETR remote `dir/a.bin`, local directory `out/` | Local `out/a.bin` |
| Auto type 200 requested=current 200 | No TYPE and no file classifier |
| STOR classifier bytes `A\r\n` versus `A\tB` | 201 versus 203 (actual control bytes) |
| After PORT, completed reply 299 | Accepted class 2, regardless of exact code |
| RETR/STOR command reply `150-...` followed by `125 ...` | First line nonfinal; second final class 1 permits accept |
| Terminal reply `150 ...` followed by `226 ...` | 00813d30 skips the class-1 reply and returns class 2 |
| Sender: request 512, socket returns true/count 300 | 00811da0 false with no 212-byte retry |
| STOR read count 512, sender false, then EOF and final 226 | Transfer can still succeed; byte counter includes 512 |
| RETR first receive count zero, accepted connection, final 226 | Can succeed with truncated empty file |
| Total moves 0→40000 in one successful block | One progress event reporting 32768 |
| Total 16383→16384 | No event before boundary; one at equality |
| Command length 10, short sends 6 then 4 | 00812290 submits original 10-byte prefix both times, then succeeds |
| Final message target starts another transfer synchronously | New entry sees state 2 and closed data sockets; STOR still owns its local file wrapper |

## 6. Deliberately unresolved Q7 boundary

Login, listing acquisition, format selection and disconnect are now specified in
[web-network-ftp-control.md](web-network-ftp-control.md). Outer scheduling, initialization
and the other checklist gaps remain open as recorded there. No shutdown/drain or cancellation
suppression is inferred from RETR/STOR; unsafe fixed-column cases still have no invented output.

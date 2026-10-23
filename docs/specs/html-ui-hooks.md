# HTML UI hook and link dispatch

Windows SimCity 4 Deluxe 1.1.641.0. This specifies `cSC4UIHTMLHook` and
`cSC4HTMLLinkHandler::DoURL`, using the original executable's instructions and
vtable data. It describes owner decisions, including awkward results; it does
not supply a URL codec, database implementation, advisor implementation, or OS
link launcher. No native calls or links were executed for this publication.

## Outline

- §1: evidence and complete attribution ledger
- §2: interfaces, object state and lifetime
- §3: URL classification (`TestURL`)
- §4: resource document selection (`LoadDocument`)
- §5: resource read helper (`0078D2B0`)
- §6: link dispatch (`DoURL`) and consumer return convention
- §7: independent proposed traces
- §8: foreign boundaries and open questions

## 1. Evidence and attribution

The working original's SHA-256 is
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
It matches `ghidra/export/windows/export-summary.txt`, whose image base is
`00400000`; the original's file version is `1.1.641.0`. Evidence below is from
that export's byte-bearing instructions, original read-only PE data, and primary
call sites. Local decompilation was supporting navigation only: in particular,
its apparent argument counts, byte offsets and some constructor names are wrong.

The eight attributed hook entries total 904 bytes; DoURL adds 368, making
9 entries / 1272 bytes. These are attribution inputs, not a claim that every
byte implements independent behavior or that the whole owner is implemented.

| Attributed entry | Bytes | Proven role and publication status |
|---|---:|---|
| `006581F0` | 8 | Supported adjustment thunk: interface pointer minus 8, then Release. No invented independent method name. |
| `006626F0` | 5 | Supported forwarding tail to shared refcount Release, §2. |
| `0078CF80` | 49 | Supported QueryInterface decisions and adjustment; invalid-pointer domain remains partial. |
| `0078CFC0` | 97 | Supported Init service lookup, replacement order and result; foreign service lookup remains partial. |
| `0078D030` | 24 | Supported Shutdown clear-before-release and result. |
| `0078D0C0` | 114 | Supported TestURL scheme comparison; full textual URL parsing is foreign. |
| `0078D2B0` | 115 | Supported private resource read choreography, §5; original method name unknown. Resource acquisition/read operations are foreign. |
| `0078D330` | 492 | Supported LoadDocument selection, key stores, output routing and return; URL/string/hex/allocation primitives have the limits below. |
| `0078D140` | 368 | Supported DoURL selection, action no-dispatch path, advice call and exact result forwarding; advisor effects remain foreign. |

Additional **supporting** bodies discovered through vtable stores and callers:
`0078D050` (39 bytes) is a constructor-shaped body and `0078D080` (57 bytes)
is its deleting-destructor-shaped body. Neither is one of the eight attributed
entries. Their roles follow from their writes and the allocation caller, rather
than a fabricated symbol. Shared AddRef is `007637A0` → `0057EB00` →
`00968B60`; shared Release is `0090D96B`. Shared refcount construction,
destruction and non-deleting decrement are `0090D957`, `0090D964`, and
`0090D981`. These support lifetime facts, not additional owner coverage credit.

The temporary construction table `00AB5568` has seven pure-call entries;
it supplies no hidden hook behavior. The final hook table is `00AB5584`.
The refcount table is `00AB55A0`. Raw strings and service operations are typed
primitives, never a substitute provider for the hook's complete result.

## 2. Interfaces, object state and lifetime

Let **B** denote the allocation base, and **H = B + 8** the public hook
interface. The observed allocation size is 16 bytes (`004510EA`–`004510FA`).
The constructor calls refcount construction, sets temporary/final vtables,
and clears the service pointer. The refcount starts at zero, not one
(`0090D957`–`0090D963`, `0078D050`–`0078D076`).

| Base-relative field | Meaning established here |
|---|---|
| `B+0` | Refcount virtual table `00AB55A0` |
| `B+4` | 32-bit reference count |
| `B+8` | Hook virtual table `00AB5584` |
| `B+12` = `H+4` | Retained service interface, initially null |

| Hook slot (byte offset) | Contract |
|---|---|
| 0 (`+0`) | QueryInterface, two stack arguments: IID and output pointer |
| 1 (`+4`) | AddRef thunk subtracts 8 before shared AddRef |
| 2 (`+8`) | Release thunk subtracts 8 before `006626F0` |
| 3 (`+12`) | Init, no stack arguments |
| 4 (`+16`) | Shutdown, no stack arguments |
| 5 (`+20`) | TestURL, one URL-object argument |
| 6 (`+24`) | LoadDocument, URL object and mutable byte-string output |

The base table has deleting destruction, Release, AddRef, and RemoveRef at
offsets 0, 4, 8 and 12 respectively. Do not apply hook-interface slot numbering
to B (`00AB55A0`, `0078D080`, `006626F0`, `0057EB00`, `0090D981`).

### 2.1 QueryInterface and shared references

IID `1` and IID `0xC49B192C` both succeed, write **H** into the output, invoke
base AddRef through `B+0`'s slot `+8`, and return true in AL. They do not return
B for IID 1. Other IIDs return false and leave the output unchanged
(`0078CF80`–`0078CFAE`). This assumes a live, valid object and writable output;
the pointer-adjustment instructions are not null-object validation.

AddRef increments the 32-bit count and returns it (`00968B60`–`00968B66`).
Release uses an unsigned comparison: counts above 1 are decremented and returned;
0 or 1 invokes the base deleting destructor with flag 1 and returns zero
(`0090D96B`–`0090D980`). It does not clamp an already-zero count and keep the
object alive. RemoveRef decrements a nonzero count without destruction and
returns the resulting count (`0090D981`–`0090D98F`). No synchronization or
overflow guard appears in these shared bodies.

The application creation call site allocates 16 bytes, constructs B if allocation
succeeds, AddRefs it through the base table, passes H to a service registration
slot `+0x64`, then Releases its local base reference. On allocation failure it
passes null instead (`004510EA`–`00451127`). Registration's retention policy is
foreign; this caller alone does not guarantee the object survives registration.

### 2.2 Init and Shutdown

Init starts a temporary service pointer at null. If `RZGetFramework` returns
non-null, it calls framework slot `+0x14` with class ID `0x056B906E`, IID
`0x656B8EFC`, and the temporary output address. The boolean result of that lookup
is not tested; the resulting pointer is what controls replacement.

Compare that pointer with the retained pointer. If different: AddRef the new
pointer when non-null; store it at `H+4`; Release the old pointer when non-null.
If equal, skip those three operations. Finally release the temporary lookup
reference when non-null and return true in AL. A missing framework or null
lookup output therefore clears an old service and still returns true
(`0078CFCE`–`0078D020`; temporary cleanup `004495DF`–`004495EF`).

Shutdown: if retained service is non-null, clear `H+4` **before** invoking its
Release; otherwise make no service call. Always return true in AL
(`0078D030`–`0078D047`). Repeated Shutdown thus releases at most once unless
another call changes the state.

Destruction restores the final tables, releases a non-null retained service,
then runs base refcount destruction. Unlike Shutdown, it does **not** first
clear the service field. If the low bit of the destruction flag is set, it
then frees B through `GlobalHeapFree`; it returns B's address
(`0078D080`–`0078D0B8`). Reentrancy during that release is not specified as
equivalent to Shutdown. Allocation failures, thrown exceptions and calls on
destroyed objects are outside the supported live-object domain (§8).

## 3. URL classification: TestURL

TestURL uses the URL object's `GetScheme` result, compares it with literal
`sc4`, and returns that string interface's `IsEqual(literal, false)` result.
It does not inspect the resource label, path, host, command or retained service
(`0078D0CC`–`0078D10D`, literal at `00AB55B4`).

For the original cRZString primitive, false means **case-insensitive** equality:
length must match, both strings are lowercased and compared. This is equality,
not a prefix match (`0090EC6B`–`0090EC80`, `0090ECCB`–`0090ED29`). ASCII
`sc4`, `SC4`, and `sC4` classify true; empty, `sc`, `sc4x`, `http`, and `file`
classify false. These are literal **scheme-output** vectors. How raw input
text yields a scheme belongs to the foreign URL object, not this classifier.
Non-ASCII lowercasing/encoding is outside this publication.

Classification does not guarantee LoadDocument acceptance: LoadDocument checks
a case-sensitive full-URL prefix independently. No default-true fallback exists.

## 4. Resource document selection: LoadDocument

Input is a URL object and a mutable native byte-string output. LoadDocument
copies the foreign `GetFullURL` string, then requires its first six bytes to be
exactly `sc4://`. A short or mismatching prefix returns false before resource
access or output resizing. It does not call TestURL first
(`0078D338`–`0078D388`; exact-byte prefix primitive `0090E443`–`0090E477`).

After removing those six bytes, extract the first slash-separated nonempty
token into a temporary **label**. If no token is obtained, return false. The
remaining string is **R**. Initialize the three-dword resource key to
**(type `0x856DDBAC`, group 0, instance 0)**
(`0078D3B4`–`0078D429`). Type/group/instance terminology follows the published
[resource-key convention](legacy-control.md).

- If R contains another `/`, extract its first slash-separated token and send
  that token to `RZAtoUint32HexA`; store the 32-bit result as **group**. This
  branch ignores label spelling completely.
- Otherwise, lowercase label. Equal to `html` selects group `0x0A133FC0`;
  equal to `image` selects `0xCA133ECB`; other labels leave group zero.
- After either branch, if the remaining R has unsigned byte length at least
  8, pass its buffer to `RZAtoUint32HexA` and store the result as **instance**.
  If shorter, leave instance zero. There is no exact-eight requirement, no
  owner-side character validation, and no owner-side check of conversion success.

Evidence: group conversion/store `0078D42B`–`0078D46A`, mnemonic selection
`0078D46C`–`0078D4AB`, instance threshold/store `0078D4B3`–`0078D4CB`.
The group store at `0078D44E` occurs while a 4-byte cdecl argument is still
on the stack: its displayed stack offset must not be mistaken for the instance
store after stack restoration at `0078D4CB`.

Call the private helper §5 with B, the key, and the original output. Return its
AL result, preserving it across local string cleanup (`0078D4CF`–`0078D51B`).
There are no owner-side filesystem, HTTP, browser, script, window-opening or
command-execution branches. A slash is used as a resource separator, not
evidence of an OS pathname operation. No path normalization, percent decoding,
dot-segment removal, query removal or fragment removal is done by this body.
Any such transformation by `GetFullURL` remains a foreign question.

The string token primitive skips leading runs of the separator, extracts the
first nonempty token, and removes that token plus its following separator run
when separators are present. With no separator it copies the whole nonempty
source to the token **without clearing the source**. Empty/all-separator input
fails (`0090F39D`–`0090F42F`, `0090E4F1`, byte comparisons `0090E3F6`). Thus a
missing numeric path does not automatically reject a nonempty label. See §7.

## 5. Private resource reader: 0078D2B0

This body's sole direct caller is LoadDocument (`0078D4DC`), which adjusts
H by minus 8. It takes a key and mutable byte-string output. Its name is unknown;
the following is the observed choreography, not a proposed original symbol.

1. Read service at `B+12`, initialize local segment output to null, and call
   service slot `+0x80(key, &segment)`. A null service is **not** checked.
2. If returned AL is zero, return false. There is no output resize or segment
   Release on this branch, even if the foreign call populated the segment output.
3. On nonzero AL, initialize a local 32-bit size to zero and call segment slot
   `+0x70(key, null, &size)`. Replace local size with the full 32-bit EAX return.
4. If EAX is exactly `0xFFFFFFFF`, Release segment through slot `+8` and
   return false. No output resize occurs.
5. Otherwise resize the output byte string to EAX bytes, with fill byte zero.
   Then call segment slot `+0x70(key, output-buffer, &size)` again. Ignore its
   return, leave the output at the requested size, Release segment, and return
   true. A second-read failure is not converted into false, and a changed size
   output is not used to shrink or grow the string.

Evidence: `0078D2B1`–`0078D2ED`, `0078D2F0`–`0078D322`. Zero size is allowed
through the resize/read/true path; values other than exactly `0xFFFFFFFF` are
not rejected as negative. Excessive allocations are a partial domain, not
promised successful loads. A success flag with a null segment would lead to
unchecked dereference; it is not an owner-defined false result.

## 6. Link dispatch: DoURL

DoURL consumes a string interface and a second stack argument. It reads the
first argument through string slots `+0x1C` (length) and `+0x18` (bytes), copies
that byte range, and never reads the second argument. It does not use incoming
ECX as owner state (`0078D140`–`0078D193`; both returns pop 8 argument bytes).
Consumer callers supply zero as the second argument. It is not an output pointer.

For a valid byte string S with length N, apply these tests in order:

| Candidate | Exact owned gate | Behavior |
|---|---|---|
| action | N ≥ 6; the **first** exact lowercase occurrence of `action` starts at offset 6; N ≥ 12; byte at offset 12 is `/` | Erase the first 13 bytes; split a token at `?` into a temporary; destroy temporaries; return false. No service dispatch. |
| advice | If the action gate did not pass: N ≥ 6; the **first** exact lowercase occurrence of `advice` starts at offset 6; N ≥ 12; byte at offset 12 is `/` | Erase the first 13 bytes; call global city pointer `00B43CEC` slot `+0x144` (GetAdvisorSystem); call returned interface slot `+0x2C` (DoLink) with the entire remainder; preserve and return its AL. |
| neither | Any failed gates above | Return false, no advisor call. |

The substring primitive performs exact byte comparisons and returns the first
match or the end pointer (`004426C0`–`00442756`). DoURL verifies **no content
of the first six bytes**. It does not require `sc4://`, and does not extract
a scheme. In the normal valid string domain a length of exactly 12 has a
terminating zero at offset 12, so it fails the slash test; the body explicitly
tests N ≥ 12, not N ≥ 13 (`0078D188`–`0078D1C6`, `0078D22B`–`0078D25D`).

Action processing calls erase and SplitTokenSeparated, but there is no command
server invocation between that branch and its false return
(`0078D1C8`–`0078D228`). Do not invent command execution, success, a fallback
launcher, or a result derived from the discarded token. Advice dispatch does
not split `?` or validate its payload here. Null city/advisor pointers are not
checked (`0078D25F`–`0078D2AD`). Advisor behavior remains the existing
[advisors contract](advisors.md); it is not rederived here.

### 6.1 Consumer result reconciliation

The [View3D link row](view3d-window.md)
correctly records the inversion: it passes link `data2`, calls DoURL with zero
as the second argument, and returns 1 when AL is zero, 0 otherwise
(`007F412E`–`007F4150`). Here “DoURL failed” means **returned zero**; it does
not establish that all work failed or that the action branch executes a command.
For an action-looking link, DoURL returns false and View3D reports true.
For advice, the consumer inverts the advisor's returned byte.

The [advisor popup](advisors.md) and [UI window](ui-windows.md) descriptions
that follow the link then end the dialog remain consumer contracts; they do
not establish generic successful link handling. The handler's full dispatch
rules above supply that previously unspecified boundary.

## 7. Independent proposed traces

These are expected traces from instructions, **not executed tests or native
probe results**. String literals are ASCII. They assume valid live interfaces,
normal byte-string allocation and the stated foreign primitive results. For
hex examples, valid hex input is shown as the named conversion primitive's
ordinary result; malformed/overflow conversion is deliberately not inferred.

### 7.1 Classifier and full-URL selection

| Input at the stated foreign boundary | Expected owner decision |
|---|---|
| Scheme `sc4`, `SC4`, `sC4` | TestURL true |
| Scheme empty, `sc`, `sc4x`, `http`, `file` | TestURL false |
| Full URL empty, `sc4:/html/12345678`, `SC4://html/12345678` | LoadDocument false before resource lookup, output unchanged by owner |
| Scheme `SC4`, full URL `SC4://html/12345678` | Classifier true, loader false: distinct checks |
| Full URL `sc4://` or `sc4:////` | No label token; false, no resource lookup |
| `sc4://HTML/12345678` | Key (`856DDBAC`, `0A133FC0`, `12345678`), then helper result |
| `sc4://image/87654321` | Key (`856DDBAC`, `CA133ECB`, `87654321`), then helper result |
| `sc4://other/12345678` | Unknown label is allowed; key (`856DDBAC`, 0, `12345678`) |
| `sc4://html/1234567` | Instance length 7: key (`856DDBAC`, `0A133FC0`, 0); helper still called |
| `sc4://html` | Label extraction succeeds without erasing source; length 4: key (`856DDBAC`, `0A133FC0`, 0) |
| `sc4://anything/11223344/55667788` | Explicit group branch ignores label: key (`856DDBAC`, `11223344`, `55667788`) |
| `sc4://image/11223344/55` | Explicit group overrides mnemonic: key (`856DDBAC`, `11223344`, 0) |
| `sc4://html/12345678?x` | Owner passes all of `12345678?x` to hex primitive; no owner query removal; result conditional on foreign conversion |
| `sc4://html/../12345678` | Explicit group conversion receives `..`; no OS pathname operation; group result remains a foreign question |

Key numbers above are hexadecimal. These vectors do not assert that the database
contains any such resource. Unknown labels and zero-valued keys can reach it.

### 7.2 Link dispatch

| Literal S | Expected trace and result |
|---|---|
| empty, `action`, `sc4://advice`, `sc4://ADVICE/x` | false, no advisor call |
| `sc4://action/run?arg` | erase prefix, local token processing; false, no command call |
| `sc4://action/` | same action path with empty remainder; false |
| `sc4://advice/1234?x` | Advisor DoLink receives exactly `1234?x`; return its AL |
| `sc4://advice/` | Advisor DoLink receives empty string; no owner rejection |
| `xxxxxxadvice/payload` | Accepted advice position despite arbitrary first six bytes; DoLink receives `payload` |
| `adviceadvice/payload` | First `advice` occurs at offset 0; false even though another starts at 6 |
| `sc4://adviceXpayload` | byte 12 is `X`, not slash; false |
| `http://example` | false; no browser/HTTP fallback |

For `sc4://advice/1234?x`, a foreign DoLink return of 0 produces DoURL 0 and
View3D 1; a return of 1 produces DoURL 1 and View3D 0. These return traces
make no claim about the advisor's internal effects.

### 7.3 Resource failure and output traces

Assume an initial output `old`, valid retained service and a fixed key:

| Foreign primitive outcome | Owner trace |
|---|---|
| Service `+0x80` returns false | Return false; no resize, no segment Release; owner leaves `old` |
| Acquisition succeeds; size-call EAX = `FFFFFFFF` | Release segment; return false; owner leaves `old` |
| Acquisition succeeds; size-call EAX = 3; second read writes `ABC` | Resize to 3, read, Release segment, true; output `ABC` |
| Size-call writes 9 through its size pointer but returns EAX = 3 | Owner overwrites size with 3, resizes to 3, second read gets size initialized to 3 |
| Size-call EAX = 3; second read returns failure and writes no bytes | Resize to 3 (same-size prefix can preserve `old`), read, Release, **true**; no owner rollback |
| Size-call EAX = 0 | Resize to zero, still perform second read, Release, true |
| Size-call EAX = 3; second read changes size to 1 | No final resize to 1; output length remains 3; true |

The fill byte applies to newly added bytes, not to preserved bytes during a
same-size or shrinking resize. Data supplied by failed/short foreign reads
cannot be promised valid merely because the owner returns true.

### 7.4 Reference and lifecycle traces

- Fresh B: count 0, retained service null. Hook AddRef gives 1. QI for IID 1
  returns H and gives 2; rejected IID leaves a sentinel output unchanged and
  count 2. Release gives 1; Release then destroys and frees, returning zero.
- Init with old service A and newly looked-up B: AddRef B, store B, Release A,
  Release temporary B, return true. If lookup delivered one owning reference,
  the stored B retains one reference after temporary cleanup.
- Init with old A and lookup A: no replacement AddRef/store/Release; release
  temporary A, true. The lookup's owning-reference convention is a precondition.
- Init with old A and absent framework: store null, Release A, true.
- Shutdown with A: store null, Release A, true. A second Shutdown: true with
  no Release. Deleting destruction afterward sees null and frees B.
- Destruction with A without prior Shutdown: Release A while the field still
  contains A; base destruction; free if flag low bit is set. This order differs
  observably from Shutdown to a reentrant foreign Release callback.

## 8. Foreign boundaries and precise limits

Supported owner behavior includes selection, pointer adjustments, reference
order, key stores, thresholds, local mutation, call arguments, failure branches
and return bytes. No float/FPU arithmetic is involved. Counts, key fields,
resource-size returns and length differences here are 32-bit; gates explicitly
identified as unsigned retain that interpretation. Boolean returns are AL;
unused upper EAX bits are not promised zero.

Foreign primitives remain separately typed: framework/service lookup; resource
segment acquisition and two-phase read; URL GetScheme/GetFullURL; native string
copy/resize/token/lowercase/hex conversion; advisor retrieval and DoLink; heap
allocation/free. This publication does not turn their implementations into
hook-owned behavior. The literal owner traces above are independent conditional
expectations, not claims of execution or complete foreign conformance.

Open issues live in [questions/html-ui-hooks.md](questions/html-ui-hooks.md):
HU1 (URL/encoding/hex domains), HU2 (foreign service identity and read semantics),
HU3 (allocation, exception and invalid-memory domains), HU4 (registration and
reentrancy ownership). No original code, assembly listing or decompilation is
part of this spec; address citations identify evidence only.

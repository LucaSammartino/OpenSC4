# UI feedback owners

## 1. Scope and evidence

This is the first dedicated contract for `cSC4ToolTipManager` and
`cSC4ProgressScreen`. It supplements the identity-only inventory in
[ui-windows §14](ui-windows.md#14-ui-window-class-inventory) and the consumer
boundary in [view3d-window §4](view3d-window.md). Requirements below are *[compat]*.
No game execution, implementation comparison, or native run supports these rules.
All literal traces are **PROPOSED / UNEXECUTED**, derived independently from Windows
1.1.641 instruction branches and bit operations.

Primary evidence: canonical Windows export queried with `re_query.py func`, `asm
--bytes`, `callers`, and `vtable`. Local per-class decompilation was navigation
only; names and inferred pseudocode types are not behavioral proof. Address ranges
below identify the proof, rather than reproducing instructions. Normal completion
of foreign calls, valid receivers and valid supplied pointers are preconditions.
Allocation exceptions, nonreturn, arbitrary callback reentry and invalid pointers
remain unqualified. This document does not claim whole-class, whole-byte-count or
native coverage. Foreign window layout, rendering, timer implementation, resource
decoding and region algorithms remain outside its contracts.

The canonical PE read for literal verification has SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`,
image base `00400000`, machine `014C`, SizeOfImage `007C8000`. Raw PE words
agree with the tooltip table and progress vptr stores in the instructions.
The initial optional `pefile` helper failed to import; a read-only standard-library
PE-section reader then obtained these literals. No REA session was used or repaired.

## 2. Ownership and body ledger

`cSC4ToolTipManager` constructs its primary vptr as `00ABCB60` at `007E9305`.
Receiver offsets below are from that object, with no incoming adjustment. Its
six vtable entries are QI, shared AddRef, reference-release/teardown, DoMessage,
Init and Shutdown. The seventh attributed body is the static timer callback,
not a seventh virtual slot. `cGZWinToolTip`, `cGZWinToolTipMgr`, and
`cSC4WinCalloutBox` are different owners. The manager's `+0C` is the latter
object's primary interface; calls through its window interface use `+4`.

Progress construction is proved at the direct caller `007E5BDB`, targeting
`00776F90`, not by a progress factory name. Its constructor installs primary
vptr `00AB52D4` and a refcount subobject vptr at `+4` (`00AB52E4`).
Its five named bodies are not a five-slot vtable extent. No public ABI edit is
implied by these locally observed tables.
The tooltip constructor's sole exported direct call is `0044CB1C` in the
application initializer; this is a construction boundary, not a standalone
factory interface. No application algorithm is expanded here.

| Owner/body | Qualification in this publication | Anchor / normal return |
|---|---|---|
| Tooltip QI | Supported | `007E6030–007E6052`; AL bool; pops 8 bytes |
| Tooltip static callback | Supported, foreign message delivery | `007E6060–007E60AB`; no status contract |
| Tooltip Init | Partial: lifecycle and primitive detail below | `007E7C80–007E7E96`; AL true; pops 12 bytes |
| Tooltip Shutdown | Supported on ordinary returns | `007E7EA0–007E7F8B`; AL true |
| Tooltip constructor | Supported | `007E9300–007E9328`; EAX receiver |
| Tooltip release/teardown, named destructor | Supported | `007E97F0–007E9846`; EAX remaining count or zero |
| Tooltip DoMessage | Partial: raw string primitives retained | `007EFBD0–007EFE57`; AL bool; pops 4 bytes |
| Shared AddRef | Foreign/folded | `005F47F0`; separate shared body, not tooltip-specific algorithm |
| Progress constructor | Partial: construction detail below | `00776F90–0077709B` with alignment gap; EAX receiver, two stack words |
| Progress DoMessage | Partial: foreign resource boundary | `007770B0–00777219`; AL bool; pops 4 bytes |
| Progress repaint | Partial: raw drawing primitives | `00777220–007773A8`; no status contract |
| Progress Shutdown | Supported on ordinary returns | `00777760–007777F6`; no status contract |
| Progress Init | Partial: ordinary failure/initialization paths | `007778D0–00777B8E`; no status contract |
| Progress QI | Supported | `00776BF0–00776C22`; AL bool; pops 8 bytes |
| Progress update | Supported state/call schedule; foreign strings and timer | `00777640–007776FE`; two stack words, no status contract |
| Progress normal repaint | Partial, internal image helper unresolved | `007773B0–00777638` |
| Progress direct destruction | Supported schedule | `00777800–007778CC`; no status contract |
| Progress deleting destruction | Supported | `00777B90–00777BAD`, thunk `007770A0–007770A7`; EAX receiver; pops 4 bytes |
| Progress image helper | Unknown algorithm; not a foreign owner oracle | `00776C30–00776F49`, 794 bytes; called at `00777503` |
| Progress sequence shuffle | Partial library boundary | `00777700–0077775B`, 89 bytes; not included in owner completion |

AL-only returns do not establish a normalized 32-bit EAX boolean.
The seven tooltip names total 1658 bytes and five progress names total 1874
bytes, exactly the inventory's 12 names / 3532 bytes. Those sums omit unnamed
bodies, include the tooltip static callback, and are **metadata extent only**.
They are not a behavioral coverage denominator. Progress primary table has QI
at +00, shared AddRef at +04, shared Release at +08, DoMessage at +0C; the
adjacent refcount table begins at `00AB52E4` with the deleting-destructor thunk,
Release adjustment and AddRef adjustment. No extent beyond inspected words is asserted.

## 3. Tooltip manager state and event contract

### 3.1 State, identity and release

Construction zeroes the 32-bit refcount at `+04`, initialized byte `I` at `+08`,
armed byte `A` at `+09`, callout pointer `C` at `+0C`, resource pointers `R0/R1`
at `+10/+14`, then constructs the foreign callback timer at `+18`.
The constructor does not establish a current-target pointer.

QI accepts IDs `1` and `22C010CD`: write the unadjusted receiver to the supplied
output, call its AddRef, return AL=1. For any other ID return AL=0 without writing
the output. There is no accepted-ID null-output guard (`007E6030–007E6052`).

Slot 2 compares refcount **signed** with 1. Above 1, decrement it and return the
32-bit decremented count, without shutdown. Otherwise install the owner vptr,
call Shutdown, destroy the timer, release any still-nonnull R1, R0, C in that
order, free the receiver, return EAX=0. This is not evidence for a separately
callable nondeleting C++ destructor (`007E97F3–007E9846`).

### 3.2 Message dispatch and armed state

The incoming message is a raw structure: `u32 kind` at +0 and a pointer at +4.
For start/update kinds, a text-source pointer is read at **that pointer's +8**.
No target identity comparison, current-target AddRef/Release, query-interface
lookup or selected-control cache occurs in this body.

| Kind | Owned decision and call chronology | Return |
|---|---|---|
| `22C010D0`, `22C010D1` | If text-source pointer is null, do nothing. Otherwise perform text setup (§3.3), then choose immediate notification or timer start, then destroy temporary strings. | AL=1 |
| `22C010D2`, `22C010D8` | timer virtual +1C; C window-interface +118; C window-interface +170; only then A=0 | AL=1 |
| `CA56DD76` | If A=0, no effects. If A≠0, perform the point/window sequence below. | AL=1 |
| every other kind, including registered `22C010D5/D6` | no owned effects | AL=0 |

On the armed notification path, get window manager via `00913C46`, call its
virtual +84 with two `i32*` outputs `(x,y)`, copy the outputs unchanged into a
two-word point and pass it to C primary virtual +3C. Then C window-interface
virtual +5C, then +11C (AL result); only if AL=0 call its +114.
A stays set. No coordinate clamp, rectangle selection or timer arithmetic is
performed by this owner (`007EFDE2–007EFE4C`). Foreign primitive results do not
stand in for these owned branches.

### 3.3 Text branch (raw string boundary)

The owner constructs a string from the nonnull source and calls foreign
SplitToken with delimiter data `00A81388` (literal `|`); details of its character grammar
remain Partial. Call its returned/copied token `T` and the residual string `S`.
The rich branch requires the application byte at `00B43CD8`'s +F0A nonzero,
and both S and T nonempty (begin/end comparisons at `007EFC95–007EFCA7`).
It calls C primary +18(T), +20(S), +10(R0,8,8), +44 with four signed words
`(-28,-28,28,28)`, in order. The other branch calls +18 with a newly constructed
empty string, destroys that temporary, then +20 with T if T is nonempty,
otherwise S (both are string-object pointers),
+10(R1,5,3), +44 with `(-4,-4,20,20)` (`007EFCFC–007EFD6C`).
These are raw foreign setter calls; this contract does not compute their final
window geometry or drawn text.

Next foreign Find on T uses data `00ABCB9C` (literal `ESRB`), start=0, flag=1. If its signed
32-bit result is exactly -1: timer +1C, timer +14(0,0), then A=1. Any other result:
first A=1, then invoke the static callback with stack argument 0. There is no
same-text or same-target shortcut, and no cancellation before the immediate
branch (`007EFD6F–007EFDCD`).

### 3.4 Static callback

Get the message service (`00913B97`); if null, return without initializing the
static payload. Otherwise, on the first call whose static flag bit 0 is clear,
set bit 0 and initialize four words at `00B4E428` to
`(CA56DD76,0,0,0)`. Call service virtual +10(payload pointer,0). Neither this
callback's argument nor an owner pointer is used. Later calls reuse the same
payload rather than restoring its words (`007E6060–007E60AB`). No status test,
per-owner timer delivery or synchronous-delivery guarantee is established.

### 3.5 Init and Shutdown

Init consumes three stack words but reads none. If I≠0 it returns AL=1 without
effects. Otherwise get WM; null WM also returns AL=1, leaving I=0 and allowing
a retry. With WM present get MS. Nonnull MS receives +14(receiver,kind) in order
`CA56DD76,22C010D0,22C010D1,22C010D2,22C010D8,22C010D5,22C010D6`.
Statuses are not tested. Null MS skips registrations but continues.

Allocate 436 bytes and construct a foreign callout. Replace C with the same
identity-aware sequence as §4: AddRef new, store new, Release old, with nonnull
checks on both references. Then C+2C(`A85F1A83`), C+28(`E9C86C9D`), window+118.
Load R0 with raw key `(856DDBAC,46A006B0,14416190)`, options `(1,0)`; retain its
output by the same replacement sequence. C+10(R0,-1,-1); WM+0C→parent P;
C+0C→child W; P+38(W). Load/retain R1 with same type/group/options and instance
`14416192`. Timer +0C(750), +04(750), +24(callback `007E6060`,0).
Only then I=1; destroy temporary resource handles R1 then R0; return AL=1
(`007E7C80–007E7E96`). The 750 values are raw timer parameters, not an owner
elapsed-time formula or independent position threshold.

No graceful null-allocation outcome is proved: C can be stored null and the
following call dereferences it. Resource outputs may be null; setters are still
called. Normal-return false statuses from registration, attachment, setters or
timer configuration do not stop initialization.

Shutdown when I=0 returns AL=1 without cleanup. Otherwise: timer+1C; get MS;
if present unregister (+18) in order `CA56DD76,D0,D2,D8,D1,D5,D6` (D suffixes
retain prefix `22C010`). Get WM. **Only if WM nonnull**, if C exists call
WM+0C→P, C+0C→W, P+40(W), then clear C before Release. Null WM leaves C retained.
Clear/release R0, then R1, set I=0, return AL=1. A is not cleared
(`007E7EA0–007E7F8B`). Repeated Shutdown at I=0 does nothing even with residual
C; destruction handles that reference. Reentry safety is not asserted.

## 4. Progress lifecycle and state

### 4.1 Construction and caller boundary

The native caller allocates 192 bytes; on success supplies a rectangle pointer
from its window-interface +C0 and the caller's stack word (constructor reads
its low byte). It stores constructor return at caller+404, calls AddRef, then
Init (`007E5BB0–007E5C00`). Null allocation is stored and then dereferenced for
AddRef; no normal false/null return is proved. This boundary does not release
a previous +404 object. It does not specify surrounding View3D lifecycle.
Known consumer Shutdown call is `007F14C4`; existing View3D §4 owns its ordering.

Constructor zeroes +0C,+10,+14,+18,+1C and vector +20/+24/+28; constructs three
20-byte foreign strings at +2C,+40,+54; copies rectangle words to both +78 and
+88; zeroes +98; constructs timer +A0 with raw argument 4; sets J=0, stores
mode low byte at +B9. In the second rectangle only, overwrite +8C with input
word 1 +12, and +94 with input word 1 +100 (32-bit wrap). No rectangle
validation occurs (`00776F90–0077709B`). +68/+6C/+70/+74 are not initialized here.

QI accepts 1 and `452294AA`, writes unadjusted receiver, calls AddRef, returns
AL=1. Other IDs return AL=0 with unchanged output; no accepted-ID null-output
guard (`00776BF0–00776C22`). Shared primary AddRef/Release thunks add 4 to
receiver; counter is at owner+8, initialized zero. Shared Release compares
**unsigned**: count>1 decrements; otherwise invokes refcount-table destructor
with flag 1, returns EAX=0 (`005BE3F0`, `0090D96B–0090D980`). This differs from
tooltip's signed comparison.

### 4.2 Initialization prefix and failures

Progress uses initialized byte `J` at +B8, mode byte +B9, owned pointers at
+0C/+10/+14/+18/+1C/+98, a string vector at +20 and timer at +A0. Init is
guarded solely by J. If J≠0 it returns without effects. Otherwise **set J=1
before** registering receiver/kind `26D31EC0` through service `00B43CCC` virtual
+14. Obtain control objects by WCM virtual +14 with IDs `0A9C7973`, then
`4A9C7970`. For each, if new pointer differs from the stored pointer, AddRef new
if nonnull, store it, then Release old if nonnull. Store order is observable
to foreign callbacks (`007778D7–00777964`).

If either control pointer is null, return immediately. J **remains 1** and
registration and acquired references remain; a repeated Init does not retry.
If both exist, call first control +8C then second control +8C. For their raw
32-bit outputs h0,h1 and old word y at +8C, store +94 as
`(y + 2*h0 + h1 + 15) mod 2^32`. This is integer address arithmetic, with no
float conversion, clamp or signed-overflow assumption (`0077797A–0077799B`).

Clear old +0C before releasing it, then call service `00B43C9C` virtual +0C
with `&(+0C)`. Test **AL**, independently of the written pointer. AL=0 calls
Shutdown and returns. There is no rollback to constructor state. AL≠0 proceeds
to §4.3. AL≠0 with null output is subsequently dereferenced without a guard;
this is not equivalent to the AL=0 path.

### 4.3 Buffer, colors and string sequence

Buffer+0C receives `(rect[2]-rect[0],rect[3]-rect[1],1,0)` as four u32 words;
differences wrap. AL=0 calls Shutdown and returns. Otherwise buffer+78 returns
raw colors for triples below, storing each result before the next call:

| Mode low byte | Ordered channel triples → fields |
|---|---|
| zero | `(255,255,255)`→+68; `(138,150,190)`→+6C; `(118,129,163)`→+70; `(187,204,204)`→+74 |
| nonzero | `(255,0,0)`→+68; `(255,255,255)`→+74; +6C/+70 unchanged |

Load `(856DDBAC,6A1EED2C,4A2805FF)` with options `(1,0)`. Null output calls
Shutdown **before** destroying temporary handle, then returns. Otherwise retain
into +98 by identity-aware replacement, destroy temporary, construct temporary
string; LoadString(out,`4A5CB171`,`0A6DEB35`,0). For each AL≠0 append its
resulting string, increment instance modulo 2^32, try next. First AL=0 stops,
even if it wrote nonempty text. No failed resource is appended, no count cap
is present. Construct random provider seed -1; invoke shuffle `00777700` on
vector begin/end; timer Start, then SetElapsedTime(binary32 `447A0000`=1000.0);
destroy temporary string; return (`007779D6–00777B8E`). Shuffle permutation and
timer internals remain foreign/library Partial. This is not a percent scale.

### 4.4 Shutdown and direct destruction

If J=0 return without actions. Otherwise set J=0 **before** cleanup. If +0C
nonnull call its +10, reload it, then clear/release if still nonnull. Clear the
vector, Stop timer, clear/release +98, then +18, then +1C; unregister receiver/
kind `26D31EC0`. +10/+14 and the three strings remain retained
(`00777760–007777F6`). Repeated Shutdown at J=0 performs no unregister or other
cleanup. No usable return status is established.

Direct destructor `00777800` installs both owner vptrs; calls Shutdown; invokes
folded one-byte-return `00A6D837` on +A0 (its exported name is misleading; no
normal effects); releases residual +98. Destroy strings +54,+40,+2C; destroy
remaining vector elements forward with stride 20 and free vector storage if
nonnull. Release residual +1C,+18,+14,+10,+0C in order; finish refcount base
destructor. Residual releases do **not** null fields first. No implicit cleanup
is invented for Shutdown's J=0 branch (`00777800–007778CC`).

Deleting destructor `00777B90` always calls direct destructor, then frees owner
only if stack flag bit 0 is set; returns original receiver bits in EAX even
after free. Refcount-table thunk `007770A0` subtracts 4 before reaching it.
Direct destructor does not free owner; flag 2 also does not free it. This
return differs from shared Release's zero.

### 4.5 Resource-change message

DoMessage calls incoming object's +10→u32 kind. Anything except `26D31EC0`
returns AL=0. For that kind construct temporary string, call global `00B43CEC`
+28(&string), then clear/release +14 followed by +10. Call foreign `005DDA40`
with `(string*, &+10, &+14, 0, 0)`: solely a raw resource-read boundary with
independent AL and output writes. No region algorithm is specified.
AL≠0 skips fallback, even for null outputs. AL=0 takes fallback **without
clearing output writes again**.

Fallback obtains u32 a via global `00B43C94`+8C→object, object+48; obtains b via
global `00B43CEC`+15C. Offset=`8*(3*a+b) mod 2^32`. Read instance words at
`00AB52A4+offset` and +4. Load both in order, type/group `(856DDBAC,6A1EED2C)`,
options `(9,0)`. Nonnull first replaces +10, nonnull second replaces +14;
null loads preserve whatever the failed reader wrote. Destroy second temporary
then first, destroy string, return AL=1 regardless of reader/load failure.
There is no J guard (`007770B0–00777219`).

Inspected table pairs indexed by `3*a+b`: 0=`(6A231946,6A231947)`,
1=`(EA23195D,EA23195E)`, 2=`(0A2312D9,0A2312D8)`,
3=`(6A6CA89E,6A6CA89F)`, 4=`(6A6CA6DF,6A6CA6DE)`,
5=`(0A6CAB89,0A6CAB88)`. No bound check; only these rows are literal-qualified.
Do not infer selector domains or row-0 fallback.

### 4.6 Update: elapsed threshold and string consumption

Body `00777640` takes unadjusted owner receiver, u32 selector k and pointer p
to a foreign string object (not a percent or character pointer). Always query
timer→u32 e. Instructions signed-FILD the word, add binary32 2^32 if sign bit
set, then x87 compare against binary32 1000.0. For ordinary x87 precisions
(24/53/64 bits) and rounding modes this branch is exactly **unsigned** e≥1000.
999 and 1000 are exact; rounding of high values cannot cross 1000. No elapsed
subtraction or rollover correction occurs (`0077764D–00777677`). NaN cannot
enter via u32 FILD; abnormal floating environment/traps remain unqualified.

If `(e≥1000 OR current string +40 empty)` AND vector nonempty: reset timer to
positive binary32 zero; assign last element's character range to data subobject
+44 unless source data subobject is identical to +44; decrement vector end by
20; destroy popped element via +70(0). Reset precedes assignment/pop. Raw
foreign helper `00408A70` receives `(source begin,source end,scratch address)`
on receiver +44. String copy/allocation semantics remain foreign. Wrapped
vector end=20 would produce null source and dereference; valid storage excludes it.

Then k=0 calls string +2C virtual +34(p), then +40 virtual +58(0,-1), then +54
virtual +58(0,-1). k≠0 skips these three. Consumption precedes them and is
independent of k. Always invoke own repaint dispatcher `007773B0`, even with
J=0; no status return. Native boundary at `007EAC43` supplies k=1 and a
constructed empty string. No enclosing View3D lifecycle is specified.

### 4.7 Repaint and bounded residual

Obliteration repaint `00777220` returns if buffer +0C null. Otherwise graphics
object `00B43CA4` gets +30(four raw binary32 1.0 values), +3C(1), +2C(`4000`).
Query buffer+18(`8001`)→AL; only nonzero calls buffer+48(color +74,0), then
buffer+1C(`8001`). First control+DC(buffer)→AL: zero skips text/blit but still
calls graphics+AC. Nonzero loads text with raw key `(6A231EAA,CA81FF90,1)`,
obtains characters through foreign string length/character primitives, measures
first control+C0(chars)→i32 w. Let q=`(+90 - +88 - w) mod 2^32`, interpreted
signed. x=q/2 **truncated toward zero**, y=+8C unchanged. Call first control+CC
with `(buffer,x,y,chars,color +68,color +74,0,0)`, then +E0(buffer). Buffer+30()
returns raw source rectangle pointer; global `00B43CA0`+34(buffer,
that pointer,&rect +78); destroy temporary text; graphics+AC. No stored progress value
changes (`00777220–007773A8`). This is not width/2 minus textwidth/2.

Dispatcher `007773B0` first checks mode: nonzero delegates to obliteration and
returns. Zero mode calls global `00B43CA0`+30 even if +0C/+10/+14 null, then
returns early if any is null. Otherwise conditionally clears buffer for +18
(`8001`) as above; tests +18(`8010`): nonzero obtains +98/+0C rectangles,
calls foreign tiled blit, then buffer+1C(`8010`). It computes an image rectangle
from raw +10 rectangle and destination +80/+84 words, then calls **internal
helper `00776C30`** with rectangle, raw source rectangle and +10/+14 images.
That helper's composition algorithm is **Unknown**, not an allowed host
primitive. No complete normal-mode final rectangle/image oracle is published.
Subsequent three-string layout/drawing paths in this dispatcher remain Partial.
This does not weaken independently proved §4.6 state schedule. Implementers
must not fill these residuals from familiar progress UI behavior.

## 5. Typed raw foreign boundaries

Addresses/slots locate evidence; they are not new ABI exports. Pointer/u32/i32
width is 32; bool status is AL only. Ignored returns are unspecified. Each
foreign output is supplied individually, never a host-computed owner result.

| Primitive | Raw inputs / outputs |
|---|---|
| MS/WM/WCM getters | no stack input; nullable pointer in EAX; only expressly stated null guards apply |
| Message service | +14/+18(owner*,u32 kind), ignored status; +10(four-word message*,u32 zero), ignored status |
| Ref calls / replacement | AddRef()/Release()→u32; if different pointer: AddRef new if nonnull, store new, Release old if nonnull; if identical: no calls/store (`005447B0–005447DE`) |
| Callback timer +18 | +0C(u32 750), +04(u32 750), +24(callback*,u32 zero), +1C(), +14(u32 zero,u32 zero); returns ignored |
| Elapsed timer +A0 | construct(u32 4), Start(), Stop(), GetElapsedTime()→u32, SetElapsedTime(binary32); units/internal clock foreign |
| Callout primary | +0C()→window*; +10(resource*,i32,i32); +18(string*); +20(string*); +28(u32); +2C(u32); +3C(two-i32-point*); +44(four-i32-words*); setter returns ignored |
| Callout window C+4 | +118(), +170(), +5C(), +11C()→AL, +114(); no final visibility oracle |
| WM / parent | WM+0C()→parent*; WM+84(i32*x,i32*y); parent+38(window*), +40(window*); returns ignored except parent pointer |
| String primitives | construct/copy/destroy; SplitToken(output-string*,delimiter)→temporary string; Find(literal,start,flag)→i32; +34(string*), +58(u32 zero,i32 -1), +70(u32 zero); character ranges foreign |
| Resource handle | load(u32 type/group/instance,u32 option,u32 option)→pointer in handle; destroy handle; transient resource ref effects supplied independently of owned retention |
| Progress creation | service+0C(interface** out)→AL, independent output write; buffer+0C(u32 width,u32 height,u32 one,u32 zero)→AL |
| Buffer | +10(), +18(u32 flag)→AL, +1C(u32 flag), +48(u32 color,u32 zero), +78(u32,u32,u32)→u32 color, +30()→rectangle* |
| Control | WCM+14(u32 id)→control*; +8C()→u32 height; +DC(buffer*)→AL; +C0(chars*)→i32 width; +CC(buffer*,i32 x,i32 y,chars*,u32 color,u32 color,u32,u32); +E0(buffer*) |
| Progress text | LoadString(string*out,u32 group,u32 instance,u32 zero)→AL, independent output; append/clear/destroy and shuffle are library operations in owner-selected order |
| Resource-change reader | `005DDA40(string*,interface** image0,interface** image1,u32 zero,u32 zero)`→AL plus independent outputs/reference effects; region/data logic wholly excluded |
| Graphics | +30(binary32,binary32,binary32,binary32), +3C(u32), +2C(u32), +AC(); screen+34(buffer*,source-rectangle*,destination-rectangle*); returns ignored |

No foreign primitive computes owner gating, state mutation or final rectangle.
Internal helper `00776C30` and shuffle `00777700` keep their ledger qualifications.

## 6. Independent literal traces

All traces here are **PROPOSED / UNEXECUTED**. Symbols denote distinct valid
foreign objects unless equality is explicitly stated. Return values and output
writes of foreign calls are independent supplied inputs, not computed by a host
owner oracle.

| Trace | Literal inputs / initial state | Expected owned state, calls and result |
|---|---|---|
| T-null | A=1; kind=D0 (full ID `22C010D0`); nested text pointer=0 | no setter, timer or ref calls; A=1; AL=1 |
| T-unarmed | A=0; kind=`CA56DD76` | no WM lookup; A=0; AL=1 |
| T-point | A=1; notification; WM outputs x=-17,y=203; window +11C returns AL=0 | C+3C receives (-17,203), window+5C, window+11C, window+114; A=1; AL=1 |
| T-point-repeat | same event and state; +11C returns AL=7 | repeat point and +5C/+11C; omit +114; no A clear; AL=1 |
| T-clear | A=1; kind=`22C010D8`; primitive returns arbitrary normally | timer+1C, window+118, window+170, A=0; AL=1; repeating repeats all three calls |
| T-unknown | A=1; kind=`22C010D5` | no effects; AL=0 |
| T-QI-fail | id=2; output initially `12345678` | unchanged output; no AddRef; AL=0 |
| T-release | refcount=3, I=1 | refcount=2; EAX=2; no timer/resource/window action |
| P-missing | J=0; old controls null; first lookup→F, second→0 | J=1; register; AddRef(F), store +18=F; +1C=0; no height/buffer call; void return |
| P-missing-repeat | immediately Init again after P-missing | no lookup, registration or ref changes; J=1 |
| P-height-prefix | J=0; both controls valid; y=23,h0=7,h1=19; buffer call AL=0, output null | +94=71 before buffer call; calls Shutdown on failure; no boolean Init result |

### 6.1 Tooltip distinguishing sequences

Each text trace supplies the raw SplitToken results S/T and Find result. It
does not assume an unproved string grammar. The payload pointer itself must
be valid; a null nested source pointer is qualified, a null payload is not.

| Trace | Literal supplied inputs / state | Expected chronological effects |
|---|---|---|
| T-rich-new | I=1,A=0,R0=R,R1=Q; D0; nonnull source at address X; S=`body 73`,T=`head 9`; app flag=1; Find=-1 | C+18(T), +20(S), +10(R,8,8), +44(-28,-28,28,28); timer+1C; timer+14(0,0); A=1; temporary destruction; AL=1 |
| T-rich-same | repeat exactly T-rich-new with same X and same string results | same setters and timer stop/start again; no identity/cache shortcut; A=1; AL=1 |
| T-rich-other | next D1 with source at distinct Y; identical raw S/T | same effects; no old-target Release or new-target AddRef |
| T-plain-empty-token | A=0; D0; S=`left 83`,T=empty; app flag=1; Find=-1 | C+18(empty temporary), destroy empty; C+20(S), +10(Q,5,3), +44(-4,-4,20,20); timer+1C,+14(0,0); A=1; AL=1 |
| T-plain-flag | A=1; D1; S=`tail`,T=`ESRB`; app flag=0; Find=0; MS=null | empty header; +20(T), +10(Q,5,3), +44(-4,-4,20,20); A=1 before callback; callback MS lookup only; no timer stop/start and no static payload initialization; AL=1 |
| T-found-negative | S=`x`,T=`rating`; app flag=1; Find=-2; MS=M; static flag bit0 clear; delivery returns normal false | rich setup; A=1; static payload=(CA56DD76,0,0,0), flag bit0 set; M+10(payload,0); no rollback; AL=1 |
| T-init-no-WM | I=0,A=0; WM=null | WM lookup only; I=0; AL=1; retry performs lookup again |
| T-init-no-MS | I=0; WM=P; MS=null; C allocation/construct and resources normal; all ignored statuses return false | no registrations; complete setters/attachment/configuration; I=1; AL=1; repeat Init makes no calls |
| T-shutdown-no-WM | I=1,A=1,C=C0,R0=R,R1=Q; MS=M,WM=null | timer+1C; seven unregisters in §3.5 order; C=C0 retained; clear/release R then Q; I=0,A=1; AL=1 |
| T-shutdown-repeat | immediately repeat previous Shutdown | no foreign calls; C still C0; AL=1 |
| T-destroy-residual | after previous traces refcount=1 | Shutdown gate performs nothing; timer destruction; Release C0; free owner; EAX=0 |
| T-destroy-direct | refcount=0,I=1; WM=P,MS=M; all resources nonnull | full Shutdown, clearing C/R0/R1 before Releases; timer destruction; no second residual releases; free owner; EAX=0 |
| T-signed-ref | refcount bits=FFFFFFFF,I=0,all refs null | signed comparison selects teardown, not FFFFFFFE decrement; timer destruction, free; EAX=0 |

Clear/hide events do not consult I or a previous target. Normal false results
from timer/window calls in T-clear still lead to A=0. Any mutation prefix
before a nonreturn or reentrant call remains unqualified rather than rolled back.

### 6.2 Progress distinguishing sequences

For update traces choose mode=1 and +0C=null so owned repaint delegates and
returns without drawing. This isolates the state algorithm using real owned
guards, without a host-computed final image. Foreign string primitives perform
only the supplied normal character-copy/destruction effects. Queue brackets
show vector order (last element consumed first).

| Trace | Literal inputs / state | Expected chronological effects |
|---|---|---|
| P-construct | rectangle=(11,23,312,428), mode word=00000102 | first rect unchanged; second=(11,35,312,123); mode=2; J=0; refs/vector null; colors not initialized |
| P-wrap-construct | rectangle=(9,FFFFFFFC,35,71),mode=0 | second rect=(9,8,35,96); first rect unchanged |
| P-null-control-ref | J=0,+18=F,+1C=G; lookups 0,G | register, store +18=0 before Release(F); second pointer identical so no G ref calls; J=1; no heights/buffer |
| P-buffer-false-output | both controls valid, service AL=0 but writes B to +0C | J first becomes 1; controls retained; buffer output B preserved until Shutdown; J=0 before B+10; clear/release B; clear vector; Stop; release controls; unregister; no buffer-size call |
| P-size-fail | rect=(11,23,312,428); service AL=1 writes B; B+0C returns AL=0 | B+0C(301,405,1,0); Shutdown releases B and controls; no color/resource calls; void result |
| P-colors-mode0 | normal init mode=0; four color returns 11,22,33,44 | +68=11,+6C=22,+70=33,+74=44 in order; no percent/clamp computation |
| P-colors-mode2 | mode=2; old +6C=77,+70=88; two color returns 55,66 | +68=55,+74=66,+6C=77,+70=88; two calls only |
| P-resource-null | buffer allocation/size succeeds; mode=0 colors stored; image load returns null | Shutdown before temporary handle destruction; colors remain; no LoadString, shuffle or Start |
| P-text-stop | text loads IDs 0A6DEB35→AL=1,`Alpha`; 0A6DEB36→AL=7,`Beta 29`; 0A6DEB37→AL=0,`ignored` | append first two only; library shuffle supplied permutation [Beta 29,Alpha]; Start then SetElapsedTime(1000.0); J=1 |
| P-init-repeat | immediately call Init after successful P-text-stop | no registration/lookups/resources/timer calls; J=1 |
| P-time-999 | e=999,k=1,p=empty-string object; current=`old`,queue=[A,B] | no reset/pop; current unchanged; invoke repaint; queue unchanged |
| P-time-1000 | same but e=1000 | reset(0.0), copy B to current, vector end -=20 before destroy B; queue=[A]; invoke repaint |
| P-time-repeat | after P-time-1000, e=0,k=1,current=B | no pop; queue=[A]; repaint still invoked |
| P-time-repeated1000 | after P-time-1000, next supplied e=1000,k=1,current=B,queue=[A] | reset/pop A as well; queue=[]; identical selector/current does not suppress consumption |
| P-time-high | e=FFFFFFFF,k=1,current=B,queue=[A,C] | unsigned correction makes value ≥1000; reset/pop C; queue=[A]; not signed-negative early-out |
| P-time-rollover-input | next e=0,current=C,queue=[A] | no pop; owner has no previous-elapsed subtraction or rollover correction |
| P-current-empty | e=0,k=1,current empty,queue=[A,D] | reset/pop D even below threshold; queue=[A]; repaint |
| P-empty-queue | e=1000,current empty,queue=[] | no reset or pop; repaint |
| P-selector0 | e=1000,k=0,p=`Stage 17` string object; queue=[A,E] | reset/copy/pop E first; +2C+34(p), +40+58(0,-1), +54+58(0,-1); repaint; no numeric progress formula |
| P-event-unknown | message+10→26D31EC1; refs X,Y | AL=0; no releases/resource calls |
| P-reader-success-null | J=0; old +10=X,+14=Y; reader AL=3, writes null/null | clear/release Y then X before reader; no fallback; both null; AL=1 despite uninitialized owner |
| P-reader-fail-prefix | old +10=X,+14=Y; reader AL=0 writes U,V; a=1,b=2; first load→0,second→W | Release Y then X; row5 instances 0A6CAB89,0A6CAB88; +10=U preserved; AddRef W, store +14=W, Release V; destroy second then first handles; AL=1 |
| P-shutdown | J=1; B,+98=H,+18=F,+1C=G,+10=X,+14=Y; two queued strings | J=0 first; B+10, clear/release B; vector clear; Stop; clear/release H,F,G; unregister; X/Y and current strings retained |
| P-shutdown-repeat | immediately repeat previous Shutdown | no effects, X/Y still retained |
| P-paired-destruct | direct destructor after P-shutdown | Shutdown gate no effects; folded timer no-op; destroy strings +54,+40,+2C; empty vector storage freed if nonnull; Release Y then X; base destruction; no owner free |
| P-direct-J0 | J=0; +0C=B,+98=H,+18=F,+1C=G,+10=X,+14=Y; vector=[A,Btext] | no unregister/Stop/buffer+10; Release H; destroy three strings; destroy vector A then Btext/free storage; Release G,F,Y,X,B; base destruction |
| P-delete-flag2 | same valid direct-destruct state, deleting wrapper flag=2 | direct teardown; no owner free; EAX owner pointer |
| P-delete-flag1 | same valid direct-destruct state, wrapper flag=1 | direct teardown then owner free; EAX original pointer bits; shared Release caller instead returns zero |
| P-unsigned-ref | refcount bits=FFFFFFFF | shared Release decrements to FFFFFFFE and returns it; no destructor (contrasts T-signed-ref) |
| P-repaint-lockfail | mode=1; B nonnull; buffer flag8001 AL=0; first control+DC AL=0 | graphics setup; no fill/unlock, text load or screen blit; graphics+AC; no owner field changes |
| P-repaint-negative | mode=1; control lock AL=1; +88=11,+90=312,+8C=35; measured text width=304 | q=-3, x=-1,y=35; draw then +E0; screen blit receives source rectangle and owner destination rect separately; graphics+AC |
| P-repaint-zero-width | same but measured width=301 | q=0,x=0,y=35; no clamp or centering by separate half-widths |

Ref-count deltas in these traces exclude independently supplied resource-handle
and reader internal ref effects; owner AddRef/Release calls are explicit. No
null status/output equivalence, normal false-status rollback, restored window
state, nested Init scope, modal pump, monotonic progress or whole-render claim
is introduced.

## 7. Residual questions

See [the owner question file](questions/ui-feedback-owners.md). Supported rows
do not qualify external calls' algorithms, allocation failure, callback reentry
or all unnamed owner bodies. Numeric timing thresholds and percent formulas
must not be supplied by UI convention.

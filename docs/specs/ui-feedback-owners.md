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
| Progress destruction / other unnamed entries | Unknown pending attribution | Not inferred from the five names |

AL-only returns do not establish a normalized 32-bit EAX boolean.

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
SplitToken with delimiter data `00A81388`; details of its character grammar
remain Partial. Call its returned/copied token `T` and the residual string `S`.
The rich branch requires the application byte at `00B43CD8`'s +F0A nonzero,
and both S and T nonempty (begin/end comparisons at `007EFC95–007EFCA7`).
It calls C primary +18(T), +20(S), +10(R0,8,8), +44 with four signed words
`(-28,-28,28,28)`, in order. The other branch calls +18 with a newly constructed
empty string, destroys that temporary, then +20 with T's character pointer,
+10(R1,5,3), +44 with `(-4,-4,20,20)` (`007EFCFC–007EFD6C`).
These are raw foreign setter calls; this contract does not compute their final
window geometry or drawn text.

Next foreign Find on T uses data `00ABCB9C`, start=0, flag=1. If its signed
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

## 4. Progress initialization prefix and failure state

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
to the remaining initialization work, qualified below in subsequent sections.

## 5. Independent literal traces

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

## 6. Residual questions

See [the owner question file](questions/ui-feedback-owners.md). Supported rows
do not qualify external calls' algorithms, allocation failure, callback reentry
or all unnamed owner bodies. Numeric timing thresholds and percent formulas
must not be supplied by UI convention.

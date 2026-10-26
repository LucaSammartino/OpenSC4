# Network tool UI front end

## 1. Scope and evidence

This is the first dedicated behavior contract for `cSC4NetworkToolUI` in Windows
1.1.641. Requirements below are *[compat]* unless marked otherwise. It is a
bounded partial contract, not coverage of all 41 catalog entries / 8005 bytes.
The earlier [network-tools §8.2](network-tools.md#82-who-pays-csc4networktooluidoplacement-0x00645370)
is valid partial input. [City tools and input](city-tools-input.md) owns the base
view-input control and PlaceLot discussion, not this owner's input algorithm.

The owner-history audit found the broad catalog in `re-citytools4` and the
DoPlacement subsection published by `re-specs14b`; neither reports a dedicated
NetworkToolUI front-end attempt. Native Windows instructions govern this text;
automatic names and decompilation are navigation aids only. No native execution,
portable output, game run or shadow comparison supplies evidence here.

Backend solve, geometry, bridge/rules/type information, core-city algorithms,
PowerLine, Query, PlayerDrive, View3D and Widget behavior are outside this scope.
NetworkPreview is a separate foreign owner. A raw foreign call returns only its
own value/outputs; it never supplies the whole UI transition or placement outcome.

## 2. State and lifecycle

Offsets here identify observed state, not a proposed portable object layout.
Primary receiver means the start of the complete object.

| State | Observed storage |
|---|---|
| backend, sound, attached context | `+14`, `+18`, `+1C` pointer words |
| initialized, active, move-enabled, deferred-update flag | bytes `+24`, `+25`, `+26`, `+27` |
| saved terrain-view byte, mode bytes | `+29`, `+2A`, `+2B` |
| pick height / pick state | words `+2C`, `+30` |
| current cell, starting cell | pairs `+34/+38`, `+3C/+40` |
| anchor vector | begin/end/capacity `+44/+48/+4C`, 8 bytes per pair |
| preferred bridge, altitude state | `+64`, `+6C`, `+70` |
| selected type, auxiliary window | `+80`, `+130` |

Constructor `0x00645D50` installs primary vtable `0x00AA9140` and secondary
tables at receiver `+4`, `+8`, `+C`. It zeroes backend/sound, bytes
`24..27`, `29..2B`, words `2C/30`, the anchor vector, auxiliary window and the
resource/vector fields explicitly written there. It does **not** establish that
every intervening field is zero: in particular no default attached context,
current/start cell, type, bridge or altitude value is inferred. Allocation and
exception behavior are unknown; the ordinary constructor returns the receiver
in EAX (`0x00645E4E`). The initializer is specified below. The ordinary backend factory caller allocates `0x134` bytes,
constructs, AddRefs, calls the initializer below with `(backend, backend type)`,
and writes the UI pointer to its output (`0x00623B20`). It ignores initializer
AL. Its allocation-null branch subsequently dereferences null; it is not a
supported graceful allocation-failure path.

### 2.1 Initialization and ownership

The unnamed initializer `0x00645240` takes `(backend, type)` (pop 8), sets
initialized **first**, then compares the backend with the stored pointer. If
different: AddRef the incoming nonnull pointer, store it, Release the old nonnull
pointer. Equal pointers cause no reference calls. Store the raw type word at
+80. Apply the same compare/AddRef/store/Release sequence to the global sound
service and +18. Call incoming backend slot 48 with output/configuration storage
at +84; copy byte +A8 to +2A. Subscribe animation service slot 5 with
`(receiver+8, 0xFF)` and return AL=1 (`0x0064529E..2D5`). None of those primitive
results is checked, and no rollback prefix is present. Null backend is unsafe
at the unconditional slot 48 call. Repeated initialization repeats configuration
and subscription; there is no initialized guard and no implied idempotent
subscription policy. Context/anchors/active are not reset by this method.

Destructor `0x00645E90` reinstalls its four tables, performs the initialized
unsubscribe/abort/clear sequence, then releases a nonnull +130, destroys its
configuration/string/vector storage, releases sound, releases backend, and
destroys the shared refcount subobject. Shutdown alone does not release backend
or sound. Deleting wrapper `0x00646240` and adjusted wrapper `0x00645E80` are
distinct from Shutdown; allocator/exception details remain outside this contract.

Shutdown (`0x00645A90`) tests initialized. If set: call animation service slot 6
with `(receiver+8, 0xFF)`, abort placement, clear initialized. Either path returns
AL=1. Repeated shutdown therefore performs no second unsubscribe or abort.

AttachToView (`0x006457C0`, one word, callee pops 4) aborts using the **old**
context, then stores the new context without AddRef/Release. EnterUI and LeaveUI
(`0x00645310/340`) obtain backend slot 47's word; if nonzero pass it to view
manager slot 4/5 respectively. No bool result is established for these methods.
No conventional OS capture or drag threshold is implied by these operations.

## 3. Interfaces and return channels

QueryInterface (`0x00644F40`, two words, pop 8) accepts IID `1` and `0x8B54BE51`
at primary receiver, `0x22E85D8E` at receiver+4, and `0x452294AA` at receiver+8.
A hit writes that pointer and calls primary slot 1 before returning AL=1. A miss
returns AL=0 without writing the output. Upper EAX is unspecified in both cases
(`0x00644F69/92/9E/AE`). Valid receiver/output storage is a
precondition; the null-receiver branches do not establish a safe null-object API.
AddRef/Release (`0x008DC540/550`) adjust receiver by +C and tail-call the shared
refcount primitives. Their allocation/destruction semantics are not rederived.
IsActive (`0x00644FC0`) loads only AL from byte +25; upper EAX is not a result.
Primary slots 4/5/6/14 (`0x006452E0/2F0/300/7B0`) tail-forward to backend
slots 42/43/45/44 respectively (corrected 2026-10-25, re-nettools8: the jumps go
through `+0xA8`, `+0xAC`, `+0xB4`, `+0xB0`; slot 6 passes two output words,
network-tool-owners.md §6); their return channel is the foreign callee's.
The UI is not derived behaviorally from the base view-input implementation.

The complete exported primary table has these slots; pointer arguments and
event words are 32 bits. “No result” means this contract provides no return
value, even if a last foreign call happens to leave a register value.

| Primary slot | Entry | Input words / result |
|---|---|---|
| 0 | QueryInterface | IID, output pointer / AL |
| 1, 2 | AddRef, Release | none / shared refcount result |
| 3 | Shutdown | none / AL=1 |
| 4, 5, 6 | ViewInputControlId, CursorId, PenWidth | none / raw forwarded result |
| 7, 8 | EnterUI, LeaveUI | none / no result |
| 9, 10, 11 | OnMouseDownL, OnMouseUpL, OnMouseMove | screenX, screenY, raw event word / AL |
| 12 | CancelInput | none / no result |
| 13 | IsActive | none / AL |
| 14 | DirectionalCursorBase | none / raw forwarded result |
| 15 | LastDragDirection | none / EAX, partially specified numeric rule |
| 16 | AttachToView | context word / no result |

Secondary table `0x00AA9128` at primary+4 has QI `0x00645E60` (subtract 4),
AddRef/Release `0x008DCC00/10` (subtract 4 then primary ref thunk),
DoWinProcMessage, and shared DoWinMsg `0x0041D4C0`. That shared entry is a stub:
AL=0, pop 20, no owner state change. Table `0x00AA9118` at primary+8 has QI
`0x00645E70` (subtract 8), AddRef/Release `0x008DCC30/40` (subtract 8 then
primary thunk), DoMessage. The refcount table `0x00AA9104` at primary+C begins
with deleting thunk `0x00645E80` (subtract C), Release `0x008DCC60`, AddRef
`0x008DCC70` (both subtract C then primary thunk); do not mistake it for a QI
table. Deleting wrapper `0x00646240` takes one word, always destructs, frees
through GlobalHeapFree only when its low flag byte has bit0 set, returns the
original primary pointer in EAX, pop 4. No additional exported ABI is proposed.

## 4. Press, cancel and cleanup chronology

OnMouseDownL (`0x00645F50`, three words, pop 12) returns AL=0 without an attached
context. With one it returns AL=1, even when the pick fails. An active press first
aborts the previous placement. It queries raw key `0x10`, clears +2A, and sets
+2B to zero when the key is down, one otherwise. It calls ScreenToCell with
the first two event words and its output buffers, with the four trailing option
words zero. Failed picking does not start a placement.

On successful picking (`0x00645FB9..006460EB`), it sets active and move-enabled,
bridge word `0xFFFFFFFF`, altitude `-1.0f`, other altitude state `0.0f`; increments
city UI lock (slot 106), then calls backend slot 11. A configured sound is optional.
It saves the picked cell as current and start, calls terrain-view slot 10 with
zero and saves AL in +29, clears the anchor vector, appends two copies of the
picked cell, calls backend SetAnchors, records clock, empties the UI string,
sets current cell to `(-1,-1)`, then invokes its own mouse-move slot with the
original three words. Thus the first update is not skipped as a same-cell move.

CancelInput (`0x006457A0`) tail-jumps to AbortPlacement. Abort (`0x00645570`)
does nothing if inactive. If active it clears bytes +25/+26/+27 **before** calls.
If an auxiliary window exists, obtain context slot 3 and window slot 3, call the
former result's slot 16 with the latter result; reread +130, clear it before
Release if still nonzero. Then backend calls are Reset (5), slot 16, slot 17;
terrain-view slot 10 receives saved +29, then terrain-view slot 9 is called.
When context is nonnull, call context slot 44 with zero, then slot 41 with
`0xE9C87F80`. Finally tail-call city UIDecreaseLockCount (107). No refund or
anchor-vector clear occurs in this body. An inactive repeat does none of this.

## 5. Placement and payment

DoPlacement (`0x00645370`) is inactive-gated. Its active path removes/releases
the auxiliary window as above, then calls backend GetCost (20). The exact
32-bit word is the low half of a 64-bit withdrawal, with high half zero
(`0x006453CD..3D5`). It tests withdrawal AL, calls context result-sound slot 44
with that truth value in the low byte, then clears active. Upper bytes of that
sound argument are not normalized by this body (`0x006453DF..3F1`).

Withdrawal false skips both placement passes. Withdrawal true performs:

1. Reset; preferred bridge slot 32 with +64; altitude override slot 34 with +70;
   SetAnchors slot 22 with the UI vector; PlaceAllSegments slot 19 with
   `(false, zero-extended byte +2B, true, type +80)`.
2. GetPossibleNeighborConnection slot 41 with an output buffer. Only if its AL
   is nonzero, call GetFailure slot 14 and test **AL**, not the whole EAX word.
   Only if that byte is zero call neighbor slot 5 with the four output words.
   A nonzero callback AL sets the real-pass neighbor flag low byte to one;
   otherwise it remains zero.
3. Repeat Reset/bridge/altitude/anchors, then PlaceAllSegments with
   `(true, zero-extended byte +2B, neighbor flag, type)`. Neither placement-pass
   result is tested. The neighbor flag's upper bytes are not initialized by
   this routine; the established flag contract is its low byte.
4. If sound exists and the type-indexed result-sound entry is nonzero, call sound
   slot 11 with `(1, soundID, 0, 0, 0)`, regardless of real-pass success.

Both paid and unpaid paths then call backend **17 then 16**, terrain-view slot
10 with saved +29, context slot 41 with `0xE9C87F80`; the paid path alone calls
QueryUserForPotentialIntersections (49). Finally Reset and city decrease-lock
107 (`0x00645506..55A`). There is no refund, real-pass success branch, clearing
of +26/+27, or canonical overall success return. Treat this method as having no
specified return value; do not derive success from residual EAX/AL.

The earlier §8.2 economics remains true partial input. Its shorthand “not
GetFailureState” is qualified here by the actual low-byte test; neither a
negative cost nor raw `0xFFFFFFFF` forces withdrawal false in the UI: that
decision belongs to the raw budget primitive. No backend/preview algorithm is
claimed by this orchestration.

## 6. Picking and conversion

ScreenToCell (`0x006457E0`, pop 32, AL) takes eight words:
`(screenX, screenY, cellOut, optionalHeightOut, constrainByte, fallbackHeight,
lowerHeight, upperHeight)`. Float arguments occupy one word. Without context it
returns false without writing either output. It first asks backend slot 46 for
a raw pick option, then context slot 36 for a float32 world `(x,y,z)` result.
The raw pick/intersection queries are foreign; the following decision and
conversion are owned by the UI:

- Pick true, constrain false: use that point.
- Pick true, constrain true: on ordinary ordered finite values, if `y < lower`,
  intersect the screen ray with that lower plane; if `y > upper`, intersect
  with the upper plane; otherwise keep the pick. Bounds are not reordered.
- Pick false, constrain true: intersect with fallbackHeight.
- Pick false, constrain false, or a required plane intersection false: AL=0,
  no output-cell/height stores. The plane helper `0x00644FD0` is treated only as
  a raw intersection primitive here, not as a UI-state oracle.

For a usable point obtain terrain cell scale (slot 6, x87 result rounded into
float32 storage), width (slot 7), depth (slot 8), in that order. Form `1.0f/scale`
in x87, multiply by world x, call `_ftol2`, retain low EAX; multiply the retained
reciprocal by world z, call `_ftol2`, retain low EAX (`0x0064590E..92B`).
Each low word is compared as signed: negative becomes zero, then values at
least the signed dimension become dimension minus one. Store z to cellOut+4,
then x to cellOut; only afterward copy world y to a nonnull height output.
Return AL=1. This order even permits dimension zero to yield -1; no general
modern “clamp to valid city” rule is asserted.

Supported numeric domain: finite coordinates, positive finite scale, positive
signed dimensions, and conversions representable in signed 32 bits; `_ftol2`
truncates toward zero on that domain. Reciprocal/multiply use ambient x87
precision and rounding, not an asserted exact f64 calculation. NaNs, infinities,
overflow/high-word conversion, output aliasing, uninitialized query outputs,
invalid pointers and exception paths remain Partial/Unknown. Constants are
literal float32 `1.0` at `0x00A81228` and `50.0` at `0x00A84D24`.

## 7. Draft update and anchors

### 7.1 TestPlacement

`0x00645640` takes one word (pop 4); that word is passed to backend slot 21.
First slot 45 writes two raw words; only the first, interpreted signed, is
compared with 1. Above 1, preserve the deltas current-minus-old-first-anchor,
replace the first anchor with the starting cell. If exactly two anchors, lower
its x by one if x is nonzero and `2*deltaX >= abs(deltaZ)` signed; lower z by
one if z is nonzero and `2*deltaZ >= abs(deltaX)` signed. Deltas here are **not**
both absolutized. This rule is supported when the signed arithmetic does not
overflow; native wrap/INT_MIN edge cases are not silently widened.

Copy penultimate anchor and current cell into two local pairs, then Reset,
preferred bridge, altitude override, slot 21(argument), slot 13, slot 18
`(&penultimateCopy, &currentCopy, zero-extended +2B, type)`. Slot 18 AL true
causes the possibly modified current copy to overwrite the final UI anchor,
SetAnchors, then PlaceAllSegments `(false, +2B, false, type)`; slot 18 false
skips those three operations. Always call slot 21(1), GetFailure, and return
EAX=1 exactly when GetFailure **AL** is zero, else EAX=0 (`0x0064578F..79E`).
There is no count check before reading penultimate anchor: at least two valid
anchors and valid primitive output storage are supported-path preconditions.

### 7.2 Preview orchestration

UpdatePreview (`0x00646710`) first calls TestPlacement(1). It obtains failure
text through backend slot 15, compares it with the saved UI text, and only on a
change copies it and calls context slot 40 with message `0xE9C87F80` and two
zero words (`0x00646762..7A8`). The string primitives remain foreign.

On TestPlacement true: GetCost, context slot 43(cost), budget slot 6 returning
EDX:EAX. Affordability is false for negative signed high word, true for positive
high word, and otherwise unsigned low-budget-word >= raw-cost-word
(`0x006467EE..818`). Format money with `(cost, 0, textOut)`. This display path
does **not** withdraw. On TestPlacement false: context result-sound slot 44(0),
backend slot 24(0), and empty money text. If window and child `0x3003` exist,
send that child the text (slot 74). Dispose temporary strings, return AL equal
to test success **and** affordability. Preview geometry is foreign and does not
replace these UI decisions.

### 7.3 Mouse movement

OnMouseMove (`0x00646AC0`, pop 12) requires context and +26 nonzero, else AL=0.
It calls ScreenToCell with constraint=true, fallback raw word +30, height output
+30, and lower/upper formed from float32 +2C minus/plus `50.0f` with separate
float32 argument stores. Pick false or a cell equal to saved current causes no
preview/anchor/sound/window update and returns AL=1. Changed cell is saved
**before** UpdatePreview. Preserve the previous final anchor for later rollback.

When +2A is zero, skip all extra-anchor operations below. This is the mode
explicitly established by a press; no distance threshold is tested. Optional
sound uses the type-indexed success table `0x00AA9014` for preview AL true,
failure table `0x00AA907C` otherwise; only nonzero entries are played with
`(1, id, 0, 0, 0)`. Then window manager slot 19 is called; AL=1 is returned.

When +2A is nonzero, preview false first restores the first anchor to start and
the final anchor to its saved old value, SetAnchors, draft placement
`(false,+2B,false,type)`, slot 18 with local old-final/current pairs, slot 24(0).
Regardless of preview success, with more than two anchors and current different
from penultimate, search earlier segments through index count-3 in ascending
order. On first LiesOnSegment hit keep anchors through that segment's start,
append current, SetAnchors (`0x00646BFC..C74`). The exact numerical hit rule is
Partial (NUI1); this subpath is not a fully specified input outcome.

After that, compare raw unsigned clock with saved clock + configured +D4 word
(32-bit addition). At or beyond it call UpdateAnchorPoint(final-anchor pointer);
otherwise, if clock < saved clock, reset saved clock to zero. No millis/seconds
unit or standard drag-delay value is inferred (`0x00646C77..CA6`).

### 7.4 Anchor commitment

UpdateAnchorPoint (`0x006460F0`, one pointer word, pop 4) returns without calls
if last two anchors equal. Otherwise get backend slot 25 word n, zoom/rotation
outputs, compare n unsigned against the configured threshold at +AC[zoom].
Below threshold: stop. Otherwise slot 26's signed result j must be outside
1..3, and signed `(n-j)` must also be outside 1..3. Require slot 28(n-1) AL true.
Copy the input pair; get slot 27, and if signed n is less than that result let
slot 29 modify the copied pair. Drop the previous final anchor. With more than
one left, the local direction helper may drop one more (NUI1). Append the copied
pair **twice**, save clock, SetAnchors (`0x006461B7..6230`). No whole backend
anchor-update primitive substitutes for these checks/stores.

## 8. Release, selection and messages

OnMouseUpL (`0x00646D00`, three words, pop 12) requires context and +26, else
AL=0. It clears +26 **before** GetFailure. Nonzero failure AL plays optional
sound `(1,0xAA62C9D5,0,0,0)`, aborts, returns AL=1. Otherwise empty bridge-entry
vector +74, then ask backend slot 30 for a candidate-word vector. AL false calls
DoPlacement directly; AL true takes the dialog branch, not immediate placement.

The dialog branch releases old +130, then requests script window with IDs
`0xEBD0D36C/0x0C525B9E/0x5386D516`, context slot 3, output +130
(`0x00646D82..DBD`). Creation false reaches local-vector disposal and AL=1,
**without abort/decrease-lock/placement**. It leaves active set and move-enabled
clear. On success it attaches receiver+4 as window procedure, aligns window,
stores backend slot 39 AL in +28, and conditionally touches children
`0x3000/0x3004`. Candidate enumeration/AddBridgeEntry and detailed dialog layout
are Partial (NUI2); they are not a NetworkPreview algorithm. The inspected
candidate loop sets preferred word to each candidate, altitude=-1.0f,
TestPlacement(0), and only on success obtains caption/height/cost and calls
AddBridgeEntry (`0x00646EA0..F21`). At the end force preferred=`0xFFFFFFFF`,
activate `0xA1000`, dispose temporary candidate storage and return AL=1.

OnButtonActivated (`0x006468B0`, one word, pop 4; no specified result) handles:

| Raw value | Owned transition |
|---|---|
| `0x2000` | detach/release auxiliary window, then DoPlacement |
| `0x2001` | AbortPlacement |
| Other | first dereference window/root, then unsigned index=`value-0xA1000`; if outside bridge-entry count stop |

A valid index first enumerates controls `0xA1000+i` with interface `0x8810`
until query AL false. Release the previous returned control before each query,
set each successful control's slot 9 low byte to `(i==index)`; upper bytes of
that argument are not normalized. Compare selected entry's first word with
preferred as **raw integer bits**, not a float comparison (`0x00646954..965`).
Equal: no altitude/text/preview update. Different: store preferred, load entry
float height into +68, add float +6C and store +70; if child `0x3002` exists,
obtain backend slot 35 caption and send it to child slot 74. Then UpdatePreview
and UpdateIndicators with its result. Release final nonnull queried control.
No window-null guard precedes the index check; arbitrary button injection is
not a supported safe input domain.

DoMessage (`0x00646A80`, receiver is **primary+8**, pop 4) obtains message ID
through message slot 4. ID `0x86AD10EE`: if primary byte +27 is set, clear it,
UpdatePreview, UpdateIndicators(result); return AL=1 even when flag was clear.
Other IDs return AL=0 without clearing the flag. Do not apply decompiler field
offsets to the primary receiver for this entry.

DoWinProcMessage (`0x00647000`, receiver **primary+4**, pop 8) ignores its first
word and examines the message pointed to by the second. Word0 must equal 3;
word1 `0x287259F6` forwards word2 to OnButtonActivated and returns AL=1.
Word1 `0x887113A3` is handled (AL=1), including unrecognized word2 values;
other messages return AL=0. For word2=`0x3000`, query root for interface
`0x21325207`; on success take its signed slot 4 integer, multiply in x87 by
float32 `0x3DCCCCCD` (approximately 0.1), store float32 offset +6C, add base
height +68 and store +70, set deferred-update +27=1 (`0x0064711D..13E`). Release
the queried interface if nonnull. For `0x3001`, query the same interface,
conditionally adjust child `0x1000/0x1001` with negated slot 4 word and refresh
layout; no owned altitude store is present. Detailed window-layout behavior is
Partial. Failed queries do not imply a UI cancellation.

## 9. Address and primitive boundary ledger

“Supported” means the stated supported-domain rules, not arbitrary native
inputs or a replacement/shadow verdict. Addresses identify bodies actually
inspected. No whole 41-entry / 8005-byte credit follows from this table.

| Body/family | Status and limits |
|---|---|
| QI `644F40`; IsActive `644FC0`; ref thunks `8DC540/550` | Supported; QI AL and output untouched on miss; shared refcount foreign |
| ctor `645D50`; initializer `645240`; shutdown `645A90`; destructor `645E90` | Supported ordinary sequencing; allocation/exception paths Unknown |
| primary vtable `AA9140`, 17 entries | slots 0..16 identified by export; no inferred extra primary Init slot |
| secondary QI/ref thunks `645E60/70`, `8DCC00/10/30/40/60/70`, deleting `645E80/646240` | Supported receiver/flag/return sequence; shared refcount/heap primitives foreign |
| ID/cursor/pen/directional forwarding `6452E0/2F0/300/7B0` | Supported forwarding only; raw backend results foreign |
| enter/leave `645310/340`; attach `6457C0` | Supported; no ownership/capture inference |
| placement `645370`; abort `645570`; cancel `6457A0` | Supported normal CFG; no owner bool placement result |
| test `645640`; pick `6457E0` | Supported conditional on documented numeric/storage domain |
| preview `646710`; indicators `645980` | Preview orchestration supported; indicator numerical predicate Partial (NUI2) |
| down `645F50`; move `646AC0`; anchor `6460F0` | Press and simple move supported; extra-anchor numerical helper Partial |
| release `646D00`; button `6468B0`; messages `646A80/647000` | Supported branches stated; dialog layout/enumeration Partial |
| LiesOnSegment `645AC0`; direction helpers `645090/5130`; LastDragDirection `645BE0` | Partial, no ordinary geometric/octant rule inferred (NUI1) |
| AddBridgeEntry `646260`, container helpers `645180/5A60/5CA0` | Partial/navigation only; no full body credit |
| base input / shared `DoWinMsg 41D4C0` | Foreign/shared; the latter's AL=0/pop20 stub identified, not an owned UI algorithm |
| GetCost, Reset, SetAnchors, PlaceAllSegments, GetFailure, possible-neighbor query | Single foreign backend primitives; no solve/route/occupant/rule/numeric algorithm covered |
| withdraw, neighbor callback, screen pick, ray-plane, terrain scale/dimensions | Single foreign raw primitives with explicit result/output contracts at caller |
| preview/resource/window/context/sound/string/ref/clock services | Single foreign raw primitives; no whole UI orchestration outcome |

LastDragDirection illustrates why names are insufficient: fewer than three
returned points, or zero difference between last and third-last, yields EAX=-1.
The nonzero path uses signed deltas, x87 FPATAN, a double multiplier, float bias,
**float32 store followed by integer bit-mask 7**, with no integer conversion
(`0x00645C63..C87`). An ordinary nearest-octant interpretation is unproved.

## 10. Proposed unexecuted literal trace ledger

These are independent **PROPOSED, UNEXECUTED** expectations derived from the
above native branches before any portable outputs. Raw primitives supply only
the listed values/outputs. No mock implements the UI owner; no golden trace,
native run, shadow exactness or replacement/default credit is claimed. All
unspecified dependencies are valid and return normally; sound/window absent
unless a row says otherwise. Calls inside each named UI method expand exactly
as specified above, never as a foreign “UI success” result.

| ID | Literal setup/input | Expected state, calls and outputs |
|---|---|---|
| T01 | QI IID `0x22E85D8E`, output sentinel `0x12345678`, receiver R | output R+4; one primary AddRef; AL=1, upper EAX unchecked |
| T02 | QI IID `0xDEADBEEF`, same sentinel | AL=0; output still `0x12345678`; no ref call |
| T03 | ctor then Init(B,6), existing backend/sound null, sound S | initialized=1 before AddRef(B); type=6; AddRef(S); slot48(+84), copy +A8, subscribe(R+8,255); AL=1; no context default assumed |
| T04 | repeat Init(B,3), stored sound S and current global S | no backend/sound ref calls; type=3, config/subscription repeat; AL=1; no active/anchor reset |
| T05 | initialized=1, active=0; Shutdown twice | first unsubscribe, abort no-op, initialized=0; second no unsubscribe; both AL=1 |
| T06 | context null; press `(101,203,7)` | AL=0, no pick/lock/anchor calls |
| T07 | inactive, context valid, raw key16 false, pick `(7,19)` height=32 | +2B=1; active/move=1; bridge FFFFFFFF, altitude -1; increase lock before slot11; anchors `[(7,19),(7,19)]`, SetAnchors; current forced -1/-1 before own move call |
| T08 | same press with raw key16 true, pick false | +2A=0,+2B=0; AL=1; no lock or new anchors; previous active press would first abort |
| T09 | after T07's initial move current `(7,19)`; pick same cell on move | no UpdatePreview/sound/window-manager calls; AL=1 |
| T10 | simple mode +2A=0, anchors `[(7,19),(11,23)]`, current changes to `(14,29)`; slot45 first=1; slot18 true modifies currentCopy to `(13,28)`; failureAL=0; budget sufficient | current=(14,29), final anchor=(13,28); Reset/bridge/altitude/slot21(1)/slot13/slot18, SetAnchors, draft(false,+2B,false,type), slot21(1), GetFailure; preview cost/text processing; no extra-anchor commitment |
| T11 | slot45 first=2, start `(7,19)`, old first `(7,19)`, current `(11,23)`, exactly two anchors | deltas=(4,4), both decrements apply: first=(6,18), before slot18; second anchor updated only on slot18 AL true |
| T12 | finite world `(40.0,9.0,104.0)`, scale=8, width=20, depth=30, pick true, unconstrained | cell=(5,13), then height=9; z store before x; AL=1 |
| T13 | world `(-8.0,9.0,400.0)`, same scale/dimensions | cell=(0,29); AL=1; no symmetric coordinate assumption |
| T14 | pick false, constrain=false, cell sentinel=(91,37), height=123 | AL=0; both output sentinels unchanged; no terrain conversion calls |
| T15 | active=1,+26=1,+27=1, no window; Cancel twice | first clears all three bytes, Reset/16/17, restore terrain, terrain slot9, context44(0)/41(message), decrease lock once; second no calls |
| T16 | active=1, cost word=25, withdraw AL=0 | withdraw int64=25, context result low byte0, active=0; no draft/real/neighbor/intersection; backend17/16, terrain restore, context notification, Reset, decrease lock |
| T17 | active=1, cost word=`0xFFFFFFFB` (-5), withdraw AL=0 | withdrawal amount=4294967291, same unpaid cleanup as T16; UI does not pre-reject or sign-extend |
| T18 | active=1, cost=`0xFFFFFFFF`, withdraw AL=1, draft/real AL=0, possible-neighbor AL=0 | withdraw=4294967295; paid setup/draft/possible-query/setup/real; real neighbor low byte0; both placement returns ignored; intersection query then Reset/decrease; no refund |
| T19 | cost=0, withdraw true; possible=true; GetFailure EAX=`0xF0000001` | callback skipped (AL=1); real pass still called, flag low byte0; paid cleanup |
| T20 | cost=25, withdraw true; possible=true; GetFailure EAX=`0xF0000100`; callback AL=1 | callback occurs (failure AL=0); real neighbor flag low byte1; callback output is not whole UI success |
| T21 | same T20, callback AL=0 | real neighbor low byte0; real pass still occurs; no refund regardless of real result |
| T22 | active=1,+26=1, release; failure AL=1 | +26 cleared before query, optional failure sound then abort/decrease; AL=1; repeated release returns AL=0 |
| T23 | active=1,+26=1, failure AL=0, slot30 AL=0 on release | immediate DoPlacement path, local disposal, AL=1; no dialog primitive |
| T24 | same but slot30 AL=1, window creation AL=0 | active remains1,+26=0; no placement/abort/decrease; local vector disposed; AL=1; later Cancel can balance lock |
| T25 | entry vector `[(42,80.0),(71,120.0)]`, preferred=42, base=80, offset=2; click `0xA1000` | control checks/ref sequence still runs; preferred/base/altitude unchanged; no preview/indicators |
| T26 | same vector/preferred/offset; click `0xA1001` | preferred=71, base=120, altitude=122; optional caption child update, UpdatePreview then UpdateIndicators, final queried-control release |
| T27 | primary+8 message ID86AD10EE, primary+27=1, then repeat | first clears flag before preview/indicator; repeat no update; both AL=1; foreign ID AL=0 leaves flag |
| T28 | primary+4 message `(3,887113A3,3000)`, queried control slot4=20, base height=120 | deferred flag=1; float offset from 20*float32(0.1), stored before altitude sum; ordinary rounding gives offset2, altitude122; release control; AL=1 |

No null-backend initialization “failure return,” query failure with invented
outputs, unchecked empty-anchor execution, or allocation-failure execution is
proposed. Missing numerical-helper/dialog cases are questions, not fabricated
test outcomes.

## 11. Open limits

See [NUI1–NUI3](questions/network-tool-ui.md). Floating control state, overflow,
uninitialized coordinates, unchecked indices, reentry, allocator failure and
exceptions are not modernized or promised to return false. Unsafe actual paths
were not executed. This specification is useful bounded partial documentation;
it makes no all-entry, ABI-export, native-run or runtime-equivalence claim.

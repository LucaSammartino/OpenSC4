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
| initialized, active, move-enabled, release/drag flag | bytes `+24`, `+25`, `+26`, `+27` |
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
in EAX (`0x00645E4E`). Initialization is specified separately below as evidence
is established.

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
A hit writes that pointer and calls primary slot 1 before returning EAX=1. A miss
returns EAX=0 without writing the output. Valid receiver/output storage is a
precondition; the null-receiver branches do not establish a safe null-object API.
AddRef/Release (`0x008DC540/550`) adjust receiver by +C and tail-call the shared
refcount primitives. Their allocation/destruction semantics are not rederived.
IsActive (`0x00644FC0`) loads only AL from byte +25; upper EAX is not a result.

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

## 6. Remaining bounded work

Input move/release, selection, initialization, conversion and bridge-dialog
details require the address-level ledger below before they are treated as
supported. Floating precision/control state, overflow, uninitialized coordinates,
unchecked indices, reentry, allocator failure and exceptions are not implicitly
modernized or promised to return false. Unsafe paths are not executed.

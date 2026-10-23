# Video preferences window

## 1. Scope and evidence

This is a bounded behavior contract for `cSC4VideoPreferencesWinProc` on Windows
1.1.641. The canonical export identifies `SimCity 4.exe`, image base `00400000`,
SHA-256 `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
A read-only SHA-256 measurement of `bin/SimCity4.exe` on 2026-10-23 matches it.
Instruction addresses below refer to that export; instruction bytes govern field
widths, calls and return channels. Local decompilation is navigation only.

[UI inventory §14.3](ui-windows.md#143-panels-data-views-and-dialogs-s5) lists
eight functions / 14,965 bytes and vtable `00ab55f8`; these are inventory figures,
not behavior coverage. [Modal dialogs](modal-dialogs.md) supplies the foreign
window-manager boundary. [Application](application.md) supplies the application
interface boundary. Neither makes this panel's entire behavior supported.

The window owns control mapping and branch order. App apply, persistence, mode
enumeration, rendering, widget graphs and modal loops remain foreign primitives;
their implementations and startup catalogue rebuild timing are outside this spec.
No native execution, preference writes or device operations were performed.

## 2. Raw state and primitive vocabulary

`W` denotes the owner object, `H = *(W+4)` its host, and `P = *(W+8)` the
preference storage used by `SetPreferenceOptions`. Offsets are byte offsets.
The audited ShowDialog prefix retains an existing P or obtains the global
preference pointer; it does not construct a candidate copy (§7). The comparison
words in §3 are independent panel fields. Their complete initialization remains
partial; do not treat them as a saved copy of the preference object.

At `0078f550–0078f698`, host virtual `+0x0c` returns a root pointer. Root virtual
`+0x94` takes `(controlID:u32, interfaceID:u32, output:pointer*)`, returning a
boolean in AL. Interface `0x8810` virtual `+0x20` returns checked state in AL;
virtual `+8` releases a reference. These are callsite types, not claims about the
complete interface. Success is followed by an unconditional output dereference;
success with null output is outside the supported domain. No widget is synthesized.

## 3. Reading checked options into preferences

### 3.1 Ordered byte assignments

`SetPreferenceOptions` is a thiscall entry at `0078f550`. In the following
sequence it repeats root acquisition and lookup for each row. A failed lookup
skips the checked-state call and store. A successful lookup stores only if
checked AL is nonzero. There is no clearing of the destination before these
rows, no enum validation, and no mutually-exclusive-group enforcement in this
sequence. Later checked rows overwrite earlier checked rows targeting the same byte.

| Order | Raw control ID | Interface | Destination | Stored byte | Evidence |
|---|---|---|---|---|---|
| 1 | `6a57da49` | `8810` | `P+f0d` | `1` | `0078f577–0078f5a4` |
| 2 | `6a57da4a` | `8810` | `P+f0d` | `2` | `0078f5c6–0078f5f3` |
| 3 | `6a57da4b` | `8810` | `P+f0d` | `3` | `0078f615–0078f642` |
| 4 | `6a57da4c` | `8810` | `P+f0e` | `1` | `0078f664–0078f691` |
| 5 | `6a57da4d` | `8810` | `P+f0e` | `2` | `0078f6b3–0078f6e0` |
| 6 | `6a57da4e` | `8810` | `P+f0e` | `3` | `0078f702–0078f72f` |
| 7 | `6a57da4f` | `8810` | `P+f0f` | `1` | `0078f751–0078f77e` |
| 8 | `6a57da50` | `8810` | `P+f0f` | `2` | `0078f7a0–0078f7cd` |
| 9 | `6a57da51` | `8810` | `P+f0f` | `3` | `0078f7ef–0078f81c` |

The local interface pointer starts null. Before replacing it, a nonnull prior
pointer is cleared locally and released. For successive rows the next root is
obtained **before** the previous interface is released (`0078f5ab–0078f5c6`,
`0078f5fa–0078f615`). Do not coalesce these lookups or reorder the release ahead
of the host callback. No root AddRef/Release is established by this slice.

### 3.2 Remaining readback order and change marker

Continue §3.1 with the following operations, in table order. Unless identified
as a utility, each row uses the same root/lookup `8810`/checked-AL pattern.
Lookup failure leaves that row's preference fields unchanged. A successful
unchecked radio leaves them unchanged too. A successful unchecked checkbox
performs its listed zero assignment. The table calls them radio/checkbox only
to describe these measured policies, without claiming resource widget classes.

`C = byte(W+1a)` is a sticky marker here: this function can set it to 1 but
never clears it. `Bxx` denotes `u16(W+xx)`, compared at that width unless stated.
No row tests whether a control is enabled. This does not establish that a native
disabled widget generates events.

| Order | Control(s) | Readback effect | Marker condition after selected store | Evidence |
|---|---|---|---|---|
| 10–12 | `2ab6c373`, `2ab6c374`, `2ab6c375` | If checked, `u8(P+f16)=1,2,3` respectively; last checked wins | `B16 != 0,1,2` respectively | `0078f83e–0078f92c` |
| 13 | `ab81ded5` through `ab81ded7`, ascending | First successful checked lookup writes `u8(P+f28)=index+1` (1–3), then exits this loop; none checked preserves byte | None | `0078f930–0078f987` |
| 14–16 | `0b85cf92`, `2b85cf9f`, `2b85cfaa` | If checked, `u8(P+f10)=1,2,3`; last checked wins | None | `0078f98d–0078fa73` |
| 17 | `6a57da57` | Checked: `u8(P+f13)=1` | If `B12!=1`, set C then `u8(P+f0c)=1` | `0078fa95–0078fada` |
| 18 | `6a57da58` | Checked: `u8(P+f13)=0` | If `B12!=0`, set C then `u8(P+f0c)=1` | `0078fb02–0078fb43` |
| 19 | `6a57da54` | Utility toggle result normalized to 0/1 into `u8(P+f11)` | None | `0078fb4a–0078fb68` |
| 20 | `6a57da55` | Successful lookup: `u8(P+f12)=checked ? 1 : 0` | None | `0078fb89–0078fbc2` |
| 21 | `6a57da56` | Successful lookup: `u8(P+f15)=checked ? 1 : 0` | Checked: `B1c!=0`; unchecked: `B1c!=1` | `0078fbe4–0078fc2e` |
| 22 | `6a57da60` | Successful lookup: `u8(P+f14)=checked ? 1 : 0` | Checked: `B10!=1`; unchecked: `B10!=0` | `0078fc4d–0078fc97` |
| 23 | `0a8b7e00` | Utility toggle result normalized to 0/1 into `u8(P+f18)` | None | `0078fc9b–0078fcb9` |
| 24 | `6a57da5a` | Checked: `u32(P+f1c)=800`, then `u32(P+f20)=600` | `B18!=0` | `0078fcda–0078fd24` |
| 25 | `6a57da5b` | Checked: `u32(P+f1c)=1024`, then `u32(P+f20)=768` | `B18!=1` | `0078fd43–0078fd8f` |
| 26 | `6a57da5c` | Checked: `u32(P+f1c)=1280`, then `u32(P+f20)=1024` | `B18!=2` | `0078fdae–0078fdf5` |
| 27 | `6a57da5d` | Checked: `u32(P+f1c)=1600`, then `u32(P+f20)=1200` | `B18!=3` | `0078fe15–0078fe60` |
| 28 | `2ab6c376` | Checked: `u32(P+f24)=16` | zero-extended `B14 != u32(P+f24)` after store | `0078fe7f–0078fec5` |
| 29 | `2ab6c377` | Checked: `u32(P+f24)=32` | zero-extended `B14 != u32(P+f24)` after store | `0078fee4–0078ff2a` |

For the two utility rows, the window calls `009afebc` with
`(root:pointer, controlID:u32)` as stack arguments and caller stack cleanup;
only returned AL is consumed, then normalized with a nonzero test. This spec
does not give that foreign utility a missing-control success policy. The
current retained lookup interface remains alive across each utility call;
the following root acquisition precedes its release.

The final nonnull interface is released at `0078ff2e–0078ff3b`. This entry has
no specified result value (bare return, no uniform EAX assignment); its outputs
are the stores and callbacks above. No float conversion, clamp, string parsing,
mode-list enumeration, candidate copy or application call occurs in this body.
All nine initial assignments and all remaining table operations have been
instruction-reviewed through the normal return at `0078ff3f`. Baseline words
are not updated by this entry; they are inputs, not inferred preference copies.

## 4. Independent distinguishing traces

All vectors are **PROPOSED / UNEXECUTED**. Expected values are analytic native
branch consequences, not portable outputs or native execution results. All
successful lookups below return valid interface pointers; checked results are
literal AL bytes. Each vector currently stops after row 4 of §3.1.

| Vector | Initial bytes `(P+f0d,P+f0e)` | Lookup/checked AL for rows 1–4 | Expected bytes |
|---|---|---|---|
| VP-T1 | `(7,9)` | success/1, success/0, success/0, success/0 | `(1,9)` |
| VP-T2 | `(7,9)` | success/1, success/2, success/255, success/1 | `(3,1)` |
| VP-T3 | `(7,9)` | failure, success/0, failure, success/0 | `(7,9)` |

For VP-T2 the ordered effects are root; lookup `49`; checked; byte store 1;
root; release `49`; lookup `4a`; checked; byte store 2; root; release `4a`;
lookup `4b`; checked; byte store 3; root; release `4b`; lookup `4c`; checked;
byte store 1. Full IDs are in §3.1. No modal-result or EAX assertion is made by
these truncated traces. Failure returning a nonnull output, success returning
null, callback reentrancy, and invalid `P` are unsupported qualification cases.

### 4.1 Full readback vectors

These additional vectors are also **PROPOSED / UNEXECUTED**, authored from
§3's branches. Foreign callbacks in these vectors return the stated values
and have no other side effects. All roots, H and P are valid. Lookup failures
return AL=0 and leave the already-null output null; successes return AL=1
and a distinct valid retained interface reference. Utility responses are
literal AL bytes, independent of widget lookup behavior.

**VP-T4, asymmetric complete readback.** Initial preference bytes at offsets
`f0c,f0d,f0e,f0f,f10,f11,f12,f13,f14,f15,f16,f18,f28` are respectively
`0,7,8,9,10,11,12,13,14,15,16,18,28`; initial u32 values at
`f1c,f20,f24` are `1111,777,24`. Initial panel words
`B10,B12,B14,B16,B18,B1c` are `1,1,32,1,1,1`; C=0. All lookups succeed.
Provide checked AL bytes in the following literal order:

| Raw ID sequence | Checked AL sequence |
|---|---|
| `6a57da49,6a57da4a,6a57da4b` | `0,1,0` |
| `6a57da4c,6a57da4d,6a57da4e` | `0,0,255` |
| `6a57da4f,6a57da50,6a57da51` | `1,0,0` |
| `2ab6c373,2ab6c374,2ab6c375` | `0,2,0` |
| `ab81ded5` | `1`; no lookup of `ab81ded6` or `ab81ded7` |
| `0b85cf92,2b85cf9f,2b85cfaa` | `0,1,1` |
| `6a57da57,6a57da58` | `1,0` |
| `6a57da55,6a57da56,6a57da60` | `0,1,0` |
| `6a57da5a,6a57da5b,6a57da5c,6a57da5d` | `0,1,0,0` |
| `2ab6c376,2ab6c377` | `0,1` |

Utility `6a57da54` returns AL=255; utility `0a8b7e00` returns AL=0.
Expected final bytes in the initial offset order are
`0,2,3,1,3,1,0,1,0,1,2,0,1`; final u32 values are `1024,768,32`.
All six baseline words remain exactly `1,1,32,1,1,1`. C becomes 1 at the
`6a57da56` comparison even though its stored preference is true: that row
compares the saved word against zero. The `6a57da60` comparison also sets C.
The initial `P+f0c=0` remains 0; C and that preference byte are distinct.
`P+f10` is stored first as 2 then as 3. The width/height stores are ordered
1024 then 768. No other preference bytes are modified by this body.

The callback trace is the §3 row order, with loop row 13 shortened to its
first successful checked ID. For each lookup row: acquire root; clear/Release
the preceding retained interface if present; lookup; checked; conditional
store(s)/marker updates. For utility rows: acquire root; utility; normalized
byte store, retaining the preceding lookup reference. Release the last
reference exactly once at function exit. No save/apply/notification occurs.
EAX/AL has no asserted readback return value.

**VP-T5, lookup failures preserve raw domains.** Use exactly VP-T4's initial
storage and words, but every `8810` lookup fails and both utility results
are AL=0. All three `ab81ded5–d7` lookups are attempted. Only preference
bytes `f11` and `f18` change to 0; every other listed byte/dword and all
baseline words retain the initial literals. C stays 0. There are no interface
Release calls because all outputs stay null. No native available-mode list
is an input to this readback contract.

**VP-T6, sticky marker without new changes.** Repeat VP-T5 with initial C=1.
Expected C=1; unchanged bytes do not clear it. This distinguishes baseline
comparisons from a whole-preference equality test.

**VP-T7, ordered resolution overwrite.** Use VP-T5 failure responses except
all four `6a57da5a–5d` lookups succeed and return checked AL=1; B18=3.
Expected width/height stores are `(800,600)`, `(1024,768)`, `(1280,1024)`,
`(1600,1200)`, in that order. Final pair is `(1600,1200)` and C=1: earlier
rows differ from B18 even though the final selected row agrees. This is
multiple checked controls, not proof about duplicate enumerated modes.

### 4.2 Event, lifetime and output-channel vectors

All vectors remain **PROPOSED / UNEXECUTED**. Unless an invalid domain is
explicitly listed, foreign operations succeed and have no unstated effects.
An event message is the literal three-u32 record `{3,287259f6,ID}`. Source is
the host's valid root, source `+11c` returns AL=1. The raw message-entry
return asserted here is AL only. No sentinel upper EAX value is expected.

| Vector | Literal input / state | Expected owner effects and output |
|---|---|---|
| VP-T8 | Message `{2,287259f6,6a57da48}` | Root acquisition; source `+11c`; no helper/EndDialog; false AL |
| VP-T9 | Message `{3,0,6a57da48}` | Root acquisition; source `+11c`; no helper/EndDialog; true AL |
| VP-T10 | Message `{3,287259f6,6a57da52}` | Root acquisition; source `+11c`; no helper; true AL (unadmitted gap ID) |
| VP-T11 | Message `{3,287259f6,6a57da48}`; guard=1, owner count=4; root `+18` returns null | EndDialog: owner AddRef, host root acquisition, root `+18`, clear H, Release H, guard=0, owner Release; helper true AL and message true AL. All P and baseline values unchanged. A later EndDialog on the live object makes no callbacks |
| VP-T12 | ID `ea57da59`; §3 responses VP-T5, initial C=0; guard=0; valid A/N/message | Readback (only utility-byte stores), A AddRef, SavePreferences, ApplyVideoPreferences(`P+f0c`), A Release, allocate 44, construct message, message AddRef, message `+14(231bbf91)`, N `+10(message,0)`, EndDialog guard-zero path, message Release, true AL. Even unchanged settings take the foreign save/apply path |
| VP-T13 | ID `ea57da59`; VP-T5 readback, initial C=1 | Readback; raw widget chain `2a57cb83:+110(1,1)`, then `2a57cb84:+110(1,0)`; true AL. No A call, message allocation, notification or EndDialog; C remains 1 |
| VP-T14 | ID `ea57da6f`; VP-T5 readback, initial C=1; guard=0; valid A/N/message | Same complete save/apply/notification/EndDialog sequence as VP-T12. C does not suppress it and stays 1 |
| VP-T15 | ID `ea5e99d9`; A=null, P otherwise VP-T4 initial | True AL; no AddRef, Auto, redraw, save, apply, notification, EndDialog or preference store |
| VP-T16 | ID `ea5e99d9`; valid A, P=null | A AddRef, A Release, true AL; no Auto or redraw; P stays null |
| VP-T17 | ID `ab81ded6`; all three group lookups succeed, initial selected states `1,0,1` | For IDs `d5,d6,d7`: clear, select, clear requests; fresh root before each local clear/Release; final interface Release; true AL. Repeat event gives identical callbacks, irrespective of the resulting selection. No P/baseline/C writes |
| VP-T18 | SetButtonsBasedOnPreferences; P=null; `W+1e=255` | No host/control callbacks, `W+1e=0`, no defined return register assertion |
| VP-T19 | Redraw first-group slice; `u8(P+f0d)=2`; `49` lookup fails, `4a,4b` succeed | No request for `49`; select `4a`, clear `4b`; repeated slice repeats both. Invalid enum 255 instead makes no lookups for any of the three; no synthetic selection |
| VP-T20 | QI unknown ID `2`, output initially `12345678` | False AL; output still `12345678`; no AddRef |
| VP-T21 | Release count initially 3, valid live owner | Store count=2; EAX=2; no Shutdown/free. Separately, still-live Shutdown twice returns true AL both times and leaves pointer `W+8=null` |
| VP-T22 | DoModalWin, H=null | No foreign calls; true AL; no conventional modal-result output |
| VP-T23 | ShowDialog selector=1; `00b4dfd5=1`, guard=0; valid existing H | False AL; H unchanged; no host Release, script creation or redraw |
| VP-T24 | ShowDialog selector=1; `00b4dfd5=1`, guard=1; valid existing H | True AL; H unchanged; no selector-table rejection or host callbacks |

For VP-T11 the count is deliberately above destruction threshold; the exact
AddRef implementation's arithmetic is not asserted. Release's own supported
count behavior is separately tested in VP-T21. For redraw VP-T19 the vector
ends at `0079020d`; it does not assert later controls or whole-method output.

**Qualification cases with no successful expected fallback:** valid A/P Auto
may supply raw defaults, but their literal values and complete subsequent
selection are unqualified until VP1/VP2 are resolved. A default-then-close
sequence is therefore not a claimed complete output vector. Empty, duplicate,
equal, absent/current or unsupported native modes and unavailable drivers have
no golden selection outcome here. Neither a failed required raw widget lookup
nor successful `8810` lookup with null output, null A on a save path, failed
message allocation, absent modal S/manager, invalid output pointer, callback
reentrancy or arbitrary upper EAX is converted into successful no-op behavior.
There is no host-f64 or x87 numerical golden: the supported mappings use
integer/byte operations only. No portable implementation or mock whole-window
result was used to author these traces.

## 5. Message admission and lifetime

### 5.1 Owner message entry (supported)

`DoWinProcMessage 00792f50` is thiscall, two stack arguments, callee pops 8.
Arguments are `source:pointer`, `message:pointer`. It gets the host root before
testing whether source equals it. A different source returns false in AL. An
equal source must return nonzero AL from virtual `+11c`, otherwise false.
Next it reads `u32(message+0)`: anything other than 3 returns false. No
message pointer null check precedes that read. For type 3, every path returns
true AL, including an unrecognized subtype or control ID. Upper EAX bits are
not a boolean result contract (`00792f50–0079306a`).

Subtype `u32(message+4)=287259f6` admits the following `u32(message+8)` IDs to
`HandleControlMessage(this, ID, source)`, in that stack argument order:

- `0a8b7e00`, `0b85cf92`, `2b85cf9f`, `2b85cfaa`;
- inclusive `2ab6c373–2ab6c377`, `6a57da48–6a57da51`, `6a57da54–6a57da58`,
  `6a57da5a–6a57da5d`, `ab81ded5–ab81ded7`;
- `6a57da60`, `ea57da59`, `ea57da6f`, `ea5e99d9`.

No other message fields are read, including any button/down-state argument.
The helper's return value is discarded and AL is forced to 1. Admission tests
source's `+11c` but does not query the target control's enabled state. All ID
comparisons defining ranges are unsigned (`00792f98–00793058`).

### 5.2 EndDialog (supported for nonreentrant valid objects)

At `0078ff40`, if global byte `00b4dfd4` is zero, return without host callbacks
or owner AddRef. Otherwise:

1. Call owner virtual `+4` (AddRef).
2. If H is nonnull, get its root; call root `+18` to obtain parent. There is no
   null-root check. If parent is nonnull, call root `+10c(0x1000)`. If AL is
   nonzero, call parent `+24(15,root,1)`; then, regardless of that AL, call
   parent `+5c(root)`. No parent reference acquisition/release is visible here.
3. Read H again. If nonnull, clear `W+4` to null **before** H `+8` Release.
4. Set global byte `00b4dfd4=0`; tail-dispatch owner `+8` Release.

Evidence: `0078ff4c–0078ffb8`. This entry does not call App apply or write
preference bytes. It does not explicitly set a dialog-result value. With the
guard zero, AL initially contains zero; with the guard set, the tail Release
governs EAX. It is not a uniform boolean API. Calling EndDialog again on a
still-live object after completion takes the guard-zero path. Reentrant
callbacks and use after the tail Release destroys the object are unsupported.

### 5.3 Shared slots and Release

The actual `00ab55f8` table has eight slots: QI `004f1f30`, AddRef `004664a0`,
Release `00792f10`, message `00792f50`, foreign DoWinMsg `0041d4c0`, ShowDialog
`00791490`, Shutdown `0076aaa0`, SetPreferenceOptions `0078f550`. The eight
inventory bodies are not these eight slots: three principal owner methods and
EndDialog/DoModalWin are direct-call helpers. Do not count folded code twice.

The table positively attributes shared QI and Shutdown to this owner. QI
accepts raw IDs `1` and `22e85d8e`, writes this to the caller's pointer output,
calls owner AddRef, then returns true AL; other IDs return false AL without
writing the output (`004f1f30–004f1f52`). A null output for accepted IDs is
outside its supported domain. Shutdown only clears `W+8` (32 bits), returns
true AL, and performs no release (`0076aaa0–0076aaa9`). Repeated Shutdown on
a still-live object repeats that store. Upper EAX is unspecified for both.

Release reads `u32(W+0c)`. If unsigned count >1 it decrements, stores and
returns the new count in EAX. Otherwise (including zero), it calls virtual
Shutdown, resets the vptr to `00ab55f8`, releases H if nonnull, frees W through
`005e5620`, and returns EAX=0 (`00792f10–00792f46`). It does not first decrement
the count in the destruction branch. After free no repeated-call domain is
claimed. Preference storage is not freed by this body. AddRef and allocation/
constructor semantics remain unqualified; no concurrency guarantee is implied.

## 6. Qualified redraw and action contracts

### 6.1 SetButtonsBasedOnPreferences: qualified slices

The thiscall entry `0078ffc0` first checks P. If null, it skips all control
operations and clears `u8(W+1e)=0`, then returns without a defined EAX result.
For nonnull P, its first group reads `u8(P+f0d)` exactly once. Values 1, 2, 3
produce the following requests, always in ID order `6a57da49,4a,4b`:

| Raw byte | Interface `8810` virtual calls after successful lookup |
|---|---|
| 1 | `+18`, `+1c`, `+1c` |
| 2 | `+1c`, `+18`, `+1c` |
| 3 | `+1c`, `+1c`, `+18` |
| Any other byte, including 0 or 255 | No lookup or request for this group |

Here `+18` is the owner's selected-state request and `+1c` its clear-state
request; neither takes a stack argument and their results are ignored. The
foreign control must implement the requested state change. Actual widget
notifications/redraws caused by those primitives are not specified here.
Evidence covers every branch of this first group (`0078ffd6–0079020d`). Each
control gets a fresh root and `+94(ID,8810,&local)` lookup. Lookup failure skips
that request; success with null output is unsupported. As in §3, the next root
is acquired before clearing/releasing the previous interface. No enable-state
test guards these requests. Repeated redraw of this group repeats the requests
even if the existing selected state already agrees; no equality fast path exists.

**Later-body slice, only on reaching `00791201`:** read `u32(P+f24)` once.
For 16, request `+18` on `2ab6c376` then `+1c` on `2ab6c377`; for 32, request
`+1c` then `+18`. Other values leave both controls untouched. Each lookup uses
the same lifecycle and AL gate. Afterward, if `u8(W+1e)!=0`, copy the low
16 bits of current `P+f24` to `u16(W+14)`. Release the final nonnull interface,
then clear `W+1e` to zero (`00791201–00791350`). No conclusion about the
intervening groups, driver availability or resolution-selection rules follows
from these two slices. This method remains Partial.

### 6.2 Primitive boundary types for actions

`A = pointer at 00b43c94` is the application interface; `N = pointer at
00b43ccc` is the notification service. The owned action callsites use:

| Primitive | Callsite type / consumed output |
|---|---|
| A `+4`, `+8` | thiscall AddRef / Release, no args; result ignored |
| A `+44` | thiscall SavePreferences, no args; result ignored |
| A `+4c` | thiscall ApplyVideoPreferences, one pointer to `P+f0c`; result ignored |
| A `+50` | thiscall GetAutoVideoPreferences, one pointer to `P+f0c`; result ignored |
| root `+2c`, returned object's `+8c(ID)` | pointer-returning foreign operations; no null checks in qualified action path |
| returned widget `+110(u32,u32)` | request with literal arguments below; result ignored |
| message `+14(u32)` | notification-ID assignment, result ignored |
| N `+10(message:pointer,0:u32)` | notification operation; result ignored |

The names Save/Apply/Auto are prior application interface names, not new
research into those capped bodies. Their return types beyond the consumed
channels, persistence timing, failure policies, renderer effects, defaults and
any mutation of the input structure remain foreign unknowns. In particular,
there is no promise that SavePreferences persists the later Apply result.

The notification path requests 44 bytes from the allocator and calls
`009134d6` to construct a message. On normal success it AddRefs the constructed
object, sets ID `231bbf91`, calls N `+10(message,0)`, calls EndDialog, then
Releases the message. Allocation/constructor null flows reach a dereference,
so allocation failure is **not** a successful no-op contract. Valid A, N and
message objects are requirements for supported save paths.

The shared reference helpers at these owner callsites have misleading exported
class names. `004f20c0` stores the incoming pointer locally, AddRefs it if
nonnull, and returns the local address; `004f20a0` clears/releases a nonnull
prior pointer and returns its local address. These are reference primitives,
not lot-list or vector algorithms. Their 24-byte bodies establish the single
AddRef/Release effects consumed here; no foreign window algorithm is delegated.

### 6.3 Action IDs and exact order

`HandleControlMessage` is thiscall with two stack arguments `(ID:u32,
source:pointer)`, callee pops 8. The decompilation's one-argument prototype is
not authoritative; dispatcher pushes and terminal returns prove two arguments
(`00793054–00793058`, `00792ec2–00792ec7`). These qualified actions do not
consume the second argument and return true in AL, with upper EAX unspecified.

| ID | Normal owned chronology | Evidence |
|---|---|---|
| `6a57da48` | EndDialog; return true. No readback, save, apply or rollback in this branch | jump-table entry `00792ec8=007921f3`; `007921dd–00792200` |
| `ea57da59` | Owner virtual `+1c` (normally §3 readback), then inspect C. If nonzero, run the two-widget requests below and return, keeping dialog open. If zero, AddRef A, SavePreferences, ApplyVideoPreferences(`P+f0c`), Release A, notification path above including EndDialog, then message Release | `00792bf0–00792d10` |
| `ea57da6f` | Owner virtual `+1c` readback; AddRef A if nonnull; SavePreferences; ApplyVideoPreferences(`P+f0c`); Release A; notification path including EndDialog; message Release. C does not gate this branch | `007925ef–007925fb`, `00792df0–00792e82` |
| `ea5e99d9` | If A null, return true. Otherwise AddRef A. If P nonnull, GetAutoVideoPreferences(`P+f0c`), then SetButtonsBasedOnPreferences. Release A; return true. No readback/save/apply/notification/EndDialog in this branch | `00792e84–00792ec7` |

The `ea57da59` C-nonzero branch obtains fresh root, root `+2c`, its
`+8c(2a57cb83)` widget, then requests widget `+110(1,1)`. It repeats that
chain for `2a57cb84`, requesting `+110(1,0)`. No interface acquisition or
release is visible on these raw widget chains. There is no null handling;
missing required widgets are unsupported. The requests happen in that order
(`00792c22–00792c80`). Their concrete visibility/enabled meaning is left raw.
The initial ShowDialog tail requests the opposite pair: `83:+110(1,0)` then
`84:+110(1,1)` (`00791d1d–00791d65`).

Thus a change-marker confirmation branch and an unconditional save/apply/close
branch are distinct. There is no preference-structure equality comparison in
these action branches. C is a sticky comparison result from selected §3 rows;
even C=0 goes through save/apply. Close does not roll back bytes already changed
by readback or the foreign Auto primitive. The owner does not implement an
independent temporary preference object in these branches. User-facing labels
for these raw resource IDs remain unproved; do not silently equate them with
standard OK/Apply/Cancel conventions.

### 6.4 One qualified selection-event group

For ID in `ab81ded5–ab81ded7`, the branch visits all three IDs in ascending
order. For each successful `8810` lookup, it requests `+18` if that row's ID
equals the event ID, otherwise `+1c`. Each next root precedes local pointer
clear/Release, and the final nonnull pointer is released before returning true
AL (`00792d13–00792deb`, shared tail `00792b03–00792b1f`). Preferences and
baseline words are not written in this branch. A repeated event repeats every
request. Enabled-state and existing selection are not consulted. A failed
lookup skips that request without preventing later lookups; it does not create
a control. This is a bounded event contract, not coverage of other admitted IDs.

## 7. Initialization and modal call boundary

### 7.1 ShowDialog prefix and tail (Partial)

Thiscall `00791490` takes one raw u32 selector, callee pops 4. It lazily sets
global byte `00b4dfd5=1` and guard `00b4dfd4=0` on its first call. On later
calls with the guard already nonzero it returns true AL immediately, before
selector validation or any host replacement (`00791490–007914e8`). There is
one 12-byte record at `00ab55e8`: `{selector=0, resource=8a7e052f,
thirdWord=2a57cb82}`. The loop's exclusive end is `00ab55f4`. Other selectors
return false AL when the guard did not short-circuit. Those three literal
words and the action jump table were read from the hash-matching PE data.

For selector zero: clear/release any existing H; construct the three-word
resource key `{0,96a006b0,8a7e052f}`; call the foreign
`SC4CreateWindowFromScript` (`005f9480`) with stack arguments
`(&key,0,2a57cb82,5386d516,&H)`, caller cleanup. False AL exits to the common
false-AL return without the later initialization. On success, obtain root then
root `+2c` result D. If D exists, get two u32 values from D `+a4` and `+a8`;
if absent use 640 and 480. Reacquire root for `+cc(first)` then reacquire it
for `+d0(second)`. These dimension-like values do not prove desktop/mode
enumeration or a driver selection policy (`007914eb–007915ac`).

If P is null, assign the raw global pointer `00b43cd8` to `W+8`; otherwise
retain P unchanged. No AddRef, candidate allocation or preference-object copy
occurs in this prefix. Set guard=1; get root once (return unused); call host
`+1c(this)`; then call SetButtonsBasedOnPreferences. Next, the three reviewed
binding sequences look up raw root `+8c` widgets `ea57da6f`, `ea57da59`,
`6a57da48`; each widget is dereferenced before another host-root acquisition,
then widget `+158(root)` is called. There is no null fallback for those
widgets (`007915b2–00791677`).

The middle binding sequences are not yet instruction-qualified here. The
reviewed tail makes the §6.3 initial two-widget requests, then **returns false
AL even after successful initialization** (`00791d1d–00791d6f`). Upper EAX is
unspecified. It neither calls DoModalWin nor sets a conventional modal result
in that tail. No end-to-end initialization success credit is claimed until
the intervening binding sequence is qualified.

### 7.2 DoModalWin (Supported normal foreign-success domain)

Thiscall `007913a0` has no stack arguments and returns true AL on every normal
return, including null H; upper EAX is unspecified. For null H it performs no
pause, framework or window-manager calls. For nonnull H, the exact order is:

1. Construct a local auto-pause token via `005f9320` with arguments `(0,0,1,1)`.
2. Initialize local service S=null; call RZGetFramework. If nonnull, call its
   `+14(a417445e,5a4,&S)`; returned AL/EAX is not tested. The subsequent
   dereference requires S nonnull even if framework was absent. Call S `+90()`
   and retain returned raw 32-bit value F.
3. Get host root R; AddRef R if nonnull. Call service at `00b43ce0`
   `+28(R,1)`, then S `+94(R)`.
4. Call RZ::WM, then manager `+a4(R)` (foreign modal operation). The owner's
   next step ignores the manager's returned modal result.
5. Call `00b43ce0` service `+18(R)`, then S `+60(F)`. Only if that AL is
   nonzero, call S `+94(F)`.
6. Call EndDialog. Release R if nonnull. Release S through `004177f0` on its
   nonnull local pointer, then destroy auto-pause token via `005f9360`.
7. Return AL=1.

Evidence: full normal body `007913a0–0079148c`; reference cleanup primitive
`004177f0–00417839`. S/F are kept distinct from the root and manager. The
window owns this sequence, not the pause implementation, focus policy or modal
loop. A missing required service/manager is not a proven graceful failure.
No preference readback/save/apply is performed directly by this method; nested
events may perform §6 actions while the foreign modal operation runs.

## 8. Coverage ledger

| Entry | Status | Qualified scope |
|---|---|---|
| `SetPreferenceOptions 0078f550`, 2,544 bytes | Supported | Complete normal body, §3; valid foreign outputs required |
| `SetButtonsBasedOnPreferences 0078ffc0`, 5,009 bytes | Partial | First enum group, null-P path, final depth group and cleanup only (§6.1) |
| `HandleControlMessage 00791d70`, 4,440 bytes | Partial | Four action IDs and one three-ID selection group (§6); remaining groups unqualified |
| `ShowDialog 00791490`, 2,272 bytes | Partial | Prefix, first three bindings and tail; middle binding sequence unqualified (§7.1) |
| `DoModalWin 007913a0`, 237 bytes | Supported | Full normal caller-owned sequence; foreign services must succeed (§7.2) |
| `EndDialog 0078ff40`, 125 bytes | Supported | §5.2; nonreentrant valid objects |
| message `00792f50`, 283 bytes; `Release 00792f10`, 55 bytes | Supported | §5; raw AL versus EAX distinguished |
| shared `004f1f30`, 35 bytes; `0076aaa0`, 10 bytes | Supported | §5.3; actual owner vtable dispatch, no duplicate coverage claim |
| App apply/persistence, renderer, mode enumeration, widget/modal loop | Foreign | Callsite-only boundary; no implementation credit |

The eight separately inventoried owner bodies above sum to 14,965 bytes.
That arithmetic independently matches the existing inventory; it does not
convert Partial methods or unreviewed bytes to Supported. Shared slots are
additional attribution facts, not extra owned-function coverage. Constructor,
allocation caller and AddRef semantics remain Unknown. This publication is
usable for the supported readback/action boundaries and literal vectors, not
a complete portable replacement panel. Open gaps are tracked in
[video preference questions](questions/video-preferences.md).

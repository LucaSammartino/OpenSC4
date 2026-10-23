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
Storage lifetime and whether this is a retained original or candidate are not
yet established here. Do not infer ownership from the field name.

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

## 6. Coverage ledger

| Entry | Status | Qualified scope |
|---|---|---|
| `SetPreferenceOptions 0078f550`, 2,544 bytes | Supported | Complete normal body, §3; valid foreign outputs required |
| `SetButtonsBasedOnPreferences 0078ffc0`, 5,009 bytes | Unknown | No behavior credit yet |
| `HandleControlMessage 00791d70`, 4,440 bytes | Unknown | No behavior credit yet |
| `ShowDialog 00791490`, `DoModalWin 007913a0` | Unknown | Owner chronology pending |
| `EndDialog 0078ff40`, 125 bytes | Supported | §5.2; nonreentrant valid objects |
| message `00792f50`, 283 bytes; `Release 00792f10`, 55 bytes | Supported | §5; raw AL versus EAX distinguished |
| shared `004f1f30`, 35 bytes; `0076aaa0`, 10 bytes | Supported | §5.3; actual owner vtable dispatch, no duplicate coverage claim |
| App apply/persistence, renderer, mode enumeration, widget/modal loop | Foreign | Callsite-only boundary; no implementation credit |

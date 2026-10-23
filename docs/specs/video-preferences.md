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

### 3.1 Ordered byte assignments (supported slice)

`SetPreferenceOptions` is a thiscall entry at `0078f550`. In the following
sequence it repeats root acquisition and lookup for each row. A failed lookup
skips the checked-state call and store. A successful lookup stores only if
checked AL is nonzero. There is no clearing of the destination before these
rows, no enum validation, and no mutually-exclusive-group enforcement in this
slice. Later checked rows overwrite earlier checked rows targeting the same byte.

| Order | Raw control ID | Interface | Destination | Stored byte | Evidence |
|---|---|---|---|---|---|
| 1 | `6a57da49` | `8810` | `P+f0d` | `1` | `0078f577–0078f5a4` |
| 2 | `6a57da4a` | `8810` | `P+f0d` | `2` | `0078f5c6–0078f5f3` |
| 3 | `6a57da4b` | `8810` | `P+f0d` | `3` | `0078f615–0078f642` |
| 4 | `6a57da4c` | `8810` | `P+f0e` | `1` | `0078f664–0078f691` |

The local interface pointer starts null. Before replacing it, a nonnull prior
pointer is cleared locally and released. For successive rows the next root is
obtained **before** the previous interface is released (`0078f5ab–0078f5c6`,
`0078f5fa–0078f615`). Do not coalesce these lookups or reorder the release ahead
of the host callback. No root AddRef/Release is established by this slice.

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

## 5. Coverage ledger

| Entry | Status | Qualified scope |
|---|---|---|
| `SetPreferenceOptions 0078f550`, 2,544 bytes | Partial | §3.1 first four lookups and stores only |
| `SetButtonsBasedOnPreferences 0078ffc0`, 5,009 bytes | Unknown | No behavior credit yet |
| `HandleControlMessage 00791d70`, 4,440 bytes | Unknown | No behavior credit yet |
| `ShowDialog 00791490`, `DoModalWin 007913a0`, `EndDialog 0078ff40` | Unknown | Owner chronology pending |
| message `00792f50`, `Release 00792f10` | Unknown | Dispatch and lifetime pending |
| shared `004f1f30`, `0076aaa0` | Unknown | Owner attribution and repeated shutdown pending |
| App apply/persistence, renderer, mode enumeration, widget/modal loop | Foreign | Callsite-only boundary; no implementation credit |
